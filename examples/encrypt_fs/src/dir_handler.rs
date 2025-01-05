use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use easy_fuser::types::Inode;

use crate::helpers;

const DIR_MAPPER_FILE: &str = "dir_mapper.bin";

pub struct DirMapper<'a> {
    parent_path: &'a PathBuf,
    encryption_key: [u8; 32],
    mapper: HashMap<u64, String>,
}

impl<'a> DirMapper<'a> {
    pub fn open_or_create(parent_path: &'a PathBuf, encryption_key: [u8; 32]) -> Self {
        let path = parent_path.join(DIR_MAPPER_FILE);
        let file = File::open(&path);
        if file.is_err() {
            File::create(&path).expect("Failed to create name mapper file");
            let result = DirMapper {
                parent_path,
                encryption_key,
                mapper: HashMap::new(),
            };
            result.serialize_and_save();
            return result;
        }
        let mut buffer = Vec::new();
        file.unwrap().read_to_end(&mut buffer).unwrap();
        let plain = helpers::decrypt_data(&encryption_key, &buffer);
        let mapper = bincode::deserialize(&plain).expect("Failed to deserialize hashmap");
        DirMapper {
            parent_path,
            encryption_key,
            mapper,
        }
    }

    fn serialize_and_save(&self) {
        let plain = bincode::serialize(&self.mapper).expect("Failed to serialize hashmap");
        let encrypted = helpers::encrypt_data(&self.encryption_key, &plain);
        let file_path = self.parent_path.join(DIR_MAPPER_FILE);
        fs::write(file_path, &encrypted).expect("Failed to write encrypted data to file");
    }

    pub fn list_children(&self) -> Vec<(Inode, &String)> {
        self.mapper.iter().collect()
    }

    pub fn insert(&mut self, name: OsString, hashed_name: String) -> Option<String> {
        let result = self.mapper.insert(name, hashed_name);
        self.serialize_and_save();
        result
    }

    pub fn remove(&mut self, name: &OsStr) -> Option<()> {
        let _ = self.mapper.remove(name)?;
        self.serialize_and_save();
        Some(())
    }
}
