use crate::types::*;

use super::fuse_handler::FuseHandler;
use crate::fuse_common::inode_mapping::*;

use easy_fuser_macro::implement_fuse_driver;

implement_fuse_driver!("async");
