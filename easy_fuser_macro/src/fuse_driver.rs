use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{format_ident, quote};

use crate::handler_type::HandlerType;

fn wrap_handler_execution(handler_type: HandlerType, block: TokenStream) -> TokenStream {
    match handler_type {
        HandlerType::Async => quote! {
            self.runtime.spawn(async move {
                #block
            });
        },
        HandlerType::Serial => block,
        HandlerType::Parallel => quote! {
            self.threadpool.execute(move || {
                #block
            });
        },
    }
}

fn expand_macro_placeholders(handler_type: HandlerType, input: TokenStream) -> TokenStream {
    let mut tokens = input.into_iter();
    let mut output = TokenStream::new();
    let mut function_name = String::new();
    let mut arg_names = Vec::new();

    // Extract function name
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Ident(ident) if ident == "fn" => {
                if let Some(TokenTree::Ident(name)) = tokens.next() {
                    function_name = name.to_string();
                    output.extend(vec![TokenTree::Ident(ident), TokenTree::Ident(name)]);

                    // Extract argument names
                    if let Some(TokenTree::Group(group)) = tokens.next() {
                        let args = group.stream();
                        let mut current_arg = String::new();
                        for arg in args.into_iter() {
                            match arg {
                                TokenTree::Ident(ident) => {
                                    if ident != "mut" {
                                        current_arg = ident.to_string();
                                    }
                                }
                                TokenTree::Punct(p) if p.as_char() == ':' => {
                                    if !current_arg.is_empty() && current_arg != "self" {
                                        arg_names.push(current_arg.clone());
                                    }
                                    current_arg.clear();
                                }
                                TokenTree::Punct(p) if p.as_char() == ',' => {
                                    current_arg.clear();
                                }
                                _ => {}
                            }
                        }
                        output.extend(std::iter::once(TokenTree::Group(group)));
                    }
                    break;
                }
            }
            _ => output.extend(std::iter::once(token)),
        }
    }

    if arg_names[0] != "req" {
        panic!(
            "Invalid function signature: expected 'req', found '{}'",
            arg_names[0]
        );
    }
    arg_names[0] = String::from("&req");
    arg_names.pop(); // remove reply
    let req_arg = quote!(&req);
    let arg_idents: Vec<_> = arg_names
        .iter()
        .skip(1) // Skip the first 'req' argument
        .map(|arg| format_ident!("{}", arg))
        .collect();
    let all_args = quote! {
        #req_arg, #(#arg_idents),*
    };

    // Expand the rest of the tokens
    output.extend(expand_macro_tokens(
        handler_type,
        &function_name,
        &all_args,
        false,
        tokens,
    ));

    output
}

fn expand_macro_tokens(
    handler_type: HandlerType,
    function_name: &str,
    args: &TokenStream,
    mut log_ino: bool,
    mut tokens: impl Iterator<Item = TokenTree>,
) -> TokenStream {
    let mut output = TokenStream::new();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '$' => {
                if let Some(TokenTree::Ident(ident)) = tokens.next() {
                    let key = ident.to_string();
                    let replacement = match key.as_str() {
                        "req" => quote!(let req = RequestInfo::from(req);),
                        "handler" => match handler_type {
                            HandlerType::Serial => quote!(let handler = &self.handler;),
                            _ => quote!(let handler = Arc::clone(&self.handler);),
                        },
                        "resolver" => match handler_type {
                            HandlerType::Serial => quote!(let resolver = &self.resolver;),
                            _ => quote!(let resolver = Arc::clone(&self.resolver);),
                        },
                        "ino" => {
                            log_ino = true;
                            quote!(
                                let log_ino = ino;
                                let ino = resolver.resolve_id(ino);
                            )
                        }
                        "parent" => {
                            log_ino = true;
                            quote!(
                                let log_ino = parent;
                                let parent = resolver.resolve_id(parent);
                            )
                        }
                        "fh" => quote!(let fh = unsafe { BorrowedFileHandle::from_raw(fh) };),
                        "wrap" => {
                            if let Some(TokenTree::Group(group)) = tokens.next() {
                                wrap_handler_execution(
                                    handler_type,
                                    expand_macro_tokens(
                                        handler_type,
                                        function_name,
                                        args,
                                        log_ino,
                                        group.stream().into_iter(),
                                    ),
                                )
                            } else {
                                panic!("Expected group after $wrap")
                            }
                        }
                        "args" => args.clone(),
                        "reply_attr" => reply_attr(),
                        "reply_entry" => reply_entry(),
                        "warn_error" => error_response(function_name, false, log_ino),
                        "info_error" => error_response(function_name, true, log_ino),
                        unknown => panic!("Unknown dollar identifier: {}", unknown),
                    };
                    output.extend(replacement);
                } else {
                    panic!("Expected identifier after $");
                }
            }
            TokenTree::Group(group) => {
                let content = expand_macro_tokens(
                    handler_type,
                    function_name,
                    args,
                    log_ino,
                    group.stream().into_iter(),
                );
                output.extend(std::iter::once(TokenTree::Group(Group::new(
                    group.delimiter(),
                    content,
                ))));
            }
            _ => output.extend(std::iter::once(token)),
        }
    }

    output
}

