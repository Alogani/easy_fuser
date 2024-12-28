use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Mutex;
use threadsafe_lru::LruCache;
use suppaftp::FtpStream;
use std::io;
use io::{Read, Seek, SeekFrom};
use std::error;

use crate::helpers::*;

pub struct FtpFs {
    ftp_client: Mutex<FtpStream>,
    folder_cache: LruCache<PathBuf, ()>,
    inner_fs: DefaultFuseHandler,
}

impl FtpFs {
    pub fn new(url: &str, username: &str, password: &str, cache_cap: usize) -> Result<Self, Box<dyn error::Error>> {
        let mut ftp_stream = FtpStream::connect(url)?;
        ftp_stream.login(username, password)?;
        
        Ok(Self {
            ftp_client: Mutex::new(ftp_stream),
            inner_fs: DefaultFuseHandler::new(),
            folder_cache: LruCache::new(cache_cap, (cache_cap as f64).sqrt().ceil() as usize),
        })
    }

    fn with_ftp<F, R>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&mut FtpStream) -> FuseResult<R>,
    {
        let mut ftp_client = self.ftp_client.lock().ok()?;
        func(&mut ftp_client).ok()
    }
}

impl FuseHandler<PathBuf> for FtpFs {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        &self.inner_fs
    }

    fn lookup(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<FileAttribute> {
        let path = parent_id.join(name);
        self.with_ftp(|ftp| {
            let pathname = path.to_str().unwrap();
            let size = ftp.size(pathname)?;
            let modify_time = ftp.mdtm(pathname)?;
            let is_dir = ftp.list(Some(pathname)).is_ok();
            Ok(create_file_attribute(size as u64, modify_time, is_dir))
        })
        .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
    }

    fn getattr(&self, _req: &RequestInfo, file_id: PathBuf, _file_handle: Option<FileHandle>) -> FuseResult<FileAttribute> {
        if file_id.is_filesystem_root() {
            return Ok(get_root_attribute());
        }
        self.with_ftp(|ftp| {
            let pathname = file_id.to_str().unwrap();
            let size = ftp.size(pathname)?;
            let modify_time = ftp.mdtm(pathname)?;
            let is_dir = ftp.list(Some(pathname)).is_ok();
            Ok(create_file_attribute(size as u64, modify_time, is_dir))
        })
        .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
    }

    fn read(&self, _req: &RequestInfo, file_id: PathBuf, _file_handle: FileHandle, offset: SeekFrom, size: u32, _flags: FUSEOpenFlags, _lock_owner: Option<u64>) -> FuseResult<Vec<u8>> {
        self.with_ftp(|ftp| {
            let mut cursor = ftp.retr_as_buffer(file_id.to_str().unwrap())?;
            cursor.seek(offset)?;
            let mut buffer = vec![0; size as usize];
            let bytes_read = cursor.read(&mut buffer)?;
            buffer.truncate(bytes_read);
            Ok(buffer)
        })
        .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
    }

    fn readdir(&self, _req: &RequestInfo, file_id: PathBuf, _file_handle: FileHandle) -> FuseResult<Vec<(OsString, FileKind)>> {
        let mut entries = Vec::new();
        entries.push((OsString::from("."), FileKind::Directory));
        entries.push((OsString::from(".."), FileKind::Directory));

        self.with_ftp(|ftp| {
            let list = ftp.list(Some(file_id.to_str().unwrap()))?;
            for item in list {
                let parts: Vec<&str> = item.split_whitespace().collect();
                if parts.len() >= 9 {
                    let name = OsString::from(parts[8]);
                    let kind = if parts[0].starts_with('d') { FileKind::Directory } else { FileKind::RegularFile };
                    entries.push((name, kind));
                }
            }
            Ok(())
        }).ok_or(PosixError::new(ErrorKind::InputOutputError, "Failed to read directory"))?;

        Ok(entries)
    }
}