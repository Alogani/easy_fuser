#![allow(unused_imports)]

use std::{
    collections::{HashMap, VecDeque},
    ffi::{OsStr, OsString},
};

use super::inode_mapping::FileIdResolver;
use crate::fuse_handler::FuseHandler;
use crate::types::*;

type DirIter<TAttr> = HashMap<(u64, i64), VecDeque<(OsString, u64, TAttr)>>;

#[cfg(feature = "serial")]
mod serial {
    use super::*;

    use std::cell::RefCell;

    pub(crate) struct FuseDriver<TId, THandler>
    where
        TId: FileIdType,
        THandler: FuseHandler<TId>,
    {
        handler: THandler,
        resolver: TId::Resolver,
        dirmap_iter: RefCell<DirIter<FileKind>>,
        dirmapplus_iter: RefCell<DirIter<FileAttribute>>,
    }

    impl<TId, THandler> FuseDriver<TId, THandler>
    where
        TId: FileIdType,
        THandler: FuseHandler<TId>,
    {
        pub fn new(handler: THandler, _num_threads: usize) -> FuseDriver<TId, THandler> {
            FuseDriver {
                handler,
                resolver: TId::Resolver::new(),
                dirmap_iter: RefCell::new(HashMap::new()),
                dirmapplus_iter: RefCell::new(HashMap::new()),
            }
        }

        pub fn get_handler(&self) -> &THandler {
            &self.handler
        }

        pub fn get_resolver(&self) -> &TId::Resolver {
            &self.resolver
        }

        pub fn get_dirmap_iter(&self) -> &RefCell<DirIter<FileKind>> {
            &self.dirmap_iter
        }

        pub fn get_dirmapplus_iter(&self) -> &RefCell<DirIter<FileAttribute>> {
            &self.dirmapplus_iter
        }
    }

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            $block
        };
    }

    pub(crate) use execute_task;
}

#[cfg(feature = "parallel")]
mod parallel {
    use crate::core::InodeResolvable;

    use super::*;

    use std::sync::Arc;

    use threadpool::ThreadPool;

    #[cfg(feature = "deadlock_detection")]
    use parking_lot::{Mutex, MutexGuard};
    #[cfg(not(feature = "deadlock_detection"))]
    use std::sync::{Mutex, MutexGuard};

    pub(crate) struct FuseDriver<TId, THandler>
    where
        TId: FileIdType,
        THandler: FuseHandler<TId>,
    {
        handler: Arc<THandler>,
        resolver: Arc<TId::Resolver>,
        dirmap_iter: Arc<Mutex<DirIter<FileKind>>>,
        dirmapplus_iter: Arc<Mutex<DirIter<FileAttribute>>>,
        pub threadpool: ThreadPool,
    }

    impl<TId, THandler> FuseDriver<TId, THandler>
    where
        TId: FileIdType,
        THandler: FuseHandler<TId>,
    {
        pub fn new(handler: THandler, num_threads: usize) -> FuseDriver<TId, THandler> {
            #[cfg(feature = "deadlock_detection")]
            spawn_deadlock_checker();
            FuseDriver {
                handler: Arc::new(handler),
                resolver: Arc::new(TId::create_resolver()),
                dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
                dirmapplus_iter: Arc::new(Mutex::new(HashMap::new())),
                threadpool: ThreadPool::new(num_threads),
            }
        }

        pub fn get_handler(&self) -> Arc<THandler> {
            self.handler.clone()
        }

        pub fn get_resolver(&self) -> Arc<TId::Resolver> {
            self.resolver.clone()
        }

        pub fn get_dirmap_iter(&self) -> Arc<Mutex<DirIter<FileKind>>> {
            self.dirmap_iter.clone()
        }

        pub fn get_dirmapplus_iter(&self) -> Arc<Mutex<DirIter<FileAttribute>>> {
            self.dirmapplus_iter.clone()
        }
    }

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            $self.threadpool.execute(move || $block);
        };
    }

    pub(crate) use execute_task;
}

#[cfg(feature = "async")]
mod async_task {
    use super::*;

    use std::sync::Arc;
    use tokio::runtime::Runtime;
    use tokio::sync::Mutex;

    pub(crate) struct FuseDriver<TId, THandler>
    where
        TId: FileIdType,
        THandler: FuseHandler<TId>,
    {
        handler: Arc<THandler>,
        resolver: Arc<TId::Resolver>,
        dirmap_iter: Arc<Mutex<DirIter<FileKind>>>,
        dirmapplus_iter: Arc<Mutex<DirIter<FileAttribute>>>,
        pub runtime: Runtime,
    }

    impl<TId, THandler> FuseDriver<TId, THandler>
    where
        TId: FileIdType,
        THandler: FuseHandler<TId>,
        TResolver: TId::Resolver,
    {
        pub fn new(handler: THandler, _num_threads: usize) -> FuseDriver<TId, THandler> {
            #[cfg(feature = "deadlock_detection")]
            spawn_deadlock_checker();
            FuseDriver {
                handler: Arc::new(handler),
                resolver: Arc::new(TId::Resolver::new()),
                dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
                dirmapplus_iter: Arc::new(Mutex::new(HashMap::new())),
                runtime: Runtime::new().unwrap(),
            }
        }

        pub fn get_handler(&self) -> Arc<THandler> {
            self.handler.clone()
        }

        pub fn get_resolver(&self) -> Arc<TId::Resolver> {
            self.resolver.clone()
        }

        pub fn get_dirmap_iter(&self) -> Arc<tokio::sync::Mutex<DirIter<FileKind>>> {
            self.dirmap_iter.clone()
        }

        pub fn get_dirmapplus_iter(&self) -> Arc<tokio::sync::Mutex<DirIter<FileAttribute>>> {
            self.dirmapplus_iter.clone()
        }
    }

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            $self.runtime.spawn(async move { $block });
        };
    }

    pub(crate) use execute_task;
}

#[cfg(feature = "deadlock_detection")]
fn spawn_deadlock_checker() {
    use log::{error, info};
    use parking_lot::deadlock;
    use std::thread;
    use std::time::Duration;

    // Create a background thread which checks for deadlocks every 10s
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(10));
        let deadlocks = deadlock::check_deadlock();
        if deadlocks.is_empty() {
            info!("# No deadlock");
            continue;
        }

        eprintln!("# {} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            error!("Deadlock #{}", i);
            for t in threads {
                error!("Thread Id {:#?}\n, {:#?}", t.thread_id(), t.backtrace());
            }
        }
    });
}

#[cfg(feature = "serial")]
pub use serial::*;

#[cfg(feature = "parallel")]
pub use parallel::*;

#[cfg(feature = "async")]
pub use async_task::*;
