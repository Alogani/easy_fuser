use std::{
    collections::HashMap,
    ffi::OsStr,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

use libc::c_int;
use log::{debug, error};

use fuser::{
    self, KernelConfig, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

use super::callback_handlers::{FuseCallbackHandler, ReplyCb};
use super::inode_mapping::FileIdResolver;
use crate::types::*;

type DirIter<T> = Box<dyn Iterator<Item = T> + Send>;

fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

pub struct FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseCallbackHandler<T>,
    R: FileIdResolver<Output = T>,
{
    callback_handler: U,
    id_resolver: Arc<R>,
    dirmap_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntry>>>>,
    dirplus_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntryPlus>>>>,
}

impl<T, U, R> FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseCallbackHandler<T>,
    R: FileIdResolver<Output = T>,
{
    pub fn new(fuse_cb_api: U, resolver: R) -> FuseDriver<T, U, R> {
        FuseDriver {
            callback_handler: fuse_cb_api,
            id_resolver: Arc::new(resolver),
            dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
            dirplus_iter: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<T, U, R> fuser::Filesystem for FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseCallbackHandler<T>,
    R: FileIdResolver<Output = T>,
{
    fn init(&mut self, req: &Request, _config: &mut KernelConfig) -> Result<(), c_int> {
        match FuseCallbackHandler::init(&mut self.callback_handler, req.into(), _config) {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("{:?}", e);
                Err(e.raw_error())
            } // Convert io::Error to c_int
        }
    }

    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let default_ttl = U::get_default_ttl();
        let resolver = Arc::clone(&self.id_resolver);
        let name_owned = name.to_owned();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(parent, Some(&name_owned), &mut file_attr.inode);
                let generation = file_attr.generation.unwrap_or(get_random_generation());
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    generation,
                );
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error());
            }
        });
        FuseCallbackHandler::lookup(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(parent),
            name,
            callback,
        );
    }

    fn forget(&mut self, req: &Request, ino: u64, _nlookup: u64) {
        FuseCallbackHandler::forget(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            _nlookup,
        );
    }

    fn getattr(&mut self, req: &Request, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        let default_ttl = U::get_default_ttl();
        let resolver = Arc::clone(&self.id_resolver);
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(ino, None, &mut file_attr.inode);
                reply.attr(&file_attr.ttl.unwrap_or(default_ttl), &file_attr.to_fuse());
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::getattr(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
        let default_ttl = U::get_default_ttl();
        let resolver = Arc::clone(&self.id_resolver);
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(ino, None, &mut file_attr.inode);
                reply.attr(&file_attr.ttl.unwrap_or(default_ttl), &file_attr.to_fuse());
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::setattr(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            attrs,
            callback,
        );
    }

    fn readlink(&mut self, req: &Request, ino: u64, reply: ReplyData) {
        let callback: ReplyCb<Vec<u8>> = Box::new(move |result| match result {
            Ok(link) => reply.data(&link),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::readlink(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
        let default_ttl = U::get_default_ttl();
        let resolver = Arc::clone(&self.id_resolver);
        let owned_name = name.to_owned();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(parent, Some(&owned_name), &mut file_attr.inode);
                let generation = file_attr.generation.unwrap_or(get_random_generation());
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    generation,
                );
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::mknod(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(parent),
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
        let default_ttl = U::get_default_ttl();
        let resolver = Arc::clone(&self.id_resolver);
        let owned_name = name.to_owned();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(parent, Some(&owned_name), &mut file_attr.inode);
                let generation = file_attr.generation.unwrap_or(get_random_generation());
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    generation,
                );
            }
            Err(e) => reply.error(e.raw_error()),
        });
        FuseCallbackHandler::mkdir(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(parent),
            name,
            mode,
            umask,
            callback,
        );
    }

    fn unlink(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let request_info: RequestInfo = req.into();
        let resolver = Arc::clone(&self.id_resolver);
        let name_owned = name.to_owned();
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => {
                resolver.unlink(parent, &name_owned);
                reply.ok()
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::unlink(
            &mut self.callback_handler,
            request_info,
            self.id_resolver.resolve_id(parent),
            &name,
            callback,
        );
    }

    fn rmdir(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let request_info: RequestInfo = req.into();
        let resolver = self.id_resolver.clone();
        let name_owned = name.to_owned();
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => {
                resolver.unlink(parent, &name_owned);
                reply.ok()
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::rmdir(
            &mut self.callback_handler,
            request_info,
            self.id_resolver.resolve_id(parent),
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
        let default_ttl = U::get_default_ttl();
        let resolver = self.id_resolver.clone();
        let link_name_owned = link_name.to_os_string();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(
                    parent,
                    Some(&link_name_owned),
                    &mut file_attr.inode,
                );
                let generation = file_attr.generation.unwrap_or(get_random_generation());
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    generation,
                );
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::symlink(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(parent),
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
        let resolver = self.id_resolver.clone();
        let name_owned = name.to_owned();
        let newname_cb = newname.to_os_string();
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => {
                resolver.rename(parent, &name_owned, newparent, newname_cb);
                reply.ok()
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::rename(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(parent),
            name,
            self.id_resolver.resolve_id(newparent),
            newname,
            RenameFlags::from_bits_retain(flags),
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
        let default_ttl = U::get_default_ttl();
        let resolver = self.id_resolver.clone();
        let newname_owned = newname.to_owned();
        let callback: ReplyCb<FileAttribute> = Box::new(move |result| match result {
            Ok(mut file_attr) => {
                resolver.assign_or_initialize_ino(
                    newparent,
                    Some(&newname_owned),
                    &mut file_attr.inode,
                );
                let generation = file_attr.generation.unwrap_or(get_random_generation());
                reply.entry(
                    &file_attr.ttl.unwrap_or(default_ttl),
                    &file_attr.to_fuse(),
                    generation,
                );
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::link(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            self.id_resolver.resolve_id(newparent),
            newname,
            callback,
        );
    }

    fn open(&mut self, req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.bits())
                }
                Err(e) => {
                    debug!("{:?}", e);
                    reply.error(e.raw_error())
                }
            });
        FuseCallbackHandler::open(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            OpenFlags::from_bits_retain(_flags),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::read(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            fh.into(),
            offset,
            size,
            FUSEReadFlags::from_bits_retain(flags),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::write(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            FileHandle::from(fh),
            offset,
            data,
            FUSEWriteFlags::from_bits_retain(write_flags),
            OpenFlags::from_bits_retain(flags),
            lock_owner,
            callback,
        );
    }

    fn flush(&mut self, req: &Request, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::flush(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            FileHandle::from(fh),
            lock_owner,
            callback,
        );
    }

    fn fsync(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::fsync(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            FileHandle::from(fh),
            datasync,
            callback,
        );
    }

    fn opendir(&mut self, req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.bits())
                }
                Err(e) => {
                    debug!("{:?}", e);
                    reply.error(e.raw_error())
                }
            });
        FuseCallbackHandler::opendir(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            OpenFlags::from_bits_retain(_flags),
            callback,
        );
    }

    fn readdir(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, reply: ReplyDirectory) {
        if offset < 0 {
            error!("readdir called with a negative offset");
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }
        let resolver = self.id_resolver.clone();

        // Helper function to create the callback
        fn create_callback<R: FileIdResolver>(
            mut reply: ReplyDirectory,
            dirmap_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntry>>>>,
            ino: u64,
            offset: i64,
            resolver: Arc<R>,
        ) -> ReplyCb<DirIter<FuseDirEntry>> {
            Box::new(move |result| {
                match result {
                    Ok(mut dir_iter) => {
                        let mut i = 0;
                        while let Some(mut entry) = dir_iter.next() {
                            resolver.assign_or_initialize_ino(
                                ino,
                                Some(entry.name.as_ref()),
                                &mut entry.inode,
                            );
                            if reply.add(
                                entry.inode.into(),
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
                        reply.error(e.raw_error());
                    }
                }
            })
        }

        if offset == 0 {
            // Call the high-level readdir function with the callback
            let callback = create_callback(reply, self.dirmap_iter.clone(), ino, offset, resolver);
            self.callback_handler.readdir(
                req.into(),
                self.id_resolver.resolve_id(ino),
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
                    let callback = create_callback(
                        reply,
                        Arc::clone(&self.dirmap_iter),
                        ino,
                        offset,
                        resolver,
                    );
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
        let resolver = self.id_resolver.clone();
        if offset < 0 {
            error!("readdirplus called with a negative offset");
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }

        fn create_callback<R: FileIdResolver>(
            mut reply: ReplyDirectoryPlus,
            dirplus_iter_map: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntryPlus>>>>,
            ino: u64,
            offset: i64,
            default_ttl: Duration,
            resolver: Arc<R>,
        ) -> ReplyCb<DirIter<FuseDirEntryPlus>> {
            Box::new(move |result| {
                match result {
                    Ok(mut dirplus_iter) => {
                        let mut i = 0;
                        while let Some(mut entry) = dirplus_iter.next() {
                            resolver.assign_or_initialize_ino(
                                ino,
                                Some(entry.name.as_ref()),
                                &mut entry.inode,
                            );
                            let (ttl, generation, file_attr) = (
                                entry.attr.ttl.unwrap_or(default_ttl),
                                entry.attr.generation.unwrap_or_else(get_random_generation),
                                entry.attr.to_fuse(),
                            );
                            if reply.add(
                                entry.inode.into(),
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
                        reply.error(e.raw_error());
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
                U::get_default_ttl(),
                resolver,
            );
            self.callback_handler.readdirplus(
                req.into(),
                self.id_resolver.resolve_id(ino),
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
                        Arc::clone(&self.dirplus_iter),
                        ino,
                        offset,
                        U::get_default_ttl(),
                        resolver,
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::releasedir(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            FileHandle::from(_fh),
            OpenFlags::from_bits_retain(_flags),
            callback,
        );
    }

    fn fsyncdir(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::fsyncdir(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::release(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            _fh.into(),
            OpenFlags::from_bits_retain(_flags),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::statfs(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::setxattr(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            name,
            _value,
            FUSESetXAttrFlags::from_bits_retain(flags),
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
                    reply.error(ErrorKind::ResultTooLarge.into());
                }
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::getxattr(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
                    reply.error(ErrorKind::ResultTooLarge.into());
                }
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::listxattr(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            size,
            callback,
        );
    }

    fn removexattr(&mut self, req: &Request, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::removexattr(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            name,
            callback,
        );
    }

    fn access(&mut self, req: &Request, ino: u64, mask: i32, reply: ReplyEmpty) {
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::access(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            AccessMask::from_bits_retain(mask),
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
        let default_ttl = U::get_default_ttl();
        let callback: ReplyCb<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, file_attr, response_flags)) => {
                    let generation = file_attr.generation.unwrap_or(get_random_generation());
                    reply.created(
                        &file_attr.ttl.unwrap_or(default_ttl),
                        &file_attr.to_fuse(),
                        generation,
                        file_handle.into(),
                        response_flags.bits(),
                    )
                }
                Err(e) => {
                    debug!("{:?}", e);
                    reply.error(e.raw_error())
                }
            });
        FuseCallbackHandler::create(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(parent),
            name,
            mode,
            umask,
            OpenFlags::from_bits_retain(flags),
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
            lock_type: LockType::from_bits_retain(typ),
            pid,
        };

        // Call the high-level function in FuseCallbackAPI
        let callback: ReplyCb<LockInfo> = Box::new(move |result| match result {
            Ok(lock_info) => {
                let LockInfo {
                    start,
                    end,
                    lock_type,
                    pid,
                } = lock_info;
                reply.locked(start, end, lock_type.bits(), pid)
            }
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::getlk(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
            lock_type: LockType::from_bits_retain(typ),
            pid,
        };

        // Call the high-level function in FuseCallbackAPI
        let callback: ReplyCb<()> = Box::new(move |result| match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::setlk(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            FileHandle::from(fh),
            lock_owner,
            lock_info,
            sleep,
            callback,
        );
    }

    fn bmap(&mut self, req: &Request<'_>, ino: u64, blocksize: u32, idx: u64, reply: ReplyBmap) {
        // Call the high-level function in FuseCallbackAPI
        let callback: ReplyCb<u64> = Box::new(move |result| match result {
            Ok(block) => reply.bmap(block),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::bmap(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
        // Call the high-level function in FuseCallbackAPI
        let callback: ReplyCb<(i32, Vec<u8>)> = Box::new(move |result| match result {
            Ok((result, data)) => reply.ioctl(result, &data),
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::ioctl(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
            FileHandle::from(fh),
            IOCtlFlags::from_bits_retain(flags),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::fallocate(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::lseek(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino),
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
            Err(e) => {
                debug!("{:?}", e);
                reply.error(e.raw_error())
            }
        });
        FuseCallbackHandler::copy_file_range(
            &mut self.callback_handler,
            req.into(),
            self.id_resolver.resolve_id(ino_in),
            FileHandle::from(fh_in),
            offset_in,
            self.id_resolver.resolve_id(ino_out),
            FileHandle::from(fh_out),
            offset_out,
            len,
            flags,
            callback,
        );
    }
}
