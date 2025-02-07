Spawns a FUSE filesystem in the background at the specified mountpoint.

This function mounts a FUSE filesystem and returns a `BackgroundSession` that can be used
to manage the mounted filesystem.

# Parameters

* `filesystem`: The filesystem implementation that handles FUSE operations.
* `mountpoint`: The path where the filesystem should be mounted.
* `options`: A slice of mount options for configuring the filesystem mount.
* `num_threads` (non serial argument): Number of threads for handling filesystem operations concurrently.

# Type Parameters

* `T`: Implements `FileIdType` for file identifier conversion.
* `FS`: Implements `FuseHandler<T>` for filesystem operations.
  FS must implement the `Send`, which is not the case by defualt in serial mode.
  In that case, it is advised to create the filesystem in the same dedicated thread and use mount function.

# Unmounting
Using spawn_mount, the FUSE filesystem can be unmounted using two methods:  
1. **Programmatically**: By calling the `join` method on the `BackgroundSession` returned during mounting. This will stop the filesystem and unmount it.  
2. **Manually**: Using the `fusermount -u` command, executed externally. Note that `fusermount` will fail if the filesystem is busy.

If the program crashes, the mount point may be left in an inconsistent state. To resolve this, you must manually unmount the filesystem using `fusermount -u`.

# Returns

Returns `io::Result<BackgroundSession>`, which is:
* `Ok(BackgroundSession)` on successful mount, providing a handle to manage the mounted filesystem.
* `Err(io::Error)` if the mount operation fails.