//! Storage interfaces for physical files and logical blobs.
//!
//! `FileStore` reads backend-specific physical keys. `BlobKeyEncoder` maps
//! logical blob IDs to those keys without performing I/O, and `BlobStore`
//! exposes logical blob reads to consumers such as `CatalogFs`.

mod blob_key_encoder;
mod blob_store;
mod file_store;

pub use blob_key_encoder::BlobKeyEncoder;
pub use blob_store::{BlobStore, EncodedBlobStore};
pub use file_store::FileStore;
