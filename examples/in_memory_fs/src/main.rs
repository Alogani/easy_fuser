#![doc = include_str!("../README.md")]

use easy_fuser::prelude::*;
use std::path::Path;
use std::fs;

const README_CONTENT: &[u8] = include_bytes!("../README.md") as &[u8];

mod filesystem;
pub use filesystem::InMemoryFS;

fn create_memory_fs() -> InMemoryFS {
    let memoryfs = InMemoryFS::new();
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
    let mut config = easy_fuser::prelude::Config::default();
    config.acl = easy_fuser::prelude::SessionACL::Owner;
    config.n_threads = Some(1);
    config.mount_options = vec![
        MountOption::RW,
        MountOption::FSName("in_memory_fs".to_string()),
    ];

    let memoryfs = create_memory_fs();

    println!("Mounting filesystem...");
    let session = easy_fuser::spawn_mount(memoryfs, Path::new(&mountpoint), &config).unwrap();
    println!("Filesystem mounted");
    fs::write(
        Path::new(&mountpoint).join("README.md"),
        README_CONTENT,
    )
    .expect("Failed to write README.md");

    std::io::stdin().read_line(&mut String::new()).unwrap();
    session.umount_and_join().unwrap();
    println!("Filesystem unmounted");
}
