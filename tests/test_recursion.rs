use easy_fuser::spawn_mount;
use easy_fuser::templates::{DefaultFuseHandler, MirrorFs};

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
    let mount_path = source_path.join("mount");

    // Ensure the directories are clean
    let _ = fs::remove_dir_all(&source_path);
    fs::create_dir_all(&source_path).unwrap();
    fs::create_dir_all(&mount_path).unwrap();

    #[cfg(feature = "serial")]
    let num_threads = 1;
    #[cfg(feature = "parallel")]
    let num_threads = 4;

    // Create and mount the MirrorFs
    let fs = MirrorFs::new(source_path.clone(), DefaultFuseHandler::new());
    let session = spawn_mount(fs, &mount_path, &[], num_threads).unwrap();

    // Allow some time for the filesystem to mount
    std::thread::sleep(Duration::from_secs(1));

    // Construct the deep recursive path
    let deep_path = mount_path
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

    // Clean up
    session.join();
    let _ = fs::remove_dir_all(&source_path);

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
}
