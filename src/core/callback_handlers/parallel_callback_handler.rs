/*
An example based on the an earlier version of the API

#[cfg(feature = "threadpool")]
use std::sync::Arc;

use threadpool::ThreadPool;
extern crate num_cpus;

use crate::*;
use crate::types::*;

use fuse_api::ReplyCb;




// Make the read, write, flush and fsync functions run inside a threadpool
pub struct ThreadPoolFuse<T>
where
    T: FuseAPI + Send + Sync + 'static,
{
    sublayer: Arc<T>,
    threadpool: ThreadPool,
}

impl<T> ThreadPoolFuse<T>
where
    T: FuseAPI + Send + Sync + 'static,
{
    /// Uses the
    pub fn new(sublayer: T) -> Self {
        Self {
            sublayer: Arc::new(sublayer),
            threadpool: ThreadPool::new(num_cpus::get()),
        }
    }

    pub fn with_thread_count(sublayer: T, num_threads: usize) -> Self {
        Self {
            sublayer: Arc::new(sublayer),
            threadpool: ThreadPool::new(num_threads),
        }
    }
}

impl<T> FuseAPI for ThreadPoolFuse<T>
where
    T: FuseAPI + Send + Sync + 'static,
{
    fn get_sublayer(&self) -> &impl FuseAPI {
        self.sublayer.as_ref()
    }

    fn read(
        &self,
        req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
        callback: fuse_api::ReplyCb<Vec<u8>>,
    ) {
        let sublayer = Arc::clone(&self.sublayer);

        self.threadpool.execute(move || {
            sublayer.read(
                req,
                ino,
                file_handle,
                offset,
                size,
                flags,
                lock_owner,
                callback,
            );
        });
    }

    fn write(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        callback: ReplyCb<u32>,
    ) {
        let sublayer = Arc::clone(&self.sublayer);
        let data = Vec::from(data);

        self.threadpool.execute(move || {
            sublayer.write(
                _req,
                ino,
                file_handle,
                offset,
                &data,
                write_flags,
                flags,
                lock_owner,
                callback,
            );
        });
    }

    fn flush(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        lock_owner: u64,
        callback: ReplyCb<()>,
    ) {
        let sublayer = Arc::clone(&self.sublayer);

        self.threadpool.execute(move || {
            sublayer.flush(_req, ino, file_handle, lock_owner, callback);
        });
    }

    fn fsync(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        let sublayer = Arc::clone(&self.sublayer);

        self.threadpool.execute(move || {
            sublayer.fsync(_req, ino, file_handle, datasync, callback);
        });
    }
}

*/
