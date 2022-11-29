pub mod dock;

mod cmd;
mod config;
mod conn;
mod env;
mod images;
mod logs;
mod modes;
mod rocket_utils;
mod routes;
mod secrets;
mod utils;

#[rocket::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_module_level("bollard", log::LevelFilter::Warn)
        .with_module_level("want", log::LevelFilter::Off)
        .with_module_level("mio", log::LevelFilter::Off)
        .with_module_level("rocket", log::LevelFilter::Error)
        .with_module_level("hyper", log::LevelFilter::Warn)
        .with_module_level("tracing", log::LevelFilter::Error)
        .with_module_level("tokio_util", log::LevelFilter::Error)
        .with_module_level("tonic", log::LevelFilter::Error)
        .with_module_level("h2", log::LevelFilter::Error)
        .with_module_level("bitcoincore_rpc", log::LevelFilter::Error)
        .with_module_level("_", log::LevelFilter::Error)
        .init()
        .unwrap();
    let cmd = std::env::args().nth(1).expect("no cmd given");

    let d = dock::er();
    match match cmd.as_str() {
        "demo" => modes::demo::run(d).await,
        "down" => modes::down::run(d).await,
        "test" => modes::test::run(d).await,
        "stack" => modes::stack::run(d).await,
        _ => panic!("invalid cmd"),
    } {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    };
}
