mod api;
mod images;
mod modes;
mod utils;

use bollard::Docker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_module_level("bollard", log::LevelFilter::Off)
        .with_module_level("want", log::LevelFilter::Off)
        .with_module_level("mio", log::LevelFilter::Off)
        .init()
        .unwrap();
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let mode = std::env::args().nth(1).expect("no mode given");

    match mode.as_str() {
        "demo" => modes::demo::run(&docker).await?,
        "down" => modes::down::run(&docker).await?,
        _ => panic!("invalid mode"),
    }
    // remove_container(&docker, &id2).await?;

    Ok(())
}
