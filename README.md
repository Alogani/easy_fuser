# easy_fuser

![CI](https://github.com/Alogani/easy_fuser/actions/workflows/ubuntu.yml/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/easy_fuser.svg)](https://crates.io/crates/easy_fuser)
[![Documentation](https://docs.rs/easy_fuser/badge.svg)](https://docs.rs/easy_fuser)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/Alogani/easy_fuser/blob/master/LICENSE.md)
[![dependency status](https://deps.rs/repo/github/Alogani/easy_fuser/status.svg)](https://deps.rs/repo/github/Alogani/easy_fuser)

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
  allowing you to choose between `Inode`, `PathBuf`, or `Vec<OsString>` as your file identifier type. This
  offers flexibility in how you represent and manage file identities, suitable for different
  filesystem structures and performance requirements.

- **Error Handling**: Provides a structured error handling system, facilitating the management
  of filesystem-specific errors.

- **Composable Templates and Examples**: Includes pre-built, composable templates and a comprehensive 
  examples folder to help you get started quickly, understand various implementation patterns, 
  and easily combine different filesystem behaviors. These templates are designed to be mixed 
  and matched, allowing for flexible and modular filesystem creation.

## File Identification Flexibility

`easy_fuser` supports two main approaches for file identification:

1. **Path-based Operations**: Work with file paths directly, which can be more intuitive for
   certain use cases.
2. **Inode-based Operations**: Use inode numbers for more efficient control, especially useful
   for complex filesystem structures or when performance is critical.

You can choose the approach that best fits your filesystem's needs and switch between them
as necessary.

## Usage

To use `easy_fuser`, follow these steps:

1. Implement the `FuseHandler` trait for your filesystem structure.
2. (Optional) Utilize provided templates to jumpstart your implementation.
3. Choose an appropriate concurrency model by enabling the corresponding feature.
4. Use the `mount` or `spawn_mount` functions to start your filesystem.

Here's a basic example:

```rust,no_run
use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use std::path::{Path, PathBuf};

struct MyFS {
    inner: Box<DefaultFuseHandler>,
}

impl FuseHandler<PathBuf> for MyFS {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        self.inner.as_ref()
    }
}

fn main() -> std::io::Result<()> {
    let fs = MyFS { inner: Box::new(DefaultFuseHandler::new()) };
    easy_fuser::mount(fs, Path::new("/mnt/myfs"), &[], 1)
}
```

## Templates

`easy_fuser` provides a set of templates to help you get started quickly:

- **DefaultFuseHandler**: A backbone implementation that acts as a NullFs, implementing every
  operation. It can also be used as a PanicFs for debugging purposes.
- **FdHandlerHelper**: Provides boilerplate for operations on open files (ReadOnly and ReadWrite variants available)
- **MirrorFs**: A passthrough filesystem that can be leveraged for creating more complex filesystems.

These templates serve as composable building blocks, allowing you to mix and match functionalities to create custom, complex filesystem implementations with ease. You can use them as starting points, extend them, or combine multiple templates to achieve the desired behavior for your filesystem.

## Examples

The `examples` folder in the repository is currently under construction. It is expected to include
various implementations demonstrating different aspects of filesystem creation, including:

- ZipFs: A filesystem for browsing and accessing zip archives.
- FtpFs: A filesystem that provides access to FTP servers.
- SqlFs: A filesystem that represents SQL database contents.

These examples, once completed, will serve as valuable references when building your own filesystem.

## Feature Flags

This crate provides three mutually exclusive feature flags for different concurrency models:

- `serial`: Enables single-threaded operation. Use this for simplicity and when concurrent
  access is not required. When this feature is enabled, `num_threads` must be set to 1.

- `parallel`: Enables multi-threaded operation using a thread pool. This is suitable for
  scenarios where you want to handle multiple filesystem operations concurrently on separate
  threads. It can improve performance on multi-core systems.

- `async`: _**This is not yet implemented**_ Enables asynchronous operation. This is ideal for high-concurrency scenarios and
  when you want to integrate the filesystem with asynchronous Rust code. It allows for
  efficient handling of many concurrent operations without the overhead of threads.

You must enable exactly one of these features when using this crate. The choice depends on
your specific use case and performance requirements.

Example usage in Cargo.toml:
```toml
[dependencies]
easy_fuser = { version = "0.1.0", features = ["parallel"] }
```

By leveraging `easy_fuser`, you can focus more on your filesystem's logic and less on the
intricacies of FUSE implementation, making it easier to create robust, efficient, and
maintainable filesystem solutions in Rust.

## Important notes

libfuse and by extension fuser contains a lot of flags as arguments. I tried to identify them as much of possible, but cannot guarantee it due to the lack of clear documentation on this subject.
