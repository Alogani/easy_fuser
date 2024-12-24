use super::errors::*;
use crate::posix_fs::release;
use std::ops::Deref;

/// Represents a file handle in the FUSE filesystem.
///
/// This is a wrapper around a u64 value that uniquely identifies an open file
/// within the FUSE context. It may not directly correspond to a valid system
/// file descriptor.
#[derive(Debug, Clone)]
pub struct FileHandle(u64);

impl From<u64> for FileHandle {
    fn from(value: u64) -> Self {
        FileHandle(value)
    }
}

impl From<FileHandle> for u64 {
    fn from(value: FileHandle) -> Self {
        value.0
    }
}

/// Represents a system-level file descriptor for an open file.
///
/// This struct wraps an i32 value that corresponds to the actual file descriptor
/// assigned by the operating system. It provides a type-safe way to handle
/// file descriptors within the application.
#[derive(Debug, Clone)]
pub struct FileDescriptor(i32);

impl From<FileDescriptor> for i32 {
    fn from(value: FileDescriptor) -> Self {
        value.0
    }
}

impl From<i32> for FileDescriptor {
    fn from(value: i32) -> Self {
        FileDescriptor(value)
    }
}

impl TryFrom<FileHandle> for FileDescriptor {
    type Error = PosixError;

    /// Attempts to convert a FileHandle to a FileDescriptor.
    ///
    /// This conversion can fail for the following reasons:
    /// 1. The FileHandle's u64 value might be too large to fit into an i32.
    /// 2. The resulting i32 value might not represent a valid file descriptor
    ///    on the system (e.g., negative values are typically invalid).
    ///
    /// If the conversion fails, it returns a PosixError with InvalidArgument kind.
    fn try_from(fh: FileHandle) -> Result<Self, Self::Error> {
        Ok(Self(i32::try_from(fh.0).map_err(|_| {
            PosixError::new(
                ErrorKind::InvalidArgument,
                format!("{:?}: could not be converted to file descriptor", fh),
            )
        })?))
    }
}

impl FileDescriptor {
    /// Attempts to convert a FileDescriptor to a FileHandle.
    ///
    /// This conversion can fail if the file descriptor is negative, which
    /// typically indicates an invalid file descriptor. In such cases,
    /// it returns a PosixError based on the last system error.
    pub fn to_file_handle(self) -> Result<FileHandle, PosixError> {
        let fd: i32 = self.into();
        // If called at the correct time, errno is valid
        if fd < 0 {
            return Err(PosixError::last_error(format!(
                "fd value is negative ({:?})",
                fd
            )));
        }
        return Ok(FileHandle::from(fd as u64));
    }
}

/// A guard wrapper for a FileDescriptor that ensures proper resource management.
///
/// `FileDescriptorGuard` provides RAII (Resource Acquisition Is Initialization)
/// semantics for file descriptors, helping prevent resource leaks.
///
/// ## Usage:
/// - Create a new guard using `FileDescriptorGuard::new(fd)`.
/// - The file descriptor is automatically released when the guard goes out of scope,
///   unless `take()` or `take_to_file_handle()` is called.
///
/// ## Guarantees:
/// - Prevents file descriptor leaks by automatically closing the descriptor on drop.
/// - Allows manual control over descriptor release through `take()` and `take_to_file_handle()`.
/// - Provides a safe way to convert between FileDescriptor and FileHandle.
///
/// Note: This guard does not guarantee thread-safety. Ensure proper synchronization
/// when using FileDescriptorGuard across multiple threads.
///
/// ## RAII Management inside FuseHandler:
/// The FuseHandler implementation will manage RAII for file descriptors by automatically
/// calling the release method of FuseHandler when needed. This ensures that resources
/// are properly cleaned up, even when the FileDescriptorGuard is not explicitly dropped.
pub struct FileDescriptorGuard {
    fd: FileDescriptor,
    release_on_drop: bool,
}

impl FileDescriptorGuard {
    pub fn new(fd: FileDescriptor) -> Self {
        Self {
            fd,
            release_on_drop: true,
        }
    }

    /// Prevents the automatic release of the file descriptor and returns it.
    ///
    /// This method:
    /// - Disables the automatic release of the file descriptor on drop.
    /// - Returns a clone of the internal FileDescriptor.
    /// - Transfers the responsibility of closing the file descriptor to the caller.
    /// However, the FuseHandler will typically manage this automatically through its RAII mechanisms.
    pub fn take(&mut self) -> FileDescriptor {
        self.release_on_drop = false;
        self.fd.clone()
    }

    /// Attempts to convert the FileDescriptor to a FileHandle and prevent its automatic release.
    ///
    /// This method:
    /// - Tries to convert the internal FileDescriptor to a FileHandle.
    /// - If successful, disables the automatic release and returns the FileHandle.
    /// - If conversion fails, returns the error without modifying the guard's state.
    ///
    /// Note: On success, the caller becomes responsible for closing the underlying file descriptor.
    /// However, the FuseHandler will typically manage this automatically through its RAII mechanisms.
    pub fn take_to_file_handle(&mut self) -> Result<FileHandle, PosixError> {
        match self.fd.clone().to_file_handle() {
            Ok(fd) => {
                self.release_on_drop = false;
                Ok(fd)
            }
            Err(e) => Err(e),
        }
    }
}

impl Deref for FileDescriptorGuard {
    type Target = FileDescriptor;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}

impl Drop for FileDescriptorGuard {
    fn drop(&mut self) {
        if self.release_on_drop {
            if let Err(e) = release(self.fd.clone()) {
                log::error!("Failed to release file descriptor: {}", e);
            }
        }
    }
}
