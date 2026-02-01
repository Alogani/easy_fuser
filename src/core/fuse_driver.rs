#[cfg(feature = "serial")]
mod serial {
    include!(concat!(env!("OUT_DIR"), "/serial/fuse_driver.rs"));
}

#[cfg(feature = "parallel")]
mod parallel {
    include!(concat!(env!("OUT_DIR"), "/parallel/fuse_driver.rs"));

}

#[cfg(feature = "async")]
mod async_task {
    include!(concat!(env!("OUT_DIR"), "/async/fuse_driver.rs"));
}

#[cfg(feature = "serial")]
pub(crate) use serial::*;

#[cfg(feature = "parallel")]
pub(crate) use parallel::*;

#[cfg(feature = "async")]
pub(crate) use async_task::*;

