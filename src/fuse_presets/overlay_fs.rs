//! # OverlayFs
//!
//! Think of `OverlayFs` as the part that knows how to find, combine, and change files. It reads from
//! the lower folders and saves changes in the upper folder. The first lower folder has the highest
//! priority. When two layers contain the same directory, their contents appear together. Deleting a
//! lower-layer file hides it without changing the lower folder.
//!
//! Your filesystem still needs a `FuseHandler` wrapper. `OverlayFs` handles file and folder
//! behavior, but not every required FUSE operation. Use `StatelessHandler` for simple directory
//! methods that need no state, and `UnimplementedFuseHandler` for operations your filesystem does
//! not support. These fields fill different gaps; neither adds more overlay behavior.
//! OverlayFs reuses `FileDescriptorHandler` for open-file operations, so you do
//! not need a separate helper field when using this preset.
//!
//! ## Basic: use the overlay as is
//!
//! This is the starting point for most users. `overlay` handles file and folder behavior;
//! `defaults` and `unimplemented` handle the remaining operations.
//!
//! ```rust,ignore
//! use easy_fuser::fuse_parallel::prelude::*;
//! use easy_fuser::fuse_presets::{StatelessHandler, OverlayFs, UnimplementedFuseHandler};
//! use easy_fuser_macro::delegate_fs;
//! use std::ffi::OsStr;
//! use std::path::PathBuf;
//!
//! struct AppFs {
//!     overlay: OverlayFs,
//!     defaults: StatelessHandler<PathBuf>,
//!     unimplemented: UnimplementedFuseHandler<PathBuf>,
//! }
//!
//! impl FuseHandler for AppFs {
//!     type TId = PathBuf;
//!
//!     delegate_fs! { overlay, [
//!         access, copy_file_range, create, fallocate, flush, fsync, getattr,
//!         getxattr, listxattr, link, lookup, lseek, mkdir, mknod, open,
//!         read, readdir, readlink, release, removexattr, rename, rmdir,
//!         setattr, setxattr, statfs, symlink, unlink, write
//!     ]}
//!
//!     // Directory operations that need no per-directory state
//!     delegate_fs! { defaults, [ forget, fsyncdir, opendir, releasedir ]}
//!     // Operations this filesystem does not support
//!     delegate_fs! { unimplemented, [ bmap, getlk, ioctl, readdirplus, setlk ]}
//! }
//! ```
//!
//! Initialize the preset fields with the upper and lower folders:
//!
//! ```rust,ignore
//! fn make_filesystem() -> std::io::Result<AppFs> {
//!     Ok(AppFs {
//!         overlay: OverlayFs::new("/var/lib/app/upper", ["/usr/share/app"])?,
//!         defaults: StatelessHandler::new(),
//!         unimplemented: UnimplementedFuseHandler::new(),
//!     })
//! }
//! ```
//!
//! ## Normal: add a rule around one operation
//!
//! To customize an operation, remove its name from the `overlay` list and write that method inside
//! `impl FuseHandler for AppFs`. Call `self.overlay` after applying your rule. For example, this
//! prevents deleting one important file while keeping OverlayFs's normal delete behavior for
//! everything else:
//!
//! ```rust,ignore
//! // Inside `impl FuseHandler for AppFs`:
//! fn unlink(
//!     &self,
//!     req: &RequestInfo,
//!     parent: PathBuf,
//!     name: &OsStr,
//! ) -> FuseResult<()> {
//!     if parent.as_os_str().is_empty() && name == "important.db" {
//!         return Err(ErrorKind::PermissionDenied.to_error("this file is protected"));
//!     }
//!     self.overlay.unlink(req, parent, name)
//! }
//! ```
//!
//! Remove `unlink` from the `overlay` delegation list when adding this method. The same pattern works
//! for logging, access rules, custom attributes, or any other operation you want to adjust.
//!
//! ## Complex: combine presets and custom operations
//!
//! Keep custom operations on your `FuseHandler` type and delegate the rest to
//! `OverlayFs`, `StatelessHandler`, or `UnimplementedFuseHandler`. See the
//! [composition examples](crate::fuse_presets) for a complete example with a custom rule.
//!
//! ## Which operations go where?
//!
//! | OverlayFs handles file and folder behavior | Leave simple methods to `StatelessHandler` | Return an error for unsupported methods |
//! | --- | --- | --- |
//! | Finding and listing files: `lookup`, `getattr`, `access`, `readdir`, `readlink` | Folder bookkeeping: `forget`, `fsyncdir`, `opendir`, `releasedir` |  |
//! | Reading and writing: `open`, `create`, `read`, `write`, `release`, `flush`, `fsync`, `fallocate`, `copy_file_range`, `lseek` |  |  |
//! | Changing files: `mkdir`, `mknod`, `symlink`, `link`, `unlink`, `rmdir`, `rename`, `setattr`, `setxattr`, `removexattr` |  |  |
//! | File details: `getxattr`, `listxattr`, `statfs` |  |  |
//! |  |  | Unsupported here: `bmap`, `getlk`, `ioctl`, `readdirplus`, `setlk` |
//!
//! If you write a method yourself, remove it from the delegation list. If a method is listed under
//! `StatelessHandler` but its no-op behavior does not fit your filesystem, implement it yourself
//! or send it to `UnimplementedFuseHandler`. Do not delegate a method to `OverlayFs` unless it
//! appears in its column; the macro will fail to compile because that method is not provided.
//!
//! ## Limits to know about
//!
//! The upper and lower folders must already exist and must not contain one another. Keep the mount
//! point outside all of them. Do not change these folders while the filesystem is running, and use an
//! upper folder with only one live `OverlayFs` instance. The upper folder keeps private records of
//! deleted files; Linux's OverlayFS cannot read these records. When you first change a file from a
//! lower folder, OverlayFs copies it into the upper folder. This works for ordinary files, folders,
//! and symbolic links, but not device files, pipes, or sockets. Renaming a lower or combined folder is
//! not supported. Copying a symbolic link keeps its destination but not its owner or timestamps;
//! changing a symbolic link's metadata is not supported.

