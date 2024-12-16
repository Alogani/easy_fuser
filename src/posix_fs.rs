use std::path::Path;
use std::time::{Duration, SystemTime};

use std::ffi::{CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{self as unix_fs, *};
use std::{fs, io};

use crate::types::*;
use libc::{c_char, c_void, timespec};

/// Convert file type from fs::FileType to the one expected by fuse_api
/// Be careful with symlinks
pub fn convert_filetype(f: fs::FileType) -> FileType {
    match f {
        f if f.is_file() => FileType::RegularFile,
        f if f.is_dir() => FileType::Directory,
        f if f.is_symlink() => FileType::Symlink,
        f if f.is_block_device() => FileType::BlockDevice,
        f if f.is_char_device() => FileType::CharDevice,
        f if f.is_fifo() => FileType::NamedPipe,
        f if f.is_socket() => FileType::Socket,
        _ => panic!("Unknown FileType"), // not possible in theory
    }
}

/// Convert file attribute obtained from fs::Metadata to the type expected by fuser
/// Be careful with symlink, use fs::symlink_metadata
pub fn convert_fileattribute(metadata: fs::Metadata) -> FileAttribute {
    FileAttribute {
        inode: INVALID_INODE,
        size: metadata.size(),
        blocks: metadata.blocks(),
        atime: SystemTime::UNIX_EPOCH + Duration::new(metadata.atime() as u64, 0),
        mtime: SystemTime::UNIX_EPOCH + Duration::new(metadata.mtime() as u64, 0),
        ctime: SystemTime::UNIX_EPOCH + Duration::new(metadata.ctime() as u64, 0),
        crtime: SystemTime::UNIX_EPOCH + Duration::new(metadata.mtime() as u64, 0),
        kind: convert_filetype(metadata.file_type()),
        perm: (metadata.mode() & 0o777) as u16,
        nlink: metadata.nlink() as u32,
        uid: metadata.uid(),
        gid: metadata.gid(),
        rdev: metadata.rdev() as u32,
        blksize: metadata.blksize() as u32,
        flags: 0, // macOS only; placeholder here
        ttl: None,
        generation: None,
    }
}

fn convert_stat_struct(statbuf: libc::stat) -> Option<FileAttribute> {
    // Convert timestamp values to SystemTime
    let atime = SystemTime::UNIX_EPOCH + Duration::new(statbuf.st_atime as u64, 0);
    let mtime = SystemTime::UNIX_EPOCH + Duration::new(statbuf.st_mtime as u64, 0);
    let ctime = SystemTime::UNIX_EPOCH + Duration::new(statbuf.st_ctime as u64, 0);
    // Extract permissions (lower 9 bits of st_mode)
    let perm = (statbuf.st_mode & (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO)) as u16;
    // Flags are not directly supported in `stat`, use a placeholder for now
    let flags = 0; // Update this if your system provides a way to get file flags

    Some(FileAttribute {
        inode: INVALID_INODE,
        size: statbuf.st_size as u64,
        blocks: statbuf.st_blocks as u64,
        atime,
        mtime,
        ctime,
        crtime: mtime,
        kind: stat_to_kind(statbuf)?,
        perm: perm,
        nlink: statbuf.st_nlink as u32,
        uid: statbuf.st_uid as u32,
        gid: statbuf.st_gid as u32,
        rdev: statbuf.st_rdev as u32,
        blksize: statbuf.st_blksize as u32,
        flags: flags,
        ttl: None,
        generation: None,
    })
}

fn stat_to_kind(statbuf: libc::stat) -> Option<FileType> {
    use libc::*;
    Some(match statbuf.st_mode & S_IFMT {
        S_IFREG => FileType::RegularFile,
        S_IFDIR => FileType::Directory,
        S_IFCHR => FileType::CharDevice,
        S_IFBLK => FileType::BlockDevice,
        S_IFIFO => FileType::NamedPipe,
        S_IFLNK => FileType::Symlink,
        S_IFSOCK => FileType::Socket,
        _ => return None, // Unsupported or unknown file type
    })
}

fn system_time_to_timespec(time: SystemTime) -> Result<timespec, io::Error> {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| PosixError::INVALID_ARGUMENT)?;
    Ok(timespec {
        tv_sec: duration.as_secs() as i64,
        tv_nsec: duration.subsec_nanos() as i64,
    })
}

