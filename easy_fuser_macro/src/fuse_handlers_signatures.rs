use proc_macro2::TokenStream;
use syn::{TraitItemFn, parse_quote};


pub fn get_fuse_handler_fn_impl(func_name: &str, tail: Option<TokenStream>) -> TraitItemFn {
    match func_name {
        "get_default_ttl" => parse_quote! {
            fn get_default_ttl(&self) -> Duration #tail
        },
        "init" => parse_quote! {
            fn init(&self, req: &RequestInfo, config: &mut KernelConfig) -> FuseResult<()> #tail
        },
        "destroy" => parse_quote! {
            fn destroy(&self) #tail
        },
        "access" => parse_quote! {
                fn access(&self, req: &RequestInfo, file_id: TId, mask: AccessMask) -> FuseResult<()> #tail
        },
        "bmap" => parse_quote! {
            fn bmap(
                &self,
                req: &RequestInfo,
                file_id: TId,
                blocksize: u32,
                idx: u64,
            ) -> FuseResult<u64> #tail
        },
        "copy_file_range" => parse_quote! {
            fn copy_file_range(
                &self,
                req: &RequestInfo,
                file_in: TId,
                file_handle_in: BorrowedFileHandle<'_>,
                offset_in: i64,
                file_out: TId,
                file_handle_out: BorrowedFileHandle<'_>,
                offset_out: i64,
                len: u64,
            ) -> FuseResult<u32> #tail
        },
        "create" => parse_quote! {
            fn create(
                &self,
                req: &RequestInfo,
                parent_id: TId,
                name: &OsStr,
                mode: u32,
                umask: u32,
                flags: OpenFlags,
            ) -> FuseResult<(OwnedFileHandle, TId::Metadata, FUSEOpenResponseFlags)> #tail
        },
        "fallocate" => parse_quote! {
            fn fallocate(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                offset: i64,
                length: i64,
                mode: FallocateFlags,
            ) -> FuseResult<()> #tail
        },
        "flush" => parse_quote! {
            fn flush(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                lock_owner: u64,
            ) -> FuseResult<()> #tail
        },
        "forget" => parse_quote! {
            fn forget(&self, req: &RequestInfo, file_id: TId, nlookup: u64) #tail
        },
        "fsync" => parse_quote! {
            fn fsync(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                datasync: bool,
            ) -> FuseResult<()> #tail
        },
        "fsyncdir" => parse_quote! {
            fn fsyncdir(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                datasync: bool,
            ) -> FuseResult<()> #tail
        },
        "getattr" => parse_quote! {
            fn getattr(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: Option<BorrowedFileHandle<'_>>,
            ) -> FuseResult<FileAttribute> #tail
        },
        "getlk" => parse_quote! {
            fn getlk(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                lock_owner: u64,
                lock_info: LockInfo,
            ) -> FuseResult<LockInfo> #tail
        },
        "getxattr" => parse_quote! {
            fn getxattr(
                &self,
                req: &RequestInfo,
                file_id: TId,
                name: &OsStr,
                size: u32,
            ) -> FuseResult<Vec<u8>> #tail
        },
        "ioctl" => parse_quote! {
            fn ioctl(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                flags: IOCtlFlags,
                cmd: u32,
                in_data: Vec<u8>,
                out_size: u32,
            ) -> FuseResult<(i32, Vec<u8>)> #tail
        },
        "link" => parse_quote! {
            fn link(
                &self,
                req: &RequestInfo,
                file_id: TId,
                newparent: TId,
                newname: &OsStr,
            ) -> FuseResult<TId::Metadata> #tail
        },
        "listxattr" => parse_quote! {
            fn listxattr(&self, req: &RequestInfo, file_id: TId, size: u32) -> FuseResult<Vec<u8>>
        },
        "lookup" => parse_quote! {
            fn lookup(
                &self,
                req: &RequestInfo,
                parent_id: TId,
                name: &OsStr,
            ) -> FuseResult<TId::Metadata> #tail
        },
        "lseek" => parse_quote! {
            fn lseek(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                seek: SeekFrom,
            ) -> FuseResult<i64> #tail
        },
        "mkdir" => parse_quote! {
            fn mkdir(
                &self,
                req: &RequestInfo,
                parent_id: TId,
                name: &OsStr,
                mode: u32,
                umask: u32,
            ) -> FuseResult<TId::Metadata> #tail
        },
        "mknod" => parse_quote! {
            fn mknod(
                &self,
                req: &RequestInfo,
                parent_id: TId,
                name: &OsStr,
                mode: u32,
                umask: u32,
                rdev: DeviceType,
            ) -> FuseResult<TId::Metadata> #tail
        },
        "open" => parse_quote! {
            fn open(
                &self,
                req: &RequestInfo,
                file_id: TId,
                flags: OpenFlags,
            ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)> #tail
        },
        "opendir" => parse_quote! {
            fn opendir(
                &self,
                req: &RequestInfo,
                file_id: TId,
                flags: OpenFlags,
            ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)> #tail
        },
        "read" => parse_quote! {
            fn read(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                seek: SeekFrom,
                size: u32,
                flags: FUSEOpenFlags,
                lock_owner: Option<u64>,
            ) -> FuseResult<Vec<u8>> #tail
        },
        "readdir" => parse_quote! {
            fn readdir(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
            ) -> FuseResult<Vec<(OsString, TId::MinimalMetadata)>> #tail
        },
        "readdirplus" => parse_quote! {
            fn readdirplus(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle,
            ) -> FuseResult<Vec<(OsString, TId::Metadata)>> #tail
        },
        "readlink" => parse_quote! {
            fn readlink(&self, req: &RequestInfo, file_id: TId) -> FuseResult<Vec<u8>> #tail
        },
        "release" => parse_quote! {
            fn release(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: OwnedFileHandle,
                flags: OpenFlags,
                lock_owner: Option<u64>,
                flush: bool,
            ) -> FuseResult<()> #tail
        },
        "releasedir" => parse_quote! {
            fn releasedir(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: OwnedFileHandle,
                flags: OpenFlags,
            ) -> FuseResult<()> #tail
        },
        "removexattr" => parse_quote! {
            fn removexattr(&self, req: &RequestInfo, file_id: TId, name: &OsStr) -> FuseResult<()> #tail
        },
        "rename" => parse_quote! {
            fn rename(
                &self,
                req: &RequestInfo,
                parent_id: TId,
                name: &OsStr,
                newparent: TId,
                newname: &OsStr,
                flags: RenameFlags,
            ) -> FuseResult<()> #tail
        },
        "rmdir" => parse_quote! {
            fn rmdir(&self, req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()> #tail
        },
        "setattr" => parse_quote! {
            fn setattr(
                &self,
                req: &RequestInfo,
                file_id: TId,
                attrs: SetAttrRequest<'_>,
            ) -> FuseResult<FileAttribute> #tail
        },
        "setlk" => parse_quote! {
            fn setlk(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                lock_owner: u64,
                lock_info: LockInfo,
                sleep: bool,
            ) -> FuseResult<()> #tail
        },
        "setxattr" => parse_quote! {
            fn setxattr(
                &self,
                req: &RequestInfo,
                file_id: TId,
                name: &OsStr,
                value: Vec<u8>,
                flags: FUSESetXAttrFlags,
                position: u32,
            ) -> FuseResult<()> #tail
        },
        "statfs" => parse_quote! {
            fn statfs(&self, req: &RequestInfo, file_id: TId) -> FuseResult<StatFs> #tail
        },
        "symlink" => parse_quote! {
            fn symlink(
                &self,
                req: &RequestInfo,
                parent_id: TId,
                link_name: &OsStr,
                target: &Path,
            ) -> FuseResult<TId::Metadata> #tail
        },
        "write" => parse_quote! {
            fn write(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
                seek: SeekFrom,
                data: Vec<u8>,
                write_flags: FUSEWriteFlags,
                flags: OpenFlags,
                lock_owner: Option<u64>,
            ) -> FuseResult<u32> #tail
        },
        "unlink" => parse_quote! {
            fn unlink(&self, req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()> #tail
        },
        _ => panic!("unknown function signature"),
    }
}
