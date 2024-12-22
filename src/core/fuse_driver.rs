use std::{
    collections::HashMap, ffi::{OsStr, OsString}, path::Path, sync::{Arc, Mutex}, thread::{self, Thread}, time::{Duration, Instant, SystemTime}
};

use libc::c_int;
use log::{error, info, warn};

use fuser::{
    self, KernelConfig, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

use super::{callback_handlers::{FuseCallbackHandler, ReplyCb}, fuse_operations::{FuseOperations, FuseOperationsTrait}};
use super::inode_mapping::FileIdResolver;
use crate::{templates::DefaultFuseHandler, types::*};

type DirIter<T> = Box<dyn Iterator<Item = T> + Send>;

fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

pub struct FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseCallbackHandler<T>,
    R: FileIdResolver<FileIdType = T>,
{
    callback_handler: U,
    id_resolver: Arc<R>,
    dirmap_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntry>>>>,
    dirmapplus_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntryPlus>>>>,
}

impl<T, U, R> FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseCallbackHandler<T>,
    R: FileIdResolver<FileIdType = T>,
{
    pub fn new(fuse_cb_api: U, resolver: R) -> FuseDriver<T, U, R> {
        FuseDriver {
            callback_handler: fuse_cb_api,
            id_resolver: Arc::new(resolver),
            dirmap_iter: Arc::new(Mutex::new(HashMap::new())),
            dirmapplus_iter: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<T, U, R> fuser::Filesystem for FuseDriver<T, U, R>
where
    T: FileIdType,
    U: FuseCallbackHandler<T>,
    R: FileIdResolver<FileIdType = T>,
{
    fn init(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, _config: &mut KernelConfig) -> Result<(), c_int> {
        
        match handler.init( &req, _config) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("init {:?}: {:?}", &req, e);
                Err(e.raw_error())
            }
        }
    }

    fn destroy(&mut self) {
        handler.destroy(&self.callback_handler)
    }

    fn lookup(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let resolver = resolver.clone();
        let handler = Arc::new(DefaultFuseHandler::new());
        
        let name = name.to_os_string();
        thread::spawn(move ||
            FuseOperations::lookup(handler, resolver, &req, parent, &name, reply)
        );
    }

    fn forget(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, _nlookup: u64) {
        handler.forget(
            
            req.into(),
            resolver.resolve_id(ino),
            _nlookup,
        );
    }

    fn getattr(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        
    }

    fn setattr() {}
        
    }

    fn readlink(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, reply: ReplyData) {

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

    }

    fn unlink(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, parent: u64, name: &OsStr, reply: ReplyEmpty) {

    }

    fn rmdir(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, parent: u64, name: &OsStr, reply: ReplyEmpty) {
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
        
        
        let resolver = resolver.clone();
        let name_owned = name.to_owned();
        let newname_cb = newname.to_os_string();
        match result {
            Ok(()) => {
                resolver.rename(parent, &name_owned, newparent, newname_cb);
                reply.ok()
            }
            Err(e) => {
                warn!("rename {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.rename(
            
            req,
            resolver.resolve_id(parent),
            name,
            resolver.resolve_id(newparent),
            newname,
            RenameFlags::from_bits_retain(flags),
            
        );
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
    }

    fn open(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, _flags: i32, reply: ReplyOpen) {
        
        
        let callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.bits())
                }
                Err(e) => {
                    warn!("open {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            });
        handler.open(
            
            req,
            resolver.resolve_id(ino),
            OpenFlags::from_bits_retain(_flags),
            
        );
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
        
        
        let callback: ReplyCb<Vec<u8>> = Box::new(move |result| match result {
            Ok(data_reply) => reply.data(&data_reply),
            Err(e) => {
                warn!("read {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.read(
            
            req,
            resolver.resolve_id(ino),
            fh.into(),
            offset,
            size,
            FUSEOpenFlags::from_bits_retain(flags),
            lock_owner,
            
        )
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
        let req: RequestInfo = RequestInfo::from(req);
        
        let callback: ReplyCb<u32> = Box::new(move |result| match result {
            Ok(bytes_written) => reply.written(bytes_written),
            Err(e) => {
                warn!("write {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.write(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            offset,
            data,
            FUSEWriteFlags::from_bits_retain(write_flags),
            OpenFlags::from_bits_retain(flags),
            lock_owner,
            
        );
    }

    fn flush(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("flush {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.flush(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            lock_owner,
            
        );
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
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("release {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.release(
            
            req,
            resolver.resolve_id(ino),
            _fh.into(),
            OpenFlags::from_bits_retain(_flags),
            _lock_owner,
            _flush,
            
        );
    }

    fn fsync(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("fsync {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.fsync(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            datasync,
            
        );
    }

    fn opendir(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, _flags: i32, reply: ReplyOpen) {
        
        
        let callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)> =
            Box::new(move |result| match result {
                Ok((file_handle, response_flags)) => {
                    reply.opened(file_handle.into(), response_flags.bits())
                }
                Err(e) => {
                    warn!("opendir {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            });
        handler.opendir(
            
            req,
            resolver.resolve_id(ino),
            OpenFlags::from_bits_retain(_flags),
            
        );
    }

    fn readdir(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, fh: u64, offset: i64, reply: ReplyDirectory) {
        
        
        if offset < 0 {
            error!("readdir called with a negative offset");
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }
        let resolver = resolver.clone();

        // Helper function to create the callback
        fn create_callback<R: FileIdResolver>(
            mut reply: ReplyDirectory,
            req: RequestInfo,
            dirmap_iter: Arc<Mutex<HashMap<u64, DirIter<FuseDirEntry>>>>,
            ino: u64,
            offset: i64,
            resolver: Arc<R>,
        ) -> ReplyCb<DirIter<FuseDirEntry>> {
  
    }

    fn readdirplus(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: ReplyDirectoryPlus,
    ) {
        
        
        let resolver = resolver.clone();
        if offset < 0 {
            error!("readdirplus called with a negative offset");
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }

        fn create_callback<R: FileIdResolver>(
            mut reply: ReplyDirectoryPlus,
            req: RequestInfo,
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
                        warn!("readdirplus {:?} - {:?}", e, req);
                        reply.error(e.raw_error())
                    }
                }
            })
        }

        if offset == 0 {
            // Call the high-level readdirplus function with the callback
            let callback = create_callback(
                reply,
                req,
                self.dirplus_iter.clone(),
                ino,
                offset,
                U::get_default_ttl(),
                resolver,
            );
            self.callback_handler.readdirplus(
                req,
                resolver.resolve_id(ino),
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
                        req,
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

    fn releasedir(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("releasedir {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.releasedir(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(_fh),
            OpenFlags::from_bits_retain(_flags),
            
        );
    }

    fn fsyncdir(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("fsyncdir {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.fsyncdir(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            datasync,
            
        );
    }

    fn statfs(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, reply: ReplyStatfs) {
        
        
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
                warn!("statfs {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.statfs(
            
            req,
            resolver.resolve_id(ino),
            
        );
    }

    fn setxattr(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        name: &OsStr,
        _value: &[u8],
        flags: i32,
        position: u32,
        reply: ReplyEmpty,
    ) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("setxattr {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.setxattr(
            
            req,
            resolver.resolve_id(ino),
            name,
            _value,
            FUSESetXAttrFlags::from_bits_retain(flags),
            position,
            
        )
    }

    fn getxattr(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr) {
        
        
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
                warn!("getxattr {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.getxattr(
            
            req,
            resolver.resolve_id(ino),
            name,
            size,
            
        );
    }

    fn listxattr(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, size: u32, reply: ReplyXattr) {
        
        
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
                warn!("listxattr {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.listxattr(
            
            req,
            resolver.resolve_id(ino),
            size,
            
        );
    }

    fn removexattr(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("removexattr {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.removexattr(
            
            req,
            resolver.resolve_id(ino),
            name,
            
        );
    }

    fn access(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo, ino: u64, mask: i32, reply: ReplyEmpty) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("access {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.access(
            
            req,
            resolver.resolve_id(ino),
            AccessMask::from_bits_retain(mask),
            
        );
    }

    fn create(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
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
                    warn!("create {:?} - {:?}", e, req);
                    reply.error(e.raw_error())
                }
            });
        handler.create(
            
            req,
            resolver.resolve_id(parent),
            name,
            mode,
            umask,
            OpenFlags::from_bits_retain(flags),
            
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
                warn!("getlk {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.getlk(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            lock_owner,
            lock_info,
            
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
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("setlk {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.setlk(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            lock_owner,
            lock_info,
            sleep,
            
        );
    }

    fn bmap(handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,<'_>, ino: u64, blocksize: u32, idx: u64, reply: ReplyBmap) {
        
        
        // Call the high-level function in FuseCallbackAPI
        let callback: ReplyCb<u64> = Box::new(move |result| match result {
            Ok(block) => reply.bmap(block),
            Err(e) => {
                warn!("bmap {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.bmap(
            
            req,
            resolver.resolve_id(ino),
            blocksize,
            idx,
            
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
                warn!("ioctl {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.ioctl(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            IOCtlFlags::from_bits_retain(flags),
            cmd,
            in_data,
            out_size,
            
        )
    }

    fn fallocate(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        
        
        match result {
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("fallocate {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        };
        handler.fallocate(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            offset,
            length,
            mode,
            
        )
    }

    fn lseek(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
        ino: u64,
        fh: u64,
        offset: i64,
        whence: i32,
        reply: ReplyLseek,
    ) {
        
        
        let callback: ReplyCb<i64> = Box::new(move |result| match result {
            Ok(offset) => reply.offset(offset),
            Err(e) => {
                warn!("lseek {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.lseek(
            
            req,
            resolver.resolve_id(ino),
            FileHandle::from(fh),
            offset,
            whence.into(),
            
        );
    }

    fn copy_file_range(
        handler: &Handler,
        resolver: Arc<Resolver>,
        req: &RequestInfo,
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
                warn!("copyfilerange {:?} - {:?}", e, req);
                reply.error(e.raw_error())
            }
        });
        handler.copy_file_range(
            
            req,
            resolver.resolve_id(ino_in),
            FileHandle::from(fh_in),
            offset_in,
            resolver.resolve_id(ino_out),
            FileHandle::from(fh_out),
            offset_out,
            len,
            flags,
            
        );
    }
}
