#[cfg(feature = "passthrough")]
use fuser::BackingId;
use fuser::{ReplyCreate, ReplyOpen};
#[cfg(feature = "passthrough")]
use std::sync::{Arc, Weak};

pub struct OpenHelper<'a> {
    #[allow(dead_code)]
    reply_open: &'a ReplyOpen,
}

pub struct CreateHelper<'a> {
    #[allow(dead_code)]
    reply_create: &'a ReplyCreate,
}

impl<'a> CreateHelper<'a> {
    pub(crate) fn new(reply_create: &'a ReplyCreate) -> Self {
        Self { reply_create }
    }

    #[cfg(feature = "passthrough")]
    pub fn open_backing(
        &self,
        fd: impl std::os::fd::AsFd,
    ) -> Result<PassthroughBackingId, std::io::Error> {
        self.reply_create
            .open_backing(fd)
            .map(|id| PassthroughBackingId {
                backing_id: Arc::new(id),
            })
    }
}

impl<'a> OpenHelper<'a> {
    pub(crate) fn new(reply_open: &'a ReplyOpen) -> Self {
        Self { reply_open }
    }

    #[cfg(feature = "passthrough")]
    pub fn open_backing(
        &self,
        fd: impl std::os::fd::AsFd,
    ) -> Result<PassthroughBackingId, std::io::Error> {
        self.reply_open
            .open_backing(fd)
            .map(|id| PassthroughBackingId {
                backing_id: Arc::new(id),
            })
    }
}

/// Passthrough backing ID for FUSE passthrough operations.
///
/// This structure is created by the [`OpenHelper::open_backing`]
/// and [`CreateHelper::open_backing`] methods, and allows the FUSE
/// kernel module to bypass the FUSE daemon to increase performance.
///
/// This structure is a no-op if the `passthrough` feature is not
/// enabled.
///
/// In the scope of a single file ID (e.g. [`Inode`](crate::types::inode::Inode),
/// [`PathBuf`](std::path::PathBuf), [`Vec<OsString>`](std::ffi::OsString) or
/// [`HybridId<BackingId>`](crate::types::file_id_type::HybridId<BackingId>)),
/// - All active file handle objects ([`OwnedFileHandle`](crate::types::file_handle::OwnedFileHandle))
/// must be opened in the same mode (passthrough or non-passthrough).
/// - If the file handles are opened in passthrough mode, they must share the same
/// [`PassthroughBackingId`] value. Identical [`PassthroughBackingId`] values can be
/// obtained by cloning an existing value.
/// - If either or both of these rules is not met, the FUSE kernel module will return
/// an [`EIO`](libc::EIO) error code that is very difficult to troubleshoot. It is best
/// to return [`EBUSY`](libc::EBUSY) when you detect this situation, which usually happens
/// when trying to open a file twice may result in two completely different file views.
///
/// Backing IDs can be downgraded to a [`WeakPassthroughBackingId`] value, similar to
/// the underlying [`Arc`] type. This allows the passthrough ID references to be stored in
/// a file ID to passthrough ID hash table (required to fulfill these above two rules)
/// without interfering with the ability of the backing ID to be released when the last
/// file handle to an inode is closed.
#[derive(Debug, Clone)]
pub struct PassthroughBackingId {
    #[cfg(feature = "passthrough")]
    pub(crate) backing_id: Arc<BackingId>,
}

#[cfg(feature = "passthrough")]
impl AsRef<BackingId> for PassthroughBackingId {
    fn as_ref(&self) -> &BackingId {
        self.backing_id.as_ref()
    }
}

impl PassthroughBackingId {
    #[cfg(feature = "passthrough")]
    pub fn downgrade(&self) -> WeakPassthroughBackingId {
        WeakPassthroughBackingId {
            backing_id: Arc::downgrade(&self.backing_id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeakPassthroughBackingId {
    #[cfg(feature = "passthrough")]
    pub(crate) backing_id: Weak<BackingId>,
}

impl WeakPassthroughBackingId {
    #[cfg(feature = "passthrough")]
    pub fn new() -> Self {
        Self {
            backing_id: Weak::new(),
        }
    }

    #[cfg(feature = "passthrough")]
    pub fn upgrade(&self) -> Option<PassthroughBackingId> {
        self.backing_id
            .upgrade()
            .map(|backing_id| PassthroughBackingId {
                backing_id: backing_id,
            })
    }
}
