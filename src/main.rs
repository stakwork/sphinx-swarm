pub mod dock;

mod cmd;
mod env;
mod images;
mod logs;
mod rocket_utils;
mod utils;

#[rocket::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_module_level("bollard", log::LevelFilter::Off)
        .with_module_level("want", log::LevelFilter::Off)
        .with_module_level("mio", log::LevelFilter::Off)
        .with_module_level("rocket", log::LevelFilter::Error)
        .with_module_level("_", log::LevelFilter::Error)
        .init()
        .unwrap();
    let cmd = std::env::args().nth(1).expect("no cmd given");

    let d = dock::er();
    match match cmd.as_str() {
        "demo" => cmd::demo::run(d).await,
        "down" => cmd::down::run(d).await,
        "test" => cmd::test::run(d).await,
        "stack" => cmd::stack::run(d).await,
        _ => panic!("invalid cmd"),
    } {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    };
}
