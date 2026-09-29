//! Simple directory-related responses for filesystems that keep no directory state.
//!
//! # Methods this handles
//!
//! - `forget` does nothing. Override it if your filesystem tracks lookup references.
//! - `fsyncdir` returns success without syncing directory data. Override it if needed.
//! - `opendir` returns a placeholder handle; `releasedir` ignores it. This is
//!   suitable only when your filesystem keeps no per-directory handle state.
//!
//! Implement any of these methods yourself if those responses do not fit your
//! filesystem. Use [`crate::fuse_presets::UnimplementedFuseHandler`] for operations it does not support.

use std::marker::PhantomData;

use crate::types::*;

/// Provides directory defaults for filesystems that keep no directory state.
pub struct StatelessHandler<TId> {
    phantom: PhantomData<fn() -> TId>,
}

impl<TId: FileIdType> StatelessHandler<TId> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    pub fn forget(&self, _req: &RequestInfo, _file_id: TId, _nlookup: u64) {}

    pub fn fsyncdir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _file_handle: BorrowedFileHandle,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    pub fn opendir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
        // This placeholder is only safe when releasedir ignores the handle.
        Ok((unsafe { OwnedFileHandle::from_raw(0) }, FopenFlags::empty()))
    }

    pub fn releasedir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _file_handle: OwnedFileHandle,
        _flags: OpenFlags,
    ) -> FuseResult<()> {
        Ok(())
    }
}

impl<TId: FileIdType> Default for StatelessHandler<TId> {
    fn default() -> Self {
        Self::new()
    }
}
