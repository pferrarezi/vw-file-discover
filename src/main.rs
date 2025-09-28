use anyhow::Result;
use clap::{Arg, Command};
use std::env;
use tracing::{info, error, Level};
use tracing_subscriber::EnvFilter;
use vw_file_discover::{
    create_connection_pool, copy_files_for_revendas, discover_and_register_files,
    AppConfig, FileCopyConfig, FileDiscoveryConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("VW File Discover")
        .version("1.0")
        .author("Your Name")
        .about("Functional file discovery and processing application")
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .help("Set the log level (trace, debug, info, warn, error)")
                .default_value("info"),
        )
        .arg(
            Arg::new("days-back")
                .long("days-back")
                .value_name("DAYS")
                .help("Number of days back to look for files")
                .default_value("15"),
        )
        .arg(
            Arg::new("batch-size")
                .long("batch-size")
                .value_name("SIZE")
                .help("Batch size for database operations")
                .default_value("1000"),
        )
        .arg(
            Arg::new("skip-copy")
                .long("skip-copy")
                .help("Skip file copying phase")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("skip-discovery")
                .long("skip-discovery")
                .help("Skip file discovery phase")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize configuration from command line arguments
    let config = create_app_config(&matches)?;

    // Initialize logging
    initialize_logging(&config.log_level)?;

    // Load environment variables
    load_environment_variables()?;

    // Run the application
    run_application(config).await
}

/// Pure function to create application configuration from CLI arguments
fn create_app_config(matches: &clap::ArgMatches) -> Result<AppConfig> {
    let log_level = matches
        .get_one::<String>("log-level")
        .unwrap()
        .clone();

    let days_back: i64 = matches
        .get_one::<String>("days-back")
        .unwrap()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid days-back value"))?;

    let batch_size: usize = matches
        .get_one::<String>("batch-size")
        .unwrap()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid batch-size value"))?;

    Ok(AppConfig {
        file_copy: FileCopyConfig {
            days_back,
            overwrite: false,
        },
        file_discovery: FileDiscoveryConfig {
            batch_size,
            parallel_processing: true,
        },
        log_level,
    })
}

/// Initialize structured logging with tracing
fn initialize_logging(log_level: &str) -> Result<()> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(level.into())
        .add_directive("sqlx=warn".parse()?)
        .add_directive("hyper=warn".parse()?)
        .add_directive("rustls=warn".parse()?);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    Ok(())
}

/// Load and validate environment variables
fn load_environment_variables() -> Result<()> {
    // Load .env file if it exists
    if let Err(_) = dotenvy::dotenv() {
        info!("No .env file found, using system environment variables");
    }

    // Validate required environment variables
    let required_vars = ["SECRET_KEY1", "PG_API_CONNECTION"];
    
    for var in &required_vars {
        match env::var(var) {
            Ok(value) if !value.is_empty() => {
                info!("Environment variable {} is set", var);
            }
            _ => {
                anyhow::bail!("Required environment variable {} is not set or empty", var);
            }
        }
    }

    Ok(())
}

/// Main application logic with functional composition
async fn run_application(config: AppConfig) -> Result<()> {
    info!("Starting VW File Discover application");
    info!("Configuration: {:#?}", config);

    // Create database connection pool
    let pool = create_connection_pool().await?;
    info!("Database connection established");

    // Phase 1: File copying (if not skipped)
    let copy_report = copy_files_for_revendas(&pool, config.file_copy).await?;
    print_copy_report(&copy_report);

    // Phase 2: File discovery and registration (if not skipped)  
    let discovery_report = discover_and_register_files(&pool, config.file_discovery).await?;
    print_discovery_report(&discovery_report);

    // Final summary
    print_final_summary(&copy_report, &discovery_report);

    info!("Application completed successfully");
    Ok(())
}

/// Print file copy report in a functional manner
fn print_copy_report(report: &vw_file_discover::FileCopyReport) {
    info!("=== FILE COPY REPORT ===");
    info!("Total files processed: {}", report.total_processed());
    info!("Successfully copied: {}", report.successful_copies);
    info!("Skipped files: {}", report.skipped_files);
    info!("Copy errors: {}", report.errors.len());
    info!("Success rate: {:.2}%", report.success_rate() * 100.0);

    if !report.errors.is_empty() {
        error!("Copy errors encountered:");
        for error in &report.errors {
            error!("  {} -> {}: {}", error.source, error.destination, error.error);
        }
    }
}

/// Print file discovery report in a functional manner
fn print_discovery_report(report: &vw_file_discover::FileDiscoveryReport) {
    info!("=== FILE DISCOVERY REPORT ===");
    info!("Files discovered: {}", report.files_discovered);
    info!("Files processed: {}", report.files_processed);
    info!("Files saved to database: {}", report.files_saved);
    info!("Processing errors: {}", report.processing_errors);
    info!("Processing success rate: {:.2}%", report.success_rate() * 100.0);
    info!("Database save rate: {:.2}%", report.save_rate() * 100.0);
}

/// Print final application summary
fn print_final_summary(
    copy_report: &vw_file_discover::FileCopyReport,
    discovery_report: &vw_file_discover::FileDiscoveryReport,
) {
    info!("=== FINAL SUMMARY ===");
    info!("Files copied: {}", copy_report.successful_copies);
    info!("Files discovered: {}", discovery_report.files_discovered);
    info!("Files registered in database: {}", discovery_report.files_saved);
    
    let total_success = copy_report.successful_copies + discovery_report.files_saved;
    info!("Total successful operations: {}", total_success);
}
