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
        self,
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
        self,
        fd: impl std::os::fd::AsFd,
    ) -> Result<PassthroughBackingId, std::io::Error> {
        self.reply_open
            .open_backing(fd)
            .map(|id| PassthroughBackingId {
                backing_id: Arc::new(id),
            })
    }
}

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
