use std::path::PathBuf;

use tempfile::TempDir;

use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use easy_fuser::templates::MirrorFs;

/*
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
}*/

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();

    //spawn_deadlock_checker();

    let mntpoint = TempDir::new().unwrap();
    println!("MOUNTPOINT=", mntpoint);
    //let fs = FileDescriptorBridge::<PathBuf>::new(BaseFuse::new());
    let fs = MirrorFs::new(PathBuf::from("/tmp/test"), DefaultFuseHandler::new());
    let fuse = new_serial_driver(fs);
    let r = mount(fuse, mntpoint.path(), &[]);
    print!("{:?}", r);
    drop(mntpoint);
}
