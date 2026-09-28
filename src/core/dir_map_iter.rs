use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
};

use fuser::INodeNo;

pub(crate) type DirEntry<TAttr> = (OsString, INodeNo, TAttr);

/// Keeps directory entries between successive kernel `readdir` requests.
///
/// FUSE treats the offset attached to an entry as an opaque continuation token.
/// We use the number of entries already accepted by the reply buffer as that
/// token and retain the remaining entries until the kernel requests them.
pub(crate) struct DirMapIter<TAttr> {
    continuations: HashMap<(INodeNo, u64), VecDeque<DirEntry<TAttr>>>,
}

impl<TAttr> Default for DirMapIter<TAttr> {
    fn default() -> Self {
        Self {
            continuations: HashMap::new(),
        }
    }
}

impl<TAttr> DirMapIter<TAttr> {
    pub(crate) fn start(
        ino: INodeNo,
        entries: impl IntoIterator<Item = DirEntry<TAttr>>,
    ) -> DirRead<TAttr> {
        DirRead {
            ino,
            offset: 0,
            entries: entries.into_iter().collect(),
        }
    }

    pub(crate) fn resume(&mut self, ino: INodeNo, offset: u64) -> Option<DirRead<TAttr>> {
        self.continuations
            .remove(&(ino, offset))
            .map(|entries| DirRead {
                ino,
                offset,
                entries,
            })
    }

    pub(crate) fn save(&mut self, read: DirRead<TAttr>) {
        if read.offset > 0 && !read.entries.is_empty() {
            self.continuations
                .insert((read.ino, read.offset), read.entries);
        }
    }
}

pub(crate) struct DirRead<TAttr> {
    ino: INodeNo,
    offset: u64,
    entries: VecDeque<DirEntry<TAttr>>,
}

impl<TAttr> DirRead<TAttr> {
    /// Returns an entry and the offset to attach to it in the FUSE reply.
    pub(crate) fn next(&mut self) -> Option<(u64, DirEntry<TAttr>)> {
        let entry = self.entries.pop_front()?;
        self.offset += 1;
        Some((self.offset, entry))
    }

    /// Restores an entry which did not fit in the FUSE reply buffer.
    pub(crate) fn retry(&mut self, entry: DirEntry<TAttr>) {
        debug_assert!(self.offset > 0);
        self.offset -= 1;
        self.entries.push_front(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, ino: u64, attr: u8) -> DirEntry<u8> {
        (OsString::from(name), INodeNo(ino), attr)
    }

    #[test]
    fn offsets_identify_the_next_entry() {
        let mut read =
            DirMapIter::start(INodeNo(1), [entry("first", 2, 10), entry("second", 3, 20)]);

        assert_eq!(read.next(), Some((1, entry("first", 2, 10))));
        assert_eq!(read.next(), Some((2, entry("second", 3, 20))));
        assert_eq!(read.next(), None);
    }

    #[test]
    fn rejected_entry_is_returned_on_resume() {
        let ino = INodeNo(1);
        let mut continuations = DirMapIter::default();
        let mut read = DirMapIter::start(
            ino,
            [
                entry("first", 2, 10),
                entry("second", 3, 20),
                entry("third", 4, 30),
            ],
        );

        assert_eq!(read.next(), Some((1, entry("first", 2, 10))));
        let (_, rejected) = read.next().unwrap();
        read.retry(rejected);
        continuations.save(read);

        let mut resumed = continuations.resume(ino, 1).unwrap();
        assert_eq!(resumed.next(), Some((2, entry("second", 3, 20))));
        assert_eq!(resumed.next(), Some((3, entry("third", 4, 30))));
        assert_eq!(resumed.next(), None);
    }

    #[test]
    fn continuation_is_consumed_when_resumed() {
        let ino = INodeNo(1);
        let mut continuations = DirMapIter::default();
        let mut read = DirMapIter::start(ino, [entry("entry", 2, 10)]);
        let (_, rejected) = read.next().unwrap();
        read.retry(rejected);

        // No entry was accepted, so offset zero must remain a fresh read.
        continuations.save(read);
        assert!(continuations.resume(ino, 0).is_none());

        let mut read = DirMapIter::start(ino, [entry("first", 2, 10), entry("second", 3, 20)]);
        read.next();
        let (_, rejected) = read.next().unwrap();
        read.retry(rejected);
        continuations.save(read);

        assert!(continuations.resume(ino, 1).is_some());
        assert!(continuations.resume(ino, 1).is_none());
    }

    #[test]
    fn continuations_are_scoped_by_inode_and_offset() {
        let mut continuations = DirMapIter::default();

        for ino in [INodeNo(1), INodeNo(2)] {
            let mut read = DirMapIter::start(ino, [entry("first", 10, 1), entry("second", 11, 2)]);
            read.next();
            let (_, rejected) = read.next().unwrap();
            read.retry(rejected);
            continuations.save(read);
        }

        assert!(continuations.resume(INodeNo(1), 2).is_none());
        assert_eq!(
            continuations.resume(INodeNo(2), 1).unwrap().next(),
            Some((2, entry("second", 11, 2)))
        );
        assert_eq!(
            continuations.resume(INodeNo(1), 1).unwrap().next(),
            Some((2, entry("second", 11, 2)))
        );
    }
}
