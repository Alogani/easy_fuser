use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use crate::types::*;
use crate::unix_fs;

use super::{TEMP_ID, TEMP_PREFIX, XATTR_BUFFER_LIMIT};

pub(super) fn overlaps(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

pub(super) fn metadata_if_exists(path: &Path) -> FuseResult<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn load_whiteouts(path: &Path) -> io::Result<HashSet<PathBuf>> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => return Err(error),
    };
    data.lines()
        .map(|line| {
            let bytes = line.as_bytes();
            if bytes.len() % 2 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid overlay whiteout",
                ));
            }
            let decoded = bytes
                .chunks_exact(2)
                .map(|pair| {
                    let digit = |byte: u8| match byte {
                        b'0'..=b'9' => Some(byte - b'0'),
                        b'a'..=b'f' => Some(byte - b'a' + 10),
                        b'A'..=b'F' => Some(byte - b'A' + 10),
                        _ => None,
                    };
                    match (digit(pair[0]), digit(pair[1])) {
                        (Some(high), Some(low)) => Ok(high << 4 | low),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid overlay whiteout",
                        )),
                    }
                })
                .collect::<io::Result<Vec<_>>>()?;
            Ok(PathBuf::from(OsString::from_vec(decoded)))
        })
        .collect()
}

pub(super) fn save_whiteouts(path: &Path, whiteouts: &HashSet<PathBuf>) -> io::Result<()> {
    let mut paths = whiteouts.iter().collect::<Vec<_>>();
    paths.sort();
    let mut data = String::new();
    for path in paths {
        for byte in path.as_os_str().as_bytes() {
            use std::fmt::Write as _;
            write!(&mut data, "{byte:02x}").unwrap();
        }
        data.push('\n');
    }
    let temporary = unique_sibling(path, "whiteouts").map_err(|error| error.io_error())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(data.as_bytes())?;
    file.sync_all()?;
    drop(file);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

pub(super) fn unique_sibling(path: &Path, label: &str) -> FuseResult<PathBuf> {
    let parent = path.parent().unwrap_or(Path::new(""));
    if path.file_name().is_none() {
        return Err(ErrorKind::InvalidArgument.to_error("path has no file name"));
    }
    loop {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let temporary_name = format!("{TEMP_PREFIX}{label}.{}.{}", std::process::id(), id);
        let candidate = parent.join(temporary_name);
        if metadata_if_exists(&candidate)?.is_none() {
            return Ok(candidate);
        }
    }
}

pub(super) fn copy_metadata(
    source: &Path,
    destination: &Path,
    metadata: &fs::Metadata,
) -> FuseResult<()> {
    let owner = SetAttrRequest::new()
        .uid(metadata.uid())
        .gid(metadata.gid());
    if let Err(error) = unix_fs::setattr(destination, owner) {
        let raw_error = error.io_error().raw_os_error();
        if raw_error != Some(libc::EPERM) && raw_error != Some(libc::EACCES) {
            return Err(error);
        }
    }
    fs::set_permissions(
        destination,
        fs::Permissions::from_mode(metadata.permissions().mode()),
    )?;
    copy_xattrs(source, destination)?;
    if let (Ok(atime), Ok(mtime)) = (metadata.accessed(), metadata.modified()) {
        unix_fs::setattr(
            destination,
            SetAttrRequest::new()
                .atime(TimeOrNow::SpecificTime(atime))
                .mtime(TimeOrNow::SpecificTime(mtime)),
        )?;
    }
    Ok(())
}

fn copy_xattrs(source: &Path, destination: &Path) -> FuseResult<()> {
    let names = match read_xattr_names(source) {
        Ok(names) => names,
        Err(error)
            if error.io_error().raw_os_error() == Some(libc::ENOTSUP)
                || error.io_error().raw_os_error() == Some(libc::EOPNOTSUPP) =>
        {
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    for name in names
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let name = OsStr::from_bytes(name);
        let value = read_xattr_value(source, name)?;
        unix_fs::setxattr(destination, name, &value, SetXAttrFlags::empty(), 0)?;
    }
    Ok(())
}

fn read_xattr_names(path: &Path) -> FuseResult<Vec<u8>> {
    read_xattr_with_growth(
        |size| unix_fs::listxattr(path, size),
        "too many extended attributes",
    )
}

fn read_xattr_value(path: &Path, name: &OsStr) -> FuseResult<Vec<u8>> {
    read_xattr_with_growth(
        |size| unix_fs::getxattr(path, name, size),
        "extended attribute is too large",
    )
}

fn read_xattr_with_growth(
    mut read: impl FnMut(u32) -> FuseResult<Vec<u8>>,
    too_large_message: &str,
) -> FuseResult<Vec<u8>> {
    let mut size = 256;
    loop {
        match read(size) {
            Ok(value) => return Ok(value),
            Err(error) if error.io_error().raw_os_error() == Some(libc::ERANGE) => {
                if size >= XATTR_BUFFER_LIMIT {
                    return Err(ErrorKind::ValueTooLarge.to_error(too_large_message));
                }
                size = (size * 2).min(XATTR_BUFFER_LIMIT);
            }
            Err(error) => return Err(error),
        }
    }
}
