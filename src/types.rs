use std::collections::HashMap;

use fuse::FileAttr;
use serde::{Deserialize, Serialize};

use crate::external_serialization::FileAttrDef;

pub const VERSION: &'static str = "v1";

#[derive(Serialize, Deserialize)]
pub struct MetaMessage {
    pub version: String,
    pub files: HashMap<u64, i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileLink {
    pub name: String,

    #[serde(with = "FileAttrDef")]
    pub attr: FileAttr,
}

impl FileLink {
    pub fn new(name: String, attr: FileAttr) -> FileLink {
        FileLink { name, attr }
    }
}
