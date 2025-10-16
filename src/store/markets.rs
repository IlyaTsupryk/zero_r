use sqlx::{MySql, Pool, Row};
use tracing::warn;

use crate::models::market::{CEXState, DEXState};

/// Insert a new CEX market record
pub async fn insert_cex_market(
    pool: &Pool<MySql>,
    cex_state: &CEXState,
) -> Result<u64, Box<dyn std::error::Error>> {
    let query = r#"
        INSERT INTO cex_markets (trade_id, exchange, trade_pair, bid_price, bid_volume, ask_price, ask_volume, trade_timestamp, fetch_timestamp)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            bid_price = VALUES(bid_price),
            bid_volume = VALUES(bid_volume),
            ask_price = VALUES(ask_price),
            ask_volume = VALUES(ask_volume),
            fetch_timestamp = VALUES(fetch_timestamp)
    "#;

    let result = sqlx::query(query)
        .bind(&cex_state.trade_id)
        .bind(&cex_state.exchange)
        .bind(&cex_state.trade_pair)
        .bind(&cex_state.bid_price)
        .bind(&cex_state.bid_volume)
        .bind(&cex_state.ask_price)
        .bind(&cex_state.ask_volume)
        .bind(cex_state.trade_time)
        .bind(cex_state.fetch_time)
        .execute(pool)
        .await?;

    Ok(result.last_insert_id())
}

/// Get all CEX market records
pub async fn get_all_cex_markets(
    pool: &Pool<MySql>,
) -> Result<Vec<CEXState>, Box<dyn std::error::Error>> {
    let query = "SELECT id, trade_id, exchange, trade_pair, bid_price, bid_volume, ask_price, ask_volume, trade_timestamp, fetch_timestamp FROM cex_markets ORDER BY fetch_timestamp DESC";

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let mut cex_states = Vec::new();
    for row in rows {
        cex_states.push(CEXState {
            trade_id: row.get("trade_id"),
            exchange: row.get("exchange"),
            trade_pair: row.get("trade_pair"),
            bid_price: row.get("bid_price"),
            bid_volume: row.get("bid_volume"),
            ask_price: row.get("ask_price"),
            ask_volume: row.get("ask_volume"),
            trade_time: row.get("trade_timestamp"),
            fetch_time: row.get("fetch_timestamp"),
        });
    }

    Ok(cex_states)
}

/// Update existing CEX market record
pub async fn update_cex_market(
    pool: &Pool<MySql>,
    cex_state: &CEXState,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = r#"
        UPDATE cex_markets
        SET bid_price = ?, bid_volume = ?, ask_price = ?, ask_volume = ?, fetch_timestamp = ?
        WHERE trade_id = ? AND exchange = ?
    "#;

    let result = sqlx::query(query)
        .bind(cex_state.bid_price)
        .bind(cex_state.bid_volume)
        .bind(cex_state.ask_price)
        .bind(cex_state.ask_volume)
        .bind(cex_state.fetch_time)
        .bind(cex_state.trade_id.to_string())
        .bind(&cex_state.exchange)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        warn!(
            "No CEX market record found to update: trade_id={}, exchange={}",
            cex_state.trade_id, cex_state.exchange
        );
    }

    Ok(())
}

/// Insert a new DEX market record
pub async fn insert_dex_market(
    pool: &Pool<MySql>,
    dex_state: &DEXState,
) -> Result<u64, Box<dyn std::error::Error>> {
    let query = r#"
        INSERT INTO dex_markets (trade_id, exchange, trade_pair, direction, volume, price, trade_timestamp, fetch_timestamp, block_number)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            direction = VALUES(direction),
            volume = VALUES(volume),
            price = VALUES(price),
            trade_timestamp = VALUES(trade_timestamp),
            fetch_timestamp = VALUES(fetch_timestamp),
            block_number = VALUES(block_number)
    "#;

    let result = sqlx::query(query)
        .bind(&dex_state.trade_id)
        .bind(&dex_state.exchange)
        .bind(&dex_state.trade_pair)
        .bind(&dex_state.direction)
        .bind(dex_state.volume) // Convert u64 to i64 for BIGINT
        .bind(dex_state.price)
        .bind(dex_state.trade_time)
        .bind(dex_state.fetch_time)
        .bind(dex_state.block_number as i64) // Convert u64 to i64 for BIGINT
        .execute(pool)
        .await?;

    Ok(result.last_insert_id())
}

/// Get all DEX market records
pub async fn get_all_dex_markets(
    pool: &Pool<MySql>,
) -> Result<Vec<DEXState>, Box<dyn std::error::Error>> {
    let query = "SELECT id, trade_id, exchange, trade_pair, direction, volume, price, trade_timestamp, fetch_timestamp, block_number FROM dex_markets ORDER BY fetch_timestamp DESC";

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let mut dex_states = Vec::new();
    for row in rows {
        dex_states.push(DEXState {
            trade_id: row.get("trade_id"),
            exchange: row.get("exchange"),
            trade_pair: row.get("trade_pair"),
            direction: row.get("direction"),
            volume: row.get("volume"),
            price: row.get("price"),
            trade_time: row.get("trade_timestamp"),
            fetch_time: row.get("fetch_timestamp"),
            block_number: row.get::<i64, _>("block_number") as u64, // Convert i64 to u64
        });
    }

    Ok(dex_states)
}

/// Update existing DEX market record
pub async fn update_dex_market(
    pool: &Pool<MySql>,
    dex_state: &DEXState,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = r#"
        UPDATE dex_markets
        SET direction = ?, volume = ?, price = ?, trade_timestamp = ?, fetch_timestamp = ?, block_number = ?
        WHERE trade_id = ? AND exchange = ?
    "#;

    let result = sqlx::query(query)
        .bind(&dex_state.direction)
        .bind(&dex_state.volume)
        .bind(&dex_state.price)
        .bind(dex_state.trade_time)
        .bind(dex_state.fetch_time)
        .bind(&dex_state.block_number)
        .bind(&dex_state.trade_id)
        .bind(&dex_state.exchange)
        .execute(pool)
        .await?;

    if result.rows_affected() > 0 {
        warn!(
            "No DEX market record found to update: trade_id={}, exchange={}",
            dex_state.trade_id, dex_state.exchange
        );
    }

    Ok(())
}
