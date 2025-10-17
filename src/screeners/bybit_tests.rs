use super::*;
use bybit::ws::response::OrderbookItem as WsOrderbookItem;
use rust_decimal::Decimal;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

fn make_ws_item(price: &'static str, volume: &'static str) -> WsOrderbookItem<'static> {
    WsOrderbookItem(price, volume)
}

fn make_market_item(price: &str, volume: &str) -> market::OrderBookItem {
    market::OrderBookItem::new(price, volume)
}

fn build_screener() -> BybitScreener {
    let options = MySqlConnectOptions::from_str("mysql://user:pass@localhost/db").unwrap();
    let pool = MySqlPoolOptions::new().connect_lazy_with(options);

    BybitScreener {
        db_pool: pool,
        shutdown: Arc::new(AtomicBool::new(false)),
        order_book_map: Arc::new(Mutex::new(HashMap::new())),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn merge_orderbook_snapshot_populates_empty_book() {
    let screener = build_screener();
    let mut orderbook = market::OrderBook::new("bybit", "TEST");

    let asks = vec![make_ws_item("101.0", "1.0"), make_ws_item("102.0", "2.0")];
    let bids = vec![make_ws_item("100.0", "1.5"), make_ws_item("99.5", "0.5")];

    screener.merge_orderbook(&mut orderbook, "snapshot", &asks, &bids);

    assert_eq!(orderbook.bids.len(), 2);
    assert_eq!(orderbook.bids[0].price, decimal("100.0"));
    assert_eq!(orderbook.bids[0].volume, decimal("1.5"));
    assert_eq!(orderbook.bids[1].price, decimal("99.5"));
    assert_eq!(orderbook.asks.len(), 2);
    assert_eq!(orderbook.asks[0].price, decimal("101.0"));
    assert_eq!(orderbook.asks[1].price, decimal("102.0"));
}

#[tokio::test(flavor = "current_thread")]
async fn merge_orderbook_snapshot_overwrites_existing_levels() {
    let screener = build_screener();
    let mut orderbook = market::OrderBook::new("bybit", "TEST");
    orderbook.bids = vec![make_market_item("90.0", "4.0")];
    orderbook.asks = vec![make_market_item("110.0", "1.0")];

    let asks = vec![make_ws_item("105.0", "3.0")];
    let bids = vec![make_ws_item("95.0", "2.5")];

    screener.merge_orderbook(&mut orderbook, "snapshot", &asks, &bids);

    assert_eq!(orderbook.bids.len(), 1);
    assert_eq!(orderbook.bids[0].price, decimal("95.0"));
    assert_eq!(orderbook.asks.len(), 1);
    assert_eq!(orderbook.asks[0].price, decimal("105.0"));
}

#[tokio::test(flavor = "current_thread")]
async fn merge_orderbook_delta_removes_levels_with_zero_volume() {
    let screener = build_screener();
    let mut orderbook = market::OrderBook::new("bybit", "TEST");
    orderbook.bids = vec![make_market_item("100.0", "1.0")];
    orderbook.asks = vec![make_market_item("101.0", "1.5")];

    let asks = vec![make_ws_item("101.0", "0")];
    let bids = vec![make_ws_item("100.0", "0")];

    screener.merge_orderbook(&mut orderbook, "delta", &asks, &bids);

    assert!(orderbook.bids.is_empty());
    assert!(orderbook.asks.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn merge_orderbook_delta_updates_and_inserts_levels() {
    let screener = build_screener();
    let mut orderbook = market::OrderBook::new("bybit", "TEST");
    orderbook.bids = vec![make_market_item("100.0", "1.0")];
    orderbook.asks = vec![make_market_item("101.0", "1.0")];

    let bids = vec![make_ws_item("100.0", "2.0"), make_ws_item("99.0", "3.0")];
    let asks = vec![make_ws_item("101.0", "1.5"), make_ws_item("102.0", "0.5")];

    screener.merge_orderbook(&mut orderbook, "delta", &asks, &bids);

    assert_eq!(orderbook.bids.len(), 2);
    assert_eq!(orderbook.bids[0].price, decimal("100.0"));
    assert_eq!(orderbook.bids[0].volume, decimal("2.0"));
    assert_eq!(orderbook.bids[1].price, decimal("99.0"));
    assert_eq!(orderbook.bids[1].volume, decimal("3.0"));

    assert_eq!(orderbook.asks.len(), 2);
    assert_eq!(orderbook.asks[0].price, decimal("101.0"));
    assert_eq!(orderbook.asks[0].volume, decimal("1.5"));
    assert_eq!(orderbook.asks[1].price, decimal("102.0"));
    assert_eq!(orderbook.asks[1].volume, decimal("0.5"));
}

#[tokio::test(flavor = "current_thread")]
async fn merge_orderbook_delta_handles_mixed_zero_and_non_zero_updates() {
    let screener = build_screener();
    let mut orderbook = market::OrderBook::new("bybit", "TEST");
    orderbook.bids = vec![
        make_market_item("101.0", "1.0"),
        make_market_item("100.0", "1.0"),
    ];
    orderbook.asks = vec![
        make_market_item("102.0", "2.0"),
        make_market_item("103.0", "2.5"),
    ];

    let bids = vec![
        make_ws_item("101.0", "0"),
        make_ws_item("100.0", "2.0"),
        make_ws_item("99.0", "4.0"),
    ];
    let asks = vec![
        make_ws_item("103.0", "0"),
        make_ws_item("102.0", "1.5"),
        make_ws_item("104.0", "1.0"),
    ];

    screener.merge_orderbook(&mut orderbook, "delta", &asks, &bids);

    assert_eq!(orderbook.bids.len(), 2);
    assert_eq!(orderbook.bids[0].price, decimal("100.0"));
    assert_eq!(orderbook.bids[0].volume, decimal("2.0"));
    assert_eq!(orderbook.bids[1].price, decimal("99.0"));
    assert_eq!(orderbook.bids[1].volume, decimal("4.0"));

    assert_eq!(orderbook.asks.len(), 2);
    assert_eq!(orderbook.asks[0].price, decimal("102.0"));
    assert_eq!(orderbook.asks[0].volume, decimal("1.5"));
    assert_eq!(orderbook.asks[1].price, decimal("104.0"));
    assert_eq!(orderbook.asks[1].volume, decimal("1.0"));
}
