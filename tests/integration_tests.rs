extern crate fpfs;

use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;

use simple_logger::SimpleLogger;
use tokio::task;
use tokio::time::Duration;

use fpfs::TgConnection;

#[tokio::test(flavor = "multi_thread")]
async fn create_empty_file() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let tmpfile = tempfile::tempdir().unwrap();

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    let (connection, client) = TgConnection::connect().await;

    task::spawn(async move { client.run_until_disconnected().await });

    let mut filesystem = fpfs::Fpfs::new(connection);
    filesystem.remove_meta().await;
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

    file_loop_with_dir(path, "my_dir", "another", 0, "123");

    remove_loop(path, "another", 3);

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

fn remove_loop(path: &Path, file_name: &str, amount_of_existing_files: usize) {
    let another_path = format!("{}/{}", path.as_os_str().to_str().unwrap(), file_name);

    fs::remove_file(&another_path).unwrap();

    let file_list = fs::read_dir(path)
        .unwrap()
        .into_iter()
        .map(|x| x.unwrap().path())
        .collect::<Vec<PathBuf>>();

    assert_eq!(file_list.len(), amount_of_existing_files - 1);
}

fn file_loop_with_dir(
    path: &Path,
    dir: &str,
    file_name: &str,
    amount_of_existing_files: usize,
    content: &str,
) {
    let goal_dir = format!("{}/{}", path.as_os_str().to_str().unwrap(), dir);
    let another_path = format!("{}/{}", &goal_dir, file_name);
    File::create(&another_path).unwrap();

    let file_list = fs::read_dir(goal_dir)
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
