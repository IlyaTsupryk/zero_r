# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Zero-R is a Rust-based arbitrage service that monitors real-time market data from both centralized exchanges (CEX) and decentralized exchanges (DEX) to identify arbitrage opportunities. The system uses async Rust with Tokio for concurrent WebSocket connections and market data processing.

## Architecture

### Core Components

**Screeners** (`src/screeners/`): Async services that connect to exchange APIs and process real-time market data
- `BybitScreener`: Connects to Bybit WebSocket API, maintains orderbook state via delta updates, and persists CEX market snapshots
- `MeteoraScreener`: Placeholder for DEX integration (Solana/Meteora)
- Each screener runs in its own Tokio task and supports graceful shutdown via atomic flags

**Models** (`src/models/market.rs`): Core data structures for market representation
- `OrderBook`: Maintains sorted bids/asks with delta merge logic
- `OrderBookItem`: Price/volume pairs using `rust_decimal::Decimal` for precision
- `CEXState` / `DEXState`: Snapshots of market state with timestamps for persistence

**Store** (`src/store/`): Database layer using sqlx with MySQL
- `db.rs`: Connection pooling, auto-creates database if missing, runs init.sql migrations
- `markets.rs`: Insert operations for CEX/DEX market states
- `init.sql`: Schema definitions for `cex_markets` and `dex_markets` tables

**Main Loop** (`src/main.rs`): Application entry point
- Initializes database connection pool
- Spawns screener tasks concurrently using `tokio::spawn`
- Handles graceful shutdown on Ctrl+C by awaiting task completion

### Data Flow

1. Screeners connect to exchange WebSocket APIs
2. Real-time orderbook updates are received and merged (snapshot + delta)
3. Best bid/ask extracted from orderbook state
4. Market state snapshots spawned as async DB insert tasks
5. All persisted to MySQL with microsecond timestamp precision

### Key Design Patterns

- **Async-first**: All I/O uses `async/await` with Tokio runtime
- **Shared state**: `Arc<Mutex<HashMap>>` for orderbooks accessed across async contexts
- **Graceful shutdown**: `Arc<AtomicBool>` flags for coordinated task termination
- **Fire-and-forget persistence**: DB inserts spawned as separate tasks to avoid blocking screeners
- **Decimal precision**: `rust_decimal::Decimal` for all price/volume calculations

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the application (requires .env configuration)
cargo run

# Run with release optimizations
cargo run --release
```

### Testing
```bash
# Run all tests
cargo test

# Run a specific test
cargo test <test_name>

# Run tests for a specific module
cargo test screeners::bybit_tests
```

### Code Quality
```bash
# Check for compilation errors without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy
```

### Database Setup

The application auto-creates the database and runs migrations on startup. Ensure `.env` is configured:

```bash
# Copy example env file
cp .env.example .env

# Edit with your MySQL credentials
# DB_HOST, DB_PORT, DB_USER, DB_PASSWORD, DB_NAME
```

The database will be created automatically if it doesn't exist when running the application.

## Coding Conventions (from .cursor/rules/rust.mdc)

- Use `tokio` for all async operations and task management
- Leverage `tokio::spawn` for concurrent task spawning
- Use `tokio::select!` for managing multiple async tasks
- Implement timeouts, retries, and backoff for robust async operations
- Use `tokio::sync::mpsc` for async channels, `tokio::sync::Mutex` for shared state
- Embrace `Result` and `Option` types, propagate errors with `?`
- Use `anyhow` for flexible error handling in application code
- Write async tests with `#[tokio::test]`
- Use `tracing` for structured logging (not `println!`)
- Structure code into modules: separate networking, database, and business logic
- Use `dotenvy` for environment variable configuration

## Important Implementation Notes

- **Orderbook Merging**: The current implementation uses `Vec` with linear search and sorting. The code includes a TODO to migrate to `BTreeMap` for better performance on delta updates.
- **WebSocket Error Handling**: Bybit screener uses `panic!` for shutdown signal propagation in the WebSocket callback—this is intentional for the current `rust-bybit` API.
- **Test Organization**: Tests are in separate files (e.g., `bybit_tests.rs`) and imported via `#[cfg(test)] #[path = "..."] mod` pattern.
- **Database Precision**: All price/volume fields use `DECIMAL(32,16)` to match `rust_decimal::Decimal` precision requirements.
- **Timestamp Storage**: MySQL `DATETIME(6)` provides microsecond precision for both trade and fetch timestamps.
