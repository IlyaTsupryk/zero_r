CREATE TABLE IF NOT EXISTS `cex_markets` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `trade_id` VARCHAR(128) NOT NULL,
  `exchange` VARCHAR(64) NOT NULL,
  `trade_pair` VARCHAR(64) NOT NULL,
  `bid_price` DECIMAL(32,16) NOT NULL,
  `bid_volume` DECIMAL(32,16) NOT NULL,
  `ask_price` DECIMAL(32,16) NOT NULL,
  `ask_volume` DECIMAL(32,16) NOT NULL,
  `trade_timestamp` DATETIME(6) NOT NULL,
  `fetch_timestamp` DATETIME(6) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_orders_trade_id_exchange` (`trade_id`, `exchange`),
  KEY `idx_orders_exchange_symbol_ts` (`exchange`, `trade_pair`, `trade_timestamp`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `dex_markets` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `trade_id` VARCHAR(128) NOT NULL,
  `exchange` VARCHAR(64) NOT NULL,
  `trade_pair` VARCHAR(64) NOT NULL,
  `direction` VARCHAR(16) NOT NULL,
  `volume` DECIMAL(32,16) NOT NULL,
  `price` DECIMAL(32,16) NOT NULL,
  `trade_timestamp` DATETIME(6) NOT NULL,
  `fetch_timestamp` DATETIME(6) NOT NULL,
  `block_number` BIGINT UNSIGNED NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_orders_trade_id_exchange` (`trade_id`, `exchange`),
  KEY `idx_orders_exchange_symbol_ts` (`exchange`, `trade_pair`, `trade_timestamp`),
  KEY `idx_orders_direction` (`direction`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
