use std::marker::PhantomData;

use crate::fuse_api::FuseAPI;

use super::FuseCallbackAPI;
use crate::types::IdType;

pub struct FuseSerial<T, U>
where
    T: IdType,
    U: FuseAPI<T>,
{
    phantom: PhantomData<T>,
    fuse_api: U,
}

impl<T, U> FuseSerial<T, U>
where
    T: IdType,
    U: FuseAPI<T>,
{
    pub fn new(fuse_api: U) -> Self {
        Self {
            phantom: PhantomData,
            fuse_api,
        }
    }
}

impl<T, U> FuseCallbackAPI<T> for FuseSerial<T, U>
where
    T: IdType,
    U: FuseAPI<T>,
{
    fn get_fuse_impl(&self) -> &impl FuseAPI<T> {
        &self.fuse_api
    }
}
