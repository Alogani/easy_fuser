use std::sync::Arc;

use fuse_api::ReplyCb;

use crate::types::*;
use crate::*;

use threadpool::ThreadPool;

// Make the read, write, flush and fsync functions run inside a threadpool
pub struct MultiThreadedFuse<T>
where T: FuseAPI + Send + Sync + 'static {
    api: Arc<T>,
    threadpool: ThreadPool
}

impl<T> MultiThreadedFuse<T>
where T: FuseAPI + Send + Sync + 'static {
    pub fn new(api: T, num_threads: usize) -> Self {
        Self {
            api: Arc::new(api),
            threadpool: ThreadPool::new(num_threads),
        }
    }
}

impl<T> FuseAPI for MultiThreadedFuse<T>
where T: FuseAPI + Send + Sync + 'static {
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
        let api = Arc::clone(&self.api);

        self.threadpool.execute(move || {
            api.read(req, ino, file_handle, offset, size, flags, lock_owner, callback);
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
        let api = Arc::clone(&self.api);
        let data = Vec::from(data);

        self.threadpool.execute(move || {
            api.write(_req, ino, file_handle, offset, &data, write_flags, flags, lock_owner, callback);
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
        let api = Arc::clone(&self.api);

        self.threadpool.execute(move || {
            api.flush(_req, ino, file_handle, lock_owner, callback);
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
        let api = Arc::clone(&self.api);

        self.threadpool.execute(move || {
            api.fsync(_req, ino, file_handle, datasync, callback);
        });
    }
}
