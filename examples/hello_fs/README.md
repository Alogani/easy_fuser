# Hello Filesystem Example

## Overview

This example demonstrates how to create a simple FUSE filesystem in Rust using the Easy Fuser library. It's designed to showcase the basic implementation without relying on templates, providing a clear understanding of the fundamental concepts.

## Usage

To run this example, use the following command:

Usage:
    # From the binary
    hello_fs <MOUNTPOINT>
    # From this crate source
    cargo run -- <MOUNTPOINT>

Replace `<MOUNTPOINT>` with the directory where you want to mount the filesystem.

## Key Features

1. **Inspired by fuser**: This example is largely based on the [hello.rs example from the fuser crate](https://github.com/cberner/fuser/blob/v0.15.1/examples/hello.rs), adapted for use with Easy Fuser.

2. **Template-free Implementation**: Unlike other examples, this one doesn't use templates. It's meant to illustrate the bare-bones structure of a FUSE filesystem.

3. **Inode-based File Identification**: For educational purposes, this example uses `Inode` as the `FileIdType`. This approach helps in understanding the low-level aspects of filesystem implementation.

## Learning Points

- Basic structure of a FUSE filesystem using Easy Fuser
- Implementation of essential filesystem operations
- Usage of `Inode` for file identification

## Note on File Identification

While this example uses `Inode` as `FileIdType`, many users might find it more intuitive to use `PathBuf`. Other examples in this repository demonstrate the use of `PathBuf`, which often aligns better with typical filesystem structures and can be easier to work with for many use cases.

## Getting Started

To explore this example:
1. Examine the source code, paying attention to how filesystem operations are implemented.
2. Run the example and interact with the resulting filesystem.
3. Compare this implementation with template-based examples to understand the differences.

Remember, if you're looking to quickly prototype a filesystem, the template module of easy_fuser might be more suitable starting points.