fn error_response(function_name: &str, is_info: bool, log_ino: bool) -> TokenStream {
    let error_type = if is_info { quote!(info) } else { quote!(warn) };
    match log_ino {
        true => quote! {
            Err(e) => {
                #error_type!(concat!(#function_name, ": ino {:x?}, [{}], {:?}"), log_ino, e, req);
                reply.error(e.raw_error());
                return;
            }
        },
        false => quote! {
            Err(e) => {
                #error_type!(concat!(#function_name, ": [{}], {:?}"), e, req);
                reply.error(e.raw_error());
                return;
            }
        },
    }
}

fn reply_attr() -> TokenStream {
    quote! {
        Ok(file_attr) => {
            let default_ttl = handler.get_default_ttl();
            let (fuse_attr, ttl, _) = file_attr.to_fuse(ino);
            reply.attr(&ttl.unwrap_or(default_ttl), &fuse_attr);
        },
    }
}

fn reply_entry() -> TokenStream {
    quote! {
        Ok(metadata) => {
            let default_ttl = handler.get_default_ttl();
            let (id, file_attr) = TId::extract_metadata(metadata);
            let ino = resolver.lookup(parent, name, id, true);
            let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
            reply.entry(
                &ttl.unwrap_or(default_ttl),
                &fuse_attr,
                generation.unwrap_or(get_random_generation()),
            );
        }
    }
}

