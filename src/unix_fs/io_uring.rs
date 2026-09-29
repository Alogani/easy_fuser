//! Linux io_uring implementation for the async file-descriptor presets.
//!
//! This module is enabled by the opt-in `io_uring` feature, which also enables
//! the `async` feature. BSD/macOS continue using the existing synchronous
//! implementations.
//!
//! The async descriptor presets use io_uring for positioned `read` and `write`,
//! `flush`, `fsync`, and (for writable presets) `fallocate`. `Current` and `End`
//! offsets are resolved with synchronous `lseek` before submission. Path and
//! metadata operations, locks, `ioctl`, `lseek`, and `copy_file_range` remain
//! synchronous because their behavior or lifetime rules need separate work.
//!
//! Each request currently creates one ring, submits one operation, waits for its
//! completion, and then drops the ring. The request also uses Tokio task,
//! `AsyncFd`, and `oneshot` machinery. That makes the current implementation
//! expected to lose throughput against an ordinary async syscall for cheap I/O;
//! it does not show that io_uring itself is inherently slower. In the local
//! warm-file benchmark, the current implementation measured about 3–10x lower
//! throughput than async syscalls (see `benches/README.md`). It gets none of the
//! ring reuse or batching benefits of a persistent ring.
//!
//! Before treating io_uring as a performance optimization, use a persistent
//! ring—such as one owned by each executor worker or a dedicated ring driver
//! that routes multiple in-flight submissions and completions—and rerun the
//! benchmark. Warm cached 4 KiB reads particularly favor a plain `pread`, while
//! cold storage or genuinely concurrent I/O may have different results. The
//! host kernel or seccomp policy may reject ring creation; such errors are
//! returned without falling back to synchronous syscalls. Operations can also
//! be punted to kernel io-wq.
//!
//! The request task owns its read/write buffer until completion. If the task is
//! dropped while the kernel still owns the buffer, it drains that completion
//! before releasing the buffer, which can briefly block the dropping thread.
//! The async descriptor helpers track operations by fd and wait for them before
//! `release` closes the descriptor. Code that closes an fd outside the same
//! helper must provide equivalent ordering. The implementation does not dup fds,
//! avoiding interaction with process-associated record locks.
//!
//! `Fsync` completions can be unordered with outstanding writes. Await writes
//! before calling `fsync` when persistence ordering matters. See the
//! [`io-uring` opcode documentation](https://docs.rs/io-uring/latest/io_uring/opcode/)
//! and [Tokio `AsyncFd`](https://docs.rs/tokio/latest/tokio/io/unix/struct.AsyncFd.html).

use std::collections::HashMap;
use std::io;
use std::io::SeekFrom;
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use ::io_uring::{IoUring, opcode, squeue, types};
use tokio::io::unix::AsyncFd;
use tokio::sync::{Notify, oneshot};

use crate::types::{FallocateFlags, PosixError};

const COMPLETION_TAG: u64 = 1;

struct AsyncRing {
    ring: Mutex<IoUring>,
    submitted: AtomicBool,
    completed: AtomicBool,
}

impl AsyncRing {
    fn new() -> io::Result<Self> {
        Ok(Self {
            ring: Mutex::new(IoUring::new(8)?),
            submitted: AtomicBool::new(false),
            completed: AtomicBool::new(false),
        })
    }
}

impl AsRawFd for AsyncRing {
    fn as_raw_fd(&self) -> RawFd {
        self.ring
            .lock()
            .expect("io_uring mutex poisoned")
            .as_raw_fd()
    }
}

fn lock_ring(ring: &AsyncRing) -> io::Result<MutexGuard<'_, IoUring>> {
    ring.ring
        .lock()
        .map_err(|_| io::Error::other("io_uring mutex poisoned"))
}

impl Drop for AsyncRing {
    fn drop(&mut self) {
        if !self.submitted.load(Ordering::Acquire) || self.completed.load(Ordering::Acquire) {
            return;
        }

        let ring = self
            .ring
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if ring.completion().next().is_none() {
            let _ = ring.submitter().submit_and_wait(1);
            let _ = ring.completion().next();
        }
    }
}

