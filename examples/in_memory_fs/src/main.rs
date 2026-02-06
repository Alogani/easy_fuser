#![doc = include_str!("../README.md")]

use easy_fuser::prelude::*;
use std::path::Path;
use std::fs;

const README_CONTENT: &[u8] = include_bytes!("../README.md") as &[u8];

mod filesystem;
pub use filesystem::InMemoryFS;

fn create_memory_fs() -> InMemoryFS {
    let memoryfs = InMemoryFS::new();
    // NOTE: manual call example here is removed because the [`CreateHelper`]
    // parameter is not supported
    memoryfs
}

fn main() {
    #[cfg(feature = "logging")]
    std::env::set_var("RUST_BACKTRACE", "full");
    #[cfg(feature = "logging")]
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();

    let mountpoint = std::env::args()
        .nth(1)
        .expect("Usage: in_memory_fs <MOUNTPOINT>");
    let config = MountConfig {
        mount_options: vec![],
        acl: SessionACL::Owner,
        num_threads: 4,
    };
    let memoryfs = create_memory_fs();

    println!("Mounting filesystem...");
    let session = easy_fuser::spawn_mount(memoryfs, Path::new(&mountpoint), &config).unwrap();
    println!("Filesystem mounted");
    fs::write(Path::new(&mountpoint).join("README.md"), README_CONTENT)
        .expect("Failed to write README.md");

    std::io::stdin().read_line(&mut String::new()).unwrap();
    session
        .join(&[])
        .map_err(|(_session, error)| {
            println!("Error unmounting filesystem: {:?}", error);
            error
        })
        .unwrap();
    println!("Filesystem unmounted");
}
