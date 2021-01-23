use std::ffi::OsStr;

use crate::tg::TgConnection;
use log;
use simple_logger::SimpleLogger;
use tokio::runtime::Runtime;
use tokio::task;
use std::env;

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

    let args: Vec<String> = env::args().collect();

    let mountpoint = args.last().unwrap();

    let (connection, client) = TgConnection::connect().await;

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    task::spawn(async move { client.run_until_disconnected().await });

    unsafe { fuse::spawn_mount(fpfs::Fpfs::new(connection), &mountpoint, &options).unwrap(); }
}
