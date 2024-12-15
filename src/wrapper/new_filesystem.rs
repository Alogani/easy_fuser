use crate::{types::IdType, FuseAPI};

use super::{fuse_serial::FuseSerial, fuser_wrapper::FuseFilesystem, inode_mapping::IdConverter, FuseCallbackAPI};

pub fn new_filesystem<T, U, V, C>(fuse_api: U) -> FuseFilesystem<T, V, C>
where
    T: IdType,
    U: FuseAPI<T>,
    V: FuseCallbackAPI<T>,
    C: IdConverter<Output = T>,
{
    FuseFilesystem::new(
        FuseSerial::new(fuse_api)
    )
}
