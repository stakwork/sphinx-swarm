mod api;
mod images;
mod modes;
mod routes;
mod utils;

use bollard::Docker;

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
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let mode = std::env::args().nth(1).expect("no mode given");

    match match mode.as_str() {
        "demo" => modes::demo::run(&docker).await,
        "down" => modes::down::run(&docker).await,
        _ => panic!("invalid mode"),
    } {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    };
}
