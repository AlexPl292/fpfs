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
    Runtime::new().unwrap().block_on(start());
}

async fn start() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let mountpoint = "/Users/alex.plate/Downloads/test101";

    let (connection, client) = TgConnection::connect().await;

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    task::spawn(async move { client.run_until_disconnected().await });

    unsafe { fuse::spawn_mount(fpfs::Fpfs::new(connection), &mountpoint, &options).unwrap(); }
}
