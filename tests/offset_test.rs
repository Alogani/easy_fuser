use easy_fuser::spawn_mount;
use easy_fuser::templates::{mirror_fs::*, DefaultFuseHandler};

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use tempfile::TempDir;

#[test]
fn test_mirror_fs_file_offsets() {
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

    // Create a test file
    let test_file = mntpoint.join("offset_test.txt");
    let mut file = File::create(&test_file).unwrap();

    // Write initial content
    file.write_all(b"Hello, World!").unwrap();
    file.sync_all().unwrap();

    // Test reading from different offsets
    let mut file = File::open(&test_file).unwrap();
    let mut buffer = [0u8; 5];

    // Read from the beginning
    file.seek(SeekFrom::Start(0)).unwrap();
    file.read_exact(&mut buffer).unwrap();
    assert_eq!(&buffer, b"Hello");

    // Read from an offset
    file.seek(SeekFrom::Start(7)).unwrap();
    file.read_exact(&mut buffer).unwrap();
    assert_eq!(&buffer, b"World");

    // Test writing at different offsets
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&test_file)
        .unwrap();

    // Write at an offset
    file.seek(SeekFrom::Start(7)).unwrap();
    file.write_all(b"Rust!").unwrap();
    file.sync_all().unwrap();

    // Verify the write
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    // "Hello, World!" contains one character more
    assert_eq!(content, "Hello, Rust!!");

    // Test seeking relative to current position
    file.seek(SeekFrom::Start(0)).unwrap();
    file.seek(SeekFrom::Current(7)).unwrap();
    file.read_exact(&mut buffer).unwrap();
    assert_eq!(&buffer, b"Rust!");

    // Test seeking relative to the end
    file.seek(SeekFrom::End(-1)).unwrap();
    file.read_exact(&mut buffer[0..1]).unwrap();
    assert_eq!(buffer[0], b'!');

    // Test writing beyond the end of the file
    file.seek(SeekFrom::End(0)).unwrap();
    file.write_all(b" Extended").unwrap();
    file.sync_all().unwrap();

    // Verify the extended content
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut extended_content = String::new();
    file.read_to_string(&mut extended_content).unwrap();
    assert_eq!(extended_content, "Hello, Rust!! Extended");

    drop(session);
}