fn readdir_impl(handler_type: HandlerType, is_extended_readdir: bool) -> TokenStream {
    let fn_name = match is_extended_readdir {
        true => quote!(readdirplus),
        false => quote!(readdir),
    };
    let fn_signature = match is_extended_readdir {
        true => quote!(fn #fn_name(
            &mut self,
            req: &Request,
            ino: u64,
            fh: u64,
            offset: i64,
            mut reply: ReplyDirectory,
        )),
        false => quote!(fn #fn_name(
            &mut self,
            req: &Request,
            ino: u64,
            fh: u64,
            offset: i64,
            mut reply: ReplyDirectoryPlus,
        )),
    };

    let extract_metadata = match is_extended_readdir {
        true => quote!(TId::extract_metadata(item.1)),
        false => quote!(TId::extract_minimal_metadata(item.1)),
    };

    let dirmap_entries_get = match (handler_type, is_extended_readdir) {
        (HandlerType::Serial, false) => quote!(&self.dirmap_entries),
        (HandlerType::Serial, true) => quote!(&self.dirmapplus_entries),
        (_, false) => quote!(Arc::clone(&self.dirmap_entries)),
        (_, true) => quote!(Arc::clone(&self.dirmapplus_entries)),
    };

    let dirmap_entries_borrow_mut = match handler_type {
        HandlerType::Serial => quote!(dirmap_entries.borrow_mut()),
        _ => quote!(dirmap_entries.lock().unwrap()),
    };

    let reply_add = match is_extended_readdir {
        true => quote! {
            // readdirplus: Add entries with extended attributes
            let default_ttl = handler.get_default_ttl();
            while let Some((name, ino, file_attr)) = directory_entries.pop_front() {
                let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                if reply.add(
                    ino,
                    new_offset,
                    name,
                    &ttl.unwrap_or(default_ttl),
                    &fuse_attr,
                    generation.unwrap_or(get_random_generation()),
                ) {
                    #dirmap_entries_borrow_mut
                        .insert((ino, new_offset), directory_entries);
                    break;
                }
                new_offset += 1;
            }
            reply.ok();
        },
        false => quote! {
            // readdir: Add entries until buffer is full
            while let Some((name, ino, kind)) = directory_entries.pop_front() {
                if reply.add(ino, new_offset, kind, &name) {
                    #dirmap_entries_borrow_mut
                        .insert((ino, new_offset), directory_entries);
                    break;
                }
                new_offset += 1;
            }
            reply.ok();
        },
    };

    expand_macro_placeholders(
        handler_type,
        quote! {
            #fn_signature {
                $req
                $handler
                $resolver
                let dirmap_entries = #dirmap_entries_get;
                $wrap {
                    $ino
                    $fh

                    // Validate offset
                    if offset < 0 {
                        error!("readdir called with a negative offset");
                        reply.error(ErrorKind::InvalidArgument.into());
                        return;
                    }

                    // ### Initialize directory deque
                    let mut directory_entries = match offset {
                        // First read: fetch children from handler
                        0 => match handler.#fn_name($args) {
                            Ok(children) => {
                                // Unpack and process children
                                let (child_list, attr_list): (Vec<_>, Vec<_>) = children
                                    .into_iter()
                                    .map(|item| {
                                        let (child_id, child_attr) = #extract_metadata;
                                        ((item.0, child_id), child_attr)
                                    })
                                    .unzip();

                                // Add children to resolver and create iterator
                                resolver
                                    .add_children(
                                        ino,
                                        child_list,
                                        #is_extended_readdir,
                                    )
                                    .into_iter()
                                    .zip(attr_list.into_iter())
                                    .map(|((file_name, file_ino), file_attr)| {
                                        (file_name, file_ino, file_attr)
                                    })
                                    .collect()
                            }
                            $warn_error
                        },
                        // Subsequent reads: retrieve saved iterator
                        _ => match { #dirmap_entries_borrow_mut.remove(&(ino, offset)) } {
                            Some(directory_entries) => directory_entries,
                            None => {
                                // Case when fuse tries to read again after the final item
                                reply.ok();
                                return;
                            }
                        },
                    };

                    let mut new_offset = offset + 1;

                    #reply_add
                }
            }
        },
    )
}

