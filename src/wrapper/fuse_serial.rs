use std::marker::PhantomData;

use crate::fuse_api::FuseAPI;

use super::{FuseCallbackAPI, IdType};

pub struct FuseSerial<T, U>
where
    T: FuseAPI<U>,
    U: IdType,
{
    fuse_api: T,
    phantom: PhantomData<U>
}

impl<T, U> FuseSerial<T, U>
where
    T: FuseAPI<U>,
    U: IdType,
{
    pub fn new(fuse_api: T) -> Self {
        Self {
            fuse_api,
            phantom: PhantomData
        }
    }
}

impl<T, U> FuseCallbackAPI<U> for FuseSerial<T, U>
where
    T: FuseAPI<U>,
    U: IdType,
{
    fn get_fuse_impl(&self) -> &impl FuseAPI<U> {
        &self.fuse_api
    }
}