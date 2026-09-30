//! Mounted FUSE thread-count matrix.
//!
//! Run with `cargo test --release --no-default-features --features parallel \
//!   --test thread_scaling -- --ignored --nocapture`.

#![cfg(all(target_os = "linux", feature = "parallel"))]

use std::{
    hint::black_box,
    marker::PhantomData,
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant, SystemTime},
};

use easy_fuser::{
    fuse_parallel::prelude::*,
    fuse_presets::{StatelessHandler, UnimplementedFuseHandler},
};
use easy_fuser_macro::delegate_fs;
use tempfile::TempDir;

const FILE_INODE: Inode = fuser::INodeNo(2);
const FILE_SIZE: u64 = 4096;

trait BenchmarkId: FileIdType + Sync {
    fn is_root(&self) -> bool;
    fn touch(&self);
    fn lookup_metadata() -> Self::Metadata;
}

impl BenchmarkId for Inode {
    fn is_root(&self) -> bool {
        *self == ROOT_INODE
    }

    fn touch(&self) {
        black_box(*self);
    }

    fn lookup_metadata() -> Self::Metadata {
        (FILE_INODE, attr(FileKind::RegularFile, FILE_SIZE))
    }
}

impl BenchmarkId for PathBuf {
    fn is_root(&self) -> bool {
        self.as_os_str().is_empty()
    }

    fn touch(&self) {
        black_box(self.as_os_str());
    }

    fn lookup_metadata() -> Self::Metadata {
        attr(FileKind::RegularFile, FILE_SIZE)
    }
}

impl BenchmarkId for MappedInode {
    fn is_root(&self) -> bool {
        self.inode() == ROOT_INODE
    }

    fn touch(&self) {
        black_box(self.paths());
    }

    fn lookup_metadata() -> Self::Metadata {
        attr(FileKind::RegularFile, FILE_SIZE)
    }
}

struct ConstantFs<TId: BenchmarkId> {
    defaults: UnimplementedFuseHandler<TId>,
    stateless: StatelessHandler<TId>,
    _id: PhantomData<TId>,
}

impl<TId: BenchmarkId> ConstantFs<TId> {
    fn new() -> Self {
        Self {
            defaults: UnimplementedFuseHandler::new(),
            stateless: StatelessHandler::new(),
            _id: PhantomData,
        }
    }
}

impl<TId: BenchmarkId> FuseHandler for ConstantFs<TId> {
    type TId = TId;
    type FileHandle = OwnedFileHandle;

    fn get_default_ttl(&self) -> Duration {
        Duration::ZERO
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        _fh: Option<&mut OwnedFileHandle>,
    ) -> FuseResult<FileAttribute> {
        file_id.touch();
        if file_id.is_root() {
            Ok(attr(FileKind::Directory, 0))
        } else {
            Ok(attr(FileKind::RegularFile, FILE_SIZE))
        }
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        _parent_id: TId,
        _name: &std::ffi::OsStr,
    ) -> FuseResult<TId::Metadata> {
        Ok(TId::lookup_metadata())
    }

    delegate_fs! { stateless, [forget, fsyncdir, opendir, releasedir] }
    delegate_fs! { defaults, [
        access, bmap, copy_file_range, create, fallocate, flush, fsync, getlk,
        getxattr, ioctl, link, listxattr, lseek, mkdir, mknod, open, read,
        readdir, readdirplus, readlink, release, removexattr, rename, rmdir,
        setattr, setlk, setxattr, statfs, symlink, unlink, write
    ] }
}

fn attr(kind: FileKind, size: u64) -> FileAttribute {
    let now = SystemTime::UNIX_EPOCH;
    FileAttribute {
        size,
        blocks: size.div_ceil(512),
        atime: now,
        mtime: now,
        ctime: now,
        crtime: now,
        kind,
        perm: if kind == FileKind::Directory {
            0o755
        } else {
            0o644
        },
        nlink: 1,
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
        rdev: 0,
        blksize: 4096,
        flags: 0,
        ttl: Some(Duration::ZERO),
        generation: None,
    }
}

