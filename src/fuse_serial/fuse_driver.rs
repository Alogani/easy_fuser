use crate::types::*;

use super::fuse_handler::FuseHandler;
use crate::core::inode_mapping::*;

use easy_fuser_macro::implement_fuse_driver;

//implement_fuse_driver!("serial");


use std::{
    collections::{
        HashMap,VecDeque
    },ffi::{
        OsStr,OsString
    },path::Path,time::{
        Instant,SystemTime
    },
};
use libc::c_int;
use log::{
    error,info,warn
};
use fuser::{
    self,KernelConfig,ReplyAttr,ReplyBmap,ReplyCreate,ReplyData,ReplyDirectory,ReplyDirectoryPlus,ReplyEmpty,ReplyEntry,ReplyIoctl,ReplyLock,ReplyLseek,ReplyOpen,ReplyStatfs,ReplyWrite,ReplyXattr,Request,TimeOrNow,
};
type DirMapEntries<TAttr>  = HashMap<(u64,i64),VecDeque<(OsString,u64,TAttr)>> ;
fn get_random_generation() -> u64 {
    Instant::now().elapsed().as_nanos()as u64
}
use std::cell::RefCell;
pub(crate)struct FuseDriver<TId,THandler>where TId:FileIdType,THandler:FuseHandler<TId> ,{
    handler:THandler,resolver:TId::Resolver,dirmap_entries:RefCell<DirMapEntries<FileKind>> ,dirmapplus_entries:RefCell<DirMapEntries<FileAttribute>> ,
}
impl <TId,THandler>FuseDriver<TId,THandler>where TId:FileIdType,THandler:FuseHandler<TId> ,{
    #[doc = r" num_thread is ignored in serial mode, it is kept for consistency with other modes"]
    pub fn new(handler:THandler,_num_threads:usize) -> FuseDriver<TId,THandler>{
        FuseDriver {
            handler,resolver:TId::Resolver::new(),dirmap_entries:RefCell::new(HashMap::new()),dirmapplus_entries:RefCell::new(HashMap::new()),
        }
    }

    }
