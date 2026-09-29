//! Raw FUSE handle wrappers retained for directory operations and low-level compatibility.
//!
//! Normal file callbacks use the handler's typed `FileHandle` resource, which is stored and
//! resolved by `FuseDriver`. Conversions in this module are valid only when the raw value is
//! independently known to be a real file descriptor.
//!
//! # A custom per-open handle
//!
//! Define a type for state that belongs to one open instance, return it from `open`, then access it
//! through `Option<&mut Self::FileHandle>` in callbacks such as `read`. The driver owns each value
//! until `release`; it does not require the type to be `Clone` or a file descriptor.
//!
//! The driver assigns a separate, collision-checked FUSE number to each live open. This number is
//! independent of your `FileHandle`, so the type does not need to contain a unique numeric key and
//! equal handle values from different opens do not collide in the driver's table. Each open has its
//! own callback lock. If several handle values refer to shared mutable backend state, synchronize
//! that state in your handler. The type must be `'static`; parallel and async modes also require
//! `Send`, while `Clone` and `Sync` are not required.
//!
//! ```rust,ignore
//! use easy_fuser::fuse_serial::prelude::*;
//! use std::io::{Cursor, Read, Seek};
//! use std::path::PathBuf;
//!
//! struct MemoryFs;
//! struct OpenFile { cursor: Cursor<Vec<u8>> }
//!
//! impl FuseHandler for MemoryFs {
//!     type TId = PathBuf;
//!     type FileHandle = OpenFile;
//!
//!     fn open(
//!         &self,
//!         _req: &RequestInfo,
//!         _file_id: PathBuf,
//!         _flags: OpenFlags,
//!     ) -> FuseResult<(Self::FileHandle, FopenFlags)> {
//!         let bytes = b"hello from this open".to_vec();
//!         Ok((OpenFile { cursor: Cursor::new(bytes) }, FopenFlags::empty()))
//!     }
//!
//!     fn read(
//!         &self,
//!         _req: &RequestInfo,
//!         _file_id: PathBuf,
//!         file_handle: Option<&mut Self::FileHandle>,
//!         seek: SeekFrom,
//!         size: u32,
//!         _flags: OpenFlags,
//!         _lock_owner: Option<u64>,
//!     ) -> FuseResult<Vec<u8>> {
//!         let open_file = file_handle
//!             .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing open state"))?;
//!         open_file.cursor.seek(seek)?;
//!         let mut bytes = vec![0; size as usize];
//!         let count = open_file.cursor.read(&mut bytes)?;
//!         bytes.truncate(count);
//!         Ok(bytes)
//!     }
//! }
//! ```
//!
//! This shows only the relevant callbacks; a complete filesystem also implements or delegates
//! its other required `FuseHandler` operations. When callbacks are delegated to
//! `FileDescriptorHandler`, use `std::os::fd::OwnedFd` as `FileHandle`. Use `()` for stateless
//! handlers whose operations do not need per-open state.

use std::marker::PhantomData;
pub use std::os::fd::*;

/// A wrapper around a raw file handle that represents ownership of a file handle.
/// It doesn't necessarily represent a valid file descriptor according to how fuse is implemented by the user.
/// But provide methods to work with file descriptors in a safe manner
///
/// ## Caveats
/// A file handle is represented as a u64 value, whereas a file descriptor is a i32 value.
#[derive(Debug)]
pub struct OwnedFileHandle(u64);

impl OwnedFileHandle {
    /// Creates an OwnedFileHandle from a raw u64 value.
    ///
    /// Unsafe because it assumes the provided value is a valid, open file handle.
    pub unsafe fn from_raw(handle: u64) -> Self {
        Self(handle)
    }

    /// Borrows the file handle, creating a BorrowedFileHandle with a lifetime tied to self.
    pub fn borrow(&self) -> BorrowedFileHandle<'_> {
        BorrowedFileHandle(self.0, PhantomData)
    }

    /// Borrows the file handle as a BorrowedFd.
    ///
    /// Note: This method performs an unchecked cast from `u64` to `i32`, which may lead to undefined behavior if the file handle value doesn't fit within an `i32`.
    pub fn borrow_as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.0 as i32) }
    }

    /// Attempts to convert an OwnedFd into an OwnedFileHandle.
    ///
    /// This method consumes the OwnedFd and returns an `Option<OwnedFileHandle>`.
    /// It returns None if the conversion from i32 to u64 fails, which can happen if the file descriptor is negative.
    pub fn from_owned_fd(fd: OwnedFd) -> Option<Self> {
        let raw_fd = fd.into_raw_fd().try_into().ok()?;
        Some(unsafe { Self::from_raw(raw_fd) })
    }

    /// Converts the OwnedFileHandle into an OwnedFd. Consumes self.
    ///
    /// Note: Assumes the internal u64 always represents a valid file descriptor.
    pub fn into_owned_fd(self) -> OwnedFd {
        // SAFETY: We're assuming that self.0 always contains a valid file descriptor.
        // This assumption makes this conversion safe.
        unsafe { OwnedFd::from_raw_fd(self.0 as i32) }
    }

    /// Returns the raw u64 value of the file handle.
    ///
    /// Note: This struct provides a safe abstraction over raw file handles,
    /// but some methods rely on assumptions about the validity of the internal u64 value.
    /// Use with caution when interfacing with raw file descriptors.
    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

/// A borrowed representation of a file handle, tied to a specific lifetime 'a. It wraps a u64 value representing the file handle.
#[derive(Debug, Clone, Copy)]
pub struct BorrowedFileHandle<'a>(u64, PhantomData<&'a ()>);

impl<'a> BorrowedFileHandle<'a> {
    /// Retrieves the raw u64 value of the file handle.
    pub fn as_raw(&self) -> u64 {
        self.0
    }

    /// Creates an BorrowedFileHandle from a raw u64 value.
    ///
    /// Unsafe because it assumes the provided value is a valid, open file handle.
    pub unsafe fn from_raw(handle: u64) -> Self {
        Self(handle, PhantomData)
    }

    /// Converts the BorrowedFileHandle into a BorrowedFd.
    ///
    /// Note: This method performs an unchecked cast from `u64` to `i32`, which may lead to undefined behavior if the file handle value doesn't fit within an `i32`.
    pub fn as_borrowed_fd(self) -> BorrowedFd<'a> {
        unsafe { BorrowedFd::borrow_raw(self.0 as i32) }
    }

    /// Creates a BorrowedFileHandle from a OwnedFd. Don't consume the OwnedFd.
    ///
    /// Note: This method returns `None` if the conversion from `i32` to `u64` fails,
    /// which can happen with negative file descriptors.
    pub fn from_owned_fd(fd: OwnedFd) -> Option<Self> {
        let raw_fd = fd.as_raw_fd().try_into().ok()?;
        Some(BorrowedFileHandle(raw_fd, PhantomData))
    }

    /// Creates a BorrowedFileHandle from a BorrowedFd.
    ///
    /// Note: This method returns `None` if the conversion from `i32` to `u64` fails,
    /// which can happen with negative file descriptors.
    pub fn from_borrowed_fd(fd: BorrowedFd<'a>) -> Option<Self> {
        let raw_fd = fd.as_raw_fd().try_into().ok()?;
        Some(BorrowedFileHandle(raw_fd, PhantomData))
    }
}
