Mounts a FUSE filesystem at the specified mountpoint.

# Parameters

* `filesystem`: The filesystem implementation.
* `mountpoint`: The path where the filesystem should be mounted.
* `options`: Mount options for the filesystem.
* `num_threads` (not available in serial mode): Number of threads for handling filesystem operations concurrently.

# Type Parameters

* `T`: Implements `FileIdType` for file identifier conversion.
* `FS`: Implements `FuseHandler<T>` for filesystem operations.

# Unmounting
The FUSE filesystem can only be unmounted using the `fusermount -u` command, executed externally from the program. However, the `fusermount` command will fail if the filesystem is busy.

If the program crashes, the mount point may be left in an inconsistent state. To resolve this, you will need to run `fusermount -u` to restore the mount point to a proper state.

# Returns

`io::Result<()>` indicating success or failure of the mount operation.