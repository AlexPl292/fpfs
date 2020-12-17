use fuse::{FileAttr, FileType};
use serde::{Deserialize, Serialize};
use time::Timespec;

#[derive(Serialize, Deserialize)]
#[serde(remote = "FileAttr")]
pub struct FileAttrDef {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    #[serde(with = "TimeSpecDef")]
    pub atime: Timespec,
    #[serde(with = "TimeSpecDef")]
    pub mtime: Timespec,
    #[serde(with = "TimeSpecDef")]
    pub ctime: Timespec,
    #[serde(with = "TimeSpecDef")]
    pub crtime: Timespec,
    #[serde(with = "FileTypeDef")]
    pub kind: FileType,
    pub perm: u16,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub flags: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "FileType")]
pub enum FileTypeDef {
    NamedPipe,
    CharDevice,
    BlockDevice,
    Directory,
    RegularFile,
    Symlink,
    Socket,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Timespec")]
pub struct TimeSpecDef {
    pub sec: i64,
    pub nsec: i32,
}
