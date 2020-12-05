use std::ffi::OsStr;

use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, Request,
};
use libc::ENOENT;
use rand::Rng;
use time::Timespec;
use tokio::runtime::Runtime;

use crate::tg::TgConnection;

/// Some readings:
/// CS135 FUSE Documentation:
/// - https://www.cs.hmc.edu/~geoff/classes/hmc.cs135.201001/homework/fuse/fuse_doc.html
///
///
/// Small wiki about parameters:
///   ino - the inode number
///   fh - File handle id. File identifier, may be used instead of path.

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

const UNIX_EPOCH: Timespec = Timespec { sec: 0, nsec: 0 };

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

pub struct Fpfs {
    connection: TgConnection,
    files_cache: Option<Vec<String>>,
}

impl Fpfs {
    pub fn new() -> Fpfs {
        let api_id: i32 = env!("TG_ID").parse().expect("TG_ID invalid");
        let api_hash = env!("TG_HASH").to_string();

        let connection = TgConnection::connect(api_id, api_hash);
        return Fpfs {
            connection,
            files_cache: None,
        };
    }

    fn init_cache(&mut self) {
        if self.files_cache.is_none() {
            let files = Runtime::new()
                .unwrap()
                .block_on(self.connection.get_files_names());
            self.files_cache = Some(files);
        }
    }
}

impl Filesystem for Fpfs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        self.init_cache();
        if parent == 1
            && self
                .files_cache
                .as_ref()
                .unwrap()
                .contains(&name.to_str().unwrap_or("~").to_string())
        {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        if ino == 2 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        reply.written(_data.len() as u32)
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let mut entries: Vec<(u64, FileType, String)> = vec![
            (1, FileType::Directory, String::from(".")),
            (1, FileType::Directory, String::from("..")),
        ];

        self.init_cache();

        for file in self.files_cache.as_ref().unwrap() {
            if !file.is_empty() {
                entries.push((2, FileType::RegularFile, file.to_string()))
            }
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2.as_str());
        }
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate,
    ) {
        let name = _name.to_str().unwrap();
        let mut rng = rand::thread_rng();
        self.connection.create_file(name).unwrap();

        match self.files_cache {
            Some(ref mut f) => {
                f.push(name.to_string());
            }
            None => (),
        }

        reply.created(&TTL, &HELLO_TXT_ATTR, rng.gen(), rng.gen(), _flags);
    }
}
