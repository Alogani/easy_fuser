use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

use libc::c_int;
use log::{error, info, warn};

use fuser::{
    self, KernelConfig, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};
use fuser::FileAttr as FuseFileAttr;

use crate::types::*;
use crate::types::private::*;
use super::inode_mapping::FileIdResolver;
use crate::fuse_handler::FuseHandler;


type DirIter<T> = Box<dyn Iterator<Item = T> + Send>;


fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

macro_rules! handle_fuse_reply_entry {
    ($handler:expr, $resolver:expr, $req:expr, $parent:expr, $name:expr, $reply:expr,
    $result:expr, $operation:literal) => {
        let default_ttl = $handler.get_default_ttl();
        match $result {
            Ok(metadata) => {
                let (id, file_attr) = unpack_metadata::<T>(metadata);
                let ino = $resolver.lookup($parent, $name, id, true);
                let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                $reply.entry(
                    &ttl.unwrap_or(default_ttl),
                    &fuse_attr,
                    generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => {
                warn!("{} {:?} - {:?}", $operation, e, $req);
                $reply.error(e.raw_error())
            }
        }
    };
}

macro_rules! handle_fuse_reply_attr {
    ($handler:expr, $resolve:expr, $req:expr, $ino:expr, $reply:expr,
    $result:expr, $operation:literal) => {
        let default_ttl = $handler.get_default_ttl();
        match $result {
            Ok(file_attr) => {
                let (fuse_attr, ttl, _) = file_attr.to_fuse($ino);
                $reply.attr(&ttl.unwrap_or(default_ttl), &fuse_attr);
            }
            Err(e) => {
                warn!("{} {:?} - {:?}", $operation, e, $req);
                $reply.error(e.raw_error())
            }
        }
    };
}

pub struct FuseOperations<T, Handler, Resolver>
where
    T: FileIdType,
    Handler: FuseHandler<T>,
    Resolver: FileIdResolver<FileIdType = T>,
{
    phantom1: PhantomData<Handler>,
    phantom2: PhantomData<Resolver>,
}

impl<T, Handler, Resolver> FuseOperationsTrait<T, Handler, Resolver>
    for FuseOperations<T, Handler, Resolver>
where
    T: FileIdType,
    Handler: FuseHandler<T>,
    Resolver: FileIdResolver<FileIdType = T>,
{
}

pub trait FuseOperationsTrait<T, Handler, Resolver>
where
    T: FileIdType,
    Handler: FuseHandler<T>,
    Resolver: FileIdResolver<FileIdType = T>,
{
    fn init(
        handler: &mut Handler,
        req: &RequestInfo,
        _config: &mut KernelConfig,
    ) -> Result<(), c_int> {
        match handler.init(req, _config) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("init {:?}: {:?}", req, e);
                Err(e.raw_error())
            }
        }
    }

    fn destroy(handler: &mut Handler) {
        handler.destroy()
    }

    fn lookup(
        handler: Arc<Handler>,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        handle_fuse_reply_entry!(
            handler,
            resolver,
            req,
            parent,
            name,
            reply,
            handler.lookup(req, resolver.resolve_id(parent), name,),
            "lookup"
        );
    }

    fn forget(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        nlookup: u64,
    ) {
        handler.forget(req, resolver.resolve_id(ino), nlookup);
        resolver.forget(ino, nlookup);
    }

    fn getattr(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: Option<u64>,
        reply: ReplyAttr,
    ) {
        handle_fuse_reply_attr!(
            handler,
            resolver,
            req,
            ino,
            reply,
            handler.getattr(req, resolver.resolve_id(ino), fh.map(FileHandle::from)),
            "getattr"
        );
    }

    fn setattr(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
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
        handle_fuse_reply_attr!(
            handler,
            resolver,
            req,
            ino,
            reply,
            handler.setattr(req, resolver.resolve_id(ino), attrs,),
            "setattr"
        );
    }

    fn readlink(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        reply: ReplyData,
    ) {
        match handler.readlink(req, resolver.resolve_id(ino)) {
            Ok(link) => reply.data(&link),
            Err(e) => {
                warn!("readlink {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
    }

    fn mknod(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        handle_fuse_reply_entry!(
            handler,
            resolver,
            req,
            parent,
            name,
            reply,
            handler.mknod(
                &req,
                resolver.resolve_id(parent),
                name,
                mode,
                umask,
                DeviceType::from_rdev(rdev),
            ),
            "mknod"
        );
    }

    fn mkdir(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        handle_fuse_reply_entry!(
            handler,
            resolver,
            req,
            parent,
            name,
            reply,
            handler.mkdir(req, resolver.resolve_id(parent), name, mode, umask,),
            "mkdir"
        );
    }

    fn unlink(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        match handler.unlink(req, resolver.resolve_id(parent), &name) {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("unlink {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
    }

    fn rmdir(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        match handler.rmdir(req, resolver.resolve_id(parent), name) {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("rmdir {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
    }

    fn symlink(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        link_name: &OsStr,
        target: &Path,
        reply: ReplyEntry,
    ) {
        handle_fuse_reply_entry!(
            handler,
            resolver,
            req,
            parent,
            link_name,
            reply,
            handler.symlink(req, resolver.resolve_id(parent), link_name, target),
            "symlink"
        );
    }

    fn rename(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        flags: u32,
        reply: ReplyEmpty,
    ) {
        match handler.rename(
            req,
            resolver.resolve_id(parent),
            name,
            resolver.resolve_id(newparent),
            newname,
            RenameFlags::from_bits_retain(flags),
        ) {
            Ok(()) => {
                resolver.rename(parent, name, newparent, newname);
                reply.ok()
            }
            Err(e) => {
                warn!("rename {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        }
    }

    fn link(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        newparent: u64,
        newname: &OsStr,
        reply: ReplyEntry,
    ) {
        handle_fuse_reply_entry!(
            handler,
            resolver,
            req,
            newparent,
            newname,
            reply,
            handler.link(req, resolver.resolve_id(ino), resolver.resolve_id(newparent), newname,),
            "link"
        );
    }

    fn open(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        _flags: i32,
        reply: ReplyOpen,
    ) {
        match handler.open(
            req,
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
    }

    fn read(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        match handler.read(
            req,
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
    }

    fn write(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        match handler.write(
            req,
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
    }

    fn flush(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        match handler.flush(
            req,
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
    }

    fn release(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        match handler.release(
            req,
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
    }

    fn fsync(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        datasync: bool,
        reply: ReplyEmpty,
    ) {
        match handler.fsync(
            req,
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
    }

    fn opendir(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        _flags: i32,
        reply: ReplyOpen,
    ) {
        match handler.opendir(
            req,
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
    }

    fn readdir(
        handler: &Handler,
        resolver: Arc<Resolver>,
        dirmap_iter: Arc<Mutex<HashMap<(u64, i64), DirIter<(OsString, u64, FileKind)>>>>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if offset < 0 {
            error!("readdir called with a negative offset");
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }
        let mut dir_iter = match offset {
            0 => match handler.readdir(req, resolver.resolve_id(ino), FileHandle::from(fh)) {
                Ok(children) => {
                    let (child_list, attr_list): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|item| {
                            let (child_id, child_attr) = unpack_minimal_metadata::<T>(item.1);
                            ((item.0, child_id), child_attr)
                        }
                    )
                    .unzip();
                    
                    Box::new(
                        resolver.add_children(ino, child_list, false)
                            .into_iter()
                            .zip(attr_list.into_iter())
                            .map(|((file_name, file_ino), file_attr)| (file_name, file_ino, file_attr))
                    )
                }
                Err(e) => {
                    warn!("readdir {:?}: {:?}", req, e);
                    reply.error(e.raw_error());
                    return;
                }
            },
            _ => match dirmap_iter.lock().unwrap().remove(&(ino, offset)) {
                Some(dirmap_iter) => dirmap_iter,
                None => {
                    error!("readdir called with a unknown offset");
                    reply.error(ErrorKind::InvalidArgument.into());
                    return;
                }
            },
        };
        let mut new_offset = offset + 1;
        while let Some((name, ino, kind)) = dir_iter.next() {
            if reply.add(
                ino,
                new_offset,
                kind,
                &name,
            ) {
                // Save the state of the iterator for future calls
                dirmap_iter.lock().unwrap().insert((ino, new_offset), dir_iter);
                break;
            }
            new_offset += 1;
        }
        reply.ok();
    }
}
