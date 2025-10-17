mod logger;

use tracing::{error, info};
use zero_r::screeners::bybit::BybitScreener;
use zero_r::screeners::meteora::MeteoraScreener;
use zero_r::store::db::init_database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    logger::init_logging();
    info!("🚀 Starting Zero-R arbitrage service...");

    let _pool = init_database().await?;

    let meteora_screener = std::sync::Arc::new(MeteoraScreener::new(_pool.clone()));
    info!("Starting Meteora screener...");
    let meteora_screener_clone = meteora_screener.clone();
    let meteora_screener_handle = tokio::spawn(async move {
        if let Err(e) = meteora_screener_clone.start().await {
            error!("Meteora screener failed: {}", e);
        }
    });

    let bybit_screener = std::sync::Arc::new(BybitScreener::new(_pool.clone()));
    info!("Starting Bybit screener...");
    let bybit_screener_clone = bybit_screener.clone();
    let bybit_screener_handle = tokio::spawn(async move {
        if let Err(e) = bybit_screener_clone.start().await {
            error!("Bybit screener failed: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    // Stop screener gracefully
    meteora_screener.stop().await?;
    meteora_screener_handle.await?;
    bybit_screener.stop().await?;
    bybit_screener_handle.await?;

    Ok(())
}
