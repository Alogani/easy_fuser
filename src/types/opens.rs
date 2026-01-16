#[cfg(feature = "passthrough")]
use fuser::BackingId;
use fuser::ReplyOpen;
#[cfg(feature = "passthrough")]
use std::sync::Arc;

pub struct OpenHelper<'a> {
    reply_open: &'a ReplyOpen,
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

impl AsRef<BackingId> for PassthroughBackingId {
    fn as_ref(&self) -> &BackingId {
        self.backing_id.as_ref()
    }
}
