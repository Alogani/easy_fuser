//! Mounted direct-I/O throughput comparison for parallel, async, and io_uring.
//!
//! Run the same test binary under each mode (see `benches/README.md`).

#![cfg(all(target_os = "linux", any(feature = "parallel", feature = "async")))]

use std::{
    fs,
    hint::black_box,
    io::SeekFrom,
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use easy_fuser::{
    fuse_presets::{StatelessHandler, UnimplementedFuseHandler, mirror_fs::*},
    types::MountThreads,
};
use tempfile::TempDir;

#[cfg(all(feature = "async", not(feature = "parallel")))]
use easy_fuser::fuse_async::prelude::*;
#[cfg(feature = "parallel")]
use easy_fuser::fuse_parallel::prelude::*;

#[cfg(all(feature = "async", not(feature = "parallel")))]
use async_trait::async_trait;
#[cfg(feature = "parallel")]
use easy_fuser_macro::delegate_fs;
#[cfg(all(feature = "async", not(feature = "parallel"), feature = "io_uring"))]
use easy_fuser_macro::delegate_fs_async as delegate_mirror;
#[cfg(all(feature = "async", not(feature = "parallel")))]
use easy_fuser_macro::delegate_fs_sync_to_async;
#[cfg(all(
    feature = "async",
    not(feature = "parallel"),
    not(feature = "io_uring")
))]
use easy_fuser_macro::delegate_fs_sync_to_async as delegate_mirror;

#[cfg(feature = "parallel")]
struct BenchmarkFs {
    mirror: MirrorFs,
    defaults: UnimplementedFuseHandler<PathBuf>,
    stateless: StatelessHandler<PathBuf>,
}

#[cfg(feature = "parallel")]
impl FuseHandler for BenchmarkFs {
    type TId = PathBuf;
    type FileHandle = OwnedFd;

    fn get_default_ttl(&self) -> Duration {
        Duration::ZERO
    }

    fn open(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(Self::FileHandle, FopenFlags)> {
        let (handle, _) = self.mirror.open(req, file_id, flags)?;
        Ok((handle, FopenFlags::FOPEN_DIRECT_IO))
    }

    delegate_fs! { mirror, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, readdir, readlink,
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr,
        symlink, unlink
    ] }
    delegate_fs! { stateless, [forget, fsyncdir, opendir, releasedir] }
    delegate_fs! { defaults, [bmap, getlk, ioctl, link, setlk, statfs] }
}

#[cfg(all(feature = "async", not(feature = "parallel")))]
#[cfg(feature = "io_uring")]
type AsyncMirror = MirrorFsAsync;
#[cfg(all(
    feature = "async",
    not(feature = "parallel"),
    not(feature = "io_uring")
))]
type AsyncMirror = MirrorFs;

#[cfg(all(feature = "async", not(feature = "parallel")))]
struct BenchmarkFs {
    mirror: AsyncMirror,
    defaults: UnimplementedFuseHandler<PathBuf>,
    stateless: StatelessHandler<PathBuf>,
}

#[cfg(all(feature = "async", not(feature = "parallel")))]
#[async_trait]
impl FuseHandler for BenchmarkFs {
    type TId = PathBuf;
    type FileHandle = OwnedFd;

    fn get_default_ttl(&self) -> Duration {
        Duration::ZERO
    }

    async fn open(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(Self::FileHandle, FopenFlags)> {
        #[cfg(feature = "io_uring")]
        let (handle, _) = self.mirror.open(req, file_id, flags).await?;
        #[cfg(not(feature = "io_uring"))]
        let (handle, _) = self.mirror.open(req, file_id, flags)?;
        Ok((handle, FopenFlags::FOPEN_DIRECT_IO))
    }

    delegate_mirror! { mirror, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, readdir, readlink,
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr,
        symlink, unlink
    ] }
    delegate_fs_sync_to_async! { stateless, [forget, fsyncdir, opendir, releasedir] }
    delegate_fs_sync_to_async! { defaults, [bmap, getlk, ioctl, link, setlk, statfs] }
}

