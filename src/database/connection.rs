use crate::crypto;
use anyhow::{Context, Result};
use sqlx::{PgPool, Pool, Postgres};
use std::collections::HashMap;
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
    let decripted = crypto::decrypt_from_base64_key(&key_string, &encrypted_connection)
        .context("Failed to decrypt database connection string")?;

    transform_ado_net_to_postgres(&decripted)
}

/// Transforms ADO.NET connection string format to PostgreSQL URL format
/// 
/// Example input (ADO.NET): "Server=localhost;Database=mydb;User Id=user;Password=pass;Port=5432;"
/// Example output (PostgreSQL): "postgresql://user:pass@localhost:5432/mydb"
fn transform_ado_net_to_postgres(ado_connection_string: &str) -> Result<String> {
    let params = parse_ado_net_connection_string(ado_connection_string)?;
    
    let server = params.get("Server")
        .or_else(|| params.get("Host"))
        .or_else(|| params.get("Data Source"))
        .context("Server/Host not found in connection string")?;
    
    let database = params.get("Database")
        .or_else(|| params.get("Initial Catalog"))
        .context("Database not found in connection string")?;
    
    let username = params.get("User Id")
        .or_else(|| params.get("UserId"))
        .or_else(|| params.get("Username"))
        .or_else(|| params.get("User"))
        .context("Username not found in connection string")?;
    
    let password = params.get("Password")
        .or_else(|| params.get("Pwd"))
        .context("Password not found in connection string")?;
    
    let default_port = "5432".to_string();
    let port = params.get("Port").unwrap_or(&default_port);

    // Build PostgreSQL URL: postgresql://username:password@host:port/database
    let postgres_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        urlencoding::encode(username),
        urlencoding::encode(password),
        server,
        port,
        database
    );

    Ok(postgres_url)
}

/// Parses ADO.NET connection string into key-value pairs
/// 
/// Handles various formats:
/// - "Key=Value;Key2=Value2;"
/// - "Key=Value; Key2=Value2" (spaces)
/// - "Key='Value with spaces';Key2=Value2"
/// - "Key=\"Value with quotes\";Key2=Value2"
fn parse_ado_net_connection_string(connection_string: &str) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();
    
    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut in_key = true;
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut chars = connection_string.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '=' if in_key && !in_quotes => {
                in_key = false;
                current_value.clear();
            },
            ';' if !in_quotes => {
                if !current_key.trim().is_empty() && !current_value.trim().is_empty() {
                    params.insert(
                        current_key.trim().to_string(),
                        current_value.trim().to_string()
                    );
                }
                current_key.clear();
                current_value.clear();
                in_key = true;
            },
            '\'' | '"' if !in_key => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quotes = false;
                } else {
                    current_value.push(ch);
                }
            },
            _ => {
                if in_key {
                    current_key.push(ch);
                } else {
                    if in_quotes || (!ch.is_whitespace() || !current_value.is_empty()) {
                        current_value.push(ch);
                    }
                }
            }
        }
    }
    
    // Handle last pair if no trailing semicolon
    if !current_key.trim().is_empty() && !current_value.trim().is_empty() {
        params.insert(
            current_key.trim().to_string(),
            current_value.trim().to_string()
        );
    }
    
    if params.is_empty() {
        anyhow::bail!("No valid key-value pairs found in connection string");
    }
    
    Ok(params)
}

/// Type alias for our database pool
pub type DbPool = Pool<Postgres>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_connection_string() {
        let cs = "Server=localhost;Database=testdb;User Id=testuser;Password=testpass;Port=5432;";
        let params = parse_ado_net_connection_string(cs).unwrap();
        
        assert_eq!(params.get("Server"), Some(&"localhost".to_string()));
        assert_eq!(params.get("Database"), Some(&"testdb".to_string()));
        assert_eq!(params.get("User Id"), Some(&"testuser".to_string()));
        assert_eq!(params.get("Password"), Some(&"testpass".to_string()));
        assert_eq!(params.get("Port"), Some(&"5432".to_string()));
    }

    #[test]
    fn test_parse_quoted_values() {
        let cs = "Server=localhost;Database='test db';User Id=\"test user\";Password='test@pass';";
        let params = parse_ado_net_connection_string(cs).unwrap();
        
        assert_eq!(params.get("Database"), Some(&"test db".to_string()));
        assert_eq!(params.get("User Id"), Some(&"test user".to_string()));
        assert_eq!(params.get("Password"), Some(&"test@pass".to_string()));
    }

    #[test]
    fn test_transform_to_postgres_url() {
        let cs = "Server=localhost;Database=mydb;User Id=myuser;Password=mypass;Port=5432;";
        let url = transform_ado_net_to_postgres(cs).unwrap();
        
        assert_eq!(url, "postgresql://myuser:mypass@localhost:5432/mydb");
    }

    #[test]
    fn test_transform_with_special_chars() {
        let cs = "Server=localhost;Database=mydb;User Id=my@user;Password=my:pass;Port=5432;";
        let url = transform_ado_net_to_postgres(cs).unwrap();
        
        assert_eq!(url, "postgresql://my%40user:my%3Apass@localhost:5432/mydb");
    }

    #[test]
    fn test_transform_default_port() {
        let cs = "Server=localhost;Database=mydb;User Id=myuser;Password=mypass;";
        let url = transform_ado_net_to_postgres(cs).unwrap();
        
        assert_eq!(url, "postgresql://myuser:mypass@localhost:5432/mydb");
    }

    #[test]
    fn test_alternative_parameter_names() {
        let cs = "Host=localhost;Initial Catalog=mydb;Username=myuser;Pwd=mypass;";
        let url = transform_ado_net_to_postgres(cs).unwrap();
        
        assert_eq!(url, "postgresql://myuser:mypass@localhost:5432/mydb");
    }
}