# Changelog

This changelog records notable public API changes to help users upgrade between
versions. Breaking changes and required migration steps are called out explicitly;
internal implementation changes and routine fixes are omitted.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-09-30

### Breaking changes

- `FuseHandler` now requires the associated type `FileHandle`. File operations
  use this type for per-open resources: `open` and `create` return it, and
  callbacks such as `read` receive it through `Option<&mut Self::FileHandle>`.
  Choose `()` for stateless I/O, `std::os::fd::OwnedFd` with
  `FileDescriptorHandler`, or a custom type for per-open state. Update handler
  implementations and delegated presets to use the new signatures.
- `SetAttrRequest` no longer has a lifetime parameter or a `file_handle` field
  or builder method. Remove lifetime arguments and `.file_handle(...)` calls.
- `DeviceType::from_rdev` was replaced by
  `DeviceType::from_file_type_and_rdev(FileType, libc::dev_t)`. File kind is
  now supplied separately from device numbers; `to_rdev` returns only the
  device number.
- `DefaultFuseHandler` was replaced by `StatelessHandler` for stateless
  directory callbacks and `UnimplementedFuseHandler` for unsupported
  operations. The former name has no compatibility alias.
- `FdHandlerHelper` and `FdHandlerHelperReadOnly` were renamed to
  `FileDescriptorHandler` and `FileDescriptorHandlerReadOnly`, and the module
  `fd_handler_helper` was renamed to `file_descriptor_handler`. The old type
  names remain deprecated aliases; update module paths and type names.
- Public `fuser` types now come from 0.18.0. If your project also depends
  directly on `fuser`, align it to 0.18.0 to avoid incompatible duplicate types.
- Removed `InodeMultiMapper` and `HybridId<BackingId>`. Use `InodeMapper` and
  `MappedInode`; create hard links through `FuseHandler::link` to share an
  automatically assigned FUSE inode.
- Moved mapper and resolver types under `easy_fuser::inode_mapping`. The old
  `inode_mapper` module path remains as a deprecated re-export.
- Deprecated the `FileIdType` implementation for `Vec<OsString>` in favor of
  `MappedInode`.

### Added

- Added `OverlayFs`, a preset with ordered lower layers, merged directories,
  copy-up for writes, and persistent whiteouts in the upper layer.
- Added async-compatible `MirrorFs` and file-descriptor presets. On Linux, the
  optional `io_uring` feature enables io_uring for selected descriptor-backed
  operations in these presets.
- `FileDescriptorHandler` now provides `getlk` and `setlk` for open file
  descriptors. `MirrorFs` and its read-only and async variants provide `bmap`
  and `readdirplus`.
- Re-exported `fuser::BsdFileFlags` through `easy_fuser::types` and the
  mode-specific preludes for use with `SetAttrRequest`.
- Added `check_mode_access` to check access against Unix mode bits.

- `InodeMapper` now tracks multiple directory entries for one inode. Its
  `link` method registers an additional name after a successful hard link;
  `get` exposes current links and `resolve` reconstructs their paths.
- Added the `MappedInode` handler ID, with `inode()`, `paths()`, and
  `parts_paths()` accessors. It uses ordinary `FileAttribute` and `FileKind`
  metadata values.
- Successful `link`, `unlink`, and `rename` calls update the mapper. Explicitly
  registered hard links remain associated after FUSE forgets lookup references.

## [0.6.0] - 2026-09-28

### Breaking changes

- Adopted the `fuser` 0.17 API. `FuseHandler` and related public APIs now use
  `fuser::INodeNo` (re-exported as `Inode`) instead of `u64` or the former
  crate-defined `Inode` wrapper. `FileIdResolver` methods also use `Inode`;
  construct values with `INodeNo(value)` and access the number with `.0`.
- Updated handler argument types to the corresponding `fuser` flag types:
  `AccessFlags`, `OpenFlags`, `RenameFlags`, `IoctlFlags`, `WriteFlags`,
  `CopyFileRangeFlags`, and `FopenFlags`. Several crate-defined flag types
  were renamed or removed. Copy-file-range offsets are now `u64`.
- `RequestInfo.id` is now `fuser::RequestId`, and `SetAttrRequest::flags`
  accepts `fuser::BsdFileFlags` instead of `()`.
- Replaced `PosixError::raw_error()` with `PosixError::io_error()`, returning
  `std::io::Error`.
- `FuseSession::join` now returns `std::io::Result<()>`; handle the result when
  joining a background-mounted session.

## [0.5.0] - 2026-06-25

### Breaking changes

- Replaced `easy_fuser::prelude` with mode-specific preludes:
  `easy_fuser::fuse_serial::prelude`,
  `easy_fuser::fuse_parallel::prelude`, and
  `easy_fuser::fuse_async::prelude`.
- Moved preset implementations from `easy_fuser::templates` to
  `easy_fuser::fuse_presets`.
- Presets such as `DefaultFuseHandler` and `MirrorFs` no longer implement
  `FuseHandler` directly. Implement the trait on your own type and use the
  delegation macros to select the operations supplied by each preset.

### Added

- Added async/await support through the `fuse_async` feature and
  `delegate_fs_async!` and `delegate_fs_sync_to_async!` macros. Added the
  synchronous `delegate_fs!` macro for serial and parallel handlers.
- Added the `easy_fuser_macro` crate, which provides these delegation macros
  and the `fuse_handler_fnsig!` macro.
- Added the public `FuseSession` and `FusePruner` APIs for managing a
  background-mounted session and pruning unreferenced inodes.
- Added default `readdirplus` behavior that combines `readdir` and `lookup`,
  and a no-op default implementation of `forget`.

## [0.4.4] and earlier

Please refer to the [GitHub releases page](https://github.com/Alogani/easy_fuser/releases) and git history for older changelogs.

---

[0.7.0]: https://github.com/Alogani/easy_fuser/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/Alogani/easy_fuser/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Alogani/easy_fuser/compare/v0.4.5...v0.5.0