use std::collections::{BTreeMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Mutex, MutexGuard};

use crate::types::*;

mod operations;
mod storage;

use crate::unix_fs;
use storage::{
    copy_metadata, load_whiteouts, metadata_if_exists, overlaps, save_whiteouts, unique_sibling,
};

const CONTROL_DIR: &str = ".easy_fuser_overlay";
const TEMP_PREFIX: &str = ".easy_fuser_overlay_tmp.";
const FORMAT_VERSION: &[u8] = b"1\n";
const XATTR_BUFFER_LIMIT: u32 = 1024 * 1024;
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub(super) struct ResolvedEntry {
    pub(super) path: PathBuf,
    kind: FileKind,
    from_upper: bool,
    has_lower: bool,
    directories: Vec<PathBuf>,
}

/// A reusable, writable overlay filesystem preset.
///
/// `lower_dirs` are searched from highest to lowest priority. The upper and
/// lower directories must already exist and must not overlap. The upper
/// directory stores copied-up entries and persistent whiteout metadata.
/// Use a given upper directory with only one live `OverlayFs` instance.
pub struct OverlayFs {
    upper_dir: PathBuf,
    lower_dirs: Vec<PathBuf>,
    control_dir: PathBuf,
    whiteouts: Mutex<HashSet<PathBuf>>,
    mutation_lock: Mutex<()>,
}

