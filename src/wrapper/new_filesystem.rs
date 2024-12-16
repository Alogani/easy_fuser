use crate::{types::FileIdType, FuseAPI};

use super::{fuse_serial::FuseSerial, fuser_wrapper::FuseFilesystem, inode_mapping::FileIdResolver};

pub fn new_filesystem<T, U>(fuse_api: U) -> FuseFilesystem<T, FuseSerial<T, U>, impl FileIdResolver<Output = T>>
where
    T: FileIdType,
    U: FuseAPI<T>,
{
    FuseFilesystem::new(
        FuseSerial::new(fuse_api), T::get_converter()
    )
}
