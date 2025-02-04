macro_rules! handle_fuse_reply_entry {
    ($handler:expr, $resolver:expr, $req:expr, $parent:expr, $name:expr, $reply:expr,
    $function:ident, ($($args:expr),*)) => {
        macro_rules! if_lookup {
            (lookup, $choice1:tt, $choice2:tt) => {
                $choice1
            };
            ($any:tt, $choice1:tt, $choice2:tt) => {
                $choice2
            };
        }

        let handler = $handler;
        match handler.$function($($args),*) {
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
                let (id, file_attr) = TId::extract_metadata(metadata);
                let ino = $resolver.lookup($parent, $name, id, true);
                let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                $reply.entry(
                    &ttl.unwrap_or(default_ttl),
                    &fuse_attr,
                    generation.unwrap_or(get_random_generation()),
                );
            }
            Err(e) => {
                if_lookup!($function, {
                    if e.kind() == ErrorKind::FileNotFound {
                        // Lookup is preemptivly done in normal situations, we don't need to log an error
                        // eg: before creating a file
                        info!("{}: parent_ino {:x?}, [{}], {:?}", stringify!($function), $parent, e, $req);
                    } else {
                        warn!("{}: parent_ino {:x?}, [{}], {:?}", stringify!($function), $parent, e, $req);
                    };
                }, {
                    warn!("{}: parent_ino {:x?}, [{}], {:?}", stringify!($function), $parent, e, $req);
                });
                $reply.error(e.raw_error())
            }
        }
    };
}

macro_rules! handle_fuse_reply_attr {
    ($handler:expr, $resolve:expr, $req:expr, $ino:expr, $reply:expr,
        $function:ident, ($($args:expr),*)) => {
        match $handler.$function($($args),*) {
            Ok(file_attr) => {
                let default_ttl = $handler.get_default_ttl();
                let (fuse_attr, ttl, _) = file_attr.to_fuse($ino);
                $reply.attr(&ttl.unwrap_or(default_ttl), &fuse_attr);
            }
            Err(e) => {
                warn!("{}: ino {:x?}, [{}], {:?}", stringify!($function), $ino, e, $req);
                $reply.error(e.raw_error())
            }
        }
    };
}

/// Handles directory read operations for FUSE filesystem.//+
/////+
/// This macro implements the logic for reading directory contents, supporting both//+
/// regular directory reads (`readdir`) and extended directory reads (`readdirplus`).//+
/////+
/// # Parameters//+
/////+
/// * `$self`: The current filesystem instance.//+
/// * `$req`: The FUSE request object.//+
/// * `$ino`: The inode number of the directory being read.//+
/// * `$fh`: The file handle of the open directory.//+
/// * `$offset`: The offset from which to start reading directory entries.//+
/// * `$reply`: The FUSE reply object to send the response.//+
/// * `$handler_method`: The method to call on the handler to retrieve directory entries.//+
/// * `$unpack_method`: The method to unpack metadata for each directory entry.//+
/// * `$get_iter_method`: The method to retrieve the directory iterator.//+
/// * `$reply_type`: The type of reply (readdir or readdirplus).//+
/////+
/// # Returns//+
/////+
/// This macro doesn't return a value directly, but it populates the `$reply` object//+
/// with directory entries or an error code.//
macro_rules! handle_dir_read {
    ($self:expr, $req:expr, $ino:expr, $fh:expr, $offset:expr, $reply:expr,
    $handler_method:ident, $get_iter_method:ident, $reply_type:ty) => {{
        // Inner macro to handle readdir vs readdirplus differences
        macro_rules! if_readdir {
            (readdir, $choice1:tt, $choice2:tt) => {
                $choice1
            };
            (readdirplus, $choice1:tt, $choice2:tt) => {
                $choice2
            };
        }

        let req_info = RequestInfo::from($req);
        let handler = $self.get_handler();
        let resolver = $self.get_resolver();
        let dirmap_iter = $self.$get_iter_method();

        execute_task!($self, {
            // Validate offset
            if $offset < 0 {
                error!("readdir called with a negative offset");
                $reply.error(ErrorKind::InvalidArgument.into());
                return;
            }

            // ### Initialize directory iterator
            let mut dir_iter = match $offset {
                // First read: fetch children from handler
                0 => match handler.$handler_method(&req_info, resolver.resolve_id($ino), unsafe {
                    BorrowedFileHandle::from_raw($fh)
                }) {
                    Ok(children) => {
                        // Unpack and process children
                        let (child_list, attr_list): (Vec<_>, Vec<_>) = children
                            .into_iter()
                            .map(|item| {
                                let (child_id, child_attr) = if_readdir!(
                                    $handler_method,
                                    { TId::extract_minimal_metadata(item.1) },
                                    { TId::extract_metadata(item.1) }
                                );
                                ((item.0, child_id), child_attr)
                            })
                            .unzip();

                        // Add children to resolver and create iterator
                        resolver
                            .add_children(
                                $ino,
                                child_list,
                                if_readdir!($handler_method, false, true),
                            )
                            .into_iter()
                            .zip(attr_list.into_iter())
                            .map(|((file_name, file_ino), file_attr)| {
                                (file_name, file_ino, file_attr)
                            })
                            .collect()
                    }
                    Err(e) => {
                        warn!("readdir {:?}: {:?}", req_info, e);
                        $reply.error(e.raw_error());
                        return;
                    }
                },
                // Subsequent reads: retrieve saved iterator
                _ => match { dirmap_iter.safe_borrow_mut().remove(&($ino, $offset)) } {
                    Some(dirmap_iter) => dirmap_iter,
                    None => {
                        // Case when fuse tries to read again after the final item
                        $reply.ok();
                        return;
                    }
                },
            };

            let mut new_offset = $offset;

            // ### Process directory entries
            if_readdir!(
                $handler_method,
                {
                    // readdir: Add entries until buffer is full
                    while let Some((name, ino, kind)) = dir_iter.pop_front() {
                        if $reply.add(ino, new_offset, kind, &name) {
                            dir_iter.push_front((name, ino, kind));
                            dirmap_iter
                                .safe_borrow_mut()
                                .insert(($ino, new_offset - 1), dir_iter);
                            break;
                        }
                        new_offset += 1;
                    }
                    $reply.ok();
                },
                {
                    // readdirplus: Add entries with extended attributes
                    let default_ttl = handler.get_default_ttl();
                    while let Some((name, ino, file_attr)) = dir_iter.pop_front() {
                        let (fuse_attr, ttl, generation) = file_attr.clone().to_fuse(ino);
                        if $reply.add(
                            ino,
                            new_offset,
                            &name,
                            &ttl.unwrap_or(default_ttl),
                            &fuse_attr,
                            generation.unwrap_or(get_random_generation()),
                        ) {
                            dir_iter.push_front((name, ino, file_attr.clone()));
                            dirmap_iter
                                .safe_borrow_mut()
                                .insert((ino, new_offset - 1), dir_iter);
                            break;
                        }
                        new_offset += 1;
                    }
                    $reply.ok();
                }
            );
        });
    }};
}

pub(super) use handle_dir_read;
pub(super) use handle_fuse_reply_attr;
pub(super) use handle_fuse_reply_entry;
