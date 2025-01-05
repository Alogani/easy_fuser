# ZipFs: A read-only FUSE filesystem for ZIP archives

This program mounts a ZIP archive as a read-only filesystem using FUSE.
It allows browsing and reading the contents of the ZIP file as if it were
a regular directory structure.

Usage:
    zip_fs <ZIP_FILE> <MOUNT_POINT>
    zip_fs --zip-file <ZIP_FILE> --mount-point <MOUNT_POINT>