async fn execute(entry: squeue::Entry) -> io::Result<i32> {
    let ring = AsyncFd::new(AsyncRing::new()?)?;
    // SAFETY: The caller owns any buffers referenced by `entry` for the full
    // duration of this future, including while it awaits the completion.
    unsafe {
        lock_ring(ring.get_ref())?
            .submission()
            .push(&entry)
            .map_err(|_| io::Error::other("io_uring submission queue is full"))?;
    }
    let submitted = lock_ring(ring.get_ref())?.submitter().submit()?;
    if submitted != 1 {
        return Err(io::Error::other("io_uring did not submit the request"));
    }
    ring.get_ref().submitted.store(true, Ordering::Release);

    loop {
        let mut ready = ring.readable().await?;
        let mut inner = lock_ring(ready.get_inner())?;
        if let Some(cqe) = inner.completion().next() {
            ring.get_ref().completed.store(true, Ordering::Release);
            if cqe.user_data() != COMPLETION_TAG {
                return Err(io::Error::other("unexpected io_uring completion"));
            }
            return Ok(cqe.result());
        }
        drop(inner);
        ready.clear_ready();
    }
}

fn io_result(result: i32, operation: &'static str) -> Result<i32, PosixError> {
    if result < 0 {
        Err(PosixError::new(
            -result,
            format!("io_uring {operation} failed"),
        ))
    } else {
        Ok(result)
    }
}

fn checked_io_result(result: io::Result<i32>, operation: &'static str) -> Result<i32, PosixError> {
    result
        .map_err(PosixError::from)
        .and_then(|result| io_result(result, operation))
}

#[derive(Clone, Default)]
pub struct IoUringExecutor {
    state: Arc<ExecutorState>,
}

#[derive(Default)]
struct ExecutorState {
    in_flight: Mutex<HashMap<RawFd, usize>>,
    idle: Notify,
}

struct FdLease {
    state: Arc<ExecutorState>,
    fd: RawFd,
}

impl Drop for FdLease {
    fn drop(&mut self) {
        let mut in_flight = self
            .state
            .in_flight
            .lock()
            .expect("io_uring in-flight map poisoned");
        if let Some(count) = in_flight.get_mut(&self.fd) {
            *count -= 1;
            if *count == 0 {
                in_flight.remove(&self.fd);
            }
        }
        drop(in_flight);
        self.state.idle.notify_waiters();
    }
}

