# A FUSE passthrough filesystem example using easy_fuser

This program creates a mirror filesystem using FUSE, allowing you to mount
a directory and access its contents through the mounted filesystem. It leverages
the MirrorFs template provided by easy_fuser.

The passthrough filesystem mirrors the contents of a source directory to a mount point,
providing transparent access to the original files and directories.

Usage:
    passthrough_fs <SOURCE_DIR> <MOUNT_POINT>
    passthrough_fs --source-dir <SOURCE_DIR> --mntpoint <MOUNT_POINT>

This example demonstrates how to use easy_fuser to create a simple yet functional
FUSE filesystem with minimal code, showcasing the power and simplicity of the library.