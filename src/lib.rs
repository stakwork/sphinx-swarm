pub mod dock;

pub mod config;
pub mod conn;
pub mod env;
pub mod images;
pub mod logs;
// pub mod modes;
pub mod cmd;
pub mod rocket_utils;
pub mod routes;
pub mod rsa;
pub mod secrets;
pub mod utils;

#[rocket::main]
async fn main() {
    println!("MODES:");
    println!("- stack");
    println!("- demo");
    println!("- btc");
    println!("- down");
    println!("- test");

    dotenv::dotenv().ok();

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
        .with_module_level("rustls", log::LevelFilter::Error)
        .with_module_level("tower", log::LevelFilter::Error)
        .with_module_level("reqwest", log::LevelFilter::Error)
        .with_module_level("_", log::LevelFilter::Error)
        .init()
        .unwrap();
    let cmd = std::env::args().nth(1).expect("no cmd given");

    // let d = dock::dockr();
    // match cmd.as_str() {
    //     "demo" => modes::demo::run(d).await,
    //     "down" => modes::down::run(d).await,
    //     "test" => modes::test::run(d).await,
    //     "stack" => modes::stack::run(d).await,
    //     "btc" => modes::btc_test::run(d).await,
    //     _ => panic!("invalid cmd"),
    // }
    // .expect("FAIL")
}