impl <TId,THandler>FuseDriver<TId,THandler>where TId:FileIdType,THandler:FuseHandler<TId> ,{
    fn access(&mut self,req: &Request,ino:u64,mask:i32,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let mask = AccessMask::from_bits_retain(mask);
        match handler.access(&req,ino,mask){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("access: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn bmap(&mut self,req: &Request<'_> ,ino:u64,blocksize:u32,idx:u64,reply:ReplyBmap){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        match handler.bmap(&req,ino,blocksize,idx){
            Ok(block) => reply.bmap(block),
            Err(e) => {
                warn!("bmap: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn copy_file_range(&mut self,req: &Request,ino_in:u64,fh_in:u64,offset_in:i64,ino_out:u64,fh_out:u64,offset_out:i64,len:u64,flags:u32,reply:ReplyWrite,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let ino_in = resolver.resolve_id(ino_in);
        let fh_in = unsafe {
            BorrowedFileHandle::from_raw(fh_in)
        };
        let ino_out = resolver.resolve_id(ino_out);
        let fh_out = unsafe {
            BorrowedFileHandle::from_raw(fh_out)
        };
        match handler.copy_file_range(&req,ino_in,fh_in,offset_in,ino_out,fh_out,offset_out,len,flags){
            Ok(bytes_written) => reply.written(bytes_written),
            Err(e) => {
                warn!("copy_file_range: [{}], {:?}",e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn create(&mut self,req: &Request,parent:u64,name: &OsStr,mode:u32,umask:u32,flags:i32,reply:ReplyCreate,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let name = name.as_ref();
        let flags = OpenFlags::from_bits_retain(flags);
        match handler.create(&req,parent,name,mode,umask,flags){
            Ok((file_handle,metadata,response_flags)) => {
                let default_ttl = handler.get_default_ttl();
                let(id,file_attr) = TId::extract_metadata(metadata);
                let ino = resolver.lookup(parent, &name,id,true);
                let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
                reply.created(&ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),file_handle.as_raw(),response_flags.bits(),);
            },
            Err(e) => {
                warn!("create: [{}], {:?}",e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn fallocate(&mut self,req: &Request,ino:u64,fh:u64,offset:i64,length:i64,mode:i32,reply:ReplyEmpty,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let mode = FallocateFlags::from_bits_retain(mode);
        match handler.fallocate(&req,ino,fh,offset,length,mode){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("fallocate: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn flush(&mut self,req: &Request,ino:u64,fh:u64,lock_owner:u64,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        match handler.flush(&req,ino,fh,lock_owner){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("flush: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn forget(&mut self,req: &Request,ino:u64,nlookup:u64){
        let req = RequestInfo::from(req);
        let ino = self.resolver.resolve_id(ino);
        self.handler.forget(&req,ino,nlookup);
        self.resolver.forget(ino,nlookup);
    }
    fn fsync(&mut self,req: &Request,ino:u64,fh:u64,datasync:bool,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        match handler.fsync(&req,ino,fh,datasync){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("fsync: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn fsyncdir(&mut self,req: &Request,ino:u64,fh:u64,datasync:bool,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        match handler.fsyncdir(&req,ino,fh,datasync){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("fsyncdir: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn getattr(&mut self,req: &Request,ino:u64,fh:Option<u64> ,reply:ReplyAttr){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        match handler.getattr(&req,ino,fh){
            Ok(file_attr) => {
                let default_ttl = handler.get_default_ttl();
                let(fuse_attr,ttl,_) = file_attr.to_fuse(ino);
                reply.attr(&ttl.unwrap_or(default_ttl), &fuse_attr);
            },
            Err(e) => {
                warn!("getattr: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn getlk(&mut self,req: &Request<'_> ,ino:u64,fh:u64,lock_owner:u64,start:u64,end:u64,typ:i32,pid:u32,reply:ReplyLock,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let lock_info = LockInfo {
            start,end,lock_type:LockType::from_bits_retain(typ),pid,
        };
        match handler.getlk(&req,ino,fh,lock_owner,lock_info){
            Ok(lock) => reply.lock(lock),
            Err(e) => {
                warn!("getlk: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn getxattr(&mut self,req: &Request,ino:u64,name: &OsStr,size:u32,reply:ReplyXattr){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let name = name.as_ref();
        match handler.getxattr(&req,ino,name,size){
            Ok(xattr_data) => {
                if size==0 {
                    reply.size(xattr_data.len()as u32);
                }else if size>=xattr_data.len()as u32 {
                    reply.data(&xattr_data);
                }else {
                    reply.error(ErrorKind::ResultTooLarge.into());
                }
            }
            Err(e) => {
                warn!("getxattr: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn ioctl(&mut self,req: &Request<'_> ,ino:u64,fh:u64,flags:u32,cmd:u32,in_data: &[u8],out_size:u32,reply:ReplyIoctl,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let in_data = in_data.to_owned();
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let flags = IOCtlFlags::from_bits_retain(flags);
        match handler.ioctl(&req,ino,fh,flags,cmd,in_data,out_size){
            Ok((result,data)) => reply.ioctl(result, &data),
            Err(e) => {
                warn!("ioctl: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn link(&mut self,req: &Request,ino:u64,newparent:u64,newname: &OsStr,reply:ReplyEntry,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let newname = newname.to_owned();
        let newname = newname.as_ref();
        let newparent = resolver.resolve_id(newparent);
        match handler.link(&req,ino,newparent,newname){
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
                let(id,file_attr) = TId::extract_metadata(metadata);
                let ino = resolver.lookup(parent,name,id,true);
                let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
                reply.entry(&ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),);
            }
            Err(e) => {
                warn!("link: [{}], {:?}",e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn listxattr(&mut self,req: &Request,ino:u64,size:u32,reply:ReplyXattr){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        match handler.listxattr(&req,ino,size){
            Ok(xattr_data) => {
                if size==0 {
                    reply.size(xattr_data.len()as u32);
                }else if size>=xattr_data.len()as u32 {
                    reply.data(&xattr_data);
                }else {
                    reply.error(ErrorKind::ResultTooLarge.into());
                }
            }
            Err(e) => {
                warn!("listxattr: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn lookup(&mut self,req: &Request,parent:u64,name: &OsStr,reply:ReplyEntry){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = parent;
        let parent = resolver.resolve_id(parent);
        match handler.lookup(&req,parent,name){
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
                let(id,file_attr) = TId::extract_metadata(metadata);
                let ino = resolver.lookup(parent,name,id,true);
                let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
                reply.entry(&ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),);
            }
            Err(e) => {
                info!("lookup: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn lseek(&mut self,req: &Request,ino:u64,fh:u64,offset:i64,whence:i32,reply:ReplyLseek,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let seek = seek_from_raw(Some(whence),offset);
        match handler.lseek(&req,ino,fh,seek,){
            Ok(new_offset) => reply.offset(new_offset),
            Err(e) => {
                warn!("lseek: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn mkdir(&mut self,req: &Request,parent:u64,name: &OsStr,mode:u32,umask:u32,reply:ReplyEntry,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = parent;
        let parent = resolver.resolve_id(parent);
        let name = name.as_ref();
        match handler.mkdir(&req,parent,name,mode,umask){
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
                let(id,file_attr) = TId::extract_metadata(metadata);
                let ino = resolver.lookup(parent,name,id,true);
                let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
                reply.entry(&ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),);
            }
            Err(e) => {
                warn!("mkdir: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn mknod(&mut self,req: &Request,parent:u64,name: &OsStr,mode:u32,umask:u32,rdev:u32,reply:ReplyEntry,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = parent;
        let parent = resolver.resolve_id(parent);
        let name = name.as_ref();
        let rdev = DeviceType::from_rdev(rdev.try_into().unwrap());
        match handler.mknod(&req,parent,name,mode,umask,rdev){
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
                let(id,file_attr) = TId::extract_metadata(metadata);
                let ino = resolver.lookup(parent,name,id,true);
                let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
                reply.entry(&ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),);
            }
            Err(e) => {
                warn!("mknod: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn open(&mut self,req: &Request,ino:u64,flags:i32,reply:ReplyOpen){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let flags = OpenFlags::from_bits_retain(flags);
        match handler.open(&req,ino,flags){
            Ok((file_handle,response_flags)) => {
                reply.opened(file_handle.as_raw(),response_flags.bits())
            }
            Err(e) => {
                warn!("open: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn opendir(&mut self,req: &Request,ino:u64,flags:i32,reply:ReplyOpen){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let flags = OpenFlags::from_bits_retain(flags);
        match handler.opendir(&req,ino,flags){
            Ok((file_handle,response_flags)) => {
                reply.opened(file_handle.as_raw(),response_flags.bits())
            }
            Err(e) => {
                warn!("opendir: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn read(&mut self,req: &Request,ino:u64,fh:u64,offset:i64,size:u32,flags:i32,lock_owner:Option<u64> ,reply:ReplyData,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let seek = seek_from_raw(Some(offset),0);
        let flags = FUSEOpenFlags::from_bits_retain(flags);
        match handler.read(&req,ino,fh,offset,size,flags,lock_owner){
            Ok(data_reply) => reply.data(&data_reply),
            Err(e) => {
                warn!("read: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn readdir(&mut self,req: &Request,ino:u64,fh:u64,offset:i64,mut reply:ReplyDirectoryPlus,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let dirmap_entries =  &self.dirmap_entries;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        if offset<0 {
            {
                let lvl = (log::Level::Error);
                if lvl<=log::STATIC_MAX_LEVEL&&lvl<=log::max_level(){
                    log::__private_api::log(log::__private_api::format_args!("readdir called with a negative offset"),lvl, &((log::__private_api::module_path!()),log::__private_api::module_path!(),log::__private_api::loc()),(),);
                }
            };
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }let mut directory_entries = match offset {
            0 => match handler.readdir(&req,ino,fh,offset){
                Ok(children) => {
                    let(child_list,attr_list):(Vec<_> ,Vec<_>) = children.into_iter().map(|item|{
                        let(child_id,child_attr) = TId::extract_minimal_metadata(item.1);
                        ((item.0,child_id),child_attr)
                    }).unzip();
                    resolver.add_children(ino,child_list,false,).into_iter().zip(attr_list.into_iter()).map(|((file_name,file_ino),file_attr)|{
                        (file_name,file_ino,file_attr)
                    }).collect()
                }
                Err(e) => {
                    warn!("readdir: ino {:x?}, [{}], {:?}",log_ino,e,req);
                    reply.error(e.raw_error());
                    return;
                }
            
                },
            _ => match {
                dirmap_entries.borrow_mut().remove(&(ino,offset))
            }{
                Some(directory_entries) => directory_entries,
                None => {
                    reply.ok();
                    return;
                }
            
                },
        
            };
        let mut new_offset = offset+1;
        while let Some((name,ino,kind)) = directory_entries.pop_front(){
            if reply.add(ino,new_offset,kind, &name){
                dirmap_entries.borrow_mut().insert((ino,new_offset),directory_entries);
                break;
            }new_offset+=1;
        }reply.ok();
    }
    fn readdirplus(&mut self,req: &Request,ino:u64,fh:u64,offset:i64,mut reply:ReplyDirectory,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let dirmap_entries =  &self.dirmapplus_entries;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        if offset<0 {
            {
                let lvl = (log::Level::Error);
                if lvl<=log::STATIC_MAX_LEVEL&&lvl<=log::max_level(){
                    log::__private_api::log(log::__private_api::format_args!("readdir called with a negative offset"),lvl, &((log::__private_api::module_path!()),log::__private_api::module_path!(),log::__private_api::loc()),(),);
                }
            };
            reply.error(ErrorKind::InvalidArgument.into());
            return;
        }let mut directory_entries = match offset {
            0 => match handler.readdirplus(&req,ino,fh,offset){
                Ok(children) => {
                    let(child_list,attr_list):(Vec<_> ,Vec<_>) = children.into_iter().map(|item|{
                        let(child_id,child_attr) = TId::extract_metadata(item.1);
                        ((item.0,child_id),child_attr)
                    }).unzip();
                    resolver.add_children(ino,child_list,true,).into_iter().zip(attr_list.into_iter()).map(|((file_name,file_ino),file_attr)|{
                        (file_name,file_ino,file_attr)
                    }).collect()
                }
                Err(e) => {
                    warn!("readdirplus: ino {:x?}, [{}], {:?}",log_ino,e,req);
                    reply.error(e.raw_error());
                    return;
                }
            
                },
            _ => match {
                dirmap_entries.borrow_mut().remove(&(ino,offset))
            }{
                Some(directory_entries) => directory_entries,
                None => {
                    reply.ok();
                    return;
                }
            
                },
        
            };
        let mut new_offset = offset+1;
        let default_ttl = handler.get_default_ttl();
        while let Some((name,ino,file_attr)) = directory_entries.pop_front(){
            let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
            if reply.add(ino,new_offset,name, &ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),){
                dirmap_entries.borrow_mut().insert((ino,new_offset),directory_entries);
                break;
            }new_offset+=1;
        }reply.ok();
    }
    fn readlink(&mut self,req: &Request,ino:u64,reply:ReplyData){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        match handler.readlink(&req,ino){
            Ok(link) => reply.data(&link),
            warn_error
        
            };
        ;
    }
    fn release(&mut self,req: &Request,ino:u64,fh:u64,flags:i32,_lock_owner:Option<u64> ,_flush:bool,reply:ReplyEmpty,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let flags = OpenFlags::from_bits_retain(flags);
        match handler.release(&req,ino,fh,flags,_lock_owner,_flush){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("release: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn releasedir(&mut self,req: &Request,ino:u64,fh:u64,flags:i32,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let flags = OpenFlags::from_bits_retain(flags);
        match handler.releasedir(&req,ino,fh,flags){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("releasedir: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn removexattr(&mut self,req: &Request,ino:u64,name: &OsStr,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let name = name.as_ref();
        match handler.removexattr(&req,ino,name){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("removexattr: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn rename(&mut self,req: &Request,parent:u64,name: &OsStr,newparent:u64,newname: &OsStr,flags:u32,reply:ReplyEmpty,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let newname = newname.to_owned();
        let log_ino = parent;
        let parent = resolver.resolve_id(parent);
        let name = name.as_ref();
        let newname = newname.as_ref();
        let flags = RenameFlags::from_bits_retain(flags);
        match handler.rename(&req,parent,name,newparent,newname,flags){
            Ok(()) => {
                resolver.rename(parent, &name,newparent, &newname);
                reply.ok()
            }
            Err(e) => {
                warn!("rename: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn rmdir(&mut self,req: &Request,parent:u64,name: &OsStr,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = parent;
        let parent = resolver.resolve_id(parent);
        let name = name.as_ref();
        match handler.rmdir(&req,parent,name){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("rmdir: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn setattr(&mut self,req: &Request,ino:u64,mode:Option<u32> ,uid:Option<u32> ,gid:Option<u32> ,size:Option<u64> ,atime:Option<TimeOrNow> ,mtime:Option<TimeOrNow> ,ctime:Option<SystemTime> ,fh:Option<u64> ,crtime:Option<SystemTime> ,chgtime:Option<SystemTime> ,bkuptime:Option<SystemTime> ,_flags:Option<u32> ,reply:ReplyAttr,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let attrs = SetAttrRequest {
            mode,uid,gid,size,atime:atime,mtime:mtime,ctime:ctime,crtime:crtime,chgtime:chgtime,bkuptime:bkuptime,flags:None,file_handle:fh.map(|fh|unsafe {
                BorrowedFileHandle::from_raw(fh)
            }),
        };
        match handler.setattr(&req,ino,attrs){
            Ok(file_attr) => {
                let default_ttl = handler.get_default_ttl();
                let(fuse_attr,ttl,_) = file_attr.to_fuse(ino);
                reply.attr(&ttl.unwrap_or(default_ttl), &fuse_attr);
            },
            Err(e) => {
                warn!("setattr: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            }
    }
    fn setlk(&mut self,req: &Request<'_> ,ino:u64,fh:u64,lock_owner:u64,start:u64,end:u64,typ:i32,pid:u32,sleep:bool,reply:ReplyEmpty,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let lock_info = LockInfo {
            start,end,lock_type:LockType::from_bits_retain(typ),pid,
        };
        match handler.setlk(&req,ino,fh,lock_owner,lock_info,sleep,){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("setlk: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn setxattr(&mut self,req: &Request,ino:u64,name: &OsStr,value: &[u8],flags:i32,position:u32,reply:ReplyEmpty,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let value = value.to_owned();
        let name = name.as_ref();
        let flags = FUSESetXAttrFlags::from_bits_retain(flags);
        match handler.setxattr(&req,ino,name,value,flags,position){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("setxattr: [{}], {:?}",e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn statfs(&mut self,req: &Request,ino:u64,reply:ReplyStatfs){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        match handler.statfs(&req,ino){
            Ok(statfs) => reply.statfs(statfs.total_blocks,statfs.free_blocks,statfs.available_blocks,statfs.total_files,statfs.free_files,statfs.block_size,statfs.max_filename_length,statfs.fragment_size,),
            Err(e) => {
                warn!("statfs: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn symlink(&mut self,req: &Request,parent:u64,link_name: &OsStr,target: &Path,reply:ReplyEntry,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let link_name = link_name.to_owned();
        let target = target.to_owned();
        let link_name = link_name.as_ref();
        let target = target.as_ref();
        match handler.symlink(&req,parent,link_name,target){
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
                let(id,file_attr) = TId::extract_metadata(metadata);
                let ino = resolver.lookup(parent,name,id,true);
                let(fuse_attr,ttl,generation) = file_attr.to_fuse(ino);
                reply.entry(&ttl.unwrap_or(default_ttl), &fuse_attr,generation.unwrap_or(get_random_generation()),);
            }
            Err(e) => {
                warn!("symlink: [{}], {:?}",e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
    }
    fn write(&mut self,req: &Request,ino:u64,fh:u64,offset:i64,data: &[u8],write_flags:u32,flags:i32,lock_owner:Option<u64> ,reply:ReplyWrite,){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let data = data.to_owned();
        let log_ino = ino;
        let ino = resolver.resolve_id(ino);
        let fh = unsafe {
            BorrowedFileHandle::from_raw(fh)
        };
        let seek = seek_from_raw(Some(offset),0);
        let write_flags = FUSEWriteFlags::from_bits_retain(write_flags);
        let flags = OpenFlags::from_bits_retain(flags);
        match handler.write(&req,ino,fh,offset,data,write_flags,flags,lock_owner){
            Ok(bytes_written) => reply.written(bytes_written),
            Err(e) => {
                warn!("write: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }
    fn unlink(&mut self,req: &Request,parent:u64,name: &OsStr,reply:ReplyEmpty){
        let req = RequestInfo::from(req);
        let handler =  &self.handler;
        let resolver =  &self.resolver;
        let name = name.to_owned();
        let log_ino = parent;
        let parent = resolver.resolve_id(parent);
        let name = name.as_ref();
        match handler.unlink(&req,parent,name){
            Ok(()) => reply.ok(),
            Err(e) => {
                warn!("unlink: ino {:x?}, [{}], {:?}",log_ino,e,req);
                reply.error(e.raw_error());
                return;
            }
        
            };
        ;
    }

    }