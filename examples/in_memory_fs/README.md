In-Memory Filesystem Implementation

This module provides a comprehensive implementation of an in-memory filesystem
using the `easy_fuser` crate. While it includes a wide range of filesystem
operations, a minimal subset of these functions would be sufficient for a
basic working filesystem.

Key features:
- Implements core FUSE operations (create, read, write, lookup, etc.)
- Supports file and directory operations
- Manages file attributes and permissions

This implementation serves as both a functional in-memory filesystem and
an educational example for understanding FUSE filesystem development with Rust.

Note: For production use, consider implementing additional error handling,
concurrency controls, and optimizations based on specific requirements.

WARNING: This is an in-memory filesystem. All files and data will be lost when
the filesystem is unmounted or the program terminates. Do not use this for
storing important data or in production environments where data persistence
is required.