fn generate_fuse_operation_handlers(handler_type: HandlerType) -> Vec<TokenStream> {
    let mut result = Vec::new();
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn access(&mut self, req: &Request, ino: u64, mask: i32, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    let mask = AccessMask::from_bits_retain(mask);
                    match handler.access($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
            handler_type,
            quote! {
                fn bmap(&mut self, req: &Request<'_>, ino: u64, blocksize: u32, idx: u64, reply: ReplyBmap) {
                    $req
                    $handler
                    $resolver
                    $wrap {
                        $ino
                        match handler.bmap($args) {
                            Ok(block) => reply.bmap(block),
                            $warn_error
                        }
                    }
                }
            },
        ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                $wrap {
                    let ino_in = resolver.resolve_id(ino_in);
                    let fh_in = unsafe { BorrowedFileHandle::from_raw(fh_in) };
                    let ino_out = resolver.resolve_id(ino_out);
                    let fh_out = unsafe { BorrowedFileHandle::from_raw(fh_out) };
                    match handler.copy_file_range($args) {
                        Ok(bytes_written) => reply.written(bytes_written),
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $parent
                    let name = name.as_ref();
                    let flags = OpenFlags::from_bits_retain(flags);
                    match handler.create($args) {
                        Ok((file_handle, metadata, response_flags)) => {
                            let default_ttl = handler.get_default_ttl();
                            let (id, file_attr) = TId::extract_metadata(metadata);
                            let ino = resolver.lookup(parent, &name, id, true);
                            let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                            reply.created(
                                &ttl.unwrap_or(default_ttl),
                                &fuse_attr,
                                generation.unwrap_or(get_random_generation()),
                                file_handle.as_raw(),
                                response_flags.bits(),
                            );
                        },
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    let mode = FallocateFlags::from_bits_retain(mode);
                    match handler.fallocate($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote!{
            fn flush(&mut self, req: &Request, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    match handler.flush($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn forget(&mut self, req: &Request, ino: u64, nlookup: u64) {
                $req
                self.handler.forget(&req, ino, nlookup);
                self.resolver.forget(ino, nlookup);
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote!{
            fn fsync(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    match handler.fsync($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote!{
            fn fsyncdir(&mut self, req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    match handler.fsyncdir($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn getattr(&mut self, req: &Request, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    fh.map(|fh| unsafe { BorrowedFileHandle::from_raw(fh) });
                    match handler.getattr($args) {
                        $reply_attr
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                    $req
                    $handler
                    $resolver
                    $wrap {
                        $ino
                        $fh
                        let lock_info = LockInfo {
                            start,
                            end,
                            lock_type: LockType::from_bits_retain(typ),
                            pid,
                        };
                        match handler.getlk(&req, ino, fh, lock_owner, lock_info) {
                            Ok(lock) => reply.locked(lock),
                            $warn_error
                        }
                    }
                }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn getxattr(&mut self, req: &Request, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr) {
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $ino
                    let name = name.as_ref();
                    match handler.getxattr($args) {
                        Ok(xattr_data) => {
                            if size == 0 {
                                reply.size(xattr_data.len() as u32);
                            } else if size >= xattr_data.len() as u32 {
                                reply.data(&xattr_data);
                            } else {
                                reply.error(ErrorKind::ResultTooLarge.into());
                            }
                        }
                        $warn_error
                    };
                };
            }
        }
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                let in_data = in_data.to_owned();
                $wrap {
                    $ino
                    $fh
                    let flags = IOCtlFlags::from_bits_retain(flags);
                    match handler.ioctl($args) {
                        Ok((result, data)) => reply.ioctl(result, &data),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn link(
                &mut self,
                req: &Request,
                ino: u64,
                newparent: u64,
                newname: &OsStr,
                reply: ReplyEntry,
            ) {
                $req
                $handler
                $resolver
                let newname = newname.to_owned();
                $wrap {
                    $ino
                    let newname = newname.as_ref();
                    let newparent = resolver.resolve_id(newparent);
                    match handler.link($args) {
                        $reply_entry
                        $warn_error
                    }
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn listxattr(&mut self, req: &Request, ino: u64, size: u32, reply: ReplyXattr) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    match handler.listxattr($args) {
                        Ok(xattr_data) => {
                            if size == 0 {
                                reply.size(xattr_data.len() as u32);
                            } else if size >= xattr_data.len() as u32 {
                                reply.data(&xattr_data);
                            } else {
                                reply.error(ErrorKind::ResultTooLarge.into());
                            }
                        }
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $parent
                    match handler.lookup($args) {
                        $reply_entry
                        // Lookup is preemptivly done in normal situations, we don't need to log an error
                        // eg: before creating a file
                        $info_error
                    }
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn lseek(
                &mut self,
                req: &Request,
                ino: u64,
                fh: u64,
                offset: i64,
                whence: i32,
                reply: ReplyLseek,
            ) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    let seek = seek_from_raw(Some(whence), offset);
                    match handler.lseek(
                        &req,
                        ino,
                        fh,
                        seek,
                    ) {
                        Ok(new_offset) => reply.offset(new_offset),
                        $warn_error
                    };
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn mkdir(
                &mut self,
                req: &Request,
                parent: u64,
                name: &OsStr,
                mode: u32,
                umask: u32,
                reply: ReplyEntry,
            ) {
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $parent
                    let name = name.as_ref();
                    match handler.mkdir($args) {
                        $reply_entry
                        $warn_error
                    }
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $parent
                    let name = name.as_ref();
                    let rdev = DeviceType::from_rdev(rdev.try_into().unwrap());
                    match handler.mknod($args) {
                        $reply_entry
                        $warn_error
                    }
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn open(&mut self, req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    let flags = OpenFlags::from_bits_retain(flags);
                    match handler.open($args) {
                        Ok((file_handle, response_flags)) => {
                            reply.opened(file_handle.as_raw(), response_flags.bits())
                        }
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn opendir(&mut self, req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    let flags = OpenFlags::from_bits_retain(flags);
                    match handler.opendir($args) {
                        Ok((file_handle, response_flags)) => {
                            reply.opened(file_handle.as_raw(), response_flags.bits())
                        }
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    let seek = seek_from_raw(Some(offset), 0);
                    let flags = FUSEOpenFlags::from_bits_retain(flags);
                    match handler.read($args) {
                        Ok(data_reply) => reply.data(&data_reply),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(readdir_impl(handler_type, false));
    result.push(readdir_impl(handler_type, true));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn readlink(&mut self, req: &Request, ino: u64, reply: ReplyData) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    match handler.readlink($args) {
                        Ok(link) => reply.data(&link),
                        warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn release(
                &mut self,
                req: &Request,
                ino: u64,
                fh: u64,
                flags: i32,
                _lock_owner: Option<u64>,
                _flush: bool,
                reply: ReplyEmpty,
            ) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    let flags = OpenFlags::from_bits_retain(flags);
                    match handler.release($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(handler_type, quote! {
        fn releasedir(&mut self, req: &Request, ino: u64, fh: u64, flags: i32, reply: ReplyEmpty) {
            $req
            $handler
            $resolver
            $wrap {
                $ino
                $fh
                let flags = OpenFlags::from_bits_retain(flags);
                match handler.releasedir($args) {
                    Ok(()) => reply.ok(),
                    $warn_error
                };
            };
        }
    }));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn removexattr(&mut self, req: &Request, ino: u64, name: &OsStr, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $ino
                    let name = name.as_ref();
                    match handler.removexattr($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                let name = name.to_owned();
                let newname = newname.to_owned();
                $wrap {
                    $parent
                    let name = name.as_ref();
                    let newname = newname.as_ref();
                    let flags = RenameFlags::from_bits_retain(flags);
                    match handler.rename($args) {
                        Ok(()) => {
                            resolver.rename(parent, &name, newparent, &newname);
                            reply.ok()
                        }
                        $warn_error
                    }
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn rmdir(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $parent
                    let name = name.as_ref();
                    match handler.rmdir($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                $wrap {
                    $ino
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
                        file_handle: fh.map(|fh| unsafe { BorrowedFileHandle::from_raw(fh) }),
                    };
                    match handler.setattr(&req, ino, attrs) {
                        $reply_attr
                        $warn_error
                    }
                }
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    $fh
                    let lock_info = LockInfo {
                        start,
                        end,
                        lock_type: LockType::from_bits_retain(typ),
                        pid,
                    };
                    match handler.setlk(
                        &req,
                        ino,
                        fh,
                        lock_owner,
                        lock_info,
                        sleep,
                    ) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                let name = name.to_owned();
                let value = value.to_owned();
                $wrap {
                    let name = name.as_ref();
                    let flags = FUSESetXAttrFlags::from_bits_retain(flags);
                    match handler.setxattr($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn statfs(&mut self, req: &Request, ino: u64, reply: ReplyStatfs) {
                $req
                $handler
                $resolver
                $wrap {
                    $ino
                    match handler.statfs($args) {
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
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn symlink(
                &mut self,
                req: &Request,
                parent: u64,
                link_name: &OsStr,
                target: &Path,
                reply: ReplyEntry,
            ) {
                $req
                $handler
                $resolver
                let link_name = link_name.to_owned();
                let target = target.to_owned();
                $wrap {
                    let link_name = link_name.as_ref();
                    let target = target.as_ref();
                    match handler.symlink($args) {
                        $reply_entry
                        $warn_error
                    }
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
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
                $req
                $handler
                $resolver
                let data = data.to_owned();
                $wrap {
                    $ino
                    $fh
                    let seek = seek_from_raw(Some(offset), 0);
                    let write_flags = FUSEWriteFlags::from_bits_retain(write_flags);
                    let flags = OpenFlags::from_bits_retain(flags);
                    match handler.write($args) {
                        Ok(bytes_written) => reply.written(bytes_written),
                        $warn_error
                    };
                };
            }
        },
    ));
    result.push(expand_macro_placeholders(
        handler_type,
        quote! {
            fn unlink(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
                $req
                $handler
                $resolver
                let name = name.to_owned();
                $wrap {
                    $parent
                    let name = name.as_ref();
                    match handler.unlink($args) {
                        Ok(()) => reply.ok(),
                        $warn_error
                    };
                };
            }
        },
    ));
    result
}

fn generate_fuse_driver_struct(handler_type: HandlerType) -> TokenStream {
    match handler_type {
        HandlerType::Serial => quote! {
            use std::cell::RefCell;

            pub(crate) struct FuseDriver<TId, THandler>
            where
                TId: FileIdType,
                THandler: FuseHandler<TId>,
            {
                handler: THandler,
                resolver: TId::Resolver,
                dirmap_entries: RefCell<DirMapEntries<FileKind>>,
                dirmapplus_entries: RefCell<DirMapEntries<FileAttribute>>,
            }

            impl<TId, THandler> FuseDriver<TId, THandler>
            where
                TId: FileIdType,
                THandler: FuseHandler<TId>,
            {
                /// num_thread is ignored in serial mode, it is kept for consistency with other modes
                pub fn new(handler: THandler, _num_threads: usize) -> FuseDriver<TId, THandler> {
                    FuseDriver {
                        handler,
                        resolver: TId::Resolver::new(),
                        dirmap_entries: RefCell::new(HashMap::new()),
                        dirmapplus_entries: RefCell::new(HashMap::new()),
                    }
                }
            }
        },
        HandlerType::Parallel => quote! {
            use std::sync::{Arc, Mutex};
            use threadpool::ThreadPool;

            pub(crate) struct FuseDriver<TId, THandler>
            where
                TId: FileIdType,
                THandler: FuseHandler<TId>,
            {
                handler: Arc<THandler>,
                resolver: Arc<TId::Resolver>,
                dirmap_entries: Arc<Mutex<DirMapEntries<FileKind>>>,
                dirmapplus_entries: Arc<Mutex<DirMapEntries<FileAttribute>>>,
                pub threadpool: ThreadPool,
            }

            impl<TId, THandler> FuseDriver<TId, THandler>
            where
                TId: FileIdType,
                THandler: FuseHandler<TId>,
            {
                pub fn new(handler: THandler, num_threads: usize) -> FuseDriver<TId, THandler> {
                    FuseDriver {
                        handler: Arc::new(handler),
                        resolver: Arc::new(TId::create_resolver()),
                        dirmap_entries: Arc::new(Mutex::new(HashMap::new())),
                        dirmapplus_entries: Arc::new(Mutex::new(HashMap::new())),
                        threadpool: ThreadPool::new(num_threads),
                    }
                }
            }
        },
        HandlerType::Async => quote! {
            use std::sync::{Arc, Mutex};
            use tokio::runtime::Runtime;

            pub(crate) struct FuseDriver<TId, THandler>
            where
                TId: FileIdType,
                THandler: FuseHandler<TId>,
            {
                handler: Arc<THandler>,
                resolver: Arc<TId::Resolver>,
                dirmap_entries: Arc<Mutex<DirMapEntries<FileKind>>>,
                dirmapplus_entries: Arc<Mutex<DirMapEntries<FileAttribute>>>,
                pub runtime: Runtime,
            }

            impl<TId, THandler> FuseDriver<TId, THandler>
            where
                TId: FileIdType,
                THandler: FuseHandler<TId>,
            {
                pub fn new(handler: THandler, _num_threads: usize) -> FuseDriver<TId, THandler> {
                    FuseDriver {
                        handler: Arc::new(handler),
                        resolver: Arc::new(TId::create_resolver()),
                        dirmap_entries Arc::new(Mutex::new(HashMap::new())),
                        dirmapplus_entries: Arc::new(Mutex::new(HashMap::new())),
                        runtime: Runtime::new().unwrap(),
                    }
                }
            }
        },
    }
}

fn get_dependencies() -> proc_macro2::TokenStream {
    quote! {
        use std::{
            collections::{HashMap, VecDeque},
            ffi::{OsStr, OsString},
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
    }
}

pub(crate) fn generate_fuse_driver_implementation(handler_type: HandlerType) -> TokenStream {
    let dependencies = get_dependencies();
    let fuse_driver_struct = generate_fuse_driver_struct(handler_type);
    let fn_impls = generate_fuse_operation_handlers(handler_type);
    quote! {
        #dependencies

        type DirMapEntries<TAttr> = HashMap<(u64, i64), VecDeque<(OsString, u64, TAttr)>>;

        fn get_random_generation() -> u64 {
            Instant::now().elapsed().as_nanos() as u64
        }

        #fuse_driver_struct

        impl<TId, THandler> FuseDriver<TId, THandler>
        where
            TId: FileIdType,
            THandler: FuseHandler<TId>,
        {
            #(#fn_impls)*
        }
    }
}
