use std::env;
use std::ffi::OsStr;

mod fpfs;

fn main() {
    // Timespec::new(0, 0);
    // env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    let options = ["-o", "ro", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(fpfs::Fpfs, &mountpoint, &options).unwrap();
}
