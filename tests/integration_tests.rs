extern crate fpfs;

use simple_logger::SimpleLogger;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use tokio::time::Duration;

#[test]
fn create_empty_file() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let tmpfile = tempfile::tempdir().unwrap();

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    let filesystem = fpfs::Fpfs::new();
    filesystem.remove_meta();
    let session = unsafe { fuse::spawn_mount(filesystem, &tmpfile, &options).unwrap() };

    sleep(Duration::from_secs(1));

    let path = tmpfile.path();
    let file_list = fs::read_dir(path)
        .unwrap()
        .into_iter()
        .map(|x| x.unwrap().path())
        .collect::<Vec<PathBuf>>();

    assert!(file_list.is_empty());

    file_loop(path, "another", 0, "123");
    file_loop(path, "another_one_file", 1, "456");

    let dir_path = format!("{}/{}", path.as_os_str().to_str().unwrap(), "my_dir");
    fs::create_dir(dir_path).unwrap();

    Command::new("umount")
        .arg(path.to_str().unwrap())
        .spawn()
        .unwrap();

    std::mem::drop(session);
}

fn file_loop(path: &Path, file_name: &str, amount_of_existing_files: usize, content: &str) {
    let another_path = format!("{}/{}", path.as_os_str().to_str().unwrap(), file_name);
    File::create(&another_path).unwrap();

    let file_list = fs::read_dir(path)
        .unwrap()
        .into_iter()
        .map(|x| x.unwrap().path())
        .collect::<Vec<PathBuf>>();

    assert_eq!(file_list.len(), amount_of_existing_files + 1);
    assert!(file_list
        .iter()
        .map(|x| x.to_str().unwrap())
        .any(|x| x == another_path));

    fs::write(&another_path, content).unwrap();

    let bytes = fs::read(&another_path).unwrap();
    let result = String::from_utf8(bytes).unwrap();

    assert_eq!(content, result);
}
