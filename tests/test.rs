use std::path::PathBuf;

use easy_fuser::templates;
use easy_fuser::*;

use tempfile::TempDir;

//cargo test --test test -- --nocapture
/*
#[test]
fn mount_test() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();

    let mntpoint = TempDir::new().unwrap();
    let fs = templates::PassthroughFs::new(PathBuf::from("/tmp"));
    let fuse = new_filesystem(fs);
    println!("MOUNTPOINT={:?}", mntpoint.path());
    let r = mount2(fuse, mntpoint.path(), &[]);
    print!("{:?}", r);
    drop(mntpoint);
}
*/
