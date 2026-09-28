//! Maps FUSE inode numbers to directory entries and resolves handler IDs.
//!
//! A directory entry is a `(parent inode, name)` pair. Multiple entries may
//! name the same inode after [`InodeMapper::link`] succeeds.

mod mapper;
mod resolver;

pub use mapper::*;
pub(crate) use resolver::{FileIdResolver, InodeResolvable};
