use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use log::info;

fn main() {
    // Initialize env_logger and set default level to info
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    builder.filter_module("wgpu_core", log::LevelFilter::Warn);
    builder.filter_module("wgpu_hal", log::LevelFilter::Warn);
    builder.filter_module("naga", log::LevelFilter::Warn);
    let _ = builder.try_init();

    info!("Starting Dedicated Minecrust Server...");

    // Bind on 0.0.0.0:25565
    let bind_addr: SocketAddr = "0.0.0.0:25565".parse().unwrap();
    info!("Attempting to bind on {}...", bind_addr);

    // Start the IntegratedServer
    // The server will run in a background thread, so we need to keep the main thread alive.
    let registry = std::sync::Arc::new(minecrust_shared::world::block::BlockRegistry::new());
    let (_client_tx, _client_rx) = minecrust_server::server::IntegratedServer::start(12345, Some(bind_addr), registry);

    info!("Dedicated server successfully started. Press Ctrl+C to stop.");

    // Keep the main thread alive forever
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
