use crate::{types::IdType, FuseAPI};

use super::{fuse_serial::FuseSerial, fuser_wrapper::FuseFilesystem, inode_mapping::IdConverter};

pub fn new_filesystem<T, U>(fuse_api: U) -> FuseFilesystem<T, FuseSerial<T, U>, impl IdConverter<Output = T>>
where
    T: IdType,
    U: FuseAPI<T>,
{
    FuseFilesystem::new(
        FuseSerial::new(fuse_api), T::get_converter()
    )
}
