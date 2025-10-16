use std::env;
use std::sync::OnceLock;
use tracing_appender::rolling;
use tracing_subscriber::fmt::{self, time::ChronoUtc};
use tracing_subscriber::prelude::*;

static FILE_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

pub fn init_logging() {
    // Set default log level (overridden by RUST_LOG or LOG_LEVEL)
    let env_filter = env::var("RUST_LOG")
        .or_else(|_| env::var("LOG_LEVEL"))
        .unwrap_or_else(|_| "info".into());

    // Create a daily rolling file appender
    let file_appender = rolling::daily("logs", "app.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = FILE_GUARD.set(guard);

    let timer = ChronoUtc::new("%Y-%m-%d %H:%M:%S%.3f".into());

    // Console layer (pretty, colored)
    let console_layer = fmt::layer()
        .with_timer(timer.clone())
        .with_target(false)
        .with_level(true)
        .with_writer(std::io::stdout);

    // File layer (same format, no colors)
    let file_layer = fmt::layer()
        .with_timer(timer)
        .with_target(false)
        .with_level(true)
        .with_ansi(false)
        .with_writer(file_writer);

    // Build the subscriber
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(tracing_subscriber::EnvFilter::new(env_filter))
        .init();
}
