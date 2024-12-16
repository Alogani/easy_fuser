use crate::{fuse_handler::FuseHandler, types::FileIdType};

use super::{callback_handlers::*, fuse_driver::FuseDriver, inode_mapping::FileIdResolver};

pub fn new_serial_driver<T, U>(
    fuse_handler: U,
) -> FuseDriver<T, SerialCallbackHandler<T, U>, impl FileIdResolver<Output = T>>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    FuseDriver::new(SerialCallbackHandler::new(fuse_handler), T::get_converter())
}

// TODO: new_parallel driver
// TODO: new_async_driver
