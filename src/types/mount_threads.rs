/// Thread counts for the FUSE request readers and easy_fuser operation workers.
///
/// FUSE reader threads receive kernel requests. Handler threads run filesystem
/// callbacks in parallel mode, or provide Tokio runtime workers in async mode.
/// `handler_threads` is unused by serial mode.
/// These counts are independent because increasing either can add scheduling
/// and synchronization overhead.
///
/// Passing `None` instead of a `MountThreads` value to `mount` or `spawn_mount`
/// selects this type's CPU-based default heuristic.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MountThreads {
    /// Number of fuser threads reading and dispatching kernel requests.
    pub fuser_threads: usize,
    /// Number of easy_fuser callback workers (or async runtime workers).
    pub handler_threads: usize,
    /// Give each fuser reader its own `/dev/fuse` descriptor on Linux.
    ///
    /// This can reduce contention when `fuser_threads` is greater than one.
    /// It requires Linux 4.5 or newer. `MountThreads::new` leaves this disabled;
    /// the CPU-based preset enables it when selecting multiple readers.
    pub clone_fuser_fd: bool,
}

impl MountThreads {
    /// Set the FUSE reader and callback worker counts independently.
    pub const fn new(fuser_threads: usize, handler_threads: usize) -> Self {
        Self {
            fuser_threads,
            handler_threads,
            clone_fuser_fd: false,
        }
    }

    /// Enable an independent FUSE device descriptor for each reader thread.
    pub const fn with_fuser_fd_cloning(mut self, enabled: bool) -> Self {
        self.clone_fuser_fd = enabled;
        self
    }

    /// Use the same count for fuser readers and easy_fuser handler workers.
    pub const fn same(count: usize) -> Self {
        Self::new(count, count)
    }
}

impl Default for MountThreads {
    /// Select a starting point based on the CPUs available to this process.
    ///
    /// The heuristic keeps one FUSE reader on machines with fewer than eight
    /// CPUs, and uses roughly one reader per eight CPUs on larger machines.
    /// Callback workers use one thread on a single-CPU machine, otherwise
    /// roughly half the available CPUs. Independent FUSE descriptors are
    /// enabled only when the heuristic selects multiple readers.
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(0);

        match cpus {
            0 | 1 => Self {
                fuser_threads: 1,
                handler_threads: 1,
                clone_fuser_fd: false,
            },
            2 => Self {
                fuser_threads: 1,
                handler_threads: 2,
                clone_fuser_fd: false,
            },
            n => {
                let fuser_threads = (n / 8).max(1);
                Self {
                    fuser_threads,
                    handler_threads: n.div_ceil(2),
                    clone_fuser_fd: fuser_threads > 1,
                }
            }
        }
    }
}
