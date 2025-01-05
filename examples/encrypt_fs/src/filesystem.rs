use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    sync::RwLock,
    u32,
};

use easy_fuser::{prelude::*, templates::DefaultFuseHandler, unix_fs};

use crate::{dir_handler::DirMapper, helpers, inode_hash_mapper::InodeHashMapper};

pub struct EncryptFs {
    inner: DefaultFuseHandler,
    repo: PathBuf,
    mapper: RwLock<InodeHashMapper>,
    encryption_key: [u8; 32],
}

impl EncryptFs {
    pub fn new(repo: PathBuf, encryption_key: [u8; 32]) -> Self {
        Self {
            inner: DefaultFuseHandler::new(),
            mapper: RwLock::new(InodeHashMapper::new()),
            repo,
            encryption_key,
        }
    }
}

impl FuseHandler<Inode> for EncryptFs {
    fn get_inner(&self) -> &dyn FuseHandler<Inode> {
        &self.inner
    }

    fn create(
        &self,
        _req: &RequestInfo,
        parent_id: Inode,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, (Inode, FileAttribute), FUSEOpenResponseFlags)> {
        let mut mapper = self.mapper.write().unwrap();
        let name = name.to_os_string();
        let hashed_parent = self.repo.join(mapper.resolve_id(&parent_id).unwrap());
        let hashed_name = helpers::hash_file_name(&name);
        let mut dir_mapper = DirMapper::open_or_create(&hashed_parent, self.encryption_key);
        dir_mapper.insert(name.clone(), hashed_name.clone());

        let new_inode = mapper
            .add_child(&parent_id, name, hashed_name, true)
            .expect("Could not add child to mapper");
        let hashed_path = mapper
            .resolve_id(&new_inode)
            .expect("Could not resolve inode");
        let (mut fd, file_attr) = unix_fs::create(&hashed_path, mode, umask, flags)?;
        Ok((
            fd.take_to_file_handle()?,
            (new_inode, file_attr),
            FUSEOpenResponseFlags::empty(),
        ))
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: Inode,
        name: &OsStr,
    ) -> FuseResult<(Inode, FileAttribute)> {
        {
            let mapper = self.mapper.read().unwrap();
            if let Some(inode) = mapper.lookup(&parent_id, name, true) {
                let hashed_path = self.repo.join(mapper.resolve_id(&inode).unwrap());
                return Ok((inode, unix_fs::lookup(&hashed_path)?));
            }
        }
        let mut mapper = self.mapper.write().unwrap();
        let hashed_parent = self.repo.join(mapper.resolve_id(&parent_id).unwrap());
        let hashed_name = helpers::hash_file_name(name);
        let hashed_path = hashed_parent.join(hashed_name.clone());
        let file_attr = unix_fs::lookup(&hashed_path)?;
        let new_inode = mapper
            .add_child(&parent_id, name.to_os_string(), hashed_name, true)
            .map_err(|_| PosixError::new(ErrorKind::InvalidArgument, ""))?;
        Ok((new_inode, file_attr))
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        let mapper = self.mapper.read().unwrap();
        let hashed_parent = self.repo.join(mapper.resolve_id(&file_id).unwrap());
        let dir_mapper = DirMapper::open_or_create(&hashed_parent, self.encryption_key);
        Ok(dir_mapper
            .list_children()
            .into_iter()
            .map(|(name, hashed_name)| {
                let inode = mapper.lookup(&file_id, &name, false).unwrap();
                let file_kind = unix_fs::lookup(&hashed_parent.join(hashed_name.clone()))
                    .unwrap()
                    .kind;
                (name.to_owned(), (inode, file_kind))
            })
            .collect())
    }

    fn read(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: FileHandle,
        seek: SeekFrom,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let hashed_path = self
            .repo
            .join(self.mapper.read().unwrap().resolve_id(&file_id).unwrap());
        let fd = unix_fs::open(&hashed_path, OpenFlags::empty())?;
        let all_data = unix_fs::read(&fd, SeekFrom::Start(0), u32::MAX)?;
        let decrypted_data = helpers::decrypt_data(&self.encryption_key, &all_data);
        let seek_pos = std::cmp::max(
            all_data.len() - 1,
            match seek {
                SeekFrom::Start(pos) => pos,
                SeekFrom::Current(pos) => pos.try_into().unwrap_or(0) as u64,
                SeekFrom::End(pos) => (all_data.len() as u64 - 1).saturating_sub(pos as u64),
            } as usize,
        );
        Ok(decrypted_data[seek_pos..seek_pos + size as usize].to_vec())
    }

    fn write(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: FileHandle,
        seek: SeekFrom,
        data: Vec<u8>,
        _write_flags: FUSEWriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        let hashed_path = self
            .repo
            .join(self.mapper.read().unwrap().resolve_id(&file_id).unwrap());

        let fd = unix_fs::open(&hashed_path, OpenFlags::READ_WRITE)?;

        // Read existing data
        let existing_data = unix_fs::read(&fd, SeekFrom::Start(0), u32::MAX)?;
        let mut decrypted_data = helpers::decrypt_data(&self.encryption_key, &existing_data);

        // Calculate seek position
        let seek_pos = match seek {
            SeekFrom::Start(pos) => pos as usize,
            SeekFrom::Current(pos) => (decrypted_data.len() as i64 + pos).max(0) as usize,
            SeekFrom::End(pos) => (decrypted_data.len() as i64 + pos).max(0) as usize,
        };

        // Extend the decrypted data if necessary
        if seek_pos > decrypted_data.len() {
            decrypted_data.resize(seek_pos, 0);
        }

        // Write new data
        let end_pos = seek_pos + data.len();
        if end_pos > decrypted_data.len() {
            decrypted_data.resize(end_pos, 0);
        }
        decrypted_data[seek_pos..end_pos].copy_from_slice(&data);

        // Encrypt and write back
        let encrypted_data = helpers::encrypt_data(&self.encryption_key, &decrypted_data);
        unix_fs::write(&fd, SeekFrom::Start(0), &encrypted_data)?;

        Ok(data.len() as u32)
    }
}
