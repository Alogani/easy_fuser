use super::errors::*;
use crate::posix_fs::release;
use std::ops::Deref;

/// Represents the file handle of an open file in fuse filesystem
/// May not represent a valid file descriptor
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

/// Represents the file descriptor of an open file on the system
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

pub struct FileDescriptorGuard {
    fd: FileDescriptor,
    release_on_drop: bool,
}

impl FileDescriptorGuard {
    /// Create a new guard, that will be released on drop
    pub fn new(fd: FileDescriptor) -> Self {
        Self {
            fd,
            release_on_drop: true,
        }
    }

    /// Prevent releasing the file descriptor on drop
    pub fn take(&mut self) -> FileDescriptor {
        self.release_on_drop = false;
        self.fd.clone()
    }

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
