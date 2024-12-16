use std::io;
use std::ops::Deref;
use crate::posix_fs::release;
use super::arguments::FileHandle;
use super::errors::*;

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
    type Error = io::Error;

    fn try_from(fh: FileHandle) -> Result<Self, Self::Error> {
        Ok(Self(
            i32::try_from(u64::from(fh)).map_err(|_| PosixError::INVALID_ARGUMENT)?,
        ))
    }
}

impl FileDescriptor {
    pub fn to_file_handle(self) -> Result<FileHandle, io::Error> {
        let fd: i32 = self.into();
        if fd < 0 {
            return Err(from_last_errno());
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
        Self { fd, release_on_drop: true }
    }

    /// Prevent releasing the file descriptor on drop
    pub fn take(&mut self) -> FileDescriptor {
        self.release_on_drop = false;
        self.fd.clone()
    }

    pub fn take_to_file_handle(&mut self) -> Result<FileHandle, io::Error> {
        match self.fd.clone().to_file_handle() {
            Ok(fd) => {
                self.release_on_drop = false;
                Ok(fd)
            },
            Err(e) => Err(e)
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