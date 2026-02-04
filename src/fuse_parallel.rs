#![cfg(feature = "parallel")]

pub mod fuse_driver {
    include!(concat!(env!("OUT_DIR"), "/parallel/fuse_driver.rs"));
}
pub mod fuse_handler {
    include!(concat!(env!("OUT_DIR"), "/parallel/fuse_handler.rs"));
}
pub mod mounting {
    include!(concat!(env!("OUT_DIR"), "/parallel/mounting.rs"));
}

pub use fuse_handler::FuseHandler;
pub use mounting::*;


include!(concat!(env!("OUT_DIR"), "/parallel/preludes.rs"));