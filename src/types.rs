use std::collections::HashMap;

use fuse::FileAttr;
use serde::{Deserialize, Serialize};

use crate::external_serialization::FileAttrDef;
use crate::types::Type::{DIR, FILE};

pub const VERSION: &'static str = "v1";

#[derive(Serialize, Deserialize)]
pub struct MetaMessage {
    pub version: String,
    pub files: HashMap<u64, i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileLink {
    pub name: String,
    pub link_type: Type,
    pub children: Vec<i32>,

    #[serde(with = "FileAttrDef")]
    pub attr: FileAttr,
}

impl FileLink {
    pub fn new_file(name: String, attr: FileAttr) -> FileLink {
        FileLink {
            name,
            link_type: FILE,
            children: vec![],
            attr,
        }
    }

    pub fn new_dir(name: String, children: Vec<i32>, attr: FileAttr) -> FileLink {
        FileLink {
            name,
            link_type: DIR,
            children,
            attr,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Type {
    DIR,
    FILE,
}