fn cstring_from_path(path: &Path) -> Result<CString, io::Error> {
    Ok(CString::new(path.as_os_str().as_bytes())?)
}

/// Equivalent to the fuse function of the same name
/// Get the metadata associated with a path
/// custom_inode is useful if user tracks its inode instead of wanting the one of the filesystem (which might be the case with fuse)
pub fn lookup(path: &Path) -> Result<FileAttribute, io::Error> {
    let c_path = cstring_from_path(path)?;
    let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::lstat(c_path.as_ptr(), &mut statbuf) };
    if result == -1 {
        return Err(from_last_errno());
    }
    Ok(convert_stat_struct(statbuf).ok_or(PosixError::INVALID_ARGUMENT)?)
}

/// Equivalent to the fuse function of the same name
/// Get the metadata associated with a FileDescriptor
/// custom_inode is useful if user tracks its inode instead of wanting the one of the filesystem (which might be the case with fuse)
pub fn getattr(fd: &FileDescriptor) -> Result<FileAttribute, io::Error> {
    let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::fstat(fd.clone().into(), &mut statbuf) };
    if result == -1 {
        return Err(from_last_errno());
    }
    Ok(convert_stat_struct(statbuf).ok_or(PosixError::INVALID_ARGUMENT)?)
}

/// Equivalent to the fuse function of the same name
pub fn setattr(path: &Path, attrs: SetAttrRequest) -> Result<FileAttribute, io::Error> {
    let c_path = cstring_from_path(path)?;

    // update permissions
    if let Some(mode) = attrs.mode {
        let result = unsafe { libc::chmod(c_path.as_ptr(), mode) };
        if result == -1 {
            return Err(from_last_errno());
        }
    }

    // Change file owner (UID and GID)
    if attrs.uid.is_some() || attrs.gid.is_some() {
        let uid = attrs.uid.unwrap_or(0_u32.wrapping_sub(1));
        let gid = attrs.gid.unwrap_or(0_u32.wrapping_sub(1));

        let result = unsafe { libc::chown(c_path.as_ptr(), uid, gid) };
        if result == -1 {
            return Err(from_last_errno());
        }
    }

    // Change file size (if `size` is provided)
    if let Some(size) = attrs.size {
        let result = {
            // If we have no file handle, use `open` to get one, then `ftruncate`
            let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
            if fd == -1 {
                return Err(from_last_errno());
            }
            let res = unsafe {
                libc::ftruncate(
                    fd,
                    i64::try_from(size).map_err(|_| PosixError::INVALID_ARGUMENT)?,
                )
            };
            unsafe { libc::close(fd) };
            res
        };

        if result == -1 {
            return Err(from_last_errno());
        }
    }

    // Set access and modification times (atime and mtime)
    if let (Some(atime), Some(mtime)) = (attrs.atime, attrs.mtime) {
        let times = match (atime, mtime) {
            (TimeOrNow::Now, TimeOrNow::Now) => {
                let now_spec = system_time_to_timespec(SystemTime::now())?;
                [now_spec, now_spec]
            }
            (TimeOrNow::SpecificTime(at), TimeOrNow::SpecificTime(mt)) => {
                let at_spec = system_time_to_timespec(at)?;
                let mt_spec = system_time_to_timespec(mt)?;
                [at_spec, mt_spec]
            }
            _ => return Err(PosixError::INVALID_ARGUMENT.into()),
        };
        let result = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                c_path.as_ptr(),
                &times[0],
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == -1 {
            return Err(from_last_errno());
        }
    }

    lookup(path)
}

