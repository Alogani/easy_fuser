use std::time::Instant;

#[cfg(feature = "deadlock_detection")]
pub(crate) fn spawn_deadlock_checker() {
    use log::{error, info};
    use parking_lot::deadlock;
    use std::thread;
    use std::time::Duration;

    // Create a background thread which checks for deadlocks every 10s
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                info!("# No deadlock");
                continue;
            }

            eprintln!("# {} deadlocks detected", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                error!("Deadlock #{}", i);
                for t in threads {
                    error!("Thread Id {:#?}\n, {:#?}", t.thread_id(), t.backtrace());
                }
            }
        }
    });
}

pub(crate) fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}
