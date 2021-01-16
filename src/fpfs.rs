use std::ffi::OsStr;
use std::io::Write;

use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyLock, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXTimes, ReplyXattr,
    Request,
};
use libc::{ENOENT, ENOSYS, ERANGE};
use tempfile::NamedTempFile;
use time::Timespec;
use tokio::runtime::Runtime;

use crate::tg::TgConnection;
use crate::types::FileLink;
use std::path::Path;

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
    cache_ino: u64,
}

impl Fpfs {
    pub fn new(connection: TgConnection) -> Fpfs {
        return Fpfs {
            connection,
            files_cache: None,
            cache_ino: 0,
        };
    }

    fn get_cache(&mut self, directory: &u64) -> &Vec<FileLink> {
        self.init_cache(directory);
        self.files_cache.as_ref().unwrap()
    }

    fn get_cur_cache(&mut self) -> &Vec<FileLink> {
        let ino = self.cache_ino;
        self.init_cache(&ino);
        self.files_cache.as_ref().unwrap()
    }

    fn get_cur_cache_mut(&mut self) -> &mut Vec<FileLink> {
        let ino = self.cache_ino;
        self.init_cache(&ino);
        self.files_cache.as_mut().unwrap()
    }

    fn get_cache_mut(&mut self, directory: &u64) -> &mut Vec<FileLink> {
        self.init_cache(directory);
        self.files_cache.as_mut().unwrap()
    }

    fn init_cache(&mut self, directory: &u64) {
        if self.files_cache.is_none() || self.cache_ino != *directory {
            let files = Runtime::new()
                .unwrap()
                .block_on(self.connection.get_directory_files(directory));
            self.files_cache = Some(files);
            self.cache_ino = directory.clone()
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
    pub async fn remove_meta(&mut self) {
        self.connection.cleanup().await;
    }

    fn next_ino(&mut self) -> u64 {
        Runtime::new()
            .unwrap()
            .block_on(self.connection.get_and_inc_ino())
    }

    fn get_ino(&mut self, ino: u64) -> Option<FileLink> {
        let attr = self.get_cur_cache().iter().find(|x| x.attr.ino == ino);
        if let Some(data) = attr {
            Some(data.clone())
        } else {
            Runtime::new()
                .unwrap()
                .block_on(self.connection.get_file_attr(&ino))
        }
    }
}

impl Filesystem for Fpfs {
    fn init(&mut self, _req: &Request) -> Result<(), i32> {
        self.connection.check_or_init_meta(&HELLO_DIR_ATTR);
        self.init_cache(&HELLO_DIR_ATTR.ino);
        Ok(())
    }

    fn destroy(&mut self, _req: &Request) {}

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let my_file_name = name.to_str().unwrap_or("~").to_string();
        let found_file = self
            .get_cache(&parent)
            .iter()
            .find(|x| x.name == my_file_name);
        if found_file.is_some() {
            reply.entry(&TTL, &found_file.unwrap().attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn forget(&mut self, _req: &Request, _ino: u64, _nlookup: u64) {}

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let attr = self.get_ino(ino);
        if let Some(data) = attr {
            reply.attr(&TTL, &data.attr)
        } else {
            reply.error(ENOENT)
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<Timespec>,
        mtime: Option<Timespec>,
        _fh: Option<u64>,
        crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let attr = self.get_ino(ino);
        if let Some(data) = attr {
            let mut attrbts = data.attr;
            attrbts.uid = uid.unwrap_or(attrbts.uid);
            attrbts.gid = gid.unwrap_or(attrbts.gid);
            attrbts.size = size.unwrap_or(attrbts.size);
            attrbts.atime = atime.unwrap_or(attrbts.atime);
            attrbts.mtime = mtime.unwrap_or(attrbts.mtime);
            attrbts.crtime = crtime.unwrap_or(attrbts.crtime);
            attrbts.flags = flags.unwrap_or(attrbts.flags);

            // FIXME update cache
            self.connection.set_attr(ino, attrbts.clone());

            reply.attr(&TTL, &attrbts)
        } else {
            reply.error(ENOENT)
        }
    }

    fn readlink(&mut self, _req: &Request, _ino: u64, reply: ReplyData) {
        reply.error(ENOSYS);
    }

    fn mknod(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        reply.error(ENOSYS);
    }

    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        let next_ino = self.next_ino();
        let dir_name = name.to_str().unwrap().to_string();
        let attr = Fpfs::make_dir_attr(next_ino);
        let file_link = FileLink::new_dir(dir_name.clone(), vec![], attr.clone());
        self.connection
            .create_dir(dir_name.as_str(), next_ino, Some(parent), &attr);

        match self.files_cache {
            Some(ref mut f) => f.push(file_link),
            None => (),
        }

        reply.entry(&TTL, &attr, 0);
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let my_file_name = name.to_str().unwrap_or("~").to_string();
        let cache = self.get_cache_mut(&parent);

        let position = cache.iter().position(|x| x.name == my_file_name);
        if let Some(idx) = position {
            let data = cache.remove(idx);
            let file_ino = data.attr.ino;
            self.connection.remove_inode(file_ino, parent);
            reply.ok()
        } else {
            reply.error(ENOENT);
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let my_file_name = name.to_str().unwrap_or("~").to_string();
        let cache = self.get_cache_mut(&parent);

        let position = cache.iter().position(|x| x.name == my_file_name);
        if let Some(idx) = position {
            let data = cache.remove(idx);
            let file_ino = data.attr.ino;
            self.connection.remove_inode(file_ino, parent);
            reply.ok()
        } else {
            reply.error(ENOENT);
        }
    }

    fn symlink(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _link: &Path,
        reply: ReplyEntry,
    ) {
        reply.error(ENOSYS);
    }

    fn rename(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        reply: ReplyEmpty,
    ) {
        let my_file_name = name.to_str().unwrap_or("~").to_string();
        let cache = self.get_cache_mut(&parent);

        let position = cache.iter().position(|x| x.name == my_file_name);
        if let Some(idx) = position {
            let data = cache.remove(idx);
            self.files_cache = None;
            let file_ino = data.attr.ino;
            self.connection
                .rename(file_ino, newname.to_str().unwrap(), parent, newparent);
            reply.ok()
        } else {
            reply.error(ENOENT);
        }
    }

    fn link(
        &mut self,
        _req: &Request,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        reply.error(ENOSYS);
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

        self.get_cur_cache_mut()
            .iter_mut()
            .find(|x| x.attr.ino == ino)
            .unwrap()
            .attr
            .size = data.len() as u64;

        reply.written(data.len() as u32)
    }

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        reply.error(ENOSYS);
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        reply.error(ENOSYS);
    }

    fn opendir(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        self.init_cache(&ino);
        reply.opened(0, flags);
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let mut entries: Vec<(u64, FileType, String)> = vec![
            (1, FileType::Directory, String::from(".")),
            (1, FileType::Directory, String::from("..")),
        ];

        for file in self.get_cache(&ino) {
            entries.push((file.attr.ino, FileType::RegularFile, file.name.to_string()))
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2.as_str());
        }
        reply.ok();
    }

    fn releasedir(&mut self, _req: &Request, _ino: u64, _fh: u64, _flags: u32, reply: ReplyEmpty) {
        reply.ok();
    }

    fn fsyncdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply.error(ENOSYS);
    }

    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }

