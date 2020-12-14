use serde::{Deserialize, Serialize};

pub const VERSION: &'static str = "v1";

#[derive(Serialize, Deserialize)]
pub struct MetaMessage {
    pub version: String,
    pub files: Vec<FileLink>,
}

#[derive(Serialize, Deserialize)]
pub struct FileLink {
    pub name: String,
    // pub ino: u64,
    pub meta_file_link: Option<i32>,
    pub size: u64,
}

impl FileLink {
    pub fn new(name: String, meta_file_link: Option<i32>, size: u64) -> FileLink {
        FileLink {
            name,
            meta_file_link,
            size,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FileMeta {
    pub data_link: Option<u64>,
}
