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

## Platform requirements and setup

`easy_fuser` uses the host's FUSE implementation; installing the Rust crate does
not install or enable FUSE in the operating system. Linux is the only platform
on which this crate is well tested. The source also contains platform-specific
implementations for macOS, FreeBSD, OpenBSD, and NetBSD, but treat those as
best-effort. The upstream [`fuser` project](https://github.com/cberner/fuser)
also describes its platform support and setup.

### For end users: running a filesystem

If you are running an already-built filesystem, you do not need Rust or Cargo.
You need the host's FUSE runtime, permission to mount, and an existing mountpoint.
Some containers and managed hosts disable FUSE access; installing packages alone
cannot enable it.

#### Linux

Install the FUSE 3 runtime package using your distribution's package manager.
For Debian or Ubuntu:

```sh
sudo apt install fuse3
```

Confirm that `/dev/fuse` is available and that your user is allowed to mount
FUSE filesystems.

#### FreeBSD and other BSD systems

On FreeBSD, install the FUSE library package and load its kernel module:

```sh
sudo pkg install fusefs-libs
sudo kldload fusefs
```

The second command loads the module for the current boot. FreeBSD's
[Handbook](https://docs.freebsd.org/en/books/handbook/filesystems/) documents
the module and how to load it at startup. Ensure your account and system policy
allow the mount. The crate has code paths for FreeBSD, OpenBSD, and NetBSD, but
FUSE setup and compatibility differ between them. The commands above are for
FreeBSD only; OpenBSD and NetBSD have not been well tested with this crate.
Consult the relevant system documentation before attempting a mount.

#### macOS

Install [macFUSE](https://macfuse.github.io/) using its current installer and
complete any system approval or security prompts it presents. Ensure your user
is permitted to mount a filesystem. macOS support in `easy_fuser` is best-effort
and has not been well tested.

#### Unmounting

Unmount with the utility provided by the host system. On Linux this is commonly
`fusermount3 -u <mountpoint>` (some systems provide `fusermount`); on BSD and
macOS use `umount <mountpoint>`. If the mount is busy, close processes using it
and retry. After a process crash, check whether the mount is still active before
trying to mount over the same path again.

### For developers: building and testing the crate

Building from source requires a Rust toolchain and Cargo. To build or run tests
that mount a filesystem, you also need the host runtime setup above. On Linux,
the integration tests need access to `/dev/fuse` and permission to mount; a
container or CI runner without that access cannot run mounted tests.

On Debian or Ubuntu, these packages mirror the Linux CI build dependencies:

```sh
sudo apt install pkg-config libfuse-dev
```

The crate's default Rust backend does not require libfuse development headers.
Install the matching libfuse development package and `pkg-config` only when
building with the optional `libfuse` feature. For example, on Debian or Ubuntu:

```sh
sudo apt install libfuse3-dev pkg-config
```

To build and test an individual concurrency mode on Linux, use the same feature
selection as CI:

```sh
cargo build --no-default-features --features parallel
cargo test --no-default-features --features parallel
```

Replace `parallel` with `serial` or `async` to check those modes. Full tests may
mount FUSE filesystems, so run them on a host where FUSE is enabled.

The macOS CI job installs macFUSE and compiles the serial plus `libfuse` feature,
but uses `cargo test --no-run` because its hosted runner cannot mount filesystems.
That verifies compilation, not runtime behavior. There is no equivalent tested
development recipe for OpenBSD or NetBSD; FreeBSD and macOS development should
also be treated as best-effort.

## Usage

The following quickstart mounts an existing directory read-only. It uses the `parallel` feature;
add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
easy_fuser = { version = "0.7", features = ["parallel"] }
easy_fuser_macro = "0.1"
```

The mount point must already exist and must be outside the source directory.

Run it with a source directory and mount point, for example:

```sh
cargo run -- /path/to/source /mnt/myfs
```

```rust,ignore
use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::mirror_fs::{MirrorFsReadOnly, MirrorFsTrait};
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use easy_fuser_macro::delegate_fs;
use std::path::PathBuf;

struct ReadOnlyFs {
    mirror: MirrorFsReadOnly,
    defaults: StatelessHandler<PathBuf>,
    unsupported: UnimplementedFuseHandler<PathBuf>,
}

impl FuseHandler for ReadOnlyFs {
    type TId = PathBuf;
    type FileHandle = std::os::fd::OwnedFd;

    delegate_fs! { mirror, [
        access, flush, fsync, getattr, getxattr, listxattr, lookup, lseek,
        open, read, readdir, readlink, release
    ] }
    delegate_fs! { defaults, [ forget, fsyncdir, opendir, releasedir ] }
    delegate_fs! { unsupported, [
        bmap, copy_file_range, create, fallocate, getlk, ioctl, link, mkdir,
        mknod, removexattr, rename, rmdir, setattr, setlk,
        setxattr, statfs, symlink, unlink, write
    ] }
}

fn main() -> std::io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let source = PathBuf::from(args.next().expect("usage: app <SOURCE_DIR> <MOUNT_POINT>"));
    let mountpoint = PathBuf::from(args.next().expect("usage: app <SOURCE_DIR> <MOUNT_POINT>"));

    let fs = ReadOnlyFs {
        mirror: MirrorFsReadOnly::new(source),
        defaults: StatelessHandler::new(),
        unsupported: UnimplementedFuseHandler::new(),
    };

    mount(fs, mountpoint, &[MountOption::RO], None)
}
```

This example serves files from the source directory through the mount. For a custom filesystem,
implement the needed `FuseHandler` operations yourself and delegate only the remaining operations
to presets. See the [hello filesystem](examples/hello_fs/README.md) for a small inode-based
implementation and the [passthrough example](examples/passthrough_fs/README.md) for a fuller
mirror filesystem.

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

### Async

Linux filesystem calls used by the presets are generally blocking. The `async` mode is useful when your handler needs async method signatures or integrates with async-native I/O, but wrapping a blocking call does not make that call non-blocking. If the handler mainly performs blocking filesystem work, async mode offers no performance gain over the `parallel` mode; choose `parallel` when you want a worker pool for blocking callbacks.

#### Choosing async handlers

Use async-native handlers when operations can await genuinely asynchronous work, such as an async I/O backend. That lets the runtime run other tasks while an operation is waiting. `MirrorFsAsync` and `FileDescriptorHandlerAsync` provide async signatures, but their default filesystem operations still call the same blocking `unix_fs` functions as the synchronous presets. The experimental Linux-only `io_uring` feature makes some file operations truly asynchronous; current benchmarks show significant performance issues, so it is not recommended. See the feature list below for details.

#### Async delegation

When using the `async` concurrency model, the `FuseHandler` trait is decorated with `#[async_trait]`. Because outer attribute macros expand before inner macro invocations, a standard delegation macro like `delegate_fs!` cannot be desugared by `#[async_trait]`.

`easy_fuser` provides two specialized async delegation macros that perform **manual signature desugaring** matching the expected output format of `#[async_trait]`:

1. **`delegate_fs_async!`**: Use this when delegating to a field/target that itself exposes **asynchronous** methods (returning Futures).
2. **`delegate_fs_sync_to_async!`**: Use this when delegating to a field/target that exposes **synchronous/blocking** methods. The macro wraps the synchronous method call in a pinned async block, but does not move it to a blocking pool. The call runs when the future is polled and blocks that async runtime worker until it returns. Use this for convenient integration with async handler signatures when blocking work is acceptable; it does not make blocking I/O scalable or improve its performance.

#### Example with a synchronous preset

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

#### Example with `MirrorFsAsync`

```rust,ignore
use easy_fuser::fuse_async::prelude::*;
use easy_fuser::fuse_presets::{MirrorFsAsync, UnimplementedFuseHandler};
use easy_fuser_macro::{delegate_fs_async, delegate_fs_sync_to_async};
use async_trait::async_trait;
use std::path::PathBuf;

struct MyAsyncFS {
    mirror_fs: MirrorFsAsync,
    unimplemented: UnimplementedFuseHandler<PathBuf>,
}

#[async_trait]
impl FuseHandler for MyAsyncFS {
    type TId = PathBuf;

    delegate_fs_async! { mirror_fs, [ access, getattr, lookup, open, read, readdir, release ] }
    delegate_fs_sync_to_async! { unimplemented, [ statfs, link ] }
}
```


## Feature Flags

The default feature set enables `serial`, `parallel`, and `async`. These expose all three APIs; choose the matching `FuseHandler` prelude in your code. To build only one mode, disable default features and enable the mode you want.

- `serial`: Runs callbacks serially; the thread-count setting is ignored.
- `parallel`: Runs callbacks on a worker thread pool. Use `mount_with_threads` to configure FUSE reader and handler worker counts independently.
- `async`: Runs callbacks on a Tokio runtime. Async handlers use `easy_fuser::fuse_async::prelude::*` and `#[async_trait]`.
- `deadlock_detection`: Development aid for parallel mode. It checks for deadlocks periodically and logs detected thread backtraces; enable it while debugging, not as a normal production setting.
- `io_uring`: Experimental Linux-only option that also enables `async`. It is not recommended: current benchmarks show roughly 3–10× lower throughput than the async syscall implementation. It is disabled by default.
- `libfuse`: Builds `fuser` with its libfuse backend instead of the default direct kernel interface; it requires the system libfuse development files.

Example usage in Cargo.toml:
```toml
[dependencies]
easy_fuser = { version = "0.7", default-features = false, features = ["parallel"] }
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
