use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use io::{Cursor, Read, Seek, SeekFrom};
use std::error;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;
use suppaftp::FtpStream;

use crate::{helpers::*, DirectoryDetectionMethod};

pub struct FtpFs {
    ftp_client: Mutex<FtpStream>,
    detection_method: DirectoryDetectionMethod,
    inner_fs: UnimplementedFuseHandler<PathBuf>,
    safe_defaults: StatelessHandler<PathBuf>,
}

impl FtpFs {
    pub fn new(
        url: &str,
        username: &str,
        port: u32,
        password: &str,
        detection_method: DirectoryDetectionMethod,
    ) -> Result<Self, Box<dyn error::Error>> {
        let mut ftp_stream = FtpStream::connect(format!("{}:{}", url, port))?;
        ftp_stream.login(username, password)?;

        Ok(Self {
            ftp_client: Mutex::new(ftp_stream),
            detection_method,
            inner_fs: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
        })
    }

    fn with_ftp<F, R>(&self, func: F) -> FuseResult<R>
    where
        F: FnOnce(&mut FtpStream) -> FuseResult<R>,
    {
        let mut ftp_client = self.ftp_client.lock().unwrap();
        func(&mut ftp_client)
    }
}

impl FuseHandler for FtpFs {
    type TId = PathBuf;
    type FileHandle = Cursor<Vec<u8>>;

    easy_fuser::delegate_fs! { safe_defaults, [ forget, fsyncdir, opendir, releasedir ] }
    easy_fuser::delegate_fs! { inner_fs, [ access, bmap, copy_file_range, create, fallocate, flush, fsync, getlk, getxattr, ioctl, link, listxattr, lseek, mkdir, mknod, readlink, removexattr, rename, rmdir, setattr, setlk, setxattr, statfs, symlink, write, unlink ] }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: Option<&mut Self::FileHandle>,
    ) -> FuseResult<FileAttribute> {
        if file_id.is_filesystem_root() {
            return Ok(get_root_attribute());
        }
        self.with_ftp(|ftp| {
            get_file_attribute(ftp, &file_id, &self.detection_method)
                .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
        })
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let path = parent_id.join(name);
        self.with_ftp(|ftp| {
            get_file_attribute(ftp, &path, &self.detection_method)
                .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
        })
    }

    /// Download each remote file once when it is opened, then serve read chunks from this
    /// per-open cursor. This trades memory proportional to the open file size for fewer FTP RETR
    /// transfers; each open sees a snapshot of the remote file from open time.
    fn open(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _flags: OpenFlags,
    ) -> FuseResult<(Self::FileHandle, FopenFlags)> {
        let file = self
            .with_ftp(|ftp| {
                ftp.retr_as_buffer(file_id.to_str().unwrap())
                    .map_err(|_| PosixError::new(ErrorKind::FileNotFound, "File not found"))
            })?;
        Ok((file, FopenFlags::empty()))
    }

    fn read(
        &self,
        _req: &RequestInfo,
        _file_id: PathBuf,
        file_handle: Option<&mut Self::FileHandle>,
        offset: SeekFrom,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let file = file_handle
            .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing open file"))?;
        file.seek(offset)?;
        let mut buffer = vec![0; size as usize];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, FileKind)>> {
        let mut entries = Vec::new();
        entries.push((OsString::from("."), FileKind::Directory));
        entries.push((OsString::from(".."), FileKind::Directory));

        self.with_ftp(|ftp| {
            let list = ftp.list(Some(file_id.to_str().unwrap()))?;
            for item in list {
                let parts: Vec<&str> = item.split_whitespace().collect();
                if parts.len() >= 9 {
                    let name = OsString::from(parts[8]);
                    let kind = if parts[0].starts_with('d') {
                        FileKind::Directory
                    } else {
                        FileKind::RegularFile
                    };
                    entries.push((name, kind));
                }
            }
            Ok(())
        })
        .or(Err(PosixError::new(
            ErrorKind::InputOutputError,
            "Failed to read directory",
        )))?;

        Ok(entries)
    }
}
