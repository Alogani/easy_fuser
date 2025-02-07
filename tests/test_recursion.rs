use easy_fuser::prelude::*;
use easy_fuser::templates::{mirror_fs::*, DefaultFuseHandler};

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// In theory, this test shouldn't work, but for an unknown reason it works
///
/// However, when trying this kind of mount in a terminal, it hangs

#[test]
fn test_mirror_fs_recursion() {
    // Create source and mount directories
    let source_path = PathBuf::from("/tmp/easy_fuser_recursion_fs_source");
    let mntpoint = source_path.join("mount");

    // Ensure the directories are clean
    let _ = fs::remove_dir_all(&source_path);
    fs::create_dir_all(&source_path).unwrap();
    fs::create_dir_all(&mntpoint).unwrap();

    // We won't use spawn_mount because it MirrorFs doesn't implement Send in serial mode
    let mntpoint_clone = mntpoint.clone();
    let handle = std::thread::spawn(move || {
        let fs = MirrorFs::new(source_path.clone(), DefaultFuseHandler::new());
        #[cfg(feature = "serial")]
        mount(fs, &mntpoint_clone, &[]).unwrap();
        #[cfg(not(feature = "serial"))]
        mount(fs, &mntpoint_clone, &[], 4).unwrap();
    });
    std::thread::sleep(Duration::from_millis(50)); // Wait for the mount to finish

    // Allow some time for the filesystem to mount
    std::thread::sleep(Duration::from_secs(1));

    // Construct the deep recursive path
    let deep_path = mntpoint
        .join("easy_fuser_recursion_fs_source")
        .join("mount")
        .join("easy_fuser_recursion_fs_source")
        .join("mount")
        .join("easy_fuser_recursion_fs_source")
        .join("mount");

    // Try to list the contents of the deep recursive path
    let start_time = Instant::now();
    let output = Command::new("ls")
        .arg(&deep_path)
        .output()
        .expect("Failed to execute ls command");

    let elapsed_time = start_time.elapsed();

    // Check if the command completed within 5 seconds
    if elapsed_time >= Duration::from_secs(5) {
        panic!("Test failed: 'ls' command took 5 seconds or more, indicating a potential infinite recursion.");
    }

    // Check the output of the 'ls' command
    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("ls output: {}", output_str);
    println!("ls error: {}", String::from_utf8_lossy(&output.stderr));

    // The exact output might depend on how your filesystem handles this case,
    // but it should not show an infinitely recursive structure
    assert!(
        !output_str.contains("easy_fuser_recursion_fs_source"),
        "The output suggests an infinitely recursive structure"
    );

    println!("Test passed: No infinite recursion detected.");

    eprintln!("Unmounting filesystem...");
    let _ = std::process::Command::new("fusermount")
        .arg("-u")
        .arg(&mntpoint)
        .status();
    handle.join().unwrap();
}