#[test]
#[ignore = "manual mounted filesystem benchmark"]
fn benchmark_reader_and_handler_thread_matrix() {
    let clients = env_usize("EASY_FUSER_BENCH_CLIENTS", 8);
    let iterations = env_usize("EASY_FUSER_BENCH_ITERS", 10_000);
    let samples = env_usize("EASY_FUSER_BENCH_SAMPLES", 3);
    let mut configurations: Vec<_> = [1, 2, 4, 8]
        .into_iter()
        .flat_map(|readers| {
            [1, 2, 4, 8].into_iter().flat_map(move |workers| {
                [false, true].into_iter().filter_map(move |clone_fd| {
                    (readers > 1 || !clone_fd).then_some((readers, workers, clone_fd))
                })
            })
        })
        .collect();

    println!(
        "readers,handler_workers,clone_fuser_fd,clients,iterations_per_client,sample,aggregate_ops_per_second,ops_per_client_per_second"
    );
    for sample in 0..samples {
        // Rotate the order between samples to reduce systematic warm-up and
        // thermal bias. Each row is aggregate throughput for fixed clients.
        let config_count = configurations.len();
        configurations.rotate_left(sample % config_count);
        for &(readers, handler_workers, clone_fuser_fd) in &configurations {
            let elapsed = match run_case::<Inode>(
                readers,
                handler_workers,
                clone_fuser_fd,
                clients,
                iterations,
            ) {
                Ok(elapsed) => elapsed,
                Err(err) if clone_fuser_fd => {
                    eprintln!("skip clone_fuser_fd={clone_fuser_fd}: {err}");
                    continue;
                }
                Err(err) => panic!("mount constant benchmark filesystem: {err}"),
            };
            let operations = clients as u128 * iterations as u128;
            let aggregate = operations * 1_000_000_000 / elapsed.as_nanos();
            println!(
                "{readers},{handler_workers},{clone_fuser_fd},{clients},{iterations},{sample},{aggregate},{}",
                aggregate / clients as u128
            );
        }
    }
}

#[test]
#[ignore = "manual mounted filesystem benchmark"]
fn benchmark_file_id_types() {
    let iterations = env_usize("EASY_FUSER_BENCH_ITERS", 10_000);
    let samples = env_usize("EASY_FUSER_BENCH_SAMPLES", 3);
    println!(
        "id_type,readers,handler_workers,clients,iterations_per_client,sample,aggregate_ops_per_second"
    );
    for sample in 0..samples {
        for clients in [1, 4, 8] {
            let mut cases: [(&str, fn(usize, usize) -> Duration); 3] = [
                ("Inode", run_inode_case),
                ("PathBuf", run_pathbuf_case),
                ("MappedInode_paths", run_mapped_case),
            ];
            let case_count = cases.len();
            cases.rotate_left(sample % case_count);
            for (id_type, run) in cases {
                let elapsed = run(clients, iterations);
                let operations = clients as u128 * iterations as u128;
                let aggregate = operations * 1_000_000_000 / elapsed.as_nanos();
                println!("{id_type},1,2,{clients},{iterations},{sample},{aggregate}");
            }
        }
    }
}

fn run_inode_case(clients: usize, iterations: usize) -> Duration {
    run_case::<Inode>(1, 2, false, clients, iterations).expect("mount Inode benchmark filesystem")
}

fn run_pathbuf_case(clients: usize, iterations: usize) -> Duration {
    run_case::<PathBuf>(1, 2, false, clients, iterations)
        .expect("mount PathBuf benchmark filesystem")
}

fn run_mapped_case(clients: usize, iterations: usize) -> Duration {
    run_case::<MappedInode>(1, 2, false, clients, iterations)
        .expect("mount MappedInode benchmark filesystem")
}

fn run_case<TId: BenchmarkId>(
    readers: usize,
    handler_workers: usize,
    clone_fuser_fd: bool,
    clients: usize,
    iterations: usize,
) -> Result<Duration, std::io::Error> {
    let mountpoint = TempDir::new().expect("create FUSE mountpoint");
    let session = spawn_mount(
        ConstantFs::<TId>::new(),
        mountpoint.path(),
        &[],
        Some(
            MountThreads::new(readers, handler_workers)
                .with_fuser_fd_cloning(clone_fuser_fd),
        ),
    )?;

    let path = mountpoint.path().join("bench-file");
    assert_eq!(std::fs::metadata(&path).unwrap().len(), FILE_SIZE);

    let barrier = Arc::new(Barrier::new(clients + 1));
    let tasks: Vec<_> = (0..clients)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..iterations {
                    black_box(std::fs::metadata(&path).unwrap());
                }
            })
        })
        .collect();
    let start = Instant::now();
    barrier.wait();
    for task in tasks {
        task.join().expect("benchmark client panicked");
    }
    let elapsed = start.elapsed();

    session.join().expect("unmount benchmark filesystem");
    Ok(elapsed)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
