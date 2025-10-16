mod logger;

use tracing::{error, info};
use zero_r::screeners::bybit::BybitScreener;
use zero_r::store::db::init_database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logging();
    info!("🚀 Starting Zero-R arbitrage service...");

    let _pool = init_database().await?;

    let bybit_screener = std::sync::Arc::new(BybitScreener::new(_pool.clone()));
    info!("Starting Bybit screener...");

    // Start the screener in a separate task
    let screener_clone = bybit_screener.clone();
    let screener_handle = tokio::spawn(async move {
        if let Err(e) = screener_clone.start().await {
            error!("Bybit screener failed: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    // Stop the screener gracefully
    bybit_screener.stop().await?;
    screener_handle.await?;

    Ok(())
}
