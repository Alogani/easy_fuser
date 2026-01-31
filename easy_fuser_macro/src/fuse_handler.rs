use quote::quote;
use syn::{parse_quote, TraitItemFn};

use crate::handler_type::HandlerType;

fn get_function_defs() -> Vec<TraitItemFn> {
    let mut result = Vec::new();
    result.push(parse_quote! {
        /// Check file access permissions
            ///
            /// This method is called for the access() system call. If the 'default_permissions'
            /// mount option is given, this method is not called. This method is not called
            /// under Linux kernel versions 2.4.x
            fn access(&self, req: &RequestInfo, file_id: TId, mask: AccessMask) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Map block index within file to block index within device
        ///
        /// Note: This makes sense only for block device backed filesystems mounted
        /// with the 'blkdev' option
        fn bmap(
            &self,
            req: &RequestInfo,
            file_id: TId,
            blocksize: u32,
            idx: u64,
        ) -> FuseResult<u64>;
    });
    result.push(parse_quote! {
        /// Copy the specified range from the source inode to the destination inode
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
            flags: u32, // Not implemented yet in standard
        ) -> FuseResult<u32>;
    });
    result.push(parse_quote! {
        /// Create and open a file
        ///
        /// If the file does not exist, first create it with the specified mode, and then
        /// open it. Open flags (with the exception of O_NOCTTY) are available in flags.
        /// If this method is not implemented or under Linux kernel versions earlier than
        /// 2.6.15, the mknod() and open() methods will be called instead.
        fn create(
            &self,
            req: &RequestInfo,
            parent_id: TId,
            name: &OsStr,
            mode: u32,
            umask: u32,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, TId::Metadata, FUSEOpenResponseFlags)>;
    });
    result.push(parse_quote! {
        /// Preallocate or deallocate space to a file
        fn fallocate(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            offset: i64,
            length: i64,
            mode: FallocateFlags,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Flush cached data for an open file
        ///
        /// Called on each close() of the opened file. Not guaranteed to be called after writes or at all.
        /// Used for returning write errors or removing file locks.
        fn flush(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            lock_owner: u64,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Release references to an inode, if the nlookup count reaches zero (to substract from the number of lookups).
        fn forget(&self, req: &RequestInfo, file_id: TId, nlookup: u64);
    });
    result.push(parse_quote! {
        /// Synchronize file contents
        ///
        /// If datasync is true, only flush user data, not metadata.
        fn fsync(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            datasync: bool,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Synchronize directory contents
        ///
        /// If the datasync parameter is true, then only the directory contents should
        /// be flushed, not the metadata. The file_handle will contain the value set
        /// by the opendir method, or will be undefined if the opendir method didn't
        /// set any value.
        fn fsyncdir(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            datasync: bool,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Modify file attributes
        fn getattr(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: Option<BorrowedFileHandle<'_>>,
        ) -> FuseResult<FileAttribute>;
    });
    result.push(parse_quote! {
        /// Test for a POSIX file lock.
        fn getlk(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            lock_owner: u64,
            lock_info: LockInfo,
        ) -> FuseResult<LockInfo>;
    });
    result.push(parse_quote! {
        /// Get an extended attribute
        fn getxattr(
            &self,
            req: &RequestInfo,
            file_id: TId,
            name: &OsStr,
            size: u32,
        ) -> FuseResult<Vec<u8>>;
    });
    result.push(parse_quote! {
        /// control device
        fn ioctl(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            flags: IOCtlFlags,
            cmd: u32,
            in_data: Vec<u8>,
            out_size: u32,
        ) -> FuseResult<(i32, Vec<u8>)>;
    });
    result.push(parse_quote! {
        /// Create a hard link.
        fn link(
            &self,
            req: &RequestInfo,
            file_id: TId,
            newparent: TId,
            newname: &OsStr,
        ) -> FuseResult<TId::Metadata>;
    });
    result.push(parse_quote! {
        /// List extended attribute names
        fn listxattr(&self, req: &RequestInfo, file_id: TId, size: u32) -> FuseResult<Vec<u8>>;
    });
    result.push(parse_quote! {
        /// Retrieve file attributes for a directory entry by name and increment the lookup count associated with the inode.
        fn lookup(
            &self,
            req: &RequestInfo,
            parent_id: TId,
            name: &OsStr,
        ) -> FuseResult<TId::Metadata>;
    });
    result.push(parse_quote! {
        /// Reposition read/write file offset
        fn lseek(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            seek: SeekFrom,
        ) -> FuseResult<i64>;
    });
    result.push(parse_quote! {
        /// Create a new directory
        fn mkdir(
            &self,
            req: &RequestInfo,
            parent_id: TId,
            name: &OsStr,
            mode: u32,
            umask: u32,
        ) -> FuseResult<TId::Metadata>;
    });
    result.push(parse_quote! {
        /// Create a new file node (regular file, device, FIFO, socket, etc)
        fn mknod(
            &self,
            req: &RequestInfo,
            parent_id: TId,
            name: &OsStr,
            mode: u32,
            umask: u32,
            rdev: DeviceType,
        ) -> FuseResult<TId::Metadata>;
    });
    result.push(parse_quote! {
        /// Open a file and return a file handle.
        ///
        /// Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are available in flags. You may store an arbitrary file handle (pointer, index, etc) in file_handle response, and use this in other all other file operations (read, write, flush, release, fsync). Filesystem may also implement stateless file I/O and not store anything in fh. There are also some flags (direct_io, keep_cache) which the filesystem may set, to change the way the file is opened. See fuse_file_info structure in <fuse_common.h> for more details.
        fn open(
            &self,
            req: &RequestInfo,
            file_id: TId,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)>;
    });
    result.push(parse_quote! {
        /// Open a directory
        ///
        /// Allows storing a file handle for use in subsequent directory operations.
        fn opendir(
            &self,
            req: &RequestInfo,
            file_id: TId,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)>;
    });
    result.push(parse_quote! {
        /// Read data from a file
        ///
        /// Read should send exactly the number of bytes requested except on EOF or error, otherwise the rest of the data will be substituted with zeroes. An exception to this is when the file has been opened in ‘direct_io’ mode, in which case the return value of the read system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value.
        ///
        /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
        fn read(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            seek: SeekFrom,
            size: u32,
            flags: FUSEOpenFlags,
            lock_owner: Option<u64>,
        ) -> FuseResult<Vec<u8>>;
    });
    result.push(parse_quote! {
        /// Read directory contents
        ///
        /// Returns a list of directory entries with minimal metadata.
        ///
        /// Important: The returned file names (OsString) must not contain any slashes ('/').
        /// Including slashes in the file names will result in undefined behavior.
        fn readdir(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
        ) -> FuseResult<Vec<(OsString, TId::MinimalMetadata)>>;
    });
    result.push(parse_quote! {

        /// Read the target of a symbolic link
        fn readlink(&self, req: &RequestInfo, file_id: TId) -> FuseResult<Vec<u8>>;
    });
    result.push(parse_quote! {
        /// Release an open file
        ///
        /// Called when all file descriptors are closed and all memory mappings are unmapped.
        /// Guaranteed to be called once for every open() call.
        fn release(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: OwnedFileHandle,
            flags: OpenFlags,
            lock_owner: Option<u64>,
            flush: bool,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Release an open directory
        ///
        /// This method is called exactly once for every successful opendir operation.
        /// The file_handle parameter will contain the value set by the opendir method,
        /// or will be undefined if the opendir method didn't set any value.
        fn releasedir(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: OwnedFileHandle,
            flags: OpenFlags,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Remove an extended attribute.
        fn removexattr(&self, req: &RequestInfo, file_id: TId, name: &OsStr) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Rename a file or directory
        fn rename(
            &self,
            req: &RequestInfo,
            parent_id: TId,
            name: &OsStr,
            newparent: TId,
            newname: &OsStr,
            flags: RenameFlags,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Remove a directory
        fn rmdir(&self, req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Set file attributes.
        fn setattr(
            &self,
            req: &RequestInfo,
            file_id: TId,
            attrs: SetAttrRequest<'_>,
        ) -> FuseResult<FileAttribute>;
    });
    result.push(parse_quote! {
        /// Acquire, modify or release a POSIX file lock
        ///
        /// For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but
        /// otherwise this is not always the case. For checking lock ownership, 'fi->owner'
        /// must be used. The l_pid field in 'struct flock' should only be used to fill
        /// in this field in getlk().
        fn setlk(
            &self,
            req: &RequestInfo,
            file_id: TId,
            file_handle: BorrowedFileHandle<'_>,
            lock_owner: u64,
            lock_info: LockInfo,
            sleep: bool,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Set an extended attribute
        fn setxattr(
            &self,
            req: &RequestInfo,
            file_id: TId,
            name: &OsStr,
            value: Vec<u8>,
            flags: FUSESetXAttrFlags,
            position: u32,
        ) -> FuseResult<()>;
    });
    result.push(parse_quote! {
        /// Get file system statistics
        fn statfs(&self, req: &RequestInfo, file_id: TId) -> FuseResult<StatFs>;
    });
    result.push(parse_quote! {
        /// Create a symbolic link.
        fn symlink(
            &self,
            req: &RequestInfo,
            parent_id: TId,
            link_name: &OsStr,
            target: &Path,
        ) -> FuseResult<TId::Metadata>;
    });
    result.push(parse_quote! {
        /// Write data to a file
        ///
        /// Write should return exactly the number of bytes requested except on error. An exception to this is when the file has been opened in ‘direct_io’ mode, in which case the return value of the write system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value.
        ///
        /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set, the pid, uid, gid, and fh may not match the value that would have been sent if write cachin is disabled flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
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
        ) -> FuseResult<u32>;
    });
    result.push(parse_quote! {
        /// Remove a file
        fn unlink(&self, req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()>;
    });
    result
}

fn generate_readdir_plus(handler_type: HandlerType) -> proc_macro2::TokenStream {
    match handler_type {
        HandlerType::Async => quote! {
            /// Read directory contents with full file attributes
            ///
            /// Default implementation combines readdir and lookup operations.
            ///
            /// Important: The returned file names (OsString) must not contain any slashes ('/').
            /// Including slashes in the file names will result in undefined behavior.
            async fn readdirplus(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
            ) -> FuseResult<Vec<(OsString, TId::Metadata)>> {
                let readdir_result = self.readdir(req, file_id.clone(), file_handle).await?;
                let mut result = Vec::with_capacity(readdir_result.len());
                for (name, _) in readdir_result.into_iter() {
                    let metadata = self.lookup(req, file_id.clone(), &name).await?;
                    result.push((name, metadata));
                }
                Ok(result)
            }
        },
        _ => quote! {
            /// Read directory contents with full file attributes
            ///
            /// Default implementation combines readdir and lookup operations.
            ///
            /// Important: The returned file names (OsString) must not contain any slashes ('/').
            /// Including slashes in the file names will result in undefined behavior.
            fn readdirplus(
                &self,
                req: &RequestInfo,
                file_id: TId,
                file_handle: BorrowedFileHandle<'_>,
            ) -> FuseResult<Vec<(OsString, TId::Metadata)>> {
                let readdir_result = self.readdir(req, file_id.clone(), file_handle)?;
                let mut result = Vec::with_capacity(readdir_result.len());
                for (name, _) in readdir_result.into_iter() {
                    let metadata = self.lookup(req, file_id.clone(), &name)?;
                    result.push((name, metadata));
                }
                Ok(result)
            }
        },
    }
}

fn generate_function_impls(handler_type: HandlerType) -> Vec<proc_macro2::TokenStream> {
    let function_defs = get_function_defs();

    let function_impls = function_defs.into_iter().map(|func| {
        let func_name = &func.sig.ident;
        let args = &func.sig.inputs;
        let return_type = &func.sig.output;
        let attrs = &func.attrs;

        // Extract argument names without types and self
        let arg_names = args.iter().filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    Some(pat_ident.ident.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });

        match handler_type {
            HandlerType::Async => quote! {
                #(#attrs)*
                async fn #func_name(#args) #return_type {
                    self.get_inner().#func_name(#(#arg_names),*).await
                }
            },
            _ => quote! {
                #(#attrs)*
                fn #func_name(#args) #return_type {
                    self.get_inner().#func_name(#(#arg_names),*)
                }
            },
        }
    });

    function_impls.collect()
}

fn get_common_functions() -> proc_macro2::TokenStream {
    quote! {
        /// Delegate unprovided methods to another FuseHandler, enabling composition
        fn get_inner(&self) -> &dyn FuseHandler<TId>;

        /// Provide a default Time-To-Live for file metadata
        ///
        /// Can be overriden for each FileAttributes returned.
        fn get_default_ttl(&self) -> Duration {
            Duration::from_secs(1)
        }

        /// Initialize the filesystem and configure kernel connection
        fn init(&self, req: &RequestInfo, config: &mut KernelConfig) -> FuseResult<()> {
            self.get_inner().init(req, config)
        }

        /// Perform cleanup operations on filesystem exit
        fn destroy(&self) {
            self.get_inner().destroy();
        }
    }
}

fn get_dependencies() -> proc_macro2::TokenStream {
    quote! {
        use std::ffi::{OsStr, OsString};
        use std::path::Path;
        use std::time::Duration;
    }
}

pub(crate) fn generate_fuse_handler_trait(handler_type: HandlerType) -> proc_macro2::TokenStream {
    let dependencies = get_dependencies();
    let common_functions = get_common_functions();
    let function_impls = generate_function_impls(handler_type);
    let readdirplus_fn = generate_readdir_plus(handler_type);

    match handler_type {
        HandlerType::Async => quote! {
            use async_trait::async_trait;
            #dependencies

            #[async_trait]
            pub trait FuseHandler<TId: FileIdType>: 'static + Send + Sync {
                #common_functions
                #(#function_impls)*
                #readdirplus_fn
            }
        },
        HandlerType::Parallel => quote! {
            #dependencies

            pub trait FuseHandler<TId: FileIdType>: 'static + Send + Sync {
                #common_functions
                #(#function_impls)*
                #readdirplus_fn
            }
        },
        HandlerType::Serial => quote! {
            #dependencies

            pub trait FuseHandler<TId: FileIdType>: 'static {
                #common_functions
                #(#function_impls)*
                #readdirplus_fn
            }
        },
    }
}
