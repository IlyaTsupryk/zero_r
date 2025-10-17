use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tracing::info;

use bybit::WebSocketApiClient;
use bybit::ws::response::{BasePublicResponse, Orderbook, OrderbookItem, SpotPublicResponse};
use bybit::ws::spot;

use crate::models::market;
use crate::store::markets::insert_cex_market;

use anyhow::Result;

struct TradeConfig {
    pub depth: spot::OrderbookDepth,
    pub _bid_precision: u32,
    pub _ask_precision: u32,
}

fn get_trade_pairs() -> HashMap<String, TradeConfig> {
    let mut map = HashMap::new();
    map.insert(
        "TRUMPUSDC".to_string(),
        TradeConfig {
            depth: spot::OrderbookDepth::Level50,
            _bid_precision: 6,
            _ask_precision: 6,
        },
    );
    map.insert(
        "TRUMPUSDT".to_string(),
        TradeConfig {
            depth: spot::OrderbookDepth::Level50,
            _bid_precision: 6,
            _ask_precision: 6,
        },
    );
    map
}

/// Bybit exchange screener for real-time market data
pub struct BybitScreener {
    /// Database connection pool for storing market data
    db_pool: Pool<MySql>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Map of order books with symbol as key
    order_book_map: Arc<Mutex<HashMap<String, market::OrderBook>>>,
}

impl BybitScreener {
    /// Create a new BybitScreener instance
    pub fn new(db_pool: Pool<MySql>) -> Self {
        let order_book_map = Arc::new(Mutex::new(HashMap::new()));

        {
            let mut map = order_book_map.lock().unwrap();
            for symbol in get_trade_pairs().keys() {
                map.insert(symbol.to_string(), market::OrderBook::new("bybit", symbol));
            }
        }

        Self {
            db_pool,
            shutdown: Arc::new(AtomicBool::new(false)),
            order_book_map,
        }
    }

    /// Start the screener to read from WebSocket and process market data
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting Bybit screener...");

        let mut client = WebSocketApiClient::spot().build();

        for (symbol, conf) in get_trade_pairs() {
            client.subscribe_orderbook(symbol, conf.depth);
        }

        client.run(|msg: SpotPublicResponse| {
            if self.shutdown.load(Ordering::Relaxed) {
                panic!("Stop signal received!");
            }

            match msg {
                SpotPublicResponse::Orderbook(ob) => self.handle_orderbook(ob),
                _ => (),
            }
        })?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.shutdown.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn handle_orderbook(&self, msg: BasePublicResponse<Orderbook>) {
        let data = &msg.data;
        let symbol = data.s.to_string();
        let mut map = self.order_book_map.lock().unwrap();
        let orderbook = map.get_mut(&symbol).unwrap();

        self.merge_orderbook(orderbook, &msg.type_, &data.a, &data.b);

        self.save_order_book_state(msg.data.u.to_string(), orderbook.clone(), msg.ts);
    }

    fn merge_orderbook(
        &self,
        orderbook: &mut market::OrderBook,
        msg_type: &str,
        asks: &Vec<OrderbookItem>,
        bids: &Vec<OrderbookItem>,
    ) {
        // TODO: Improve merge algorithm. BTreeMap can be used for better performance.
        match msg_type {
            "snapshot" => {
                orderbook.bids = bids
                    .iter()
                    .map(|orderbook_item| {
                        market::OrderBookItem::new(orderbook_item.0, orderbook_item.1)
                    })
                    .collect();
                orderbook.asks = asks
                    .iter()
                    .map(|orderbook_item| {
                        market::OrderBookItem::new(orderbook_item.0, orderbook_item.1)
                    })
                    .collect();
            }
            "delta" => {
                for orderbook_item in bids {
                    market::OrderBook::merge_item(
                        &mut orderbook.bids,
                        orderbook_item.0,
                        orderbook_item.1,
                    );
                }
                if bids.len() > 0 {
                    orderbook.bids.sort_by(|a, b| b.price.cmp(&a.price));
                }

                for orderbook_item in asks {
                    market::OrderBook::merge_item(
                        &mut orderbook.asks,
                        orderbook_item.0,
                        orderbook_item.1,
                    );
                }
                if asks.len() > 0 {
                    orderbook.asks.sort_by(|a, b| a.price.cmp(&b.price));
                }
            }
            _ => {}
        }
    }

    fn save_order_book_state(&self, trade_id: String, orderbook: market::OrderBook, ts: u64) {
        let best_bid = &orderbook.bids[0];
        let best_ask = &orderbook.asks[0];
        let cex_state = market::CEXState {
            trade_id: trade_id,
            exchange: String::from("bybit"),
            trade_pair: orderbook.symbol,
            bid_price: best_bid.price,
            bid_volume: best_bid.volume,
            ask_price: best_ask.price,
            ask_volume: best_ask.volume,
            trade_time: DateTime::from_timestamp_millis(ts as i64).unwrap_or_else(Utc::now),
            fetch_time: Utc::now(),
        };
        cex_state.log();

        let db_pool = self.db_pool.clone();
        tokio::spawn(async move {
            let _ = insert_cex_market(&db_pool, &cex_state).await;
        });
    }
}

#[cfg(test)]
#[path = "bybit_tests.rs"]
mod bybit_tests;