#[test]
#[ignore = "manual mounted filesystem benchmark"]
fn benchmark_direct_io() {
    #[cfg(all(feature = "io_uring", not(feature = "parallel")))]
    if io_uring::IoUring::new(8).is_err() {
        eprintln!("io_uring_setup is unavailable on this host; skipping benchmark");
        return;
    }

    let iterations = env_usize("EASY_FUSER_BENCH_ITERS", 2_000);
    let samples = env_usize("EASY_FUSER_BENCH_SAMPLES", 3);
    let clients = [1, 4, 8];
    let sizes = [4096, 65_536];
    let source = TempDir::new().expect("create backing directory");
    let mountpoint = TempDir::new().expect("create mountpoint");
    for size in sizes {
        let data = vec![b'x'; size];
        for client in 0..*clients.last().unwrap() {
            fs::write(source.path().join(format!("{size}-{client}")), &data).unwrap();
        }
    }

    let readers = env_usize("EASY_FUSER_BENCH_FUSER_THREADS", 1);
    let handler_workers = env_usize("EASY_FUSER_BENCH_HANDLER_THREADS", 2);
    let session = spawn_mount(
        BenchmarkFs {
            mirror: benchmark_mirror(source.path().to_path_buf()),
            defaults: UnimplementedFuseHandler::new(),
            stateless: StatelessHandler::new(),
        },
        mountpoint.path(),
        &[],
        Some(MountThreads::new(readers, handler_workers)),
    )
    .expect("mount benchmark filesystem");

    println!(
        "mode,readers,handler_workers,clients,size_bytes,iterations_per_client,sample,operation,aggregate_ops_per_second,aggregate_mib_per_second"
    );
    for sample in 0..samples {
        for size in sizes {
            let all_files: Vec<_> = (0..*clients.last().unwrap())
                .map(|client| {
                    fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(mountpoint.path().join(format!("{size}-{client}")))
                        .unwrap()
                })
                .collect();
            for &client_count in &clients {
                let handles = clone_first(&all_files, client_count);
                for operation in operation_order(sample) {
                    let elapsed = run_io(&handles, size, iterations, operation);
                    let operations = client_count as u128 * iterations as u128;
                    let ops_per_second = operations * 1_000_000_000 / elapsed.as_nanos();
                    let mib_per_second = ops_per_second * size as u128 / (1024 * 1024);
                    println!(
                        "{},{readers},{handler_workers},{client_count},{size},{iterations},{sample},{operation},{ops_per_second},{mib_per_second}",
                        mode_name()
                    );
                }
            }
        }
    }
    session.join().expect("unmount benchmark filesystem");
}

#[cfg(feature = "parallel")]
fn benchmark_mirror(path: PathBuf) -> MirrorFs {
    MirrorFs::new(path)
}

#[cfg(all(feature = "async", not(feature = "parallel"), feature = "io_uring"))]
fn benchmark_mirror(path: PathBuf) -> AsyncMirror {
    MirrorFsAsync::new(path)
}

#[cfg(all(
    feature = "async",
    not(feature = "parallel"),
    not(feature = "io_uring")
))]
fn benchmark_mirror(path: PathBuf) -> AsyncMirror {
    MirrorFs::new(path)
}

fn clone_first(files: &[fs::File], count: usize) -> Vec<fs::File> {
    files[..count]
        .iter()
        .map(|file| file.try_clone().unwrap())
        .collect()
}

fn operation_order(sample: usize) -> [IoOperation; 2] {
    if sample % 2 == 0 {
        [IoOperation::Read, IoOperation::Write]
    } else {
        [IoOperation::Write, IoOperation::Read]
    }
}

#[derive(Clone, Copy)]
enum IoOperation {
    Read,
    Write,
}

impl std::fmt::Display for IoOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            IoOperation::Read => "read",
            IoOperation::Write => "write",
        })
    }
}

fn run_io(files: &[fs::File], size: usize, iterations: usize, operation: IoOperation) -> Duration {
    let data = vec![b'y'; size];
    for file in files {
        let mut buffer = vec![0; size];
        match operation {
            IoOperation::Read => {
                assert_eq!(file.read_at(&mut buffer, 0).unwrap(), size);
                black_box(buffer);
            }
            IoOperation::Write => assert_eq!(file.write_at(&data, 0).unwrap(), size),
        }
    }

    let barrier = Arc::new(Barrier::new(files.len() + 1));
    let data = Arc::new(data);
    let tasks: Vec<_> = files
        .iter()
        .map(|file| {
            let file = file.try_clone().unwrap();
            let data = data.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut buffer = vec![0; size];
                barrier.wait();
                for _ in 0..iterations {
                    match operation {
                        IoOperation::Read => {
                            assert_eq!(file.read_at(&mut buffer, 0).unwrap(), size);
                            black_box(&buffer);
                        }
                        IoOperation::Write => {
                            assert_eq!(file.write_at(&data, 0).unwrap(), size);
                        }
                    }
                }
            })
        })
        .collect();
    let start = Instant::now();
    barrier.wait();
    for task in tasks {
        task.join().expect("I/O client panicked");
    }
    start.elapsed()
}

fn mode_name() -> &'static str {
    #[cfg(feature = "parallel")]
    return "parallel";
    #[cfg(all(feature = "async", not(feature = "parallel"), feature = "io_uring"))]
    return "io_uring";
    #[cfg(all(
        feature = "async",
        not(feature = "parallel"),
        not(feature = "io_uring")
    ))]
    return "async_sync";
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
