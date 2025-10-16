use sqlx::{MySql, MySqlPool, Pool};
use std::env;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl DatabaseConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            host: env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("DB_PORT")
                .unwrap_or_else(|_| "3306".to_string())
                .parse()
                .map_err(|_| "Invalid DB_PORT")?,
            username: env::var("DB_USER").unwrap_or_else(|_| "root".to_string()),
            password: env::var("DB_PASSWORD").unwrap_or_else(|_| "".to_string()),
            database: env::var("DB_NAME").unwrap_or_else(|_| "zero".to_string()),
        })
    }

    /// Build database URL for sqlx
    pub fn database_url(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }

    /// Build database URL without specific database (for database creation)
    pub fn server_url(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

/// Database connection pool type alias
pub type DatabasePool = Pool<MySql>;

/// Initialize database connection and setup
pub async fn init_database() -> Result<DatabasePool, Box<dyn std::error::Error>> {
    let config = DatabaseConfig::from_env().map_err(|e| {
        error!("Failed to load database configuration: {}", e);
        e
    })?;

    info!(
        "Connecting to database: {}@{}:{}/{}",
        config.username, config.host, config.port, config.database
    );

    let server_pool = MySqlPool::connect(&config.server_url())
        .await
        .map_err(|e| {
            error!("Failed to connect to MySQL server: {}", e);
            e
        })?;

    let db_exists = database_exists(&server_pool, &config.database).await?;
    if !db_exists {
        warn!(
            "Database '{}' does not exist, creating it...",
            config.database
        );
        create_database(&server_pool, &config.database).await?;
        info!("Database '{}' created successfully", config.database);
    } else {
        info!("Database '{}' already exists", config.database);
    }
    server_pool.close().await;

    let pool = MySqlPool::connect(&config.database_url())
        .await
        .map_err(|e| {
            error!("Failed to connect to database '{}': {}", config.database, e);
            e
        })?;
    run_init_script(&pool).await?;

    // Perform final health check
    match health_check(&pool).await {
        Ok(true) => info!("✅ Database connection verified"),
        Ok(false) => {
            error!("❌ Database health check failed after initialization");
            return Err("Database health check failed after initialization".into());
        }
        Err(e) => {
            error!("❌ Database health check error after initialization: {}", e);
            return Err(e.into());
        }
    }

    info!("🎯 Database initialization completed successfully");
    Ok(pool)
}

/// Check if database exists
async fn database_exists(
    pool: &DatabasePool,
    database_name: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let query = "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = ?";
    let exists: Option<String> = sqlx::query_scalar(query)
        .bind(database_name)
        .fetch_optional(pool)
        .await?;

    Ok(exists.is_some())
}

/// Create database if it doesn't exist
async fn create_database(
    pool: &DatabasePool,
    database_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = format!(
        "CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        database_name
    );
    sqlx::query(&query).execute(pool).await?;

    Ok(())
}

/// Run the initialization SQL script
async fn run_init_script(pool: &DatabasePool) -> Result<(), Box<dyn std::error::Error>> {
    let init_sql = include_str!("../store/init.sql");

    // Split the SQL script by semicolons and execute each statement
    let statements: Vec<&str> = init_sql
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for statement in statements {
        if !statement.is_empty() {
            sqlx::query(statement).execute(pool).await.map_err(|e| {
                error!("Failed to execute SQL statement: {}", e);
                error!("Statement: {}", statement);
                e
            })?;
        }
    }

    info!("Initialization SQL script executed successfully");
    Ok(())
}

/// Get a database connection from the pool
pub async fn get_connection(
    pool: &DatabasePool,
) -> Result<sqlx::pool::PoolConnection<MySql>, sqlx::Error> {
    pool.acquire().await
}

/// Health check function
pub async fn health_check(pool: &DatabasePool) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("SELECT 1").fetch_one(pool).await;

    match result {
        Ok(_) => Ok(true),
        Err(e) => {
            error!("Database health check failed: {}", e);
            Err(e)
        }
    }
}