/// Equivalent to the fuse function of the same name
pub fn readlink(path: &Path) -> Result<Vec<u8>, io::Error> {
    let c_path = cstring_from_path(path)?;
    let mut buf = vec![0u8; 1024]; // Initial buffer size
    let ret =
        unsafe { libc::readlink(c_path.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
    if ret == -1 {
        return Err(from_last_errno());
    }
    buf.truncate(ret as usize);
    Ok(buf)
}

/// Equivalent to the fuse function of the same name
pub fn mknod(path: &Path, mode: u32, rdev: DeviceType) -> Result<FileAttribute, io::Error> {
    let c_path = cstring_from_path(path)?;
    let ret = unsafe { libc::mknod(c_path.as_ptr(), mode, rdev.to_rdev() as libc::dev_t) };
    if ret == -1 {
        return Err(from_last_errno());
    }
    lookup(path)
}

/// Equivalent to the fuse function of the same name
pub fn mkdir(path: &Path, mode: u32) -> Result<FileAttribute, io::Error> {
    let c_path = cstring_from_path(path)?;
    let ret = unsafe { libc::mkdir(c_path.as_ptr(), mode) };
    if ret == -1 {
        return Err(from_last_errno());
    }
    lookup(path)
}

/// Equivalent to the fuse function of the same name
pub fn unlink(path: &Path) -> Result<(), io::Error> {
    Ok(fs::remove_file(path)?)
}

/// Equivalent to the fuse function of the same name
pub fn rmdir(path: &Path) -> Result<(), io::Error> {
    Ok(fs::remove_dir(path)?)
}

/// Equivalent to the fuse function of the same name
pub fn symlink(path: &Path, target: &Path) -> Result<FileAttribute, io::Error> {
    unix_fs::symlink(target, path)?;
    lookup(path)
}

/// Equivalent to the fuse function of the same name
pub fn rename(oldpath: &Path, newpath: &Path, flags: RenameFlags) -> Result<(), io::Error> {
    let old_cstr = cstring_from_path(oldpath)?;
    let new_cstr = cstring_from_path(newpath)?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD, // Current working directory for the old path
            old_cstr.as_ptr(),
            libc::AT_FDCWD, // Current working directory for the new path
            new_cstr.as_ptr(),
            flags.bits(),
        )
    };
    if result == 0 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

/// Equivalent to the fuse function of the same name
/// Return a file descriptor (fuse doesn't assume it necessarly equivalent to its file handle)
pub fn open(path: &Path, flags: OpenFlags) -> Result<FileDescriptorGuard, io::Error> {
    let c_path = cstring_from_path(path)?;
    let fd = unsafe { libc::open(c_path.as_ptr(), flags.bits()) };
    if fd == -1 {
        return Err(from_last_errno());
    }
    Ok(FileDescriptorGuard::new(fd.into()))
}

/// Equivalent to the fuse function of the same name
pub fn read(fd: &FileDescriptor, offset: i64, size: u32) -> Result<Vec<u8>, io::Error> {
    let mut buffer = vec![0; size as usize];
    let bytes_read = unsafe {
        libc::pread(
            fd.clone().into(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            size as usize,
            offset,
        )
    };
    if bytes_read == -1 {
        return Err(from_last_errno());
    }
    buffer.truncate(bytes_read as usize);
    Ok(buffer)
}

/// Equivalent to the fuse function of the same name
pub fn write(fd: &FileDescriptor, offset: i64, data: &[u8]) -> Result<u32, io::Error> {
    let bytes_to_write = data.len() as usize;

    let bytes_written = unsafe {
        libc::pwrite(
            fd.clone().into(),
            data.as_ptr() as *const libc::c_void,
            bytes_to_write,
            offset,
        )
    };
    if bytes_written == -1 {
        return Err(from_last_errno());
    }

    Ok(bytes_written as u32)
}

/// Equivalent to the fuse function of the same name
pub fn flush(fd: &FileDescriptor) -> Result<(), io::Error> {
    let result = unsafe { libc::fdatasync(fd.clone().into()) };
    if result == -1 {
        return Err(from_last_errno());
    }

    Ok(())
}

/// Equivalent to the fuse function of the same name
pub fn fsync(fd: &FileDescriptor, datasync: bool) -> Result<(), io::Error> {
    let fd = fd.clone().into();
    let result = unsafe {
        if datasync {
            libc::fdatasync(fd)
        } else {
            libc::fsync(fd)
        }
    };

    if result == -1 {
        return Err(from_last_errno());
    }

    Ok(())
}

/// Equivalent to the fuse function of the same name
pub fn readdir(path: &Path) -> Result<Vec<(OsString, FileType)>, io::Error> {
    let entries = fs::read_dir(path)?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry?;
        result.push((entry.file_name(), convert_filetype(entry.file_type()?)))
    }
    Ok(result)
}

/// Equivalent to the fuse function of the same name
pub fn readdirplus(path: &Path) -> Result<Vec<(OsString, FileType, FileAttribute)>, io::Error> {
    let entries = fs::read_dir(path)?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry?;
        result.push((
            entry.file_name(),
            convert_filetype(entry.file_type()?),
            convert_fileattribute(entry.metadata()?),
        ))
    }
    Ok(result)
}

