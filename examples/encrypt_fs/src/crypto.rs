pub fn read_decrypt(fd: FileDescriptor, key: [u8; 32], seek: SeekFrom, size: u32) -> Result<Vec<u8>>