//! Reusable operations for composing a FUSE filesystem.
//!
//! A preset is an ordinary value stored in your filesystem struct. It does not
//! implement `FuseHandler` for you and presets do not automatically call one
//! another. In your `FuseHandler` implementation, route each operation to one
//! field with a delegation macro, or implement it yourself.
//!
//! A method written directly in your handler takes ownership of that operation.
//! Remove it from any delegation list; when you want to keep a preset's behavior,
//! call that preset explicitly from your method after applying your custom rule.
//!
//! # A simple filesystem
//!
//! Use [`StatelessHandler`] for directory callbacks that need no per-directory
//! state. Use [`UnimplementedFuseHandler`] for operations your filesystem does
//! not support. Its recommended mode returns `ENOSYS`.
//!
//! ```rust,no_run
//! use easy_fuser::fuse_serial::prelude::*;
//! use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
//! use easy_fuser_macro::delegate_fs;
//! use std::path::PathBuf;
//!
//! struct AppFs {
//!     directory: StatelessHandler<PathBuf>,
//!     unsupported: UnimplementedFuseHandler<PathBuf>,
//! }
//!
//! impl FuseHandler for AppFs {
//!     type TId = PathBuf;
//!
//!     delegate_fs! { directory, [ forget, fsyncdir, opendir, releasedir ] }
//!     delegate_fs! { unsupported, [
//!         access, bmap, copy_file_range, create, fallocate, flush, fsync,
//!         getattr, getlk, getxattr, ioctl, link, listxattr, lookup, lseek,
//!         mkdir, mknod, open, read, readdir, readdirplus, readlink, release,
//!         removexattr, rename, rmdir, setattr, setlk, setxattr, statfs,
//!         symlink, unlink, write
//!     ]}
//! }
//! ```
//!
//! # A customized filesystem
//!
//! For custom behavior, leave that operation out of the delegation list and
//! implement it on your own type. Call a preset from your implementation when
//! you want to add a rule around its behavior:
//!
//! ```rust,no_run
//! use easy_fuser::fuse_serial::prelude::*;
//! use easy_fuser::fuse_presets::{OverlayFs, StatelessHandler, UnimplementedFuseHandler};
//! use easy_fuser_macro::delegate_fs;
//! use std::ffi::OsStr;
//! use std::path::PathBuf;
//!
//! struct AppFs {
//!     overlay: OverlayFs,
//!     directory: StatelessHandler<PathBuf>,
//!     unsupported: UnimplementedFuseHandler<PathBuf>,
//!     panic_fallback: UnimplementedFuseHandler<PathBuf>,
//! }
//!
//! impl FuseHandler for AppFs {
//!     type TId = PathBuf;
//!
//!     delegate_fs! { overlay, [
//!         access, copy_file_range, create, fallocate, flush, fsync, getattr,
//!         getxattr, listxattr, link, lookup, lseek, mkdir, mknod, open, read,
//!         readdir, readlink, release, removexattr, rename, rmdir, setattr,
//!         setxattr, statfs, symlink, write
//!     ]}
//!     delegate_fs! { directory, [ forget, fsyncdir, opendir, releasedir ] }
//!     delegate_fs! { unsupported, [ readdirplus, setlk ] }
//!     delegate_fs! { panic_fallback, [ ioctl ] }
//!
//!     fn unlink(&self, req: &RequestInfo, parent: PathBuf, name: &OsStr) -> FuseResult<()> {
//!         if parent.as_os_str().is_empty() && name == "important.db" {
//!             return Err(ErrorKind::PermissionDenied.to_error("this file is protected"));
//!         }
//!         self.overlay.unlink(req, parent, name)
//!     }
//!
//!     fn getlk(
//!         &self,
//!         req: &RequestInfo,
//!         file_id: PathBuf,
//!         file_handle: BorrowedFileHandle<'_>,
//!         lock_owner: u64,
//!         lock_info: LockInfo,
//!     ) -> FuseResult<LockInfo> {
//!         // Implement yourself some logic before delegating back to an existing handler.
//!         if lock_info.start > lock_info.end {
//!             return Err(ErrorKind::InvalidArgument.to_error("invalid lock range"));
//!         }
//!         self.unsupported
//!             .getlk(req, file_id, file_handle, lock_owner, lock_info)
//!     }
//!
//!     fn bmap(
//!         &self,
//!         _req: &RequestInfo,
//!         _file_id: PathBuf,
//!         _blocksize: u32,
//!         _idx: u64,
//!     ) -> FuseResult<u64> {
//!         // Handle the operation yourself without delegation
//!         Err(ErrorKind::FunctionNotImplemented.to_error("block mapping is not supported"))
//!     }
//! }
//!
//! impl AppFs {
//!     pub fn new() -> std::io::Result<Self> {
//!         Ok(Self {
//!             overlay: OverlayFs::new("/tmp/app/upper", ["/tmp/app/lower"])?,
//!             directory: StatelessHandler::new(),
//!             unsupported: UnimplementedFuseHandler::new(),
//!             panic_fallback: UnimplementedFuseHandler::new_with_panic(),
//!         })
//!     }
//! }
//!
//! ```
//!
//! `unlink`, `getlk`, and `bmap` appear in no delegation list because the handler implements them.
//! This same pattern works with `MirrorFs` or any other preset: delegate the
//! operations you want unchanged, and write only the operations you need to
//! customize. Async handlers use `delegate_fs_async!` for async presets and
//! `delegate_fs_sync_to_async!` for these synchronous presets.
//!
//! # Presets and the operations they provide
//!
//! Each preset below is a small delegation target, not a complete filesystem.
//! The snippets show the field and the operations commonly delegated to it.
//!
//! ## `StatelessHandler`
//!
//! Provides `forget`, `fsyncdir`, `opendir`, and `releasedir`. These responses
//! fit filesystems that do not track lookup references or keep directory-open
//! state. Implement any of these yourself when your filesystem needs that state.
//!
//! ```rust,ignore
//! directories: StatelessHandler<PathBuf>,
//! delegate_fs! { directories, [ forget, fsyncdir, opendir, releasedir ] }
//! ```
//!
//! ## `UnimplementedFuseHandler`
//!
//! Provides explicit error or panic responses for operations your filesystem
//! does not support. Prefer `new()` (`ENOSYS`); use `new_with_panic()` while
//! developing to find operations you have not handled.
//!
//! ```rust,ignore
//! unsupported: UnimplementedFuseHandler<PathBuf>,
//! delegate_fs! { unsupported, [ bmap, getlk, ioctl, readdirplus, setlk ] }
//! // Initialize with UnimplementedFuseHandler::new().
//! ```
//!
//! ## `FileDescriptorHandler`
//!
//! Provides `flush`, `fsync`, `lseek`, `read`, `release`, `copy_file_range`,
//! `fallocate`, and `write` for file handles that contain open file descriptors.
//! Your `open` and `create` methods must return those descriptors as file handles.
//!
//! ```rust,ignore
//! file_io: FileDescriptorHandler<PathBuf>,
//! delegate_fs! { file_io, [
//!     copy_file_range, fallocate, flush, fsync, lseek, read, release, write
//! ]}
//! ```
//!
//! `FileDescriptorHandlerReadOnly` provides `flush`, `fsync`, `lseek`, `read`,
//! and `release` only. It does not make the rest of your filesystem read-only;
//! use a read-only mount option when needed.
//!
//! ```rust,ignore
//! file_io: FileDescriptorHandlerReadOnly<PathBuf>,
//! delegate_fs! { file_io, [ flush, fsync, lseek, read, release ] }
//! ```
//!
//! ## `MirrorFs`
//!
//! Mirrors an existing source directory. `MirrorFs` provides read and write
//! operations; `MirrorFsReadOnly` omits write operations. Both require your
//! handler to delegate the operations it wants to provide.
//!
//! ```rust,ignore
//! mirror: MirrorFs,
//! delegate_fs! { mirror, [ access, getattr, lookup, open, read, readdir, release ] }
//! ```
//!
//! `MirrorFsReadOnly` uses the same pattern with a `MirrorFsReadOnly` field.
//! To reject writes across the entire filesystem, also mount it read-only.
//!
//! ```rust,ignore
//! mirror: MirrorFsReadOnly,
//! delegate_fs! { mirror, [ access, getattr, lookup, open, read, readdir, release ] }
//! ```
//!
//! ## `OverlayFs`
//!
//! Combines an upper directory with ordered lower directories. It handles
//! merged lookup and directory behavior, file I/O, and updates through
//! copy-up. Start with the basic composition, then remove individual methods
//! from its delegation list when adding custom behavior.
//!
//! ```rust,ignore
//! overlay: OverlayFs,
//! delegate_fs! { overlay, [ getattr, lookup, open, read, readdir, release, write ] }
//! ```
//!
//! See the [OverlayFs guide](fuse_presets/overlay_fs.rs) for its complete
//! operation list, setup requirements, limitations, and customization examples.

mod stateless_handler;
pub use stateless_handler::StatelessHandler;

mod unimplemented_fuse_handler;
pub use unimplemented_fuse_handler::UnimplementedFuseHandler;

pub mod fd_handler_helper;
pub use fd_handler_helper::{FileDescriptorHandler, FileDescriptorHandlerReadOnly};

pub mod mirror_fs;

pub mod overlay_fs;
pub use overlay_fs::OverlayFs;