impl OverlayFs {
    /// Creates an overlay over `upper_dir` and the ordered `lower_dirs`.
    ///
    /// The first lower directory has the highest priority. An empty lower
    /// list creates an upper-only filesystem.
    pub fn new<U, L, I>(upper_dir: U, lower_dirs: I) -> io::Result<Self>
    where
        U: Into<PathBuf>,
        L: Into<PathBuf>,
        I: IntoIterator<Item = L>,
    {
        let upper_dir = fs::canonicalize(upper_dir.into())?;
        if !upper_dir.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "overlay upper layer must be a directory",
            ));
        }

        let lower_dirs = lower_dirs
            .into_iter()
            .map(|path| fs::canonicalize(path.into()))
            .collect::<io::Result<Vec<_>>>()?;
        for lower_dir in &lower_dirs {
            if !lower_dir.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "overlay lower layers must be directories",
                ));
            }
            if overlaps(&upper_dir, lower_dir) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "overlay upper and lower layers must not overlap",
                ));
            }
            match fs::symlink_metadata(lower_dir.join(CONTROL_DIR)) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{CONTROL_DIR} is reserved at the root of overlay layers"),
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        for (index, lower_dir) in lower_dirs.iter().enumerate() {
            if lower_dirs[..index]
                .iter()
                .any(|other| overlaps(other, lower_dir))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "overlay lower layers must not overlap",
                ));
            }
        }

        let control_dir = upper_dir.join(CONTROL_DIR);
        match fs::symlink_metadata(&control_dir) {
            Ok(metadata) if metadata.file_type().is_dir() => {
                let version = fs::read(control_dir.join("version"))?;
                if version.as_slice() != FORMAT_VERSION {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "unsupported overlay metadata format",
                    ));
                }
            }
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{CONTROL_DIR} is reserved at the root of overlay layers"),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&control_dir)?;
                fs::write(control_dir.join("version"), FORMAT_VERSION)?;
            }
            Err(error) => return Err(error),
        }

        let whiteouts = load_whiteouts(&control_dir.join("whiteouts"))?;
        Ok(Self {
            upper_dir,
            lower_dirs,
            control_dir,
            whiteouts: Mutex::new(whiteouts),
            mutation_lock: Mutex::new(()),
        })
    }

    /// Returns the canonical upper directory.
    pub fn upper_dir(&self) -> &Path {
        &self.upper_dir
    }

    /// Returns lower directories in search-priority order.
    pub fn lower_dirs(&self) -> &[PathBuf] {
        &self.lower_dirs
    }

    pub(super) fn normalize_path(&self, path: &Path) -> FuseResult<PathBuf> {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => {
                    if part.as_bytes().starts_with(TEMP_PREFIX.as_bytes()) {
                        return Err(ErrorKind::PermissionDenied
                            .to_error("overlay temporary file prefix is reserved"));
                    }
                    normalized.push(part);
                }
                Component::CurDir => {}
                _ => {
                    return Err(ErrorKind::InvalidArgument
                        .to_error("overlay paths must be relative and cannot contain '..'"));
                }
            }
        }
        if normalized
            .components()
            .next()
            .is_some_and(|component| component.as_os_str() == OsStr::new(CONTROL_DIR))
        {
            return Err(ErrorKind::PermissionDenied
                .to_error(format!("{CONTROL_DIR} is reserved by OverlayFs")));
        }
        Ok(normalized)
    }

    pub(super) fn child_path(&self, parent: &Path, name: &OsStr) -> FuseResult<PathBuf> {
        if name.is_empty() || name == OsStr::new(".") || name == OsStr::new("..") {
            return Err(ErrorKind::InvalidArgument.to_error("invalid overlay entry name"));
        }
        if name.as_bytes().contains(&b'/') {
            return Err(ErrorKind::InvalidArgument.to_error("overlay entry name contains '/'"));
        }
        let mut path = self.normalize_path(parent)?;
        path.push(name);
        self.normalize_path(&path)
    }

    fn whiteout_lock(&self) -> FuseResult<MutexGuard<'_, HashSet<PathBuf>>> {
        self.whiteouts
            .lock()
            .map_err(|_| ErrorKind::InputOutputError.to_error("overlay whiteout lock poisoned"))
    }

    pub(super) fn mutation_guard(&self) -> FuseResult<MutexGuard<'_, ()>> {
        self.mutation_lock
            .lock()
            .map_err(|_| ErrorKind::InputOutputError.to_error("overlay mutation lock poisoned"))
    }

    fn has_whiteout(&self, path: &Path) -> FuseResult<bool> {
        Ok(self.whiteout_lock()?.contains(path))
    }

    fn set_whiteout(&self, path: &Path, value: bool) -> FuseResult<bool> {
        let mut whiteouts = self.whiteout_lock()?;
        let was_present = whiteouts.contains(path);
        if was_present == value {
            return Ok(was_present);
        }
        if value {
            whiteouts.insert(path.to_path_buf());
        } else {
            whiteouts.remove(path);
        }
        if let Err(error) = save_whiteouts(&self.control_dir.join("whiteouts"), &whiteouts) {
            if was_present {
                whiteouts.insert(path.to_path_buf());
            } else {
                whiteouts.remove(path);
            }
            return Err(error.into());
        }
        Ok(was_present)
    }

    pub(super) fn resolve(&self, path: &Path) -> FuseResult<Option<ResolvedEntry>> {
        let path = self.normalize_path(path)?;
        if path.as_os_str().is_empty() {
            let mut directories = vec![self.upper_dir.clone()];
            directories.extend(self.lower_dirs.iter().cloned());
            return Ok(Some(ResolvedEntry {
                path: self.upper_dir.clone(),
                kind: FileKind::Directory,
                from_upper: true,
                has_lower: !self.lower_dirs.is_empty(),
                directories,
            }));
        }

        let mut parent_directories = vec![self.upper_dir.clone()];
        parent_directories.extend(self.lower_dirs.iter().cloned());
        let mut prefix = PathBuf::new();
        let components = path.components().collect::<Vec<_>>();

        for (index, component) in components.iter().enumerate() {
            if parent_directories.is_empty() {
                return Err(ErrorKind::NotADirectory.to_error("path component is not a directory"));
            }
            prefix.push(component.as_os_str());
            let upper_parent = self
                .upper_dir
                .join(prefix.parent().unwrap_or(Path::new("")));
            let upper_is_parent = parent_directories.iter().any(|dir| dir == &upper_parent);
            let upper_path = self.upper_dir.join(&prefix);
            let whiteouted = self.has_whiteout(&prefix)?;
            let upper_metadata = if upper_is_parent {
                metadata_if_exists(&upper_path)?
            } else {
                None
            };
            let lower_parents = parent_directories
                .iter()
                .filter(|dir| *dir != &upper_parent)
                .collect::<Vec<_>>();

            let (entry_path, metadata, from_upper) = if let Some(metadata) = upper_metadata {
                (upper_path.clone(), metadata, true)
            } else {
                if whiteouted {
                    return Ok(None);
                }
                let mut found = None;
                for parent in &lower_parents {
                    let candidate = parent.join(component.as_os_str());
                    if let Some(metadata) = metadata_if_exists(&candidate)? {
                        found = Some((candidate, metadata));
                        break;
                    }
                }
                let Some((entry_path, metadata)) = found else {
                    return Ok(None);
                };
                (entry_path, metadata, false)
            };
            let kind = unix_fs::convert_filetype(metadata.file_type());

            let mut directories = Vec::new();
            let mut has_lower = false;
            if from_upper {
                for parent in &lower_parents {
                    let candidate = parent.join(component.as_os_str());
                    let Some(lower_metadata) = metadata_if_exists(&candidate)? else {
                        continue;
                    };
                    has_lower = true;
                    if !whiteouted && kind == FileKind::Directory {
                        if lower_metadata.file_type().is_dir() {
                            directories.push(candidate);
                        }
                    }
                    if !lower_metadata.file_type().is_dir() {
                        break;
                    }
                }
                if kind == FileKind::Directory {
                    directories.insert(0, entry_path.clone());
                }
            } else if kind == FileKind::Directory {
                let mut collecting = false;
                for parent in &lower_parents {
                    let candidate = parent.join(component.as_os_str());
                    let Some(lower_metadata) = metadata_if_exists(&candidate)? else {
                        continue;
                    };
                    if !lower_metadata.file_type().is_dir() {
                        if collecting {
                            break;
                        }
                        continue;
                    }
                    has_lower = true;
                    collecting = true;
                    directories.push(candidate);
                }
            } else {
                has_lower = lower_parents.iter().try_fold(false, |found, parent| {
                    if found {
                        return Ok::<_, PosixError>(true);
                    }
                    Ok(metadata_if_exists(&parent.join(component.as_os_str()))?.is_some())
                })?;
            }

            if index + 1 < components.len() && kind != FileKind::Directory {
                return Err(ErrorKind::NotADirectory.to_error("path component is not a directory"));
            }
            if index + 1 < components.len() && directories.is_empty() {
                return Ok(None);
            }
            parent_directories = directories.clone();
            if index + 1 == components.len() {
                return Ok(Some(ResolvedEntry {
                    path: entry_path,
                    kind,
                    from_upper,
                    has_lower,
                    directories,
                }));
            }
        }
        Ok(None)
    }

    pub(super) fn ensure_upper_parents(&self, path: &Path) -> FuseResult<()> {
        let parent = path.parent().unwrap_or(Path::new(""));
        let mut prefix = PathBuf::new();
        for component in parent.components() {
            prefix.push(component.as_os_str());
            let upper_path = self.upper_dir.join(&prefix);
            match metadata_if_exists(&upper_path)? {
                Some(metadata) if metadata.file_type().is_dir() => continue,
                Some(_) => {
                    return Err(
                        ErrorKind::NotADirectory.to_error("upper parent is not a directory")
                    );
                }
                None => {}
            }
            let Some(source) = self.resolve(&prefix)? else {
                return Err(ErrorKind::FileNotFound.to_error("parent directory does not exist"));
            };
            if source.kind != FileKind::Directory {
                return Err(ErrorKind::NotADirectory.to_error("parent is not a directory"));
            }
            fs::create_dir(&upper_path)?;
            let metadata = fs::symlink_metadata(&source.path)?;
            if let Err(error) = copy_metadata(&source.path, &upper_path, &metadata) {
                let _ = fs::remove_dir(&upper_path);
                return Err(error);
            }
        }
        Ok(())
    }

    pub(super) fn copy_up(&self, path: &Path) -> FuseResult<PathBuf> {
        let path = self.normalize_path(path)?;
        let destination = self.upper_dir.join(&path);
        if metadata_if_exists(&destination)?.is_some() {
            return Ok(destination);
        }
        let entry = self
            .resolve(&path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        if entry.from_upper {
            return Ok(entry.path);
        }
        self.ensure_upper_parents(&path)?;
        let metadata = fs::symlink_metadata(&entry.path)?;

        if entry.kind == FileKind::Symlink {
            let target = fs::read_link(&entry.path)?;
            symlink(target, &destination)?;
            return Ok(destination);
        }

        if entry.kind == FileKind::Directory {
            let temporary = unique_sibling(&destination, "copy")?;
            fs::create_dir(&temporary)?;
            if let Err(error) = copy_metadata(&entry.path, &temporary, &metadata) {
                let _ = fs::remove_dir(&temporary);
                return Err(error);
            }
            if let Err(error) = fs::rename(&temporary, &destination) {
                let _ = fs::remove_dir(&temporary);
                return Err(error.into());
            }
            return Ok(destination);
        }
        if entry.kind != FileKind::RegularFile {
            return Err(
                ErrorKind::NotSupported.to_error("copy-up of special files is not supported")
            );
        }

        let temporary = unique_sibling(&destination, "copy")?;
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| -> FuseResult<()> {
            let mut source = File::open(&entry.path)?;
            io::copy(&mut source, &mut target)?;
            target.flush()?;
            copy_metadata(&entry.path, &temporary, &metadata)?;
            target.sync_all()?;
            fs::rename(&temporary, &destination)?;
            Ok(())
        })();
        if result.is_err() {
            drop(target);
            let _ = fs::remove_file(&temporary);
        }
        result?;
        Ok(destination)
    }

    pub(super) fn copy_up_for_metadata(&self, path: &Path) -> FuseResult<PathBuf> {
        let entry = self
            .resolve(path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        if entry.kind == FileKind::Symlink {
            return Err(ErrorKind::NotSupported
                .to_error("metadata changes on symbolic links are not supported"));
        }
        self.copy_up(path)
    }

    pub(super) fn readdir_entries(&self, path: &Path) -> FuseResult<Vec<(OsString, FileKind)>> {
        let entry = self
            .resolve(path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("directory does not exist"))?;
        if entry.kind != FileKind::Directory {
            return Err(ErrorKind::NotADirectory.to_error("overlay entry is not a directory"));
        }
        let mut names = HashSet::new();
        for directory in entry.directories {
            for item in fs::read_dir(&directory)? {
                let item = item?;
                let name = item.file_name();
                if name.as_bytes().starts_with(TEMP_PREFIX.as_bytes()) {
                    continue;
                }
                if path.as_os_str().is_empty()
                    && directory == self.upper_dir
                    && name == OsStr::new(CONTROL_DIR)
                {
                    continue;
                }
                names.insert(name);
            }
        }
        let mut visible = BTreeMap::new();
        for name in names {
            let child = self.child_path(path, &name)?;
            if let Some(entry) = self.resolve(&child)? {
                visible.insert(name, entry.kind);
            }
        }
        let mut entries = vec![
            (OsString::from("."), FileKind::Directory),
            (OsString::from(".."), FileKind::Directory),
        ];
        entries.extend(visible);
        Ok(entries)
    }

    pub(super) fn unlink_path(&self, path: &Path) -> FuseResult<()> {
        let path = self.normalize_path(path)?;
        let entry = self
            .resolve(&path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        let existing_whiteout = self.has_whiteout(&path)?;
        let add_whiteout = entry.has_lower || existing_whiteout;
        let was_whiteouted = if add_whiteout {
            Some(self.set_whiteout(&path, true)?)
        } else {
            None
        };
        if entry.from_upper {
            if let Err(error) = unix_fs::unlink(&entry.path) {
                if was_whiteouted == Some(false) {
                    let _ = self.set_whiteout(&path, false);
                }
                return Err(error);
            }
        }
        if !entry.from_upper && !add_whiteout {
            self.set_whiteout(&path, true)?;
        }
        Ok(())
    }

    pub(super) fn rmdir_path(&self, path: &Path) -> FuseResult<()> {
        let path = self.normalize_path(path)?;
        let entry = self
            .resolve(&path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay directory does not exist"))?;
        if entry.kind != FileKind::Directory {
            return Err(ErrorKind::NotADirectory.to_error("overlay entry is not a directory"));
        }
        if self
            .readdir_entries(&path)?
            .iter()
            .any(|(name, _)| name != "." && name != "..")
        {
            return Err(ErrorKind::DirectoryNotEmpty.to_error("overlay directory is not empty"));
        }
        let existing_whiteout = self.has_whiteout(&path)?;
        let add_whiteout = entry.has_lower || existing_whiteout;
        let was_whiteouted = if add_whiteout {
            Some(self.set_whiteout(&path, true)?)
        } else {
            None
        };
        if entry.from_upper {
            if let Err(error) = unix_fs::rmdir(&entry.path) {
                if was_whiteouted == Some(false) {
                    let _ = self.set_whiteout(&path, false);
                }
                return Err(error);
            }
        } else if !add_whiteout {
            self.set_whiteout(&path, true)?;
        }
        Ok(())
    }

    pub(super) fn rename_path(
        &self,
        old_path: &Path,
        new_path: &Path,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        let old_path = self.normalize_path(old_path)?;
        let new_path = self.normalize_path(new_path)?;
        if old_path == new_path {
            return Ok(());
        }
        let source = self
            .resolve(&old_path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("rename source does not exist"))?;
        if flags.intersects(RenameFlags::RENAME_EXCHANGE | RenameFlags::RENAME_WHITEOUT) {
            return Err(ErrorKind::NotSupported.to_error("requested rename flag is not supported"));
        }
        if source.kind == FileKind::Directory
            && (!source.from_upper || source.directories.len() > 1)
        {
            return Err(ErrorKind::InvalidCrossDeviceLink
                .to_error("renaming lower or merged directories is not supported"));
        }
        let destination = self.resolve(&new_path)?;
        if flags.contains(RenameFlags::RENAME_NOREPLACE) && destination.is_some() {
            return Err(ErrorKind::FileExists.to_error("rename destination already exists"));
        }
        if let Some(destination) = &destination {
            match (
                source.kind == FileKind::Directory,
                destination.kind == FileKind::Directory,
            ) {
                (true, false) => {
                    return Err(
                        ErrorKind::NotADirectory.to_error("rename destination is not a directory")
                    );
                }
                (false, true) => {
                    return Err(
                        ErrorKind::IsADirectory.to_error("rename destination is a directory")
                    );
                }
                (true, true)
                    if self
                        .readdir_entries(&new_path)?
                        .iter()
                        .any(|(name, _)| name != "." && name != "..") =>
                {
                    return Err(
                        ErrorKind::DirectoryNotEmpty.to_error("rename destination is not empty")
                    );
                }
                _ => {}
            }
        }
        let old_whiteout = self.has_whiteout(&old_path)?;
        let new_whiteout = self.has_whiteout(&new_path)?;
        let source_needs_whiteout = source.has_lower || old_whiteout || !source.from_upper;
        let destination_needs_whiteout =
            destination.as_ref().is_some_and(|entry| entry.has_lower) || new_whiteout;

        let source_path = if source.from_upper {
            source.path
        } else {
            self.copy_up(&old_path)?
        };
        self.ensure_upper_parents(&new_path)?;
        if source_needs_whiteout && !old_whiteout {
            self.set_whiteout(&old_path, true)?;
        }
        if destination_needs_whiteout && !new_whiteout {
            if let Err(error) = self.set_whiteout(&new_path, true) {
                if source_needs_whiteout && !old_whiteout {
                    let _ = self.set_whiteout(&old_path, false);
                }
                return Err(error);
            }
        }
        if let Err(error) = unix_fs::rename(&source_path, &self.upper_dir.join(&new_path), flags) {
            if source_needs_whiteout && !old_whiteout {
                let _ = self.set_whiteout(&old_path, false);
            }
            if destination_needs_whiteout && !new_whiteout {
                let _ = self.set_whiteout(&new_path, false);
            }
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn link_path(
        &self,
        source_path: &Path,
        destination_path: &Path,
    ) -> FuseResult<FileAttribute> {
        let source_path = self.normalize_path(source_path)?;
        let destination_path = self.normalize_path(destination_path)?;
        let source = self
            .resolve(&source_path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("hard link source does not exist"))?;
        if source.kind == FileKind::Directory {
            return Err(
                ErrorKind::PermissionDenied.to_error("hard links to directories are not supported")
            );
        }
        if self.resolve(&destination_path)?.is_some() {
            return Err(ErrorKind::FileExists.to_error("hard link destination already exists"));
        }
        let source_upper = self.copy_up(&source_path)?;
        self.ensure_upper_parents(&destination_path)?;
        let destination_upper = self.upper_dir.join(destination_path);
        fs::hard_link(source_upper, &destination_upper)?;
        unix_fs::lookup(&destination_upper)
    }
}

#[cfg(test)]
mod tests;
