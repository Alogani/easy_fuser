use crate::types::FuseResult;

/// Converts logical blob IDs into backend-specific physical storage keys.
///
/// This performs no I/O and need not be cryptographic: implementations may
/// add prefixes, shard directories, format hashes, scramble IDs, or construct
/// a backend-specific key.
pub trait BlobKeyEncoder: Send + Sync + 'static {
    type BlobId: Send + Sync + 'static;
    type Key: Send + Sync + 'static;

    fn encode(&self, blob: &Self::BlobId) -> FuseResult<Self::Key>;
}