impl IoUringExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    fn lease(&self, fd: RawFd) -> FdLease {
        *self
            .state
            .in_flight
            .lock()
            .expect("io_uring in-flight map poisoned")
            .entry(fd)
            .or_default() += 1;
        FdLease {
            state: self.state.clone(),
            fd,
        }
    }

    /// Wait until submitted operations using `fd` have completed.
    ///
    /// Async descriptor helpers call this before closing a handle, so dropping
    /// a caller's read/write future cannot leave an io_uring request using a
    /// closed descriptor or freed buffer.
    pub async fn wait_for_fd(&self, fd: RawFd) {
        loop {
            let notified = self.state.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let active = self
                .state
                .in_flight
                .lock()
                .expect("io_uring in-flight map poisoned")
                .contains_key(&fd);
            if !active {
                return;
            }
            notified.await;
        }
    }

    pub async fn read(
        &self,
        fd: BorrowedFd<'_>,
        seek: SeekFrom,
        size: usize,
    ) -> Result<Vec<u8>, PosixError> {
        let size = u32::try_from(size)
            .map_err(|_| PosixError::new(libc::EINVAL, "read size exceeds io_uring limit"))?;
        let raw_fd = fd.as_raw_fd();
        let lease = self.lease(raw_fd);
        let offset = super::resolve_io_offset(fd, seek)?;
        let (reply, result) = oneshot::channel::<Result<Vec<u8>, PosixError>>();
        tokio::spawn(async move {
            let _lease = lease;
            let mut buffer = vec![0; size as usize];
            let entry = opcode::Read::new(types::Fd(raw_fd), buffer.as_mut_ptr(), size)
                .offset(offset as u64)
                .build()
                .user_data(COMPLETION_TAG);
            let result = checked_io_result(execute(entry).await, "read").map(|bytes_read| {
                buffer.truncate(bytes_read as usize);
                buffer
            });
            let _ = reply.send(result);
        });
        result
            .await
            .map_err(|_| PosixError::new(libc::EIO, "io_uring read task ended before completion"))?
    }

    pub async fn write(
        &self,
        fd: BorrowedFd<'_>,
        seek: SeekFrom,
        data: Vec<u8>,
    ) -> Result<usize, PosixError> {
        let size = u32::try_from(data.len())
            .map_err(|_| PosixError::new(libc::EINVAL, "write size exceeds io_uring limit"))?;
        let raw_fd = fd.as_raw_fd();
        let lease = self.lease(raw_fd);
        let offset = super::resolve_io_offset(fd, seek)?;
        let (reply, result) = oneshot::channel::<Result<usize, PosixError>>();
        tokio::spawn(async move {
            let _lease = lease;
            let entry = opcode::Write::new(types::Fd(raw_fd), data.as_ptr(), size)
                .offset(offset as u64)
                .build()
                .user_data(COMPLETION_TAG);
            let result =
                checked_io_result(execute(entry).await, "write").map(|bytes| bytes as usize);
            let _ = reply.send(result);
            // Keep the write buffer alive through completion, even if the
            // caller dropped its receiver while the ring request was pending.
            drop(data);
        });
        result.await.map_err(|_| {
            PosixError::new(libc::EIO, "io_uring write task ended before completion")
        })?
    }

    pub async fn flush(&self, fd: BorrowedFd<'_>) -> Result<(), PosixError> {
        self.sync(fd, true).await
    }

    pub async fn fsync(&self, fd: BorrowedFd<'_>, datasync: bool) -> Result<(), PosixError> {
        self.sync(fd, datasync).await
    }

    async fn sync(&self, fd: BorrowedFd<'_>, datasync: bool) -> Result<(), PosixError> {
        let raw_fd = fd.as_raw_fd();
        let lease = self.lease(raw_fd);
        let (reply, result) = oneshot::channel::<Result<(), PosixError>>();
        tokio::spawn(async move {
            let _lease = lease;
            let flags = if datasync {
                types::FsyncFlags::DATASYNC
            } else {
                types::FsyncFlags::empty()
            };
            let entry = opcode::Fsync::new(types::Fd(raw_fd))
                .flags(flags)
                .build()
                .user_data(COMPLETION_TAG);
            let result = checked_io_result(execute(entry).await, "fsync").map(|_| ());
            let _ = reply.send(result);
        });
        result.await.map_err(|_| {
            PosixError::new(libc::EIO, "io_uring fsync task ended before completion")
        })?
    }

    pub async fn fallocate(
        &self,
        fd: BorrowedFd<'_>,
        offset: i64,
        length: i64,
        mode: FallocateFlags,
    ) -> Result<(), PosixError> {
        let raw_fd = fd.as_raw_fd();
        let lease = self.lease(raw_fd);
        let (reply, result) = oneshot::channel::<Result<(), PosixError>>();
        tokio::spawn(async move {
            let _lease = lease;
            let entry = opcode::Fallocate::new(types::Fd(raw_fd), length as u64)
                .offset(offset as u64)
                .mode(mode.bits())
                .build()
                .user_data(COMPLETION_TAG);
            let result = checked_io_result(execute(entry).await, "fallocate").map(|_| ());
            let _ = reply.send(result);
        });
        result.await.map_err(|_| {
            PosixError::new(libc::EIO, "io_uring fallocate task ended before completion")
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom};
    use std::os::fd::AsFd;

    #[tokio::test]
    async fn positioned_io_and_sync_operations_round_trip() {
        // io_uring can be disabled by the host's security policy. Keep this
        // feature test portable across Linux environments with that restriction.
        if IoUring::new(8).is_err() {
            return;
        }

        let mut file = tempfile::tempfile().unwrap();
        let executor = IoUringExecutor::new();
        let data = b"ring-backed positioned io".to_vec();

        let written = executor
            .write(file.as_fd(), SeekFrom::Start(0), data.clone())
            .await
            .unwrap();
        assert_eq!(written, data.len());
        executor.flush(file.as_fd()).await.unwrap();
        let read = executor
            .read(file.as_fd(), SeekFrom::Start(0), data.len())
            .await
            .unwrap();
        assert_eq!(read, data);
        executor.fsync(file.as_fd(), false).await.unwrap();
        assert_eq!(file.stream_position().unwrap(), 0);
    }
}
