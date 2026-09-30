use easy_fuser::types::MountThreads;

pub const MOUNT_THREADS: MountThreads = if cfg!(target_os = "linux") {
    MountThreads::new(2, 4)
} else {
    MountThreads::new(1, 4)
};
