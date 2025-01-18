use crate::types::*;

use easy_fuser_macro::implement_fuse_handler;

implement_fuse_handler!("async");

/* # Examples
struct MyFuse {
    inner: Box<dyn FuseHandler<Inode>>,
}

#[async_trait]
impl FuseHandler<Inode> for MyFuse {
    fn get_inner(&self) -> &dyn FuseHandler<Inode> {
        self.inner.as_ref()
    }

    async fn read(
        &self,
        _req: &RequestInfo,
        _file_id: Inode,
        _file_handle: BorrowedFileHandle<'_>,
        _seek: SeekFrom,
        _size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        return Ok(Vec::new());
    }
}
 */
