use crate::types::FuseResult;
use std::ops::Range;

/// Reads byte ranges from objects addressed by backend-specific physical keys.
pub trait FileStore: Send + Sync + 'static {
    type Key: Send + Sync + 'static;

    /// Reads bytes in the half-open range `range` from the physical object.
    fn read(&self, key: &Self::Key, range: Range<u64>) -> FuseResult<Vec<u8>>;
}
