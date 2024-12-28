use easy_fuser::spawn_mount;
use easy_fuser::templates::{mirror_fs::*, DefaultFuseHandler};

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
fn test_mirror_fs_operations() {
    env_logger::init();

    // Create temporary directories for mount point and source
    let mount_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();

    let mntpoint = mount_dir.path().to_path_buf();
    let source_path = source_dir.path().to_path_buf();
    #[cfg(feature = "serial")]
    let num_threads = 1;
    #[cfg(feature = "parallel")]
    let num_threads = 4;

    // Create and mount the MirrorFs
    let fs = MirrorFs::new(source_path.clone(), DefaultFuseHandler::new());
    let session = spawn_mount(fs, &mntpoint, &[], num_threads).unwrap();

    // Create a file and check its existence
    let test_file = mntpoint.join("test_file.txt");
    File::create(&test_file).unwrap();
    assert!(test_file.exists());

    // Create a directory and check its existence
    let test_dir = mntpoint.join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    assert!(test_dir.exists());

    // Write to a file and retrieve its content
    let content = "Hello, MirrorFs!";
    fs::write(&test_file, content).unwrap();
    let read_content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, read_content);

    // Remove a file
    fs::remove_file(&test_file).unwrap();
    assert!(!test_file.exists());

    // Remove a directory
    fs::remove_dir(&test_dir).unwrap();
    assert!(!test_dir.exists());

    // Print and modify file attributes
    let new_file = mntpoint.join("attribute_test.txt");
    File::create(&new_file).unwrap();

    let metadata = fs::metadata(&new_file).unwrap();
    println!("Initial permissions: {:o}", metadata.permissions().mode());
    println!("Initial owner: {}:{}", metadata.uid(), metadata.gid());

    // Change permissions
    let new_permissions = 0o644;
    fs::set_permissions(&new_file, fs::Permissions::from_mode(new_permissions)).unwrap();

    let updated_metadata = fs::metadata(&new_file).unwrap();
    println!(
        "Updated permissions: {:o}",
        updated_metadata.permissions().mode()
    );
    assert_eq!(
        updated_metadata.permissions().mode() & 0o777,
        new_permissions
    );

    // Truncate a file
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&new_file)
        .unwrap();
    file.write_all(b"Initial content").unwrap();
    drop(file);

    let initial_len = fs::metadata(&new_file).unwrap().len();
    assert_eq!(initial_len, 15);

    let truncate_len = 5;
    let file = fs::OpenOptions::new().write(true).open(&new_file).unwrap();
    file.set_len(truncate_len).unwrap();
    drop(file);

    let truncated_len = fs::metadata(&new_file).unwrap().len();
    assert_eq!(truncated_len, truncate_len);

    // Clean up
    drop(session);
}
