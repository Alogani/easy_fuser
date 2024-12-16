use std::path::PathBuf;

use easy_fuser::templates::{PassthroughFs, BaseFuse};
use easy_fuser::*;

use tempfile::TempDir;
use templates::FileDescriptorBridge;
use types::Inode;

fn spawn_deadlock_checker() {
    #[cfg(feature = "deadlock_detection")]
    { // only for #[cfg]
    use std::thread;
    use std::time::Duration;
    use parking_lot::deadlock;

    // Create a background thread which checks for deadlocks every 10s
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                eprintln!("no deadlok");
                continue;
            }

            println!("{} deadlocks detected", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                println!("Deadlock #{}", i);
                for t in threads {
                    println!("Thread Id {:#?}", t.thread_id());
                    println!("{:#?}", t.backtrace());
                }
            }
        }
    });
    }
}

//cargo test --test test -- --nocapture
#[test]
fn mount_test() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();

    spawn_deadlock_checker();

    let mntpoint = TempDir::new().unwrap();
    //let fs = FileDescriptorBridge::<PathBuf>::new(BaseFuse::new());
    let fs = PassthroughFs::new(PathBuf::from("/tmp/test"), BaseFuse::new_with_panic());
    let fuse = new_filesystem(fs);
    println!("MOUNTPOINT={:?}", mntpoint.path());
    let r = mount2(fuse, mntpoint.path(), &[]);
    print!("{:?}", r);
    drop(mntpoint);
}
