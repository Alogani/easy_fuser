use std::{
    ffi::OsStr,
    path::Path,
    time::{Instant, SystemTime},
};

use libc::c_int;
use log::{error, info, warn};

use fuser::{
    self, KernelConfig, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

use super::{
    fuse_driver_types::{execute_task, FuseDriver},
    inode_mapping::FileIdResolver,
    macros::*,
};
use crate::{fuse_handler::FuseHandler, types::*};
use private::{unpack_metadata, unpack_minimal_metadata, SafeBorrowable};

fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

impl<T, U, R> fuser::Filesystem for FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseHandler<T>,
    R: FileIdResolver<FileIdType = T>,
{
    fn init(&mut self, req: &Request, config: &mut KernelConfig) -> Result<(), c_int> {
        let req = RequestInfo::from(req);
        match self.get_handler().init(&req, config) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("init {:?}: {:?}", req, e);
                Err(e.raw_error())
            }
        }
    }

    fn destroy(&mut self) {
        self.get_handler().destroy();
    }

    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
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

    fn forget(&mut self, req: &Request, ino: u64, nlookup: u64) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        handler.forget(&req, resolver.resolve_id(ino), nlookup);
        resolver.forget(ino, nlookup);
    }

    fn getattr(&mut self, req: &Request, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
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
                (&req, resolver.resolve_id(ino), fh.map(FileHandle::from))
            );
        });
    }

    fn setattr(
        &mut self,
        req: &Request,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        ctime: Option<SystemTime>,
        fh: Option<u64>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
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
            file_handle: fh.map(FileHandle::from),
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

    fn readlink(&mut self, req: &Request, ino: u64, reply: ReplyData) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.readlink(&req, resolver.resolve_id(ino)) {
                Ok(link) => reply.data(&link),
                Err(e) => {
                    warn!("readlink {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
                    DeviceType::from_rdev(rdev)
                )
            );
        });
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

    fn unlink(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.unlink(&req, resolver.resolve_id(parent), &name) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("unlink {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn rmdir(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.rmdir(&req, resolver.resolve_id(parent), &name) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("rmdir {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn symlink(
        &mut self,
        req: &Request,
        parent: u64,
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
                RenameFlags::from_bits_retain(flags),
            ) {
                Ok(()) => {
                    resolver.rename(parent, &name, newparent, &newname);
                    reply.ok()
                }
                Err(e) => {
                    warn!("rename {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            }
        });
    }

    fn link(
        &mut self,
        req: &Request,
        ino: u64,
        newparent: u64,
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

    fn open(&mut self, req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.open(
                &req,
                resolver.resolve_id(ino),
                OpenFlags::from_bits_retain(_flags),
            ) {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.bits())
                }
                Err(e) => {
                    warn!("open {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.read(
                &req,
                resolver.resolve_id(ino),
                fh.into(),
                offset,
                size,
                FUSEOpenFlags::from_bits_retain(flags),
                lock_owner,
            ) {
                Ok(data_reply) => reply.data(&data_reply),
                Err(e) => {
                    warn!("read {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let data = data.to_owned();
        execute_task!(self, {
            match handler.write(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                offset,
                data,
                FUSEWriteFlags::from_bits_retain(write_flags),
                OpenFlags::from_bits_retain(flags),
                lock_owner,
            ) {
                Ok(bytes_written) => reply.written(bytes_written),
                Err(e) => {
                    warn!("write {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn flush(&mut self, req: &Request, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.flush(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                lock_owner,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("flush {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.release(
                &req,
                resolver.resolve_id(ino),
                _fh.into(),
                OpenFlags::from_bits_retain(_flags),
                _lock_owner,
                _flush,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("release {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn fsync(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.fsync(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                datasync,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("fsync {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn opendir(&mut self, req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.opendir(
                &req,
                resolver.resolve_id(ino),
                OpenFlags::from_bits_retain(_flags),
            ) {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.bits())
                }
                Err(e) => {
                    warn!("opendir {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn readdir(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
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
            unpack_minimal_metadata,
            get_dirmap_iter,
            ReplyDirectory
        );
    }

    fn readdirplus(
        &mut self,
        req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
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
            unpack_metadata,
            get_dirmapplus_iter,
            ReplyDirectoryPlus
        );
    }

    fn releasedir(&mut self, req: &Request, ino: u64, fh: u64, flags: i32, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.releasedir(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                OpenFlags::from_bits_retain(flags),
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("releasedir {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn fsyncdir(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.fsyncdir(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                datasync,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("fsyncdir {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn statfs(&mut self, req: &Request, ino: u64, reply: ReplyStatfs) {
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
                    warn!("statfs {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn setxattr(
        &mut self,
        req: &Request,
        ino: u64,
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
                    warn!("setxattr {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn getxattr(&mut self, req: &Request, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr) {
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
                        reply.error(ErrorKind::ResultTooLarge.into());
                    }
                }
                Err(e) => {
                    warn!("getxattr {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn listxattr(&mut self, req: &Request, ino: u64, size: u32, reply: ReplyXattr) {
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
                        reply.error(ErrorKind::ResultTooLarge.into());
                    }
                }
                Err(e) => {
                    warn!("listxattr {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn removexattr(&mut self, req: &Request, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.removexattr(&req, resolver.resolve_id(ino), &name) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("removexattr {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn access(&mut self, req: &Request, ino: u64, mask: i32, reply: ReplyEmpty) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.access(
                &req,
                resolver.resolve_id(ino),
                AccessMask::from_bits_retain(mask),
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("access {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let name = name.to_owned();
        execute_task!(self, {
            match handler.create(
                &req,
                resolver.resolve_id(parent),
                &name,
                mode,
                umask,
                OpenFlags::from_bits_retain(flags),
            ) {
                Ok((file_handle, metadata, response_flags)) => {
                    let default_ttl = handler.get_default_ttl();
                    let (id, file_attr) = unpack_metadata::<T>(metadata);
                    let ino = resolver.lookup(parent, &name, id, true);
                    let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                    reply.created(
                        &ttl.unwrap_or(default_ttl),
                        &fuse_attr,
                        generation.unwrap_or(get_random_generation()),
                        file_handle.into(),
                        response_flags.bits(),
                    );
                }
                Err(e) => {
                    warn!("create {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
                FileHandle::from(fh),
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
                    warn!("getlk {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
                FileHandle::from(fh),
                lock_owner,
                lock_info,
                sleep,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("setlk {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }

    fn bmap(&mut self, req: &Request<'_>, ino: u64, blocksize: u32, idx: u64, reply: ReplyBmap) {
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.bmap(&req, resolver.resolve_id(ino), blocksize, idx) {
                Ok(block) => reply.bmap(block),
                Err(e) => {
                    warn!("bmap {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        let in_data = in_data.to_owned();
        execute_task!(self, {
            match handler.ioctl(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                IOCtlFlags::from_bits_retain(flags),
                cmd,
                in_data,
                out_size,
            ) {
                Ok((result, data)) => reply.ioctl(result, &data),
                Err(e) => {
                    warn!("ioctl {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.fallocate(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                offset,
                length,
                mode,
            ) {
                Ok(()) => reply.ok(),
                Err(e) => {
                    warn!("fallocate {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.lseek(
                &req,
                resolver.resolve_id(ino),
                FileHandle::from(fh),
                offset,
                whence.into(),
            ) {
                Ok(new_offset) => reply.offset(new_offset),
                Err(e) => {
                    warn!("lseek {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
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
        let req = RequestInfo::from(req);
        let handler = self.get_handler();
        let resolver = self.get_resolver();
        execute_task!(self, {
            match handler.copy_file_range(
                &req,
                resolver.resolve_id(ino_in),
                FileHandle::from(fh_in),
                offset_in,
                resolver.resolve_id(ino_out),
                FileHandle::from(fh_out),
                offset_out,
                len,
                flags,
            ) {
                Ok(bytes_written) => reply.written(bytes_written),
                Err(e) => {
                    warn!("copy_file_range {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            };
        });
    }
}
