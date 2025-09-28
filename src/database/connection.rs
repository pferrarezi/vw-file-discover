use crate::crypto;
use anyhow::{Context, Result};
use sqlx::{PgPool, Pool, Postgres};
use std::env;

/// Creates a database connection pool from encrypted environment variables
pub async fn create_connection_pool() -> Result<PgPool> {
    let connection_string = get_decrypted_connection_string()?;
    
    PgPool::connect(&connection_string)
        .await
        .context("Failed to connect to PostgreSQL database")
}

/// Functional approach to get and decrypt connection string
fn get_decrypted_connection_string() -> Result<String> {
    let secret_key = env::var("SECRET_KEY1")
        .context("SECRET_KEY1 environment variable not set")?;
    
    let encrypted_connection = env::var("PG_API_CONNECTION")
        .context("PG_API_CONNECTION environment variable not set")?;

    // Decode the secret key from base64
    let key = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &secret_key)
        .context("Failed to decode SECRET_KEY1 from base64")?;
    
    let key_string = String::from_utf8(key)
        .context("SECRET_KEY1 is not valid UTF-8")?;

    // Decrypt the connection string
    crypto::decrypt_from_base64_key(&key_string, &encrypted_connection)
        .context("Failed to decrypt database connection string")
}

/// Type alias for our database pool
pub type DbPool = Pool<Postgres>;