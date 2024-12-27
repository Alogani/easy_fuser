use easy_fuser::spawn_mount;
use easy_fuser::templates::{mirror_fs::*, DefaultFuseHandler};

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
fn test_mirror_fs_operations() {
    println!("1. Starting test_mirror_fs_operations");
    
    // Create temporary directories for mount point and source
    println!("2. Creating temporary directories");
    let mount_dir = TempDir::new().unwrap();
    println!("3. Created mount_dir");
    let source_dir = TempDir::new().unwrap();
    println!("4. Created source_dir");

    println!("5. Getting paths");
    let mntpoint = mount_dir.path().to_path_buf();
    let source_path = source_dir.path().to_path_buf();
    
    println!("6. Setting num_threads");
    #[cfg(feature = "serial")]
    let num_threads = 1;
    #[cfg(feature = "parallel")]
    let num_threads = 4;

    // Create and mount the MirrorFs
    println!("7. Creating MirrorFs");
    let fs = MirrorFs::new(source_path.clone(), DefaultFuseHandler::new());
    println!("8. Spawning mount");
    let session = spawn_mount(fs, &mntpoint, &[], num_threads).unwrap();

    // Create a file and check its existence
    println!("9. Creating test file");
    let test_file = mntpoint.join("test_file.txt");
    File::create(&test_file).unwrap();
    println!("10. Asserting test file exists");
    assert!(test_file.exists());

    // Create a directory and check its existence
    println!("11. Creating test directory");
    let test_dir = mntpoint.join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    println!("12. Asserting test directory exists");
    assert!(test_dir.exists());

    // Write to a file and retrieve its content
    println!("13. Writing content to test file");
    let content = "Hello, MirrorFs!";
    fs::write(&test_file, content).unwrap();
    println!("14. Reading content from test file");
    let read_content = fs::read_to_string(&test_file).unwrap();
    println!("15. Asserting content matches");
    assert_eq!(content, read_content);

    // Remove a file
    println!("16. Removing test file");
    fs::remove_file(&test_file).unwrap();
    println!("17. Asserting test file doesn't exist");
    assert!(!test_file.exists());

    // Remove a directory
    println!("18. Removing test directory");
    fs::remove_dir(&test_dir).unwrap();
    println!("19. Asserting test directory doesn't exist");
    assert!(!test_dir.exists());

    // Print and modify file attributes
    println!("20. Creating new file for attribute test");
    let new_file = mntpoint.join("attribute_test.txt");
    File::create(&new_file).unwrap();

    println!("21. Getting initial metadata");
    let metadata = fs::metadata(&new_file).unwrap();
    println!("22. Initial permissions: {:o}", metadata.permissions().mode());
    println!("23. Initial owner: {}:{}", metadata.uid(), metadata.gid());

    // Change permissions
    println!("24. Changing permissions");
    let new_permissions = 0o644;
    fs::set_permissions(&new_file, fs::Permissions::from_mode(new_permissions)).unwrap();

    println!("25. Getting updated metadata");
    let updated_metadata = fs::metadata(&new_file).unwrap();
    println!(
        "26. Updated permissions: {:o}",
        updated_metadata.permissions().mode()
    );
    println!("27. Asserting permissions changed correctly");
    assert_eq!(
        updated_metadata.permissions().mode() & 0o777,
        new_permissions
    );

    // Truncate a file
    println!("28. Opening file for truncation");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&new_file)
        .unwrap();
    println!("29. Writing initial content");
    file.write_all(b"Initial content").unwrap();
    println!("30. Dropping file");
    drop(file);

    println!("31. Getting initial file length");
    let initial_len = fs::metadata(&new_file).unwrap().len();
    println!("32. Asserting initial length");
    assert_eq!(initial_len, 15);

    println!("33. Setting truncate length");
    let truncate_len = 5;
    println!("34. Opening file for truncation");
    let file = fs::OpenOptions::new().write(true).open(&new_file).unwrap();
    println!("35. Truncating file");
    file.set_len(truncate_len).unwrap();
    println!("36. Dropping file");
    drop(file);

    println!("37. Getting truncated file length");
    let truncated_len = fs::metadata(&new_file).unwrap().len();
    println!("38. Asserting truncated length");
    assert_eq!(truncated_len, truncate_len);

    // Clean up
    println!("39. Dropping session");
    drop(session);
    
    println!("40. Test completed");
}