    fn setxattr(
        &mut self,
        _req: &Request,
        ino: u64,
        name: &OsStr,
        value: &[u8],
        _flags: u32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        let name = name.to_str().unwrap().to_string();
        let vec = value.to_vec();
        self.connection.set_xattr(ino, name.clone(), vec.clone());
        self.get_cur_cache_mut().iter_mut().for_each(|x| {
            if x.attr.ino == ino {
                x.xattr.insert(name.clone(), vec.clone());
            }
        });
        reply.ok();
    }

    fn getxattr(&mut self, _req: &Request, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr) {
        let file_link = self.get_ino(ino);
        if let Some(data) = file_link {
            let attr_name = name.to_str().unwrap().to_string();
            let attr_value = data.xattr.get(&attr_name);
            let attr_size = attr_value.map(|x| x.len()).unwrap_or(0) as u32;
            if size == 0 {
                reply.size(attr_size as u32);
            } else if size >= attr_size {
                reply.data(attr_value.unwrap_or(&vec![]));
            } else {
                reply.error(ERANGE)
            }
        } else {
            reply.error(ENOSYS);
        }
    }

    fn listxattr(&mut self, _req: &Request, ino: u64, size: u32, reply: ReplyXattr) {
        let file_link = self.get_ino(ino);

        if let Some(data) = file_link {
            let names: Vec<String> = data.xattr.keys().map(|x| x.to_string()).collect();
            let name_string: String = names.join("\0");
            let attr_size = name_string.len() as u32;
            if size == 0 {
                reply.size(attr_size);
            } else if size >= attr_size {
                reply.data(name_string.as_bytes());
            } else {
                reply.error(ERANGE);
            }
        } else {
            reply.error(ENOSYS);
        }
    }

    fn removexattr(&mut self, _req: &Request, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        let attr_name = name.to_str().unwrap().to_string();
        self.connection.remove_xattr(ino, attr_name.clone());

        self.get_cur_cache_mut().iter_mut().for_each(|x| {
            if x.attr.ino == ino {
                x.xattr.remove(attr_name.as_str());
            }
        });
        reply.ok();
    }

    fn access(&mut self, _req: &Request, _ino: u64, _mask: u32, reply: ReplyEmpty) {
        reply.error(ENOSYS);
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        flags: u32,
        reply: ReplyCreate,
    ) {
        let next_ino = self.next_ino();
        let file_name = name.to_str().unwrap().to_string();
        let attr = Fpfs::make_attr(0, next_ino);
        let file_link = FileLink::new_file(file_name.clone(), attr.clone());
        self.connection
            .create_file(file_name.as_str(), next_ino, parent, &attr);

        match self.files_cache {
            Some(ref mut f) => f.push(file_link),
            None => (),
        }

        reply.created(&TTL, &attr, 0, 0, flags);
    }

    fn getlk(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: u32,
        _pid: u32,
        reply: ReplyLock,
    ) {
        reply.error(ENOSYS);
    }

    fn setlk(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: u32,
        _pid: u32,
        _sleep: bool,
        reply: ReplyEmpty,
    ) {
        reply.error(ENOSYS);
    }

    fn bmap(&mut self, _req: &Request, _ino: u64, _blocksize: u32, _idx: u64, reply: ReplyBmap) {
        reply.error(ENOSYS);
    }

    fn setvolname(&mut self, _req: &Request, _name: &OsStr, reply: ReplyEmpty) {
        reply.error(ENOSYS);
    }

    fn exchange(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _newparent: u64,
        _newname: &OsStr,
        _options: u64,
        reply: ReplyEmpty,
    ) {
        reply.error(ENOSYS);
    }

    fn getxtimes(&mut self, _req: &Request, _ino: u64, reply: ReplyXTimes) {
        reply.error(ENOSYS);
    }
}

impl Fpfs {
    pub fn write_my_file(data: &[u8]) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write(data).unwrap();
        temp_file
    }
}
