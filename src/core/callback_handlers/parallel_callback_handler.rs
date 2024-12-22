#![cfg(feature = "threadpool")]
use std::sync::Arc;

use threadpool::ThreadPool;

use std::marker::PhantomData;

use crate::{
    fuse_handler::FuseHandler,
    types::{FUSEOpenFlags, FileHandle, RequestInfo},
};

use super::{FuseCallbackHandler, ReplyCb};
use crate::types::FileIdType;

pub struct ParallelCallbackHandler<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    phantom: PhantomData<T>,
    fuse_api: U,
    threadpool: ThreadPool,
}

impl<T, U> ParallelCallbackHandler<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    pub fn new(fuse_api: U, num_cpus: u64) -> Self {
        Self {
            phantom: PhantomData,
            fuse_api,
            threadpool: ThreadPool::new(num_cpus),
        }
    }
}

impl<T, U> FuseCallbackHandler<T> for ParallelCallbackHandler<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    fn get_fuse_handler(&self) -> &impl FuseHandler<T> {
        &self.fuse_api
    }

    fn read(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEOpenFlags,
        lock_owner: Option<u64>,
        callback: ReplyCb<Vec<u8>>,
    ) {
        self.threadpool.execute(move || {
            callback(self.get_fuse_handler().read(
                req,
                file,
                file_handle,
                offset,
                size,
                flags,
                lock_owner,
            ))
        });
    }
}
