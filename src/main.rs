use std::env;
use std::ffi::OsStr;

use crate::tg::TgConnection;
use log;
use simple_logger::SimpleLogger;
use tokio::runtime;

mod fpfs;
mod tg;
mod utils;

async fn async_main() {
    let api_id: i32 = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();
    let mut connection = TgConnection::connect(api_id, api_hash);
    connection.create_file("xx");
}

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let mountpoint = "/Users/alex.plate/Downloads/test55";

    // async_main()

    let options = ["-f", "-o", "fsname=fpfs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(fpfs::Fpfs::new(), &mountpoint, &options).unwrap();
}
