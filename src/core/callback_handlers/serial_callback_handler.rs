use std::marker::PhantomData;

use crate::fuse_handler::FuseHandler;

use super::FuseCallbackHandler;
use crate::types::FileIdType;

pub struct SerialCallbackHandler<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    phantom: PhantomData<T>,
    fuse_api: U,
}

impl<T, U> SerialCallbackHandler<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    pub fn new(fuse_api: U) -> Self {
        Self {
            phantom: PhantomData,
            fuse_api,
        }
    }
}

impl<T, U> FuseCallbackHandler<T> for SerialCallbackHandler<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    fn get_fuse_handler(&self) -> &impl FuseHandler<T> {
        &self.fuse_api
    }
}
