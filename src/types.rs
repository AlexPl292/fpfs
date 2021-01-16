use std::collections::HashMap;

use fuse::FileAttr;
use grammers_tl_types as tl;
use serde::{Deserialize, Serialize};

use crate::external_serialization::FileAttrDef;

pub const VERSION: &'static str = "v1";

#[derive(Serialize, Deserialize)]
pub struct MetaMessage {
    pub version: String,
    pub files: HashMap<u64, i32>,
    pub next_ino: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileLink {
    pub name: String,
    pub children: Vec<u64>,
    pub file: Option<FpfsInputFile>,
    pub xattr: HashMap<String, Vec<u8>>,

    #[serde(with = "FileAttrDef")]
    pub attr: FileAttr,
}

impl FileLink {
    pub fn new_file(name: String, attr: FileAttr) -> FileLink {
        FileLink {
            name,
            children: vec![],
            file: None,
            xattr: HashMap::new(),
            attr,
        }
    }

    pub fn new_dir(name: String, children: Vec<u64>, attr: FileAttr) -> FileLink {
        FileLink {
            name,
            children,
            file: None,
            xattr: HashMap::new(),
            attr,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FpfsInputFile {
    pub id: i64,
    pub parts: i32,
    pub name: String,
    pub md5_checksum: String,
}

impl From<tl::enums::InputFile> for FpfsInputFile {
    fn from(input_file: tl::enums::InputFile) -> Self {
        match input_file {
            tl::enums::InputFile::File(data) => FpfsInputFile {
                id: data.id,
                parts: data.parts,
                name: data.name,
                md5_checksum: data.md5_checksum,
            },
            _ => panic!("Panic"),
        }
    }
}

impl From<FpfsInputFile> for tl::enums::InputFile {
    fn from(data: FpfsInputFile) -> Self {
        tl::enums::InputFile::File(tl::types::InputFile {
            id: data.id,
            parts: data.parts,
            name: data.name,
            md5_checksum: data.md5_checksum,
        })
    }
}
