# FUSE throughput benchmarks

These ignored tests are manual benchmarks; they do not run in the normal test suite. Use release builds and a working Linux FUSE mount. All throughput printed by the mounted tests is aggregate operations per second across the stated number of client threads. Divide by the client count for an approximate per-client rate. Do not multiply by the server reader or handler counts.

## Reader and handler thread counts

Run the independent count matrix, including fuser's shared versus cloned `/dev/fuse` descriptors:

```sh
cargo test --release --no-default-features --features parallel \
  --test thread_scaling benchmark_reader_and_handler_thread_matrix \
  -- --ignored --nocapture
```

The test uses eight clients by default, zero attribute TTL, a constant-time `Inode` handler, and no counter in the timed callback. Each cell is aggregate `stat` operations per second. It tests 1, 2, 4, and 8 fuser readers against the same handler worker counts. With multiple readers it also tests `clone_fd`, which asks fuser to give each reader an independent FUSE device descriptor. Override `EASY_FUSER_BENCH_CLIENTS`, `EASY_FUSER_BENCH_ITERS`, and `EASY_FUSER_BENCH_SAMPLES` to change the load and sample count.

On this host, three no-clone samples with eight clients and 10,000 operations per client produced these median aggregate rates:

| FUSE readers | 1 handler worker | 2 handler workers | 4 handler workers | 8 handler workers |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 178k/s | **226k/s** | 203k/s | 166k/s |
| 2 | 182k/s | 213k/s | 200k/s | 176k/s |
| 4 | 140k/s | 184k/s | 158k/s | 155k/s |
| 8 | 159k/s | 159k/s | 147k/s | 140k/s |

For this CPU-light callback, one reader and two callback workers were the best measured setting. The CPU heuristic selects one reader and four workers on this 8-CPU host, which reached about 203k operations/s in this matrix. Explicit `MountThreads::new(1, 2)` reached about 226k/s. This is a measured starting point for short CPU-light callbacks, not a universal setting for handlers that spend time waiting on slow storage or remote services. Cloned descriptors did not show a consistent throughput improvement through eight readers; they are enabled by the automatic heuristic only when it selects more than one reader, or explicitly with `with_fuser_fd_cloning(true)`.

The public `MountThreads` setting lets callers tune `fuser_threads` and `handler_threads` independently; both fields are required when constructing one. `mount_with_threads` and `spawn_mount_with_threads` accept `Option<MountThreads>`: `Some(...)` uses the supplied settings, and `None` selects one reader/one worker on a one-CPU machine, one reader/two workers on a two-CPU machine, and otherwise `max(cpus / 8, 1)` readers with `ceil(cpus / 2)` workers. The heuristic enables cloned FUSE descriptors only when it selects more than one reader. The legacy `mount` and `spawn_mount` functions still use the same explicit `num_threads` value for both counts; when it is `None`, they use the same CPU heuristic. `MountThreads::with_fuser_fd_cloning(true)` explicitly enables fuser's Linux 4.5+ cloned-descriptor option.

## Comparing file ID types

Run the same zero-TTL mounted metadata workload with `Inode`, `PathBuf`, and `MappedInode`:

```sh
cargo test --release --no-default-features --features parallel \
  --test thread_scaling benchmark_file_id_types -- --ignored --nocapture
```

This test uses one reader and two handler workers. Its `MappedInode` handler calls `paths()` on every `getattr`, so that row includes the mapper read lock and path reconstruction. On this host, median aggregate rates across three samples were:

| Clients | `Inode` | `PathBuf` | `MappedInode` with `paths()` |
| ---: | ---: | ---: | ---: |
| 1 | 36.7k/s | 34.5k/s | 35.8k/s |
| 4 | 134k/s | 130k/s | 132k/s |
| 8 | 227k/s | 225k/s | 225k/s |

The mounted differences are within run-to-run variation. The resolver itself is measurable in isolation, but it was not the limit on this end-to-end short metadata workload.

## Resolver microbenchmarks

Run the direct in-process resolver benchmarks with:

```sh
cargo test --release --lib benchmark_id_resolution_costs -- --ignored --nocapture
cargo test --release --lib benchmark_request_paths -- --ignored --nocapture
```

`benchmark_id_resolution_costs` reports aggregate resolver calls per second for `Inode`, request guard registration, guarded and unguarded component and `PathBuf` resolution, `MappedInode` handle creation, and `MappedInode::parts_paths()` / `paths()`. Its path map contains three components. `benchmark_request_paths` measures `resolve_id` and a `lookup`/`forget` cycle. In both tests the unguarded rows bypass request registration only as a measurement control; they do not provide the lifetime safety required by queued FUSE callbacks.