/// Equivalent to the fuse function of the same name
/// Integrated automatically into fuse_api (so file_handle as fd transmitted to it doesn't need to be released)
/// Useless to call for FileDescriptorGuard
pub fn release(fd: FileDescriptor) -> Result<(), io::Error> {
    // Attempt to close the file descriptor.
    let result = unsafe { libc::close(fd.into()) };

    if result == -1 {
        // Handle errors from the close system call.
        return Err(from_last_errno());
    }
    Ok(())
}

/// Equivalent to the fuse function of the same name
pub fn statfs(path: &Path) -> Result<StatFs, io::Error> {
    let c_path = cstring_from_path(path)?;
    let mut stat: libc::statvfs64 = unsafe { std::mem::zeroed() };

    // Use statvfs64 to get file system stats
    let result = unsafe { libc::statvfs64(c_path.as_ptr(), &mut stat) };
    if result != 0 {
        return Err(from_last_errno());
    }

    Ok(StatFs {
        total_blocks: stat.f_blocks as u64,
        free_blocks: stat.f_bfree as u64,
        available_blocks: stat.f_bavail as u64,
        total_files: stat.f_files as u64,
        free_files: stat.f_ffree as u64,
        block_size: stat.f_bsize as u32,
        max_filename_length: stat.f_namemax as u32,
        fragment_size: stat.f_frsize as u32,
    })
}

/// Equivalent to the fuse function of the same name
pub fn setxattr(path: &Path, name: &OsStr, value: &[u8], position: u32) -> Result<(), io::Error> {
    let c_path = cstring_from_path(path)?;
    let c_name = CString::new(name.as_bytes()).map_err(|_| PosixError::INVALID_ARGUMENT)?;
    let ret = unsafe {
        libc::setxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            value.as_ptr() as *const c_void,
            value.len(),
            i32::try_from(position).map_err(|_| PosixError::INVALID_ARGUMENT)?,
        )
    };

    if ret == -1 {
        return Err(from_last_errno());
    }
    Ok(())
}

/// Equivalent to the fuse function of the same name
pub fn getxattr(path: &Path, name: &OsStr, size: u32) -> Result<Vec<u8>, io::Error> {
    let c_path = cstring_from_path(path)?;
    let c_name = CString::new(name.as_bytes()).map_err(|_| PosixError::INVALID_ARGUMENT)?;

    let mut buf = vec![0u8; size as usize];
    let ret = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
        )
    };

    if ret == -1 {
        return Err(from_last_errno());
    }

    buf.truncate(ret as usize);
    Ok(buf)
}

/// Equivalent to the fuse function of the same name
pub fn listxattr(path: &Path, size: u32) -> Result<Vec<u8>, io::Error> {
    let c_path = cstring_from_path(path)?;
    let mut buf = vec![0u8; size as usize];
    let ret = unsafe { libc::listxattr(c_path.as_ptr(), buf.as_mut_ptr() as *mut i8, buf.len()) };

    if ret == -1 {
        return Err(from_last_errno());
    }

    buf.truncate(ret as usize);
    Ok(buf)
}

/// Equivalent to the fuse function of the same name
pub fn access(path: &Path, mask: AccessMask) -> Result<(), io::Error> {
    let c_path = cstring_from_path(path)?;
    let ret = unsafe { libc::access(c_path.as_ptr(), mask.bits()) };
    if ret == -1 {
        return Err(from_last_errno());
    }
    Ok(())
}

