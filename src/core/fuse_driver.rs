use log::{info, warn};
use std::{
    ffi::OsStr,
    io,
    path::Path,
    time::{Instant, SystemTime},
};

use fuser::{
    self, AccessFlags, BsdFileFlags, FileHandle, FopenFlags, Generation, INodeNo, IoctlFlags,
    KernelConfig, LockOwner, ReadFlags, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek,
    ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow, WriteFlags,
};

use super::{
    fuse_driver_types::{FuseDriver, execute_task},
    inode_mapping::FileIdResolver,
    macros::*,
    thread_mode::*,
};
use crate::{fuse_handler::FuseHandler, types::*};

fn get_random_generation() -> Generation {
    Generation(Instant::now().elapsed().as_nanos() as u64)
}

impl<TId, THandler> fuser::Filesystem for FuseDriver<TId, THandler>
where
    TId: FileIdType,
    THandler: FuseHandler<TId>,
{
    fn init(&mut self, req: &Request, config: &mut KernelConfig) -> io::Result<()> {
        let req = RequestInfo::from(req);
        match self.get_handler().init(&req, config) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("[{}] init {:?}", e, req);
                Err(io::Error::from_raw_os_error(e.raw_error()))
            }
        }
    }

    fn destroy(&mut self) {
        self.get_handler().destroy();
    }

    fn access(&self, req: &Request, ino: INodeNo, mask: AccessFlags, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.access(&req, resolver.resolve_id(ino), AccessMask::from(mask)) {
                Ok(()) => {
                    reply.ok();
                }
                Err(e) => {
                    warn!("access: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into());
                }
            };
        });
    }

    fn bmap(&self, req: &Request, ino: INodeNo, blocksize: u32, idx: u64, reply: ReplyBmap) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.bmap(&req, resolver.resolve_id(ino), blocksize, idx) {
                Ok(block) => reply.bmap(block),
                Err(e) => {
                    warn!("bmap: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn copy_file_range(
        &self,
        req: &Request,
        ino_in: INodeNo,
        fh_in: FileHandle,
        offset_in: i64,
        ino_out: INodeNo,
        fh_out: FileHandle,
        offset_out: i64,
        len: u64,
        flags: fuser::CopyFileRangeFlags,
        reply: ReplyWrite,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.copy_file_range(
                &req,
                resolver.resolve_id(ino_in),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh_in) },
                offset_in,
                resolver.resolve_id(ino_out),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh_out) },
                offset_out,
                len,
                CopyFileRangeFlags::from(flags),
            ) {
                Ok(bytes_written) => reply.written(bytes_written),
                Err(e) => {
                    warn!("copy_file_range: ino {:x?}, [{}], {:?}", ino_in, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn create(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            let helper = CreateHelper::new(&reply);
            match handler.create(
                &req,
                resolver.resolve_id(parent),
                &name,
                mode,
                umask,
                OpenFlags::from_bits_retain(flags),
                helper,
            ) {
                Ok((file_handle, metadata, response_flags, passthrough_backing_id)) => {
                    let default_ttl = handler.get_default_ttl();
                    let (id, file_attr) = TId::extract_metadata(metadata);
                    let ino = resolver.lookup(parent, &name, id, true);
                    let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                    match passthrough_backing_id {
                        #[cfg(feature = "passthrough")]
                        Some(passthrough_backing_id) => {
                            let response_flags =
                                response_flags | FUSEOpenResponseFlags::PASSTHROUGH;
                            reply.created_passthrough(
                                &ttl.unwrap_or(default_ttl),
                                &fuse_attr,
                                generation.unwrap_or(get_random_generation()),
                                file_handle.as_fuser_file_handle(),
                                FopenFlags::from(response_flags),
                                passthrough_backing_id.as_ref(),
                            );
                        }
                        _ => {
                            let response_flags =
                                response_flags & !FUSEOpenResponseFlags::PASSTHROUGH;
                            reply.created(
                                &ttl.unwrap_or(default_ttl),
                                &fuse_attr,
                                generation.unwrap_or(get_random_generation()),
                                file_handle.as_fuser_file_handle(),
                                FopenFlags::from(response_flags),
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("create: {:?}, parent_ino: {:x?}, {:?}", parent, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn fallocate(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.fallocate(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                offset,
                length,
                FallocateFlags::from_bits_retain(mode),
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("fallocate: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn flush(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        lock_owner: LockOwner,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.flush(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                lock_owner,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("flush: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn forget(&self, req: &Request, ino: INodeNo, nlookup: u64) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        handler.forget(&req, resolver.resolve_id(ino), nlookup);
        resolver.forget(ino, nlookup);
    }

    fn fsync(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        datasync: bool,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.fsync(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                datasync,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("fsync: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn fsyncdir(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        datasync: bool,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.fsyncdir(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                datasync,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("fsyncdir: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn getattr(&self, req: &Request, ino: INodeNo, fh: Option<FileHandle>, reply: ReplyAttr) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            handle_fuse_reply_attr!(
                handler,
                resolver,
                &req,
                ino,
                reply,
                getattr,
                (
                    &req,
                    resolver.resolve_id(ino),
                    fh.map(|fh| unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) })
                )
            );
        });
    }

    fn getlk(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        lock_owner: LockOwner,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        reply: ReplyLock,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            let lock_info = LockInfo {
                start,
                end,
                lock_type: LockType::from_bits_retain(typ),
                pid,
            };
            match handler.getlk(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                lock_owner,
                lock_info,
            ) {
                Ok(lock_info) => reply.locked(
                    lock_info.start,
                    lock_info.end,
                    lock_info.lock_type.bits(),
                    lock_info.pid,
                ),
                Err(e) => {
                    warn!("getlk: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn getxattr(&self, req: &Request, ino: INodeNo, name: &OsStr, size: u32, reply: ReplyXattr) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.getxattr(&req, resolver.resolve_id(ino), &name, size) {
                Ok(xattr_data) => {
                    if size == 0 {
                        reply.size(xattr_data.len() as u32);
                    } else if size >= xattr_data.len() as u32 {
                        reply.data(&xattr_data);
                    } else {
                        reply.error(
                            PosixError::new(
                                ErrorKind::ResultTooLarge,
                                "returned result is too large",
                            )
                            .into(),
                        );
                    }
                }
                Err(e) => {
                    warn!("getxattr: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn ioctl(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        flags: IoctlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
        reply: ReplyIoctl,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let in_data = in_data.to_owned();
        execute_task!(self, {
            match handler.ioctl(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                FUSEIoctlFlags::from(flags),
                cmd,
                in_data,
                out_size,
            ) {
                Ok((result, data)) => reply.ioctl(result, &data),
                Err(e) => {
                    warn!("ioctl: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn link(
        &self,
        req: &Request,
        ino: INodeNo,
        newparent: INodeNo,
        newname: &OsStr,
        reply: ReplyEntry,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let newname = newname.to_owned();
        execute_task!(self, {
            handle_fuse_reply_entry!(
                handler,
                resolver,
                &req,
                newparent,
                &newname,
                reply,
                link,
                (
                    &req,
                    resolver.resolve_id(ino),
                    resolver.resolve_id(newparent),
                    &newname
                )
            );
        });
    }

    fn listxattr(&self, req: &Request, ino: INodeNo, size: u32, reply: ReplyXattr) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.listxattr(&req, resolver.resolve_id(ino), size) {
                Ok(xattr_data) => {
                    if size == 0 {
                        reply.size(xattr_data.len() as u32);
                    } else if size >= xattr_data.len() as u32 {
                        reply.data(&xattr_data);
                    } else {
                        reply.error(
                            PosixError::new(
                                ErrorKind::ResultTooLarge,
                                "returned result is too large than allowed size",
                            )
                            .into(),
                        );
                    }
                }
                Err(e) => {
                    warn!("listxattr: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn lookup(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            handle_fuse_reply_entry!(
                handler,
                resolver,
                &req,
                parent,
                &name,
                reply,
                lookup,
                (&req, resolver.resolve_id(parent), &name)
            );
        });
    }

    fn lseek(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: i64,
        whence: i32,
        reply: ReplyLseek,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.lseek(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                seek_from_raw(Some(whence), offset),
            ) {
                Ok(new_offset) => reply.offset(new_offset),
                Err(e) => {
                    warn!("lseek: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn mkdir(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            handle_fuse_reply_entry!(
                handler,
                resolver,
                &req,
                parent,
                &name,
                reply,
                mkdir,
                (&req, resolver.resolve_id(parent), &name, mode, umask)
            );
        });
    }

    fn mknod(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            handle_fuse_reply_entry!(
                handler,
                resolver,
                &req,
                parent,
                &name,
                reply,
                mknod,
                (
                    &req,
                    resolver.resolve_id(parent),
                    &name,
                    mode,
                    umask,
                    DeviceType::from_rdev(rdev.try_into().unwrap())
                )
            );
        });
    }

    fn open(&self, req: &Request, ino: INodeNo, flags: fuser::OpenFlags, reply: ReplyOpen) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            let open_helper = OpenHelper::new(&reply);
            match handler.open(
                &req,
                resolver.resolve_id(ino),
                OpenFlags::from(flags),
                open_helper,
            ) {
                Ok((file_handle, response_flags, backing_id)) => match backing_id {
                    #[cfg(feature = "passthrough")]
                    Some(backing_id) => {
                        reply.opened_passthrough(
                            file_handle.as_fuser_file_handle(),
                            FopenFlags::from(response_flags),
                            backing_id.as_ref(),
                        );
                    }
                    _ => {
                        let response_flags = response_flags & !FUSEOpenResponseFlags::PASSTHROUGH;
                        reply.opened(
                            file_handle.as_fuser_file_handle(),
                            FopenFlags::from(response_flags),
                        )
                    }
                },
                Err(e) => {
                    warn!("open: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn opendir(&self, req: &Request, ino: INodeNo, flags: fuser::OpenFlags, reply: ReplyOpen) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            let open_helper = OpenHelper::new(&reply);
            match handler.opendir(
                &req,
                resolver.resolve_id(ino),
                OpenFlags::from(flags),
                open_helper,
            ) {
                Ok((file_handle, response_flags, backing_id)) => match backing_id {
                    #[cfg(feature = "passthrough")]
                    Some(backing_id) => {
                        reply.opened_passthrough(
                            file_handle.as_fuser_file_handle(),
                            FopenFlags::from(response_flags),
                            backing_id.as_ref(),
                        );
                    }
                    _ => {
                        let response_flags = response_flags & !FUSEOpenResponseFlags::PASSTHROUGH;
                        reply.opened(
                            file_handle.as_fuser_file_handle(),
                            FopenFlags::from(response_flags),
                        )
                    }
                },
                Err(e) => {
                    warn!("opendir: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn read(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        size: u32,
        read_flags: ReadFlags,
        flags: fuser::OpenFlags,
        lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.read(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                SeekFrom::Start(offset),
                size,
                FUSEReadFlags::from(read_flags),
                OpenFlags::from(flags),
                lock_owner,
            ) {
                Ok(data_reply) => reply.data(&data_reply),
                Err(e) => {
                    warn!("read: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn readdir(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        handle_dir_read!(
            self,
            req,
            ino,
            fh,
            offset,
            reply,
            readdir,
            get_dirmap_iter,
            ReplyDirectory
        );
    }

    fn readdirplus(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectoryPlus,
    ) {
        handle_dir_read!(
            self,
            req,
            ino,
            fh,
            offset,
            reply,
            readdirplus,
            get_dirmapplus_iter,
            ReplyDirectoryPlus
        );
    }

    fn readlink(&self, req: &Request, ino: INodeNo, reply: ReplyData) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.readlink(&req, resolver.resolve_id(ino)) {
                Ok(link) => reply.data(&link),
                Err(e) => {
                    warn!("[{}] readlink, ino: {:x?}, {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn release(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        flags: fuser::OpenFlags,
        lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.release(
                &req,
                resolver.resolve_id(ino),
                unsafe { OwnedFileHandle::from_fuser_file_handle(fh) },
                OpenFlags::from(flags),
                lock_owner,
                _flush,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("release: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn releasedir(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        flags: fuser::OpenFlags,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.releasedir(
                &req,
                resolver.resolve_id(ino),
                unsafe { OwnedFileHandle::from_fuser_file_handle(fh) },
                OpenFlags::from(flags),
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("releasedir: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn removexattr(&self, req: &Request, ino: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.removexattr(&req, resolver.resolve_id(ino), &name) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("removexattr: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn rename(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        flags: fuser::RenameFlags,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        let newname = newname.to_owned();
        execute_task!(self, {
            match handler.rename(
                &req,
                resolver.resolve_id(parent),
                &name,
                resolver.resolve_id(newparent),
                &newname,
                RenameFlags::from(flags),
            ) {
                Ok(()) => {
                    resolver.rename(parent, &name, newparent, &newname);
                    reply.ok()
                }
                Err(e) => {
                    warn!("[{}] rename: parent_ino: {:x?}, {:?}", parent, e, req);
                    reply.error(e.into())
                }
            }
        });
    }

    fn rmdir(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.rmdir(&req, resolver.resolve_id(parent), &name) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("[{}] rmdir: parent_ino: {:x?}, {:?}", parent, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn setattr(
        &self,
        req: &Request,
        ino: INodeNo,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        ctime: Option<SystemTime>,
        fh: Option<FileHandle>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        _flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let attrs = SetAttrRequest {
            mode,
            uid,
            gid,
            size,
            atime: atime,
            mtime: mtime,
            ctime: ctime,
            crtime: crtime,
            chgtime: chgtime,
            bkuptime: bkuptime,
            flags: None,
            file_handle: fh.map(|fh| unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) }),
        };
        execute_task!(self, {
            handle_fuse_reply_attr!(
                handler,
                resolver,
                &req,
                ino,
                reply,
                setattr,
                (&req, resolver.resolve_id(ino), attrs)
            );
        });
    }

    fn setlk(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        lock_owner: LockOwner,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            let lock_info = LockInfo {
                start,
                end,
                lock_type: LockType::from_bits_retain(typ),
                pid,
            };
            match handler.setlk(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                lock_owner,
                lock_info,
                sleep,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("setlk: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn setxattr(
        &self,
        req: &Request,
        ino: INodeNo,
        name: &OsStr,
        value: &[u8],
        flags: i32,
        position: u32,
        reply: ReplyEmpty,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        let value = value.to_owned();
        execute_task!(self, {
            match handler.setxattr(
                &req,
                resolver.resolve_id(ino),
                &name,
                value,
                FUSESetXAttrFlags::from_bits_retain(flags),
                position,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("setxattr: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn statfs(&self, req: &Request, ino: INodeNo, reply: ReplyStatfs) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.statfs(&req, resolver.resolve_id(ino)) {
                Ok(statfs) => reply.statfs(
                    statfs.total_blocks,
                    statfs.free_blocks,
                    statfs.available_blocks,
                    statfs.total_files,
                    statfs.free_files,
                    statfs.block_size,
                    statfs.max_filename_length,
                    statfs.fragment_size,
                ),
                Err(e) => {
                    warn!("statfs: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into());
                }
            };
        });
    }

    fn symlink(
        &self,
        req: &Request,
        parent: INodeNo,
        link_name: &OsStr,
        target: &Path,
        reply: ReplyEntry,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let link_name = link_name.to_owned();
        let target = target.to_owned();
        execute_task!(self, {
            handle_fuse_reply_entry!(
                handler,
                resolver,
                &req,
                parent,
                &link_name,
                reply,
                symlink,
                (&req, resolver.resolve_id(parent), &link_name, &target)
            );
        });
    }

    fn write(
        &self,
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: WriteFlags,
        flags: fuser::OpenFlags,
        lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let data = data.to_owned();
        execute_task!(self, {
            match handler.write(
                &req,
                resolver.resolve_id(ino),
                unsafe { BorrowedFileHandle::from_fuser_file_handle(fh) },
                seek_from_raw(None, offset),
                data,
                FUSEWriteFlags::from(write_flags),
                OpenFlags::from(flags),
                lock_owner,
            ) {
                Ok(bytes_written) => reply.written(bytes_written),
                Err(e) => {
                    warn!("write: ino {:x?}, [{}], {:?}", ino, e, req);
                    reply.error(e.into())
                }
            };
        });
    }

    fn unlink(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.unlink(&req, resolver.resolve_id(parent), &name) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("[{}] unlink: parent_ino: {:x?}, {:?}", parent, e, req);
                    reply.error(e.into())
                }
            };
        });
    }
}
