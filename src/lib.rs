pub mod crypto;
pub mod database;
pub mod models;
pub mod services;
pub mod utils;

// Re-export commonly used types
pub use database::{create_connection_pool, DbPool};
pub use models::{FileTrace, FileTraceStatus, FvwArqDiarioExt};
pub use services::{
    copy_files_for_revendas, discover_and_register_files, FileCopyConfig, FileCopyReport,
    FileDiscoveryConfig, FileDiscoveryReport,
};

// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub file_copy: FileCopyConfig,
    pub file_discovery: FileDiscoveryConfig,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            file_copy: FileCopyConfig::default(),
            file_discovery: FileDiscoveryConfig::default(),
            log_level: "info".to_string(),
        }
    }
}