/// Equivalent to the fuse function of the same name
/// The doc of `open` apply
/// Return a file handle with write flag. Return an error if file already exists
pub fn create(path: &Path, mode: u32) -> Result<(FileDescriptorGuard, FileAttribute), io::Error> {
    let c_path = cstring_from_path(path)?;

    // Open the file with O_CREAT (create if it does not exist) and O_WRONLY (write only)
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_CREAT | libc::O_WRONLY | libc::O_EXCL, // O_EXCL ensures the file is created if it doesn't exist
            mode,
        )
    };

    if fd == -1 {
        return Err(from_last_errno());
    }

    Ok((FileDescriptorGuard::new(fd.into()), lookup(path)?))
}

/// Equivalent to the fuse function of the same name
pub fn fallocate(
    fd: &FileDescriptor,
    offset: i64,
    length: i64,
    mode: i32,
) -> Result<(), io::Error> {
    let result = unsafe { libc::fallocate(fd.clone().into(), mode, offset, length) };
    if result == -1 {
        return Err(from_last_errno());
    }
    Ok(())
}

/// Equivalent to the fuse function of the same name
pub fn lseek(fd: &FileDescriptor, offset: i64, whence: Whence) -> Result<i64, io::Error> {
    let result = unsafe { libc::lseek(fd.clone().into(), offset, whence.into()) };
    if result == -1 {
        return Err(from_last_errno());
    }
    Ok(result)
}

/// Equivalent to the fuse function of the same name
pub fn copy_file_range(
    fd_in: &FileDescriptor,
    offset_in: i64,
    fd_out: &FileDescriptor,
    offset_out: i64,
    len: u64,
) -> Result<u32, io::Error> {
    let result = unsafe {
        libc::copy_file_range(
            fd_in.clone().into(),
            offset_in as *mut libc::off_t,
            fd_out.clone().into(),
            offset_out as *mut libc::off_t,
            len as usize,
            0, // placeholder
        )
    };
    if result == -1 {
        return Err(from_last_errno());
    }
    Ok(result as u32)
}

#[cfg(test)]
mod tests {
    use tempfile::{NamedTempFile, TempDir};

    use super::*;
    use std::fs::{self, File};
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    #[test]
    fn test_convert_filetype() {
        let tmpfile = NamedTempFile::new().unwrap();
        let filetype = convert_filetype(fs::metadata(&tmpfile.path()).unwrap().file_type());
        assert_eq!(filetype, FileType::RegularFile);
        drop(tmpfile);
    }

    #[test]
    fn test_convert_fileattribute() {
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(&tmpfile.path(), "blah").unwrap();
        let metadata = fs::metadata(&tmpfile.path()).unwrap();
        let attr = convert_fileattribute(metadata);

        assert!(attr.size > 0);
        drop(tmpfile);
    }

    #[test]
    fn test_system_time_to_timespec() {
        let system_time = SystemTime::now();
        let timespec = system_time_to_timespec(system_time).unwrap();

        assert!(timespec.tv_sec > 0);
        assert!(timespec.tv_nsec >= 0);
    }

    #[test]
    fn test_cstring_from_path() {
        let path = PathBuf::from("test_cstring");
        let c_string = cstring_from_path(&path).unwrap();

        assert_eq!(c_string.to_str().unwrap(), path.to_str().unwrap());
    }

    #[test]
    fn test_get_attr() {
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(&tmpfile.path(), "blah").unwrap();
        let attr1 = lookup(&tmpfile.path()).unwrap();
        let fd = open(&tmpfile.path(), OpenFlags::READ_ONLY).unwrap();
        let attr2 = getattr(&fd).unwrap();
        assert!(attr1.size > 0);
        assert_eq!(attr1, attr2);
        drop(tmpfile);
    }

    #[test]
    fn test_readlink() {
        let tmpdir = TempDir::new().unwrap();

        let target_path = tmpdir.path().join("link_target");
        let _ = File::create_new(&target_path).unwrap();

        let symlink_path = tmpdir.path().join("symlink");
        symlink(&symlink_path, &target_path).unwrap();

        let link_target = readlink(&symlink_path).unwrap();
        fs::remove_file(&target_path).unwrap();
        fs::remove_file(&symlink_path).unwrap();
        drop(tmpdir);

        assert_eq!(Path::new(OsStr::from_bytes(&link_target)), target_path);
    }

