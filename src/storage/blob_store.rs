use super::{BlobKeyEncoder, FileStore};
use crate::types::FuseResult;
use std::ops::Range;

/// Reads content addressed by logical blob IDs.
pub trait BlobStore: Send + Sync + 'static {
    type BlobId: Send + Sync + 'static;

    fn read(&self, blob: &Self::BlobId, range: Range<u64>) -> FuseResult<Vec<u8>>;
}

/// Composes a physical [`FileStore`] with a logical-to-physical key encoder.
pub struct EncodedBlobStore<S, E> {
    store: S,
    encoder: E,
}

impl<S, E> EncodedBlobStore<S, E> {
    pub fn new(store: S, encoder: E) -> Self {
        Self { store, encoder }
    }
}

impl<S, E> BlobStore for EncodedBlobStore<S, E>
where
    S: FileStore,
    E: BlobKeyEncoder<Key = S::Key>,
{
    type BlobId = E::BlobId;

    fn read(&self, blob: &Self::BlobId, range: Range<u64>) -> FuseResult<Vec<u8>> {
        let key = self.encoder.encode(blob)?;
        self.store.read(&key, range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ErrorKind;
    use std::sync::Mutex;

    struct Store {
        seen: Mutex<Option<(String, Range<u64>)>>,
        fail: bool,
    }

    impl FileStore for Store {
        type Key = String;

        fn read(&self, key: &String, range: Range<u64>) -> FuseResult<Vec<u8>> {
            if self.fail {
                return Err(ErrorKind::FileNotFound.to_error("store failure"));
            }
            *self.seen.lock().unwrap() = Some((key.clone(), range.clone()));
            Ok(vec![range.start as u8, range.end as u8])
        }
    }

    struct Encoder {
        fail: bool,
    }

    impl BlobKeyEncoder for Encoder {
        type BlobId = u64;
        type Key = String;

        fn encode(&self, blob: &u64) -> FuseResult<String> {
            if self.fail {
                Err(ErrorKind::InvalidArgument.to_error("encoder failure"))
            } else {
                Ok(format!("object/{blob}"))
            }
        }
    }

    #[test]
    fn encoded_store_encodes_key_and_forwards_range() {
        let store = Store {
            seen: Mutex::new(None),
            fail: false,
        };
        let blobs = EncodedBlobStore::new(store, Encoder { fail: false });
        assert_eq!(blobs.read(&17, 3..29).unwrap(), vec![3, 29]);
        let seen = blobs.store.seen.lock().unwrap();
        assert_eq!(seen.as_ref().unwrap(), &("object/17".to_owned(), 3..29));
    }

    #[test]
    fn encoded_store_propagates_encoder_and_store_errors() {
        let encoder_error = EncodedBlobStore::new(
            Store {
                seen: Mutex::new(None),
                fail: false,
            },
            Encoder { fail: true },
        )
        .read(&1, 0..1)
        .unwrap_err();
        assert_eq!(encoder_error.kind(), ErrorKind::InvalidArgument);

        let store_error = EncodedBlobStore::new(
            Store {
                seen: Mutex::new(None),
                fail: true,
            },
            Encoder { fail: false },
        )
        .read(&1, 0..1)
        .unwrap_err();
        assert_eq!(store_error.kind(), ErrorKind::FileNotFound);
    }
}
