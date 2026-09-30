//! # OverlayFs
//!
//! `OverlayFs` presents one writable FUSE tree from an upper directory and zero or more read-mostly
//! lower directories. It resolves paths across those layers and directs changes to the upper
//! directory, leaving lower contents alone. It is a filesystem preset for building a FUSE handler,
//! rather than a mount manager or a wrapper around Linux's kernel OverlayFS.
//!
//! ## How the layers work
//!
//! - **Upper directory:** the writable layer. New entries are created here. When a lower entry is
//!   changed, OverlayFs copies it and its needed parent directories here first (copy-up); subsequent
//!   reads and writes use the upper copy.
//! - **Lower directories:** source layers. An entry in the upper directory takes precedence over
//!   all lowers. Otherwise, lowers are searched in the order passed to `new`; the first matching
//!   entry wins. Directories with the same path are merged by name, so their child entries can come
//!   from different layers. Deleting a lower entry records a whiteout in the upper layer, hiding
//!   that path while preserving its source.
//!
//! The whiteout list persists under the reserved `.easy_fuser_overlay` directory inside the upper
//! directory. This lets a new `OverlayFs` instance reopen the same view with the same layers. Do not
//! expose that directory as ordinary filesystem content.
//!
//! ```rust,ignore
//! let overlay = OverlayFs::new(
//!     "/var/lib/app/upper",
//!     ["/usr/share/app/site", "/usr/share/app/defaults"],
//! )?;
//! ```
//!
//! In this example, changes live in `/var/lib/app/upper`; `site` takes precedence over `defaults`,
//! and both lower trees remain unchanged. Pass an empty lower list to make an upper-only filesystem.
//!
//! ## OverlayFs and Linux OverlayFS
//!
//! Both provide an upper/lower union and copy lower files up before writes. Linux OverlayFS is a
//! kernel filesystem with its own on-disk whiteouts, opaque-directory markers, mount options, and
//! features such as directory redirects and optional metadata-only copy-up. This preset instead
//! implements the view in userspace through FUSE. It stores whiteouts in its own private file and
//! does not use Linux OverlayFS metadata; the two implementations cannot share an upper directory
//! as interchangeable layers. Their supported operations and edge-case behavior also differ.
//!
//! Prefer Linux OverlayFS on Linux when its behavior fits the application: it is the native kernel
//! implementation, avoids routing each filesystem operation through a userspace FUSE handler, and
//! provides kernel-specific features and compatibility with tools expecting OverlayFS mounts. Use
//! this preset when you need to build the filesystem into an `easy_fuser` application, add
//! application-specific behavior in Rust, or need this implementation's FUSE-facing composition.
//! See the [Linux OverlayFS documentation](https://docs.kernel.org/filesystems/overlayfs.html) for
//! kernel semantics and mount requirements.
//!
//! ## Using it in a FUSE handler
//!
//! `OverlayFs` implements the operations in the table. Delegate these methods to the overlay field;
//! `StatelessHandler` or a custom implementation can handle the directory bookkeeping methods.
//! See the [preset composition examples](crate::fuse_presets) for `delegate_fs!` syntax. If you add
//! custom behavior for an operation, implement it on your handler and remove it from the overlay
//! delegation list.
//!
//! | Purpose | Delegate to | Operations |
//! | --- | --- | --- |
//! | Layer lookup and listing | `OverlayFs` | `access`, `getattr`, `lookup`, `readdir`, `readdirplus`, `readlink` |
//! | File I/O and descriptors | `OverlayFs` | `open`, `create`, `read`, `write`, `release`, `flush`, `fsync`, `fallocate`, `copy_file_range`, `lseek`, `bmap`, `getlk`, `ioctl`, `setlk` |
//! | Create, remove, and change entries | `OverlayFs` | `mkdir`, `mknod`, `symlink`, `link`, `unlink`, `rmdir`, `rename`, `setattr` |
//! | Extended attributes and filesystem stats | `OverlayFs` | `getxattr`, `listxattr`, `setxattr`, `removexattr`, `statfs` |
//! | Directory bookkeeping | `StatelessHandler` or your own implementation | `forget`, `fsyncdir`, `opendir`, `releasedir` |
//!
//! ## Limitations
//!
//! - The upper directory and all lower directories must already exist, must be directories, and
//!   must not overlap each other. The mount point must be outside these directories.
//! - Do not modify the backing directories outside OverlayFs while the filesystem is running.
//! - Use a given upper directory with only one live `OverlayFs` instance.
//! - `.easy_fuser_overlay` is reserved at the root of each layer and stores private whiteout state
//!   in the upper directory. The format is specific to this implementation and is not compatible
//!   with Linux OverlayFS whiteouts.
//! - Copy-up supports ordinary files, directories, and symbolic links. Copying up device files,
//!   pipes, or sockets is not supported.
//! - Renaming a lower-layer or merged directory is not supported.
//! - Copying up a symbolic link preserves its destination, but not its owner or timestamps;
//!   changing symbolic-link metadata is not supported.
//! - `statfs` reports the upper directory's filesystem statistics, not combined capacity across
//!   layers.

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
        #[cfg(target_os = "linux")]
        let unsupported_flags =
            flags.intersects(RenameFlags::RENAME_EXCHANGE | RenameFlags::RENAME_WHITEOUT);
        #[cfg(not(target_os = "linux"))]
        let unsupported_flags = !flags.is_empty();
        if unsupported_flags {
            return Err(ErrorKind::NotSupported.to_error("requested rename flag is not supported"));
        }
        if source.kind == FileKind::Directory
            && (!source.from_upper || source.directories.len() > 1)
        {
            return Err(ErrorKind::InvalidCrossDeviceLink
                .to_error("renaming lower or merged directories is not supported"));
        }
        let destination = self.resolve(&new_path)?;
        #[cfg(target_os = "linux")]
        let no_replace = flags.contains(RenameFlags::RENAME_NOREPLACE);
        #[cfg(not(target_os = "linux"))]
        let no_replace = false;
        if no_replace && destination.is_some() {
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
