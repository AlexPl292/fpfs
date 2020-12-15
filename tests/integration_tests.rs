extern crate fpfs;

use simple_logger::SimpleLogger;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
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

    let another_path = format!("{}/{}", path.as_os_str().to_str().unwrap(), "another");
    File::create(&another_path).unwrap();

    let file_list = fs::read_dir(path)
        .unwrap()
        .into_iter()
        .map(|x| x.unwrap().path())
        .collect::<Vec<PathBuf>>();

    assert_eq!(file_list.len(), 1);
    assert_eq!(file_list[0].to_str().unwrap(), another_path);

    fs::write(&another_path, "123").unwrap();

    Command::new("umount")
        .arg(path.to_str().unwrap())
        .spawn()
        .unwrap();

    std::mem::drop(session);
}
