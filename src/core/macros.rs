
macro_rules! handle_fuse_reply_entry {
    ($handler:expr, $resolver:expr, $req:expr, $parent:expr, $name:expr, $reply:expr,
    $function:ident, ($($args:expr),*)) => {
        let handler = $handler;
        match handler.$function($($args),*) {
            Ok(metadata) => {
                let default_ttl = handler.get_default_ttl();
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
                warn!("{} {:?} - {:?}", stringify!($function), e, $req);
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
                warn!("{} {:?} - {:?}", stringify!($function), e, $req);
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
    $handler_method:ident, $unpack_method:ident, $get_iter_method:ident, $reply_type:ty) => {{
        // Inner macro to handle readdir vs readdirplus differences
        macro_rules! if_readdir {
            (readdir, $choice1:tt, $choice2:tt) => { $choice1 };
            (readdirplus, $choice1:tt, $choice2:tt) => { $choice2 };
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
                0 => match handler.$handler_method(&req_info, resolver.resolve_id($ino), FileHandle::from($fh)) {
                    Ok(children) => {
                        // Unpack and process children
                        let (child_list, attr_list): (Vec<_>, Vec<_>) = children
                            .into_iter()
                            .map(|item| {
                                let (child_id, child_attr) = $unpack_method::<T>(item.1);
                                ((item.0, child_id), child_attr)
                            })
                            .unzip();

                        // Add children to resolver and create iterator
                        resolver
                            .add_children($ino, child_list, if_readdir!($handler_method, false, true))
                            .into_iter()
                            .zip(attr_list.into_iter())
                            .map(|((file_name, file_ino), file_attr)| (file_name, file_ino, file_attr))
                            .collect()
                    }
                    Err(e) => {
                        warn!("readdir {:?}: {:?}", req_info, e);
                        $reply.error(e.raw_error());
                        return;
                    }
                },
                // Subsequent reads: retrieve saved iterator
                _ => match dirmap_iter.safe_borrow_mut().remove(&($ino, $offset)) {
                    Some(dirmap_iter) => dirmap_iter,
                    None => {
                        error!("readdir called with an unknown offset");
                        $reply.error(ErrorKind::InvalidArgument.into());
                        return;
                    }
                },
            };

            let mut new_offset = $offset + 1;

            // ### Process directory entries
            if_readdir!($handler_method, {
                // readdir: Add entries until buffer is full
                while let Some((name, ino, kind)) = dir_iter.pop_front() {
                    if $reply.add(ino, new_offset, kind, &name) {
                        dirmap_iter.safe_borrow_mut().insert(($ino, new_offset), dir_iter);
                        break;
                    }
                    new_offset += 1;
                }
                $reply.ok();
            }, {
                // readdirplus: Add entries with extended attributes
                let default_ttl = handler.get_default_ttl();
                while let Some((name, ino, file_attr)) = dir_iter.pop_front() {
                    let (fuse_attr, ttl, generation) = file_attr.to_fuse(ino);
                    if $reply.add(
                        ino,
                        new_offset,
                        name,
                        &ttl.unwrap_or(default_ttl),
                        &fuse_attr,
                        generation.unwrap_or(get_random_generation())
                    ) {
                        dirmap_iter.safe_borrow_mut().insert((ino, new_offset), dir_iter);
                        break;
                    }
                    new_offset += 1;
                }
                $reply.ok();
            });
        });
    }};
}

pub(super) use handle_fuse_reply_entry;
pub(super) use handle_fuse_reply_attr;
pub(super) use handle_dir_read;