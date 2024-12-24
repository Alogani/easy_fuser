#![allow(unused_imports)]

use std::{
    collections::{HashMap, VecDeque},
    ffi::{OsStr, OsString},
};

use super::inode_mapping::FileIdResolver;
use crate::fuse_handler::FuseHandler;
use crate::types::*;

type DirIter<T> = HashMap<(u64, i64), VecDeque<(OsString, u64, T)>>;

#[cfg(feature = "serial")]
mod serial {
    use super::*;

    use std::cell::RefCell;

    pub struct FuseDriver<T, U, R>
    where
        T: FileIdType,
        U: FuseHandler<T>,
        R: FileIdResolver<FileIdType = T>,
    {
        handler: U,
        resolver: R,
        dirmap_iter: RefCell<DirIter<FileKind>>,
        dirmapplus_iter: RefCell<DirIter<FileAttribute>>,
    }

    impl<T, U, R> FuseDriver<T, U, R>
    where
        T: FileIdType,
        U: FuseHandler<T>,
        R: FileIdResolver<FileIdType = T>,
    {
        pub fn new(handler: U, resolver: R, _num_threads: usize) -> FuseDriver<T, U, R> {
            FuseDriver {
                handler,
                resolver,
                dirmap_iter: RefCell::new(HashMap::new()),
                dirmapplus_iter: RefCell::new(HashMap::new()),
            }
        }

        pub fn get_handler(&self) -> &U {
            &self.handler
        }

        pub fn get_resolver(&self) -> &R {
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
    use super::*;

    use std::sync::{Arc, Mutex, MutexGuard};
    use threadpool::ThreadPool;

    pub struct FuseDriver<T, U, R>
    where
        T: FileIdType,
        U: FuseHandler<T>,
        R: FileIdResolver<FileIdType = T>,
    {
        handler: Arc<U>,
        resolver: Arc<R>,
        dirmap_iter: Arc<Mutex<DirIter<FileKind>>>,
        dirmapplus_iter: Arc<Mutex<DirIter<FileAttribute>>>,
        pub threadpool: ThreadPool,
    }

    impl<T, U, R> FuseDriver<T, U, R>
    where
        T: FileIdType,
        U: FuseHandler<T>,
        R: FileIdResolver<FileIdType = T>,
    {
        pub fn new(handler: U, resolver: R, num_threads: usize) -> FuseDriver<T, U, R> {
            FuseDriver {
                handler: Arc::new(handler),
                resolver: Arc::new(resolver),
                dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
                dirmapplus_iter: Arc::new(Mutex::new(HashMap::new())),
                threadpool: ThreadPool::new(num_threads),
            }
        }

        pub fn get_handler(&self) -> Arc<U> {
            self.handler.clone()
        }

        pub fn get_resolver(&self) -> Arc<R> {
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

    pub struct FuseDriver {
        // specific async implementation here
    }

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            tokio::spawn(async move { $block });
        };
    }

    pub(crate) use execute_task;
}

#[cfg(feature = "serial")]
pub use serial::*;

#[cfg(feature = "parallel")]
pub use parallel::*;

#[cfg(feature = "async")]
pub use async_task::*;
