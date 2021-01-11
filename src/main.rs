use std::ffi::OsStr;

use crate::tg::TgConnection;
use log;
use simple_logger::SimpleLogger;
use tokio::runtime::Runtime;
use tokio::task;

mod external_serialization;
mod fpfs;
mod serialization;
mod tg;
mod tg_tools;
mod types;
mod utils;

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let mountpoint = "/Users/alex.plate/Downloads/test75";

    let (connection, client) = Runtime::new().unwrap().block_on(TgConnection::connect());

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(fpfs::Fpfs::new(connection), &mountpoint, &options).unwrap();

    task::spawn(async move { client.run_until_disconnected().await });
}
