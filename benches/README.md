# Inode mapping performance benchmarks

These manual benchmarks are ignored by the normal test run. Use a release build:

```sh
cargo test --release --lib benchmark_request_paths -- --ignored --nocapture
cargo test --release --test performance -- --ignored --nocapture
```

`benchmark_request_paths` in `src/inode_mapping/resolver.rs` directly calls the in-process resolver. It isolates registration cost on `resolve_id` and on a `lookup`/`forget` cycle. Its `guard=false` rows bypass request registration **only for measurement**; that mode does not protect queued requests and is not the old repository baseline. It reports nanoseconds and operations per second for those direct resolver calls. These are **not FUSE requests per second**. Set `EASY_FUSER_BENCH_ITERS` and `EASY_FUSER_BENCH_SAMPLES` to change the run size.

`tests/performance.rs` mounts a parallel mirror filesystem with a zero attribute TTL. It measures metadata on one name, metadata on unique names, directory listing, create/unlink, rename, read, and write at 1, 4, and 8 client threads. The handler counters verify that each operation reached FUSE. The read and write cases include opening the file, and the unique-name metadata case can trigger more than one lookup request per call. Output includes both client operations per second and counted handler requests per second for the named operation. Direct I/O is enabled for read/write to avoid serving those calls from the kernel page cache. Set `EASY_FUSER_BENCH_ITERS` for metadata/listing/I/O, `EASY_FUSER_BENCH_CHURN_ITERS` for mutations, and `EASY_FUSER_BENCH_LOOKUP_ITERS` for unique-name lookups.

The mounted benchmark needs a working FUSE mount. Run baseline and candidate builds on the same machine, alternating their order and comparing several samples. Kernel scheduling, caches, and background load can move these numbers; the resolver microbenchmark is useful for isolating code costs, while the mounted benchmark shows whether they matter to callers.

## In-process resolver control

Within the candidate build, registration disabled versus enabled measured approximately 7.3M versus 5.3M direct `resolve_id` **operations/s** with one thread, and 10.6M versus 5.8M with eight threads. The direct `lookup`/`forget` cycle measured approximately 3.8M versus 3.2M resolver **operations/s** with one thread, and 0.92M versus 0.88M with eight threads. These figures isolate bookkeeping cost inside the process. They do not describe mounted FUSE throughput or contradict the mounted table above.
