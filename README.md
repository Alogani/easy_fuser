# easy_fuser

[![CI Ubuntu](https://github.com/Alogani/easy_fuser/actions/workflows/ubuntu.yml/badge.svg?branch=main)](https://github.com/Alogani/easy_fuser/actions/workflows/ubuntu.yml?query=branch%3Amain)
[![Crates.io](https://img.shields.io/crates/v/easy_fuser.svg)](https://crates.io/crates/easy_fuser)
[![Documentation](https://docs.rs/easy_fuser/badge.svg)](https://docs.rs/easy_fuser)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/Alogani/easy_fuser/blob/master/LICENSE.md)
[![dependency status](https://deps.rs/repo/github/Alogani/easy_fuser/status.svg)](https://deps.rs/repo/github/Alogani/easy_fuser)

> [!IMPORTANT]
> The API is not stabilized, some breaking changes can still happen.
> See CHANGELOG.md to see it.
> This crate shall still be considered experimental and not production ready.

## About

`easy_fuser` is a high-level, ergonomic wrapper around the `fuser` crate, designed to simplify
the process of implementing FUSE (Filesystem in Userspace) filesystems in Rust. It abstracts away
many of the complexities, offering a more intuitive and Rust-idiomatic approach to filesystem development.

## Key Features

- **Simplified API**: Provides a higher-level interface compared to `fuser`, reducing boilerplate
  and making filesystem implementation more straightforward.

- **Flexible Concurrency Models**: Offers three distinct concurrency models to suit different
  use cases and performance requirements.

- **Flexible File Identification**: Supports both path-based and inode-based operations,
  allowing you to choose between `Inode`, `PathBuf`, or `MappedInode` as your file identifier type. This
  offers flexibility in how you represent and manage file identities, suitable for different
  filesystem structures and performance requirements.

- **Error Handling**: Provides a structured error handling system, facilitating the management
  of filesystem-specific errors.

- **Composable Presets and Examples**: Includes pre-built, composable presets and a comprehensive
  examples folder to help you get started quickly, understand various implementation patterns,
  and easily combine different filesystem behaviors. These presets are designed to be mixed
  and matched via delegation, allowing for flexible and modular filesystem creation.

## File Identification Flexibility

`easy_fuser` supports three file identifier types:

1. **`PathBuf`**: Work with paths relative to the mount root, without a leading `/`.
   A simple choice when you do not need to track hard links.
2. **`Inode`**: Work with inode numbers that you assign and manage yourself.
3. **`MappedInode`**: Work with inode numbers assigned by easy_fuser, with access to
   paths for hard links created through `FuseHandler::link`.

See the [`FileIdType` documentation](https://docs.rs/easy_fuser/latest/easy_fuser/types/trait.FileIdType.html)
for the trade-offs and edge cases of each choice.

## Object IDs and open resources

`TId` identifies a filesystem object. `FileHandle` is the owned resource or state for one specific
open instance. `open` and `create` return it; `FuseDriver` stores it and passes typed optional
access to later file operations. Use `()` when operations are stateless. If you delegate descriptor
operations to `FileDescriptorHandler` or `MirrorFs`, choose `std::os::fd::OwnedFd`; the helper
implements those callbacks for you, but it still needs the descriptor type. For your own per-open
state, define a type and use `Option<&mut Self::FileHandle>` in file callbacks. The [typed handle docs](src/types/file_handle.rs)
show a minimal example. The [ZIP example](examples/zip_fs/src/filesystem.rs) uses a
`Cursor<Vec<u8>>` to read from the entry loaded by `open`.

## Usage

To use `easy_fuser`, follow these steps:

1. Import the appropriate prelude for your concurrency mode (e.g. `easy_fuser::fuse_parallel::prelude::*`).
2. Implement the `FuseHandler` trait for your filesystem structure, specifying the `TId` type (e.g. `PathBuf`).
3. (Optional) Add preset values as fields in your filesystem struct and delegate selected operations to them with `delegate_fs!`.
4. Mount or spawn-mount your filesystem.

Here's a basic example:

```rust,ignore
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;
#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
#[cfg(all(feature = "async", not(feature = "parallel"), not(feature = "serial")))]
use easy_fuser::fuse_async::prelude::*;

use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use easy_fuser_macro::delegate_fs;
use std::path::{Path, PathBuf};

struct MyFS {
    defaults: StatelessHandler<PathBuf>,
    unimplemented: UnimplementedFuseHandler<PathBuf>,
}

impl FuseHandler for MyFS {
    type TId = PathBuf;
    type FileHandle = ();

    // These operations need no directory state in this filesystem.
    delegate_fs! { defaults, [ forget, fsyncdir, opendir, releasedir ] }

    // Return ENOSYS for operations this filesystem has not implemented.
    delegate_fs! { unimplemented, [
        access, bmap, copy_file_range, create, fallocate, flush, fsync,
        getattr, getlk, getxattr, ioctl, link, listxattr, lookup, lseek, mkdir, mknod,
        open, read, readdir, readlink, removexattr, rename,
        rmdir, setattr, setlk, setxattr, statfs, symlink, unlink, write
    ]}
}

fn main() -> std::io::Result<()> {
    let fs = MyFS {
        defaults: StatelessHandler::new(),
        unimplemented: UnimplementedFuseHandler::new(),
    };
    
    // Mount the filesystem, optionally configuring the number of threads.
    // In parallel mode, Some(4) runs FUSE handlers on 4 worker threads.
    // In serial mode, the thread count argument is ignored.
    // In async mode, the thread count argument configures tokio threads.
    // If you pass None, a default configuration is used.
    mount(fs, Path::new("/mnt/myfs"), &[], Some(4))?;
    
    Ok(())
}
```

## Presets / Templates

A preset is a helper that provides implementations for common operations. Presets do not implement your `FuseHandler`; add them as fields on your own filesystem type and delegate only the operations you want to use. You can implement an operation yourself, or add custom logic before calling a preset from your implementation.

Presets can be composed with `delegate_fs!`, or with `delegate_fs_async!` and `delegate_fs_sync_to_async!` in async handlers.

### Available presets

`easy_fuser` provides a set of template implementations (presets) under the `easy_fuser::fuse_presets` module to help you get started quickly:

- **UnimplementedFuseHandler**: Returns `ENOSYS` for unsupported operations by default; panic mode is available for debugging.
- **StatelessHandler**: Provides simple directory responses when no directory state is needed.
- **FileDescriptorHandler**: Supplies I/O methods for file handles backed by file descriptors; a read-only variant is also available.
- **MirrorFs**: Reads and writes through to an existing folder; `MirrorFsReadOnly` omits write methods.
- **OverlayFs**: Combines files from source folders and saves changes in a separate writable folder. For setup, examples, customization, supported operations, and limits, see the [detailed OverlayFs guide](src/fuse_presets/overlay_fs.rs).

See each type's documentation for more information about its usage.

### Composing presets

Store presets as fields on your filesystem. Each `delegate_fs!` list sends the named operations to
that field. Give each operation one owner: delegate it to a preset or implement it yourself. When
you add a custom method, remove it from the delegation list.

```rust,ignore
use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::mirror_fs::MirrorFs;
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use easy_fuser_macro::delegate_fs;
use std::path::PathBuf;

struct AppFs {
    mirror: MirrorFs,
    defaults: StatelessHandler<PathBuf>,
    unsupported: UnimplementedFuseHandler<PathBuf>,
}

impl FuseHandler for AppFs {
    type TId = PathBuf;
    type FileHandle = std::os::fd::OwnedFd;

    delegate_fs! { mirror, [ lookup, getattr, open, readdir, release ] }
    delegate_fs! { defaults, [ forget, fsyncdir, opendir, releasedir ] }
    delegate_fs! { unsupported, [ bmap, getlk, ioctl, readdirplus, setlk ] }

    fn read(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: Option<&mut std::os::fd::OwnedFd>,
        seek: SeekFrom,
        size: u32,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        if file_id == PathBuf::from("secret.txt") {
            return Err(ErrorKind::PermissionDenied.to_error("this file is private"));
        }
        self.mirror
            .read(req, file_id, file_handle, seek, size, flags, lock_owner)
    }
}
```

## Logging and diagnostics

`easy_fuser` uses the [`log`](https://docs.rs/log) facade. Add and initialize a logger in your
application (for example, `env_logger`) to see its messages. Mount and unmount lifecycle events
are logged at `info`; failed filesystem handler operations are logged at `warn`. A lookup for a
missing entry is expected during normal filesystem use and is logged at `debug`.

Successful filesystem requests are not logged, so normal reads and writes do not incur per-request
logging work. To include lookup misses while debugging, configure your logger to show `debug` for
`easy_fuser`, for example with `RUST_LOG=easy_fuser=debug` when using `env_logger`.

### Async Delegation

When using the `async` concurrency model, the `FuseHandler` trait is decorated with `#[async_trait]`. Because outer attribute macros expand before inner macro invocations, a standard delegation macro like `delegate_fs!` cannot be desugared by `#[async_trait]`.

To solve this, `easy_fuser` provides two specialized async delegation macros that perform **manual signature desugaring** matching the expected output format of `#[async_trait]`:

1. **`delegate_fs_async!`**: Use this when delegating to a field/target that itself exposes **asynchronous** methods (returning Futures).
2. **`delegate_fs_sync_to_async!`**: Use this when delegating to a field/target that exposes **synchronous/blocking** methods. The macro automatically wraps the synchronous method call in a pinned async block.

#### Example for Async Mode

```rust,ignore
use easy_fuser::fuse_async::prelude::*;
use easy_fuser::fuse_presets::mirror_fs::MirrorFs;
use easy_fuser::fuse_presets::UnimplementedFuseHandler;
use easy_fuser_macro::delegate_fs_sync_to_async;
use std::path::PathBuf;

struct MyAsyncFS {
    // MirrorFs has standard synchronous/blocking methods
    mirror_fs: MirrorFs,
    unimplemented: UnimplementedFuseHandler<PathBuf>,
}

#[async_trait]
impl FuseHandler for MyAsyncFS {
    type TId = PathBuf;
    type FileHandle = std::os::fd::OwnedFd;

    // Delegate to the synchronous MirrorFs target inside an async handler
    delegate_fs_sync_to_async! { mirror_fs, [ read, write, getattr ] }

    // These operations return ENOSYS until the filesystem supports them.
    delegate_fs_sync_to_async! { unimplemented, [ statfs, link ] }
}
```


## Feature Flags

This crate provides three feature flags for different concurrency models:

- `serial`: Enables single-threaded operation. Use this for simplicity and when concurrent
  access is not required. The thread count argument (`Option<usize>`) is accepted for API consistency but ignored.

- `parallel`: Enables multi-threaded operation using a thread pool. This is suitable for
  scenarios where you want to handle multiple filesystem operations concurrently on separate
  threads. It can improve performance on multi-core systems. Pass `Some(threads)` to specify the pool size, or `None` to automatically use a default based on the system's CPU count.

- `async`: Enables asynchronous operation using tokio. This is ideal for high-concurrency scenarios and
  when you want to integrate the filesystem with asynchronous Rust code. Pass `Some(threads)` to configure tokio's worker threads, or `None` to use the default multi-threaded runtime. When this feature is enabled, you use `easy_fuser::fuse_async::prelude::*` which decorates `FuseHandler` with `#[async_trait]`.

Example usage in Cargo.toml:
```toml
[dependencies]
easy_fuser = { version = "0.5.0", features = ["parallel"] }
```

By leveraging `easy_fuser`, you can focus more on your filesystem's logic and less on the
intricacies of FUSE implementation, making it easier to create robust, efficient, and
maintainable filesystem solutions in Rust.

## Examples

Please check the README inside the examples folder for additional details and references.

## Common Caveats

When working with FUSE filesystems, be aware of the following:

1. **Crashes & Proper Unmounting**:
If a program crashes or is stopped abruptly (e.g., using Ctrl+C), it may leave the mountpoint in an inconsistent state.

To properly unmount the filesystem and stop the program (or to resolve a bad state after a crash), use the following command:

  ```bash
  fusermount -u <mountpoint>
  ```

This is the preferred method for both unmounting and resolving any issues with the mountpoint. You will find more information in the documentation of `mount` and `spawn_mount`.

2. **Modifying the source directory while mounted**: This is not well-supported behavior and can result in unexpected outcomes.

## Important Notes

libfuse and by extension fuser contains a lot of flags as arguments. We tried to identify them as much as possible, but cannot guarantee it due to the lack of clear documentation on this subject.
