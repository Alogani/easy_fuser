#[cfg(feature = "serial")]
include!(concat!(env!("OUT_DIR"), "/serial/fuse_handler.rs"));

#[cfg(feature = "parallel")]
include!(concat!(env!("OUT_DIR"), "/parallel/fuse_handler.rs"));

#[cfg(feature = "async")]
include!(concat!(env!("OUT_DIR"), "/async/fuse_handler.rs"));
