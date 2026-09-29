//! Manual end-to-end benchmark of the parallel path resolver through a FUSE mount.
//! Run with: cargo test --release --test performance -- --ignored --nocapture

#![cfg(feature = "parallel")]

use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler, mirror_fs::*};
use easy_fuser::unix_fs;
use easy_fuser_macro::delegate_fs;
use std::{
    fs,
    hint::black_box,
    io::SeekFrom,
    os::fd::OwnedFd,
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

struct MeasuredFs {
    mirror: MirrorFs,
    defaults: UnimplementedFuseHandler<PathBuf>,
    safe_defaults: StatelessHandler<PathBuf>,
    getattr_calls: Arc<AtomicUsize>,
    lookup_calls: Arc<AtomicUsize>,
    readdir_calls: Arc<AtomicUsize>,
    rename_calls: Arc<AtomicUsize>,
    read_calls: Arc<AtomicUsize>,
    write_calls: Arc<AtomicUsize>,
    create_calls: Arc<AtomicUsize>,
    unlink_calls: Arc<AtomicUsize>,
    forget_calls: Arc<AtomicUsize>,
}

impl FuseHandler for MeasuredFs {
    type TId = PathBuf;
    type FileHandle = OwnedFd;

    fn get_default_ttl(&self) -> Duration {
        Duration::ZERO
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        id: PathBuf,
        _fh: Option<&mut OwnedFd>,
    ) -> FuseResult<FileAttribute> {
        self.getattr_calls.fetch_add(1, Ordering::Relaxed);
        unix_fs::lookup(&self.mirror.source_dir().join(id))
    }

    fn lookup(
        &self,
        req: &RequestInfo,
        parent: PathBuf,
        name: &std::ffi::OsStr,
    ) -> FuseResult<FileAttribute> {
        self.lookup_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror.lookup(req, parent, name)
    }

    fn readdir(
        &self,
        req: &RequestInfo,
        id: PathBuf,
        fh: BorrowedFileHandle<'_>,
    ) -> FuseResult<Vec<(std::ffi::OsString, FileKind)>> {
        self.readdir_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror.readdir(req, id, fh)
    }

    fn rename(
        &self,
        req: &RequestInfo,
        parent: PathBuf,
        name: &std::ffi::OsStr,
        newparent: PathBuf,
        newname: &std::ffi::OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        self.rename_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror
            .rename(req, parent, name, newparent, newname, flags)
    }

    fn open(
        &self,
        req: &RequestInfo,
        id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFd, FopenFlags)> {
        let (fh, _) = self.mirror.open(req, id, flags)?;
        Ok((fh, FopenFlags::FOPEN_DIRECT_IO))
    }

    fn read(
        &self,
        req: &RequestInfo,
        id: PathBuf,
        fh: Option<&mut OwnedFd>,
        seek: SeekFrom,
        size: u32,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        self.read_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror.read(req, id, fh, seek, size, flags, lock_owner)
    }

    fn write(
        &self,
        req: &RequestInfo,
        id: PathBuf,
        fh: Option<&mut OwnedFd>,
        seek: SeekFrom,
        data: Vec<u8>,
        write_flags: WriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        self.write_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror
            .write(req, id, fh, seek, data, write_flags, flags, lock_owner)
    }

    fn create(
        &self,
        req: &RequestInfo,
        parent: PathBuf,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFd, FileAttribute, FopenFlags)> {
        self.create_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror.create(req, parent, name, mode, umask, flags)
    }

    fn unlink(&self, req: &RequestInfo, parent: PathBuf, name: &std::ffi::OsStr) -> FuseResult<()> {
        self.unlink_calls.fetch_add(1, Ordering::Relaxed);
        self.mirror.unlink(req, parent, name)
    }

    fn forget(&self, _req: &RequestInfo, _id: PathBuf, _nlookup: u64) {
        self.forget_calls.fetch_add(1, Ordering::Relaxed);
    }

    delegate_fs! { mirror, [
        flush, fsync, lseek, release,
        access, getxattr, listxattr, readlink,
        copy_file_range, fallocate,
        mkdir, mknod, removexattr, rmdir, setattr, setxattr, symlink
    ] }
    delegate_fs! { safe_defaults, [ fsyncdir, opendir, releasedir ] }
    delegate_fs! { defaults, [ bmap, getlk, ioctl, link, setlk, statfs ] }
}

#[test]
#[ignore = "manual mounted filesystem benchmark"]
fn benchmark_parallel_getattr() {
    let iterations = std::env::var("EASY_FUSER_BENCH_ITERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_000);
    let churn_iterations = std::env::var("EASY_FUSER_BENCH_CHURN_ITERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(500);
    let source = TempDir::new().unwrap();
    let mountpoint = TempDir::new().unwrap();
    fs::write(source.path().join("file"), b"data").unwrap();
    fs::create_dir(source.path().join("listing")).unwrap();
    for index in 0..8 {
        fs::write(
            source.path().join("listing").join(format!("entry-{index}")),
            b"",
        )
        .unwrap();
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let lookup_calls = Arc::new(AtomicUsize::new(0));
    let readdir_calls = Arc::new(AtomicUsize::new(0));
    let rename_calls = Arc::new(AtomicUsize::new(0));
    let read_calls = Arc::new(AtomicUsize::new(0));
    let write_calls = Arc::new(AtomicUsize::new(0));
    let create_calls = Arc::new(AtomicUsize::new(0));
    let unlink_calls = Arc::new(AtomicUsize::new(0));
    let forget_calls = Arc::new(AtomicUsize::new(0));
    let session = spawn_mount(
        MeasuredFs {
            mirror: MirrorFs::new(source.path().to_path_buf()),
            defaults: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
            getattr_calls: calls.clone(),
            lookup_calls: lookup_calls.clone(),
            readdir_calls: readdir_calls.clone(),
            rename_calls: rename_calls.clone(),
            read_calls: read_calls.clone(),
            write_calls: write_calls.clone(),
            create_calls: create_calls.clone(),
            unlink_calls: unlink_calls.clone(),
            forget_calls: forget_calls.clone(),
        },
        mountpoint.path(),
        &[],
        Some(8),
    )
    .unwrap();
    let path = mountpoint.path().join("file");
    fs::metadata(&path).unwrap();
    println!(
        "case,threads,iterations_per_thread,handler_calls,ns_per_op,ops_per_second,handler_requests_per_second"
    );
    for workers in [1, 4, 8] {
        calls.store(0, Ordering::Relaxed);
        let barrier = Arc::new(Barrier::new(workers + 1));
        let tasks: Vec<_> = (0..workers)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..iterations {
                        black_box(fs::metadata(&path).unwrap());
                    }
                })
            })
            .collect();
        let start = Instant::now();
        barrier.wait();
        for task in tasks {
            task.join().unwrap();
        }
        let elapsed = start.elapsed();
        let observed = calls.load(Ordering::Relaxed);
        report("metadata", workers, iterations, observed, elapsed, 1);
        assert!(
            observed > 0,
            "getattr was served entirely from kernel cache"
        );
    }
    for workers in [1, 4, 8] {
        create_calls.store(0, Ordering::Relaxed);
        unlink_calls.store(0, Ordering::Relaxed);
        forget_calls.store(0, Ordering::Relaxed);
        let barrier = Arc::new(Barrier::new(workers + 1));
        let tasks: Vec<_> = (0..workers)
            .map(|worker_id| {
                let root = mountpoint.path().to_path_buf();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    for index in 0..churn_iterations {
                        let path = root.join(format!("churn-{worker_id}-{index}"));
                        drop(fs::File::create(&path).unwrap());
                        fs::remove_file(path).unwrap();
                    }
                })
            })
            .collect();
        let start = Instant::now();
        barrier.wait();
        for task in tasks {
            task.join().unwrap();
        }
        let elapsed = start.elapsed();
        report(
            "create_unlink",
            workers,
            churn_iterations,
            create_calls.load(Ordering::Relaxed) + unlink_calls.load(Ordering::Relaxed),
            elapsed,
            2,
        );
        assert_eq!(
            create_calls.load(Ordering::Relaxed),
            churn_iterations * workers
        );
        assert_eq!(
            unlink_calls.load(Ordering::Relaxed),
            churn_iterations * workers
        );
    }
    let lookup_iterations = std::env::var("EASY_FUSER_BENCH_LOOKUP_ITERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(500);
    for worker in 0..8 {
        for index in 0..lookup_iterations {
            fs::write(source.path().join(format!("lookup-{worker}-{index}")), b"").unwrap();
        }
    }
    for workers in [1, 4, 8] {
        lookup_calls.store(0, Ordering::Relaxed);
        let root = mountpoint.path().to_path_buf();
        let elapsed = measure(workers, lookup_iterations, move |worker, index| {
            black_box(fs::metadata(root.join(format!("lookup-{worker}-{index}"))).unwrap());
        });
        let observed = lookup_calls.load(Ordering::Relaxed);
        report(
            "unique_lookup",
            workers,
            lookup_iterations,
            observed,
            elapsed,
            1,
        );
        assert!(observed >= lookup_iterations * workers);
    }
    for workers in [1, 4, 8] {
        readdir_calls.store(0, Ordering::Relaxed);
        let dir = mountpoint.path().join("listing");
        let elapsed = measure(workers, iterations, move |_, _| {
            black_box(fs::read_dir(&dir).unwrap().count());
        });
        let observed = readdir_calls.load(Ordering::Relaxed);
        report("readdir", workers, iterations, observed, elapsed, 1);
        assert!(observed >= iterations * workers);
    }
    for workers in [1, 4, 8] {
        for worker in 0..workers {
            fs::write(source.path().join(format!("rename-{worker}-left")), b"").unwrap();
        }
        rename_calls.store(0, Ordering::Relaxed);
        let root = mountpoint.path().to_path_buf();
        let elapsed = measure(workers, churn_iterations, move |worker, index| {
            let left = root.join(format!("rename-{worker}-left"));
            let right = root.join(format!("rename-{worker}-right"));
            if index % 2 == 0 {
                fs::rename(left, right).unwrap();
            } else {
                fs::rename(right, left).unwrap();
            }
        });
        let observed = rename_calls.load(Ordering::Relaxed);
        report("rename", workers, churn_iterations, observed, elapsed, 1);
        assert_eq!(observed, churn_iterations * workers);
        for worker in 0..workers {
            for suffix in ["left", "right"] {
                let path = source.path().join(format!("rename-{worker}-{suffix}"));
                if path.exists() {
                    fs::remove_file(path).unwrap();
                }
            }
        }
    }
    for workers in [1, 4, 8] {
        read_calls.store(0, Ordering::Relaxed);
        let path = mountpoint.path().join("file");
        let elapsed = measure(workers, iterations, move |_, _| {
            let file = fs::File::open(&path).unwrap();
            let mut bytes = [0; 4];
            assert_eq!(file.read_at(&mut bytes, 0).unwrap(), 4);
            black_box(bytes);
        });
        let observed = read_calls.load(Ordering::Relaxed);
        report("read", workers, iterations, observed, elapsed, 1);
        assert_eq!(observed, iterations * workers);
    }
    for workers in [1, 4, 8] {
        for worker in 0..workers {
            fs::write(source.path().join(format!("write-{worker}")), b"data").unwrap();
        }
        write_calls.store(0, Ordering::Relaxed);
        let root = mountpoint.path().to_path_buf();
        let elapsed = measure(workers, iterations, move |worker, _| {
            let file = fs::OpenOptions::new()
                .write(true)
                .open(root.join(format!("write-{worker}")))
                .unwrap();
            assert_eq!(file.write_at(b"data", 0).unwrap(), 4);
        });
        let observed = write_calls.load(Ordering::Relaxed);
        report("write", workers, iterations, observed, elapsed, 1);
        assert_eq!(observed, iterations * workers);
    }
    black_box(forget_calls.load(Ordering::Relaxed));
    session.join().unwrap();
}

fn measure(
    workers: usize,
    iterations: usize,
    operation: impl Fn(usize, usize) + Send + Sync + 'static,
) -> Duration {
    let barrier = Arc::new(Barrier::new(workers + 1));
    let operation = Arc::new(operation);
    let tasks: Vec<_> = (0..workers)
        .map(|worker| {
            let barrier = barrier.clone();
            let operation = operation.clone();
            thread::spawn(move || {
                barrier.wait();
                for index in 0..iterations {
                    operation(worker, index);
                }
            })
        })
        .collect();
    let start = Instant::now();
    barrier.wait();
    for task in tasks {
        task.join().unwrap();
    }
    start.elapsed()
}

fn report(
    case: &str,
    workers: usize,
    iterations: usize,
    calls: usize,
    elapsed: Duration,
    operations_per_iteration: usize,
) {
    let operations = workers * iterations * operations_per_iteration;
    let nanos = elapsed.as_nanos();
    println!(
        "{case},{workers},{iterations},{calls},{},{},{}",
        nanos / operations as u128,
        (operations as u128 * 1_000_000_000) / nanos,
        (calls as u128 * 1_000_000_000) / nanos
    );
}
