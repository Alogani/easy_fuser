use std::{
    collections::HashMap,
    ffi::OsStr,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

use fuser::{
    self, KernelConfig, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

use libc::c_int;
use log::error;

use crate::fuse_api::{FuseAPI, ReplyCb};
use crate::types::*;

use super::inode_mapping::{IdConverter, IdType};

type DirIter<T> = Box<dyn Iterator<Item = T> + Send>;

fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

pub struct FuseFilesystem<T, U, C>
where
    T: FuseAPI<U>,
    U: IdType,
    C: IdConverter<Output = U>,
{
    fs_impl: T,
    dirmap_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntry>>>>,
    dirplus_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntryPlus>>>>,
    converter: Arc<Mutex<C>>,
}


pub fn new_filesystem<T, U, C>(fuse_api: T) -> FuseFilesystem<T, U, C>
where
    T: FuseAPI<U>,
    U: IdType,
    C: IdConverter<Output = U>,
{
    FuseFilesystem {
        fs_impl: fuse_api,
        dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
        dirplus_iter: Arc::new(Mutex::new(HashMap::new())),
        converter: Arc::new(Mutex::new(C::new())),
    }
}

impl<T, U, C> fuser::Filesystem for FuseFilesystem<T, U, C>
where
    T: FuseAPI<U>,
    U: IdType,
    C: IdConverter<Output = U>,
{
    fn init(&mut self, req: &Request, _config: &mut KernelConfig) -> Result<(), c_int> {
        match FuseAPI::init(&mut self.fs_impl, req.into(), _config) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.raw_os_error().unwrap()), // Convert io::Error to c_int
        }
    }

    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                self.converter.lock().unwrap().map_inode(parent, Some(name), &mut file_attr);
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    file_attr.generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => {
                reply.error(e.raw_os_error().unwrap());
            }
        });
        FuseAPI::lookup(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(parent),
            name,
            callback,
        );
    }

    fn forget(&mut self, req: &Request, ino: u64, _nlookup: u64) {
        FuseAPI::forget(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            _nlookup,
        );
    }

    fn getattr(&mut self, req: &Request, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                self.converter.lock().unwrap().map_inode(ino, None, &mut file_attr);
                reply.attr(&file_attr.ttl.unwrap_or(default_ttl), &file_attr.to_fuse());
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::getattr(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            fh.map(FileHandle::from),
            callback,
        );
    }

    fn setattr(
        &mut self,
        req: &Request,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        #![allow(unused_variables)]
        let attrs = SetAttrRequest {
            mode,
            uid,
            gid,
            size,
            atime: _atime,
            mtime: _mtime,
            ctime: _ctime,
            crtime: _crtime,
            chgtime: _chgtime,
            bkuptime: _bkuptime,
            flags: None,
            file_handle: fh.map(FileHandle::from),
        };
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                self.converter.lock().unwrap().map_inode(ino, None, &mut file_attr);
                reply.attr(&file_attr.ttl.unwrap_or(default_ttl), &file_attr.to_fuse());
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::setattr(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            attrs,
            callback,
        );
    }

    fn readlink(&mut self, req: &Request, ino: u64, reply: ReplyData) {
        let callback: ReplyCb<Vec<u8>> = Box::new(move |result| match result {
            Ok(link) => reply.data(&link),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::readlink(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            callback,
        );
    }

    fn mknod(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                self.converter.lock().unwrap().map_inode(parent, Some(name), &mut file_attr);
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    file_attr.generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::mknod(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(parent),
            name,
            mode,
            umask,
            DeviceType::from_rdev(rdev),
            callback,
        )
    }

    fn mkdir(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                self.converter.lock().unwrap().map_inode(parent, Some(name), &mut file_attr);
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    file_attr.generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::mkdir(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(parent),
            name,
            mode,
            umask,
            callback,
        );
    }

    fn unlink(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let request_info: RequestInfo = req.into();
        let converter = Arc::clone(&self.converter);
        let name_cb = name.to_os_string();
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => {
                converter.lock().unwrap().remove(parent, &name_cb);
                reply.ok()
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::unlink(
            &mut self.fs_impl,
            request_info,
            self.converter.lock().unwrap().to_id(parent),
            &name,
            callback,
        );
    }

    fn rmdir(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let request_info: RequestInfo = req.into();
        let converter = self.converter.clone();
        let name_cb = name.to_os_string();
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => {
                converter.lock().unwrap().remove(parent, &name_cb);
                reply.ok()
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::rmdir(
            &mut self.fs_impl,
            request_info,
            self.converter.lock().unwrap().to_id(parent),
            name,
            callback,
        );
    }

    fn symlink(
        &mut self,
        req: &Request,
        parent: u64,
        link_name: &OsStr,
        target: &Path,
        reply: ReplyEntry,
    ) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let converter = self.converter.clone();
        let link_name_cb = link_name.to_os_string();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                converter.lock().unwrap()
                    .map_inode(parent, Some(&link_name_cb), &mut file_attr);
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    file_attr.generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::symlink(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(parent),
            link_name,
            target,
            callback,
        );
    }

    fn rename(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        flags: u32,
        reply: ReplyEmpty,
    ) {
        let converter = self.converter.clone();
        let name_cb = name.to_os_string();
        let newname_cb = newname.to_os_string();
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => {
                converter.lock().unwrap().rename(parent, &name_cb, newparent, newname_cb);
                reply.ok()
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::rename(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(parent),
            name,
            self.converter.lock().unwrap().to_id(newparent),
            newname,
            RenameFlags::from(flags),
            callback,
        );
    }

    fn link(
        &mut self,
        req: &Request,
        ino: u64,
        newparent: u64,
        newname: &OsStr,
        reply: ReplyEntry,
    ) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(file_attr) => {
                self.converter.lock().unwrap()
                    .map_inode(newparent, Some(newname), &mut file_attr);
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    file_attr.generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::link(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            self.converter.lock().unwrap().to_id(newparent),
            newname,
            callback,
        );
    }

    fn open(&mut self, req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.as_raw())
                }
                Err(e) => reply.error(e.raw_os_error().unwrap()),
            });
        FuseAPI::open(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            OpenFlags::from(_flags),
            callback,
        );
    }

    fn read(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let callback: ReplyCb<Vec<u8>> = Box::new(move |result| match result {
            Ok(data_reply) => reply.data(&data_reply),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::read(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            fh.into(),
            offset,
            size,
            FUSEReadFlags::from(flags),
            lock_owner,
            callback,
        )
    }

    fn write(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        let callback: ReplyCb<u32> = Box::new(move |result| match result {
            Ok(bytes_written) => reply.written(bytes_written),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::write(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            offset,
            data,
            FUSEWriteFlags::from(write_flags),
            OpenFlags::from(flags),
            lock_owner,
            callback,
        );
    }

    fn flush(&mut self, req: &Request, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::flush(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            lock_owner,
            callback,
        );
    }

    fn fsync(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::fsync(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            datasync,
            callback,
        );
    }

    fn opendir(&mut self, req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.as_raw())
                }
                Err(e) => reply.error(e.raw_os_error().unwrap()),
            });
        FuseAPI::opendir(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            OpenFlags::from(_flags),
            callback,
        );
    }

    fn readdir(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, reply: ReplyDirectory) {
        if offset < 0 {
            error!("readdir called with a negative offset");
            reply.error(PosixError::INVALID_ARGUMENT.into());
            return;
        }
        let converter = self.converter.clone();

        // Helper function to create the callback
        fn create_callback(
            mut reply: ReplyDirectory,
            dirmap_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntry>>>>,
            ino: u64,
            offset: i64,
        ) -> ReplyCb<DirIter<FuseDirEntry>> {
            Box::new(move |result| {
                match result {
                    Ok(mut dir_iter) => {
                        let mut i = 0;
                        while let Some(entry) = dir_iter.next() {
                            if reply.add(
                                entry.inode,
                                (i as i64 + offset + 1) as i64,
                                entry.kind,
                                &entry.name,
                            ) {
                                // Save the state of the iterator for future calls
                                dirmap_iter.lock().unwrap().insert(ino, dir_iter);
                                break;
                            }
                            i += 1;
                        }
                        reply.ok();
                    }
                    Err(e) => {
                        reply.error(
                            e.raw_os_error()
                                .unwrap_or(PosixError::INPUT_OUTPUT_ERROR.into()),
                        );
                    }
                }
            })
        }

        if offset == 0 {
            // Call the high-level readdir function with the callback
            let callback = create_callback(reply, self.dirmap_iter.clone(), ino, offset);
            self.fs_impl.readdir(
                req.into(),
                self.converter.lock().unwrap().to_id(ino),
                FileHandle::from(fh),
                Box::new(|result| match result {
                    Ok(entries) => callback(Ok(Box::new(entries.into_iter()))),
                    Err(e) => callback(Err(e)),
                }),
            );
        } else {
            // Handle continuation from a previously saved iterator
            match self.dirmap_iter.lock().unwrap().remove(&ino) {
                Some(entries) => {
                    let callback = create_callback(reply, self.dirmap_iter.clone(), ino, offset);
                    callback(Ok(entries));
                }
                None => {
                    reply.ok();
                }
            }
        }
    }

    fn readdirplus(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: ReplyDirectoryPlus,
    ) {
        let converter = self.converter.clone();
        if offset < 0 {
            error!("readdirplus called with a negative offset");
            reply.error(PosixError::INVALID_ARGUMENT.into());
            return;
        }

        fn create_callback(
            mut reply: ReplyDirectoryPlus,
            dirplus_iter_map: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntryPlus>>>>,
            ino: u64,
            offset: i64,
            default_ttl: Duration,
        ) -> ReplyCb<DirIter<FuseDirEntryPlus>> {
            Box::new(move |result| {
                match result {
                    Ok(mut dirplus_iter) => {
                        let mut i = 0;
                        while let Some(entry) = dirplus_iter.next() {
                            let (ttl, generation, file_attr) = (
                                entry.attr.ttl.unwrap_or(default_ttl),
                                entry
                                    .attr
                                    .generation
                                    .unwrap_or_else(get_random_generation),
                                entry.attr.to_fuse(),
                            );
                            if reply.add(
                                entry.inode,
                                (i as i64 + offset + 1) as i64,
                                &entry.name,
                                &ttl,
                                &file_attr,
                                generation,
                            ) {
                                // Save the state of the iterator for future calls
                                dirplus_iter_map.lock().unwrap().insert(ino, dirplus_iter);
                                break;
                            }
                            i += 1;
                        }
                        reply.ok();
                    }
                    Err(e) => {
                        reply.error(
                            e.raw_os_error()
                                .unwrap_or(PosixError::INPUT_OUTPUT_ERROR.into()),
                        );
                    }
                }
            })
        }

        if offset == 0 {
            // Call the high-level readdirplus function with the callback
            let callback = create_callback(
                reply,
                self.dirplus_iter.clone(),
                ino,
                offset,
                self.fs_impl.get_default_ttl(),
            );
            self.fs_impl.readdirplus(
                req.into(),
                self.converter.lock().unwrap().to_id(ino),
                FileHandle::from(fh),
                Box::new(|result| match result {
                    Ok(entries) => callback(Ok(Box::new(entries.into_iter()))),
                    Err(e) => callback(Err(e)),
                }),
            );
        } else {
            // Handle continuation from a previously saved iterator
            match self.dirplus_iter.lock().unwrap().remove(&ino) {
                Some(entries) => {
                    let callback = create_callback(
                        reply,
                        self.dirplus_iter.clone(),
                        ino,
                        offset,
                        self.fs_impl.get_default_ttl(),
                    );
                    callback(Ok(entries));
                }
                None => {
                    reply.ok();
                }
            }
        }
    }

    fn releasedir(&mut self, req: &Request, ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::releasedir(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(_fh),
            OpenFlags::from(_flags),
            callback,
        );
    }

    fn fsyncdir(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::fsyncdir(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            datasync,
            callback,
        );
    }

    fn release(
        &mut self,
        req: &Request,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::release(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            _fh.into(),
            OpenFlags::from(_flags),
            _lock_owner,
            _flush,
            callback,
        );
    }

    fn statfs(&mut self, req: &Request, ino: u64, reply: ReplyStatfs) {
        let callback: ReplyCb<StatFs> = Box::new(move |result| match result {
            Ok(statfs) => {
                reply.statfs(
                    statfs.total_blocks,
                    statfs.free_blocks,
                    statfs.available_blocks,
                    statfs.total_files,
                    statfs.free_files,
                    statfs.block_size,
                    statfs.max_filename_length,
                    statfs.fragment_size,
                );
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::statfs(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            callback,
        );
    }

    fn setxattr(
        &mut self,
        req: &Request,
        ino: u64,
        name: &OsStr,
        _value: &[u8],
        flags: i32,
        position: u32,
        reply: ReplyEmpty,
    ) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::setxattr(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            name,
            _value,
            FUSESetXAttrFlags::from(flags),
            position,
            callback,
        )
    }

    fn getxattr(&mut self, req: &Request, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr) {
        let callback: ReplyCb<Vec<u8>> = Box::new(move |result| match result {
            Ok(xattr_data) => {
                if size == 0 {
                    reply.size(xattr_data.len() as u32);
                } else if size >= xattr_data.len() as u32 {
                    reply.data(&xattr_data);
                } else {
                    reply.error(PosixError::RESULT_TOO_LARGE.into());
                }
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::getxattr(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            name,
            size,
            callback,
        );
    }

    fn listxattr(&mut self, req: &Request, ino: u64, size: u32, reply: ReplyXattr) {
        let callback: ReplyCb<Vec<u8>> = Box::new(move |result| match result {
            Ok(xattr_data) => {
                if size == 0 {
                    reply.size(xattr_data.len() as u32);
                } else if size >= xattr_data.len() as u32 {
                    reply.data(&xattr_data);
                } else {
                    reply.error(PosixError::RESULT_TOO_LARGE.into());
                }
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::listxattr(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            size,
            callback,
        );
    }

    fn removexattr(&mut self, req: &Request, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::removexattr(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            name,
            callback,
        );
    }

    fn access(&mut self, req: &Request, ino: u64, mask: i32, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::access(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            AccessMask::from(mask),
            callback,
        );
    }

    fn create(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        let default_ttl = self.fs_impl.get_default_ttl();
        let callback: ReplyCb<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, file_attr, response_flags)) => reply.created(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    file_attr.generation.unwrap_or(get_random_generation()),
                    file_handle.into(),
                    response_flags.as_raw(),
                ),
                Err(e) => reply.error(e.raw_os_error().unwrap()),
            });
        FuseAPI::create(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(parent),
            name,
            mode,
            umask,
            OpenFlags::from(flags),
            callback,
        );
    }

    fn getlk(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        reply: ReplyLock,
    ) {
        // Creating the LockInfo struct
        let lock_info = LockInfo {
            start,
            end,
            lock_type: LockType::from(typ),
            pid,
        };

        // Call the high-level function in FuseAPI
        let callback: ReplyCb<LockInfo> = Box::new(move |result| match result {
            Ok(lock_info) => {
                let LockInfo {
                    start,
                    end,
                    lock_type,
                    pid,
                } = lock_info;
                reply.locked(start, end, lock_type.as_raw(), pid)
            }
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::getlk(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            lock_owner,
            lock_info,
            callback,
        );
    }

    fn setlk(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
        reply: ReplyEmpty,
    ) {
        // Creating the LockInfo struct
        let lock_info = LockInfo {
            start,
            end,
            lock_type: LockType::from(typ),
            pid,
        };

        // Call the high-level function in FuseAPI
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::setlk(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            lock_owner,
            lock_info,
            sleep,
            callback,
        );
    }

    fn bmap(&mut self, req: &Request<'_>, ino: u64, blocksize: u32, idx: u64, reply: ReplyBmap) {
        // Call the high-level function in FuseAPI
        let callback: ReplyCb<u64> = Box::new(move |result| match result {
            Ok(block) => reply.bmap(block),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::bmap(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            blocksize,
            idx,
            callback,
        );
    }

    fn ioctl(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        fh: u64,
        flags: u32,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
        reply: ReplyIoctl,
    ) {
        // Call the high-level function in FuseAPI
        let callback: ReplyCb<(i32, Vec<u8>)> = Box::new(move |result| match result {
            Ok((result, data)) => reply.ioctl(result, &data),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::ioctl(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            IOCtlFlags::from(flags),
            cmd,
            in_data,
            out_size,
            callback,
        )
    }

    fn fallocate(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::fallocate(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            offset,
            length,
            mode,
            callback,
        )
    }

    fn lseek(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
        whence: i32,
        reply: ReplyLseek,
    ) {
        let callback: ReplyCb<i64> = Box::new(move |result| match result {
            Ok(offset) => reply.offset(offset),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::lseek(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino),
            FileHandle::from(fh),
            offset,
            whence.into(),
            callback,
        );
    }

    fn copy_file_range(
        &mut self,
        req: &Request,
        ino_in: u64,
        fh_in: u64,
        offset_in: i64,
        ino_out: u64,
        fh_out: u64,
        offset_out: i64,
        len: u64,
        flags: u32,
        reply: ReplyWrite,
    ) {
        let callback: ReplyCb<u32> = Box::new(move |result| match result {
            Ok(bytes_written) => reply.written(bytes_written),
            Err(e) => reply.error(e.raw_os_error().unwrap()),
        });
        FuseAPI::copy_file_range(
            &mut self.fs_impl,
            req.into(),
            self.converter.lock().unwrap().to_id(ino_in),
            FileHandle::from(fh_in),
            offset_in,
            self.converter.lock().unwrap().to_id(ino_out),
            FileHandle::from(fh_out),
            offset_out,
            len,
            flags,
            callback,
        );
    }
}
