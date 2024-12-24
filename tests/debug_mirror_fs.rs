use easy_fuser::mount;
use easy_fuser::templates::{DefaultFuseHandler, MirrorFs};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

// cargo test --package easy_fuser --test debug_mirror_fs --features "parallel deadlock_detection" -- debug_mirror_fs_mount --nocapture --ignored

#[test]
#[ignore]
fn debug_mirror_fs_mount() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();

    // Set up temporary directories for mount point and source
    let mount_dir = PathBuf::from("/tmp/easy_fuser_debug_mount");
    let source_dir = PathBuf::from("/tmp/easy_fuser_debug_source");

    // Create directories if they don't exist
    let _ = std::process::Command::new("fusermount")
        .arg("-u")
        .arg(&mount_dir)
        .status();
    let _ = std::fs::create_dir(&mount_dir);
    if !source_dir.exists() {
        let _ = std::fs::create_dir(&source_dir);
    }

    println!("Mount point: {:?}", mount_dir);
    println!("Source directory: {:?}", source_dir);

    // Create the MirrorFs
    let fs = MirrorFs::new(source_dir, DefaultFuseHandler::new());

    // Mount the filesystem
    println!("Mounting MirrorFs...");
    let mount_result = mount(fs, &mount_dir, &[], 1);

    match mount_result {
        Ok(_) => {
            println!("MirrorFs mounted successfully. Press Ctrl+C to unmount and exit.");
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        Err(e) => {
            eprintln!("Failed to mount MirrorFs: {:?}", e);
        }
    }

    // Note: This part will only be reached if mounting fails
    println!("Exiting debug mount.");
}
