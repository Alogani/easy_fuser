use easy_fuser::prelude::*;
use std::ffi::OsStr;

const README_CONTENT: &[u8] = include_bytes!("../README.md") as &[u8];

mod filesystem;
pub use filesystem::InMemoryFS;

fn create_memory_fs() -> InMemoryFS {
    let memoryfs = InMemoryFS::new();
    #[cfg(feature = "readme")]
    {
        let request_info = RequestInfo {id: 0, uid: 0, gid: 0, pid: 0}; // dummy RequestInfo
        let (fd, (inode, _), _) = memoryfs.create(
            &request_info, ROOT_INODE, OsStr::new("README.md"), 0, 0, OpenFlags::empty(),
        ).unwrap();
        let _ = memoryfs.write(
            &request_info, inode, fd, SeekFrom::Start(0), README_CONTENT.to_vec(), FUSEWriteFlags::empty(), OpenFlags::empty(), None,
        ).unwrap();
    }
    memoryfs
}

fn main() {
    let mountpoint = std::env::args().nth(1).expect("Usage: in_memory_fs <MOUNTPOINT>");
    let options = vec![MountOption::RW, MountOption::FSName("in_memory_fs".to_string())];

    let memoryfs = create_memory_fs();
    
    easy_fuser::mount(memoryfs, mountpoint.as_ref(), &options, 1).unwrap();
}