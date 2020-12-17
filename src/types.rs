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
    pub meta_file_link: Option<i32>,

    #[serde(with = "FileAttrDef")]
    pub attr: FileAttr,
}

impl FileLink {
    pub fn new(name: String, meta_file_link: Option<i32>, attr: FileAttr) -> FileLink {
        FileLink {
            name,
            meta_file_link,
            attr,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FileMeta {
    pub data_link: Option<u64>,
}
