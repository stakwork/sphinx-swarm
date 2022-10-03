pub mod dock;
mod env;
mod images;
mod logs;
mod modes;
mod routes;
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
    let mode = std::env::args().nth(1).expect("no mode given");

    let d = dock::er();
    match match mode.as_str() {
        "demo" => modes::demo::run(d).await,
        "down" => modes::down::run(d).await,
        "test" => modes::test::run(d).await,
        _ => panic!("invalid mode"),
    } {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    };
}
