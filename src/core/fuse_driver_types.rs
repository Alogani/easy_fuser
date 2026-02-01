#![allow(unused_imports)]

use std::{
    collections::{HashMap, VecDeque},
    ffi::{OsStr, OsString},
};

use super::inode_mapping::FileIdResolver;
use crate::fuse_handler::FuseHandler;
use crate::types::*;

type DirIter<TAttr> = HashMap<(u64, i64), VecDeque<(OsString, u64, TAttr)>>;

#[cfg(feature = "serial")]
mod serial {
    include!(concat!(env!("OUT_DIR"), "/serial/fuse_driver.rs"));

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            $block
        };
    }

    macro_rules! reply_executor {
        ($self:expr) => {
            ()
        };
    }

    macro_rules! execute_reply_task {
        ($reply_executor:expr, $block:block) => {
            $block
        };
    }

    pub(crate) use execute_reply_task;
    pub(crate) use execute_task;
    pub(crate) use reply_executor;
}

#[cfg(feature = "parallel")]
mod parallel {
    include!(concat!(env!("OUT_DIR"), "/parallel/fuse_driver.rs"));

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            $self.threadpool.execute(move || $block)
        };
    }

    macro_rules! reply_executor {
        ($self:expr) => {
            $self.reply_threadpool.clone()
        };
    }

    macro_rules! execute_reply_task {
        ($reply_executor:expr, $block:block) => {
            $reply_executor.execute(move || $block);
        };
    }

    pub(crate) use execute_reply_task;
    pub(crate) use execute_task;
    pub(crate) use reply_executor;
}

#[cfg(feature = "async")]
mod async_task {
    include!(concat!(env!("OUT_DIR"), "/async/fuse_driver.rs"));

    macro_rules! execute_task {
        ($self:expr, $block:block) => {
            $self.runtime.spawn(async move { $block })
        };
    }

    macro_rules! reply_executor {
        ($self:expr) => {
            $self.runtime.clone()
        };
    }

    macro_rules! execute_reply_task {
        ($reply_executor:expr, $block:block) => {
            $reply_executor.spawn(async move { $block })
        };
    }

    pub(crate) use execute_reply_task;
    pub(crate) use execute_task;
    pub(crate) use reply_executor;
}

#[cfg(feature = "deadlock_detection")]
fn spawn_deadlock_checker() {
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

#[cfg(feature = "serial")]
pub use serial::*;

#[cfg(feature = "parallel")]
pub use parallel::*;

#[cfg(feature = "async")]
pub use async_task::*;
