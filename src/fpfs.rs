use std::ffi::OsStr;
use std::io::Write;

use fuse::{
    FileAttr, Filesystem, FileType, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, ReplyWrite, Request,
};
use libc::ENOENT;
use tempfile::NamedTempFile;
use time::Timespec;
use tokio::runtime::Runtime;

use crate::tg::TgConnection;
use crate::types::FileLink;

/// Some readings:
/// CS135 FUSE Documentation:
/// - https://www.cs.hmc.edu/~geoff/classes/hmc.cs135.201001/homework/fuse/fuse_doc.html
///
///
/// Small wiki about parameters:
///   - ino - the inode number
///   - fh - File handle id. File identifier, may be used instead of path.

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

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 17,
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
    files_cache: Option<Vec<FileLink>>,
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

    fn get_cache(&mut self) -> &Vec<FileLink> {
        self.init_cache();
        self.files_cache.as_ref().unwrap()
    }

    fn get_cache_mut(&mut self) -> &mut Vec<FileLink> {
        self.init_cache();
        self.files_cache.as_mut().unwrap()
    }

    fn init_cache(&mut self) {
        if self.files_cache.is_none() {
            let files = Runtime::new()
                .unwrap()
                .block_on(self.connection.get_files_names());
            self.files_cache = Some(files);
        }
    }

    fn make_attr(size: u64, ino: u64) -> FileAttr {
        FileAttr {
            size,
            ino,
            ..HELLO_TXT_ATTR
        }
    }

    fn make_dir_attr(ino: u64) -> FileAttr {
        FileAttr {
            ino,
            ..HELLO_DIR_ATTR
        }
    }

    #[allow(dead_code)]
    pub fn remove_meta(&self) {
        self.connection.cleanup();
    }
}

impl Filesystem for Fpfs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let my_file_name = name.to_str().unwrap_or("~").to_string();
        let found_file = self.get_cache().iter().find(|x| x.name == my_file_name);
        if parent == 1 && found_file.is_some() {
            reply.entry(&TTL, &found_file.unwrap().attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            _ => {
                let attr = self.get_cache().iter().find(|x| x.attr.ino == ino);
                if let Some(data) = attr {
                    reply.attr(&TTL, &data.attr)
                } else {
                    reply.error(ENOENT)
                }
            }
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        _ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        reply.attr(&TTL, &HELLO_TXT_ATTR);
    }

    fn mkdir(&mut self, _req: &Request, _parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        let next_ino = self
            .get_cache()
            .iter()
            .map(|x| x.attr.ino)
            .max()
            .unwrap_or(2)
            + 1;
        let dir_name = name.to_str().unwrap().to_string();
        let attr = Fpfs::make_dir_attr(next_ino);
        let file_link = FileLink::new_dir(dir_name.clone(), vec![], attr.clone());
        self.connection.create_dir(dir_name.as_str(), next_ino, &attr);

        match self.files_cache {
            Some(ref mut f) => f.push(file_link),
            None => (),
        }

        reply.entry(&TTL, &attr, 0);
    }

    fn open(&mut self, _req: &Request, _ino: u64, flags: u32, reply: ReplyOpen) {
        reply.opened(0, flags);
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
        let file_data = Runtime::new()
            .unwrap()
            .block_on(self.connection.read_file(ino));
        match file_data {
            Some(data) => {
                let data_array = &data[offset as usize..];
                reply.data(data_array)
            }
            None => reply.error(ENOENT),
        }
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        let path = Fpfs::write_my_file(data);

        self.connection.write_to_file(path, ino);

        self.get_cache_mut()
            .iter_mut()
            .find(|x| x.attr.ino == ino)
            .unwrap()
            .attr
            .size = data.len() as u64;

        reply.written(data.len() as u32)
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

        for file in self.get_cache() {
            entries.push((file.attr.ino, FileType::RegularFile, file.name.to_string()))
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
        name: &OsStr,
        _mode: u32,
        flags: u32,
        reply: ReplyCreate,
    ) {
        let next_ino = self
            .get_cache()
            .iter()
            .map(|x| x.attr.ino)
            .max()
            .unwrap_or(2)
            + 1;
        let file_name = name.to_str().unwrap().to_string();
        let attr = Fpfs::make_attr(0, next_ino);
        let file_link = FileLink::new_file(file_name.clone(), attr.clone());
        self.connection
            .create_file(file_name.as_str(), next_ino, &attr);

        match self.files_cache {
            Some(ref mut f) => f.push(file_link),
            None => (),
        }

        reply.created(&TTL, &attr, 0, 0, flags);
    }
}

impl Fpfs {
    pub fn write_my_file(data: &[u8]) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write(data).unwrap();
        temp_file
    }
}