Five resolver samples on this host measured guarded versus unguarded three-component `PathBuf` resolution at approximately:

| Concurrent resolver calls | Guarded | Guard bypass control |
| ---: | ---: | ---: |
| 1 thread | 3.24M/s | 4.22M/s |
| 4 threads | 4.69M/s | 8.94M/s |
| 8 threads | 5.05M/s | 9.75M/s |

The current guard registers against a shared epoch and its final active request takes the epoch-state write lock to check for retired inodes, even when there is nothing to reclaim. This creates a substantial direct resolver cost under contention. In the mounted ID-type matrix, aggregate throughput at eight clients is about 225k operations/s (roughly 4.4 microseconds per operation at that throughput); the isolated resolver calls take roughly 0.1–0.4 microseconds. A safer lower-overhead guard would need to avoid shared per-request coordination while preserving the guarantee that `forget` cannot reclaim mappings used by an earlier queued request. A prior “skip cleanup unless retired” prototype did not give a repeatable direct-resolver gain, so it was not kept.

Set `EASY_FUSER_BENCH_ITERS` and `EASY_FUSER_BENCH_SAMPLES` to change the microbenchmark duration.

## Async syscalls versus io_uring

The same mounted read/write benchmark can compare async backed by ordinary syscalls with async using the `io_uring` feature. Both run with one fuser reader, two runtime workers, direct I/O at the FUSE mount, eight clients, and per-client files. The async-only comparison is:

```sh
EASY_FUSER_BENCH_FUSER_THREADS=1 EASY_FUSER_BENCH_HANDLER_THREADS=2 \
EASY_FUSER_BENCH_ITERS=2000 EASY_FUSER_BENCH_SAMPLES=3 \
  cargo test --release --no-default-features --features async \
  --test io_performance -- --ignored --nocapture

EASY_FUSER_BENCH_FUSER_THREADS=1 EASY_FUSER_BENCH_HANDLER_THREADS=2 \
EASY_FUSER_BENCH_ITERS=2000 EASY_FUSER_BENCH_SAMPLES=3 \
  cargo test --release --no-default-features --features io_uring \
  --test io_performance -- --ignored --nocapture
```

`io_uring` includes the `async` feature. The test checks `io_uring_setup` first and skips if the host disallows it. The optional `parallel` build provides an additional reference if useful.

Two samples on this host produced these median aggregate rates with eight clients:

| Read/write size | Async syscall read | Async io_uring read | Async syscall write | Async io_uring write |
| ---: | ---: | ---: | ---: | ---: |
| 4 KiB | 344k/s | 35k/s | 279k/s | 36k/s |
| 64 KiB | 151k/s | 30k/s | 101k/s | 34k/s |

For the **current `easy_fuser` implementation**, this slowdown is expected; it is not evidence that io_uring itself is inherently slower. The current code creates a new `IoUring::new(8)` for each operation, submits one SQE, waits for one CQE, and drops the ring. Each request also goes through Tokio task, `oneshot`, and `AsyncFd` handling. This pays ring setup and per-request async coordination without getting the usual ring-reuse and batching benefits.

The measured ratios were about **9.8× slower** for 4 KiB reads, **7.8×** for 4 KiB writes, **5.0×** for 64 KiB reads, and **3.0×** for 64 KiB writes (roughly 3–10× overall). This warm, local backing-file test does not measure cold-storage latency or a device that can keep multiple real I/O requests in flight. Warm cached 4 KiB reads especially favor a plain `pread`. Before judging io_uring again, the implementation should use a persistent ring—perhaps one per runtime worker or a dedicated ring driver routing many concurrent submissions and completions—and rerun this benchmark. Keep the current feature experimental/opt-in; the results do not support presenting it as a throughput optimization or consolidating it as a general replacement for async syscalls.

## Existing operation mix

`tests/performance.rs` mounts a path-based mirror and measures repeated metadata, unique-name lookups, directory listing, create/unlink, rename, reads, and writes at 1, 4, and 8 client threads. It includes callback counters to validate dispatch, enables direct I/O for read/write, and includes opening files in the timed I/O loops. Configure it with `EASY_FUSER_BENCH_ITERS`, `EASY_FUSER_BENCH_CHURN_ITERS`, and `EASY_FUSER_BENCH_LOOKUP_ITERS`.

Run benchmarks on an otherwise idle host, repeat several samples, and compare medians. The mounted tests include Linux FUSE and syscall costs; the resolver tests isolate in-process work. They answer different questions and should not be compared as though their operations per second were interchangeable.
