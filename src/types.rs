use fuse::FileAttr;
use serde::{Deserialize, Serialize};

use crate::external_serialization::FileAttrDef;

pub const VERSION: &'static str = "v1";

#[derive(Serialize, Deserialize)]
pub struct MetaMessage {
    pub version: String,
    pub files: Vec<FileLink>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileLink {
    pub name: String,
    pub ino: u64,
    pub meta_file_link: Option<i32>,
    pub size: u64,

    #[serde(with = "FileAttrDef")]
    pub attr: FileAttr,
}

impl FileLink {
    pub fn new(name: String, ino: u64, meta_file_link: Option<i32>, size: u64, attr: FileAttr) -> FileLink {
        FileLink {
            name,
            ino,
            meta_file_link,
            size,
            attr,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FileMeta {
    pub data_link: Option<u64>,
}