    #[test]
    fn test_mkdir_and_rmdir() {
        let tmpdir = TempDir::new().unwrap();
        let dir_path = tmpdir.path().join("dir");
        mkdir(&dir_path, 0o755).unwrap();
        assert!(dir_path.exists());

        rmdir(&dir_path).unwrap();
        assert!(!dir_path.exists());
        drop(tmpdir);
    }

    #[test]
    fn test_symlink() {
        let tmpdir = TempDir::new().unwrap();

        let target_path = tmpdir.path().join("link_target");
        let _ = File::create_new(&target_path).unwrap();

        let symlink_path = tmpdir.path().join("symlink");
        let attr = symlink(&symlink_path, &target_path).unwrap();

        fs::remove_file(&target_path).unwrap();
        fs::remove_file(&symlink_path).unwrap();
        drop(tmpdir);

        assert_eq!(attr.kind, FileType::Symlink);
    }

    #[test]
    fn test_unlink() {
        let tmpdir = TempDir::new().unwrap();
        let file_path = tmpdir.path().join("file");
        File::create(&file_path).unwrap();

        assert!(&file_path.exists());
        unlink(&file_path).unwrap();
        assert!(!file_path.exists());
        drop(tmpdir);
    }

    #[test]
    fn test_rename() {
        let tmpdir = TempDir::new().unwrap();
        let src_path = tmpdir.path().join("src");
        File::create(&src_path).unwrap();
        let dest_path = tmpdir.path().join("dest");

        rename(&src_path, &dest_path, RenameFlags::empty()).unwrap();
        assert!(!src_path.exists());
        assert!(dest_path.exists());

        fs::remove_file(&dest_path).unwrap();
    }

    #[test]
    fn test_open() {
        let tmpfile = NamedTempFile::new().unwrap();

        let fd = open(&tmpfile.path(), OpenFlags::empty()).unwrap();
        assert!(i32::from(fd.clone()) > 0);
        drop(tmpfile);
    }

    #[test]
    fn test_read() {
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(&tmpfile.path(), b"Hello, world!").unwrap();

        let fd = open(&tmpfile.path(), OpenFlags::READ_ONLY).unwrap();
        let result = read(&fd, 0, 5).unwrap();
        assert_eq!(result, b"Hello");

        let result = read(&fd, 7, 5).unwrap();
        assert_eq!(result, b"world");

        // Attempt to read past the end of the file
        let result = read(&fd, 50, 10);
        assert!(!result.is_err());
        assert_eq!(result.unwrap().len(), 0);
        drop(tmpfile);
    }

    #[test]
    fn test_write() {
        let tmpfile = NamedTempFile::new().unwrap();
        let fd = open(&tmpfile.path(), OpenFlags::READ_WRITE).unwrap();

        // Write data to the file
        let bytes_written = write(&fd, 0, b"Hello, world!").unwrap();
        assert_eq!(bytes_written, 13);

        // Verify written content
        lseek(&fd, 0, Whence::Start).unwrap();
        let content = read(&fd, 0, 100).unwrap();
        assert_eq!(&String::from_utf8(content).unwrap(), "Hello, world!");

        // Overwrite part of the file
        let bytes_written = write(&fd, 7, b"Rustaceans!").unwrap();
        assert_eq!(bytes_written, 11);

        lseek(&fd, 0, Whence::Start).unwrap();
        let content = read(&fd, 0, 100).unwrap();
        assert_eq!(&String::from_utf8(content).unwrap(), "Hello, Rustaceans!");
        drop(tmpfile);
    }

    #[test]
    fn test_readdir() {
        let tmpdir = TempDir::new().unwrap();
        let file1 = tmpdir.path().join("file1");
        File::create(&file1).unwrap();

        let entries = readdir(&tmpdir.path()).unwrap();
        assert!(entries.iter().any(|(name, _)| name == Path::new("file1")));

        fs::remove_file(&file1).unwrap();
        drop(tmpdir);
    }

    #[test]
    fn test_statfs() {
        let dir_path = Path::new("/tmp");
        let stat = statfs(dir_path).unwrap();

        assert!(stat.total_blocks > 0);
        assert!(stat.block_size > 0);
    }
}
