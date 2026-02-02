//! # Template Implementations for easy_fuser
//!
//! This module provides a collection of template implementations and utility helpers
//! designed to simplify the creation of FUSE filesystems using the easy_fuser library.
//!
//! ## Key Features:
//!
//! - **Composable Templates**: All templates in this module are designed with composition
//!   in mind. They can be used individually or combined to create more complex filesystem
//!   behaviors.
//!
//! - **Customizable Behavior**: Each function within these templates can be overridden,
//!   allowing for fine-grained control and customization of filesystem operations.
//!
//! - **DefaultFuseHandler**: A comprehensive starting point for implementing FUSE
//!   filesystems. It provides a default behavior for all standard FUSE operations.
//!
//! ## Usage:
//!
//! Users typically leverage these templates as a foundation for their own filesystem
//! implementations. By extending or composing these templates, developers can rapidly
//! prototype and implement custom FUSE filesystems while maintaining the flexibility
//! to override specific behaviors as needed.
//!
//! ## Available Templates:
//!
//! - `DefaultFuseHandler`: A complete implementation of basic FUSE operations.
//! - `fd_handler_helper`: Utilities for handling file descriptors in FUSE operations.
//! - `mirror_fs`: Templates for creating mirror filesystems.
//!
//! For detailed information on each template, refer to their respective documentation.

mod default_fuse_handler;
pub use default_fuse_handler::DefaultFuseHandler;

pub mod fd_handler_helper;

pub mod mirror_fs;

/*
# POSSIBILITY 1:
- Pros: simple to resonnate
- Cons: no delegation (harder to wrap), harder documentation (MirrorFs::write don't exist)
*/
struct TestFs {}

pub trait MirrorFs {
    fn source() -> PathBuf;
}

impl MirrorFs for Test {
    fn source() -> PathBuf {
        todo!()
    }
}

impl FuseHandler<PathBuf> for TestFs {
    implement_fuse_mirror_fs!(["write"])
    /* Equivalent to
    fn write(&self, req: &RequestInfo, file_id: TId, file_handle: BorrowedFileHandle, seek: SeekFrom, data: Vec<u8>,
        write_flags: FUSEWriteFlags, flags: OpenFlags, lock_owner: Option<u64>,) -> FuseResult<u32>
    where Self: MirrorFs {
        // implementation_of_mirror_fs.write
    }
     */
}

/*
# POSSIBILITY 2:
- Pros: documentation, easy to wrap
- Cons: lifetime / send+sync ?
*/

struct TestFs {
    mirror_fs: MirrorFs, // MirrorFs already implements its own functions but doesn't inherit FuseHandler trait
}

impl FuseHandler<PathBuf> for TestFs {
    delegate_fs!(mirror_fs, ["write"])
    /* Equivalent to
    fn write(&self, req: &RequestInfo, file_id: TId, file_handle: BorrowedFileHandle, seek: SeekFrom, data: Vec<u8>,
        write_flags: FUSEWriteFlags, flags: OpenFlags, lock_owner: Option<u64>,) -> FuseResult<u32> {
        self.#mirror_fs.write(...)
    }
     */
}