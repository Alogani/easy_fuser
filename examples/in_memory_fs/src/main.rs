#![doc = include_str!("../README.md")]

use easy_fuser::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

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
    let options = vec![
        MountOption::RW,
        MountOption::FSName("in_memory_fs".to_string()),
    ];

    let memoryfs = create_memory_fs();

    println!("Mounting filesystem...");
    let session = easy_fuser::spawn_mount(memoryfs, Path::new(&mountpoint), &options, 1).unwrap();
    // Insert the readme here
    #[cfg(feature = "readme")]
    File::create(Path::new(&mountpoint).join("README.md"))
        .unwrap()
        .write_all(README_CONTENT)
        .unwrap();
    let mut wait_string = String::new();
    println!("Press Enter to unmount...");
    std::io::stdin().read_line(&mut wait_string).unwrap();
    session.unmount_and_join().unwrap();
}
