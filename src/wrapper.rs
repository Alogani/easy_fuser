mod fuse_callback_api;
mod fuse_parallel;
mod fuse_serial;
mod fuser_wrapper;
mod inode_mapping;
pub use fuse_callback_api::FuseCallbackAPI;
pub use inode_mapping::IdType;

/*
pub fn new_filesystem<T, U, C>(fuse_cb_api: T) -> FuseFilesystem<T, U, C>
where
    T: FuseCallbackAPI<U>,
    U: IdType,
    C: IdConverter<Output = U>,
{
    FuseFilesystem {
        fs_impl: fuse_cb_api,
        converter: Arc::new(Mutex::new(C::new())),
        dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
        dirplus_iter: Arc::new(Mutex::new(HashMap::new())),
    }
}
    */
