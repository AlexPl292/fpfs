use std::ffi::OsStr;

use log;
use simple_logger::SimpleLogger;

mod fpfs;
mod tg;
mod types;
mod utils;

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let mountpoint = "/Users/alex.plate/Downloads/test75";

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(fpfs::Fpfs::new(), &mountpoint, &options).unwrap();
}
