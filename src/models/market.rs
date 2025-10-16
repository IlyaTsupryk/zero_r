use chrono::{DateTime, Utc};
use tracing::info;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub exchange: String,
    pub symbol: String,
    pub last_update_ts: DateTime<Utc>,
    pub bids: Vec<OrderBookItem>,
    pub asks: Vec<OrderBookItem>,
}

impl OrderBook {
    pub fn new(exchange: &str, symbol: &str) -> Self {
        Self { 
            exchange: exchange.to_string(), 
            symbol: symbol.to_string(), 
            last_update_ts: Utc::now(), 
            bids: vec![], 
            asks: vec![] 
        }
    }

    pub fn log(&self) {
        info!("[{}] {}", self.exchange, self.symbol);
        info!(" bids:");
        for bid in &self.bids {
            info!("     price={} volume={}", bid.price, bid.volume);
        }
        info!(" asks:");
        for ask in &self.asks {
            info!("     price={} volume={}", ask.price, ask.volume);
        }
    }

    pub fn merge_item(items: &mut Vec<OrderBookItem>, price: &str, volume: &str) {
        let price_dec = price.parse::<Decimal>().unwrap();
        if volume == "0" {
            items.retain(|item| item.price != price_dec);
        } else {
            let volume_dec = volume.parse::<Decimal>().unwrap();
            if let Some(item) = items.iter_mut().find(|item| item.price == price_dec) {
                item.volume = volume_dec;
            } else {
                items.push(OrderBookItem { price: price_dec, volume: volume_dec });
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookItem {
    pub price: Decimal,
    pub volume: Decimal,
}

impl OrderBookItem {
    pub fn new(price: &str, volume: &str) -> Self {
        Self { 
            price: price.parse::<Decimal>().unwrap(),
            volume: volume.parse::<Decimal>().unwrap(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CEXState {
    pub trade_id: String,
    pub exchange: String,
    pub trade_pair: String,
    pub bid_price: Decimal,
    pub bid_volume: Decimal,
    pub ask_price: Decimal,
    pub ask_volume: Decimal,
    pub trade_time: DateTime<Utc>,
    pub fetch_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DEXState {
    pub trade_id: String,
    pub exchange: String,
    pub trade_pair: String,
    pub direction: String,
    pub price: Decimal,
    pub volume: Decimal,
    pub trade_time: DateTime<Utc>,
    pub fetch_time: DateTime<Utc>,
    pub block_number: u64,
}

impl CEXState {
    pub fn log(&self) {
        info!(
            "[{}] {} bid price={} volume={} ask price={} volume={}",
            self.exchange,
            self.trade_pair,
            self.bid_price,
            self.bid_volume,
            self.ask_price,
            self.ask_volume,
        );
    }
}

impl DEXState {
    pub fn log(&self) {
        info!(
            "[{}] {} {} price={} volume={}",
            self.exchange,
            self.trade_pair,
            self.direction,
            self.price,
            self.volume,
        );
    }
}