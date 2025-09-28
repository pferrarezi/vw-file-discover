use crate::database::DbPool;
use crate::models::{create_file_trace_from_path, FileTrace, FvwArqDiarioExt};
use crate::utils::list_files_with_extensions;
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn, error};

/// Configuration for file discovery operations
#[derive(Debug, Clone)]
pub struct FileDiscoveryConfig {
    pub batch_size: usize,
    pub parallel_processing: bool,
}

impl Default for FileDiscoveryConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            parallel_processing: true,
        }
    }
}

/// Pure function to extract output directories from revendas
pub fn extract_output_directories(revendas: &[FvwArqDiarioExt]) -> Vec<String> {
    revendas
        .iter()
        .map(|revenda| &revenda.pasta_output)
        .filter(|dir| !dir.is_empty())
        .cloned()
        .collect()
}

/// Pure function to extract unique file extensions from revendas
pub fn extract_unique_extensions(revendas: &[FvwArqDiarioExt]) -> Vec<String> {
    let mut extensions: Vec<String> = revendas
        .iter()
        .map(|revenda| &revenda.extensao)
        .filter(|ext| !ext.is_empty())
        .cloned()
        .collect();
    
    extensions.sort();
    extensions.dedup();
    extensions
}

/// Main file discovery operation - functional composition
pub async fn discover_and_register_files(
    pool: &DbPool,
    config: FileDiscoveryConfig,
) -> Result<FileDiscoveryReport> {
    info!("Starting file discovery and registration...");

    // Get revendas data
    let revendas = crate::database::arq_vw_ext::get_revendas(pool).await?;
    
    if revendas.is_empty() {
        warn!("No revendas found in database");
        return Ok(FileDiscoveryReport::empty());
    }

    // Extract configuration data functionally
    let output_directories = extract_output_directories(&revendas);
    let extensions = extract_unique_extensions(&revendas);

    info!("Scanning {} directories for {} file extensions", 
          output_directories.len(), extensions.len());
    info!("Extensions: {:?}", extensions);

    // Discover files across all directories
    let discovered_files = discover_files_in_directories(&output_directories, &extensions)?;
    
    info!("Discovered {} files", discovered_files.len());

    if discovered_files.is_empty() {
        info!("No files found for processing");
        return Ok(FileDiscoveryReport::empty());
    }

    let discovered_count = discovered_files.len();

    // Process files to create FileTrace objects
    let file_traces = process_files_to_traces(discovered_files).await;
    let successful_traces: Vec<FileTrace> = file_traces
        .into_iter()
        .filter_map(|result| match result {
            Ok(trace) => Some(trace),
            Err(e) => {
                error!("Failed to process file: {}", e);
                None
            }
        })
        .collect();

    info!("Successfully processed {} files", successful_traces.len());

    // Save to database in batches
    let saved_count = save_file_traces_in_batches(pool, &successful_traces, config.batch_size).await?;

    let report = FileDiscoveryReport {
        files_discovered: discovered_count,
        files_processed: successful_traces.len(),
        files_saved: saved_count,
        processing_errors: discovered_count - successful_traces.len(),
    };

    info!(
        "File discovery completed. Discovered: {}, Processed: {}, Saved: {}, Errors: {}",
        report.files_discovered,
        report.files_processed,
        report.files_saved,
        report.processing_errors
    );

    Ok(report)
}

/// Discover files in multiple directories
fn discover_files_in_directories(
    directories: &[String],
    extensions: &[String],
) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    for directory in directories {
        match list_files_with_extensions(directory, extensions, None) {
            Ok(mut files) => {
                info!("Found {} files in directory: {}", files.len(), directory);
                all_files.append(&mut files);
            }
            Err(e) => {
                warn!("Failed to scan directory {}: {}", directory, e);
                // Continue processing other directories
            }
        }
    }

    Ok(all_files)
}

/// Process discovered files into FileTrace objects
async fn process_files_to_traces(files: Vec<PathBuf>) -> Vec<Result<FileTrace>> {
    // In a real-world scenario, you might want to use tokio::task::spawn_blocking
    // for CPU-intensive file processing to avoid blocking the async runtime
    
    let mut results = Vec::with_capacity(files.len());
    
    for file_path in files {
        let result = tokio::task::spawn_blocking(move || {
            create_file_trace_from_path(file_path)
        })
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("Task join error: {}", e)));
        
        results.push(result);
    }
    
    results
}

/// Save file traces to database in batches
async fn save_file_traces_in_batches(
    pool: &DbPool,
    file_traces: &[FileTrace],
    batch_size: usize,
) -> Result<usize> {
    let mut total_saved = 0;

    for batch in file_traces.chunks(batch_size) {
        match crate::database::file_trace::save_batch(pool, batch).await {
            Ok(saved) => {
                total_saved += saved as usize;
                info!("Saved batch of {} file traces to database", saved);
            }
            Err(e) => {
                error!("Failed to save batch to database: {}", e);
                // Continue with next batch
            }
        }
    }

    Ok(total_saved)
}

/// Report structure for file discovery operations
#[derive(Debug, Clone)]
pub struct FileDiscoveryReport {
    pub files_discovered: usize,
    pub files_processed: usize,
    pub files_saved: usize,
    pub processing_errors: usize,
}

impl FileDiscoveryReport {
    pub fn empty() -> Self {
        Self {
            files_discovered: 0,
            files_processed: 0,
            files_saved: 0,
            processing_errors: 0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.files_discovered == 0 {
            0.0
        } else {
            self.files_processed as f64 / self.files_discovered as f64
        }
    }

    pub fn save_rate(&self) -> f64 {
        if self.files_processed == 0 {
            0.0
        } else {
            self.files_saved as f64 / self.files_processed as f64
        }
    }
}

/// Functional utility functions for file discovery
pub mod functional {
    use super::*;

    /// Higher-order function to create a directory filter
    pub fn create_directory_filter(
        allowed_patterns: Vec<String>,
    ) -> impl Fn(&str) -> bool {
        move |directory: &str| -> bool {
            allowed_patterns.iter().any(|pattern| directory.contains(pattern))
        }
    }

    /// Map files to their processing results with a custom processor
    pub fn map_files_with_processor<F, T>(
        files: Vec<PathBuf>,
        processor: F,
    ) -> Vec<T>
    where
        F: Fn(PathBuf) -> T,
    {
        files.into_iter().map(processor).collect()
    }

    /// Filter and transform file traces based on criteria
    pub fn filter_and_transform_traces<F, P, T>(
        traces: Vec<FileTrace>,
        predicate: P,
        transformer: F,
    ) -> Vec<T>
    where
        P: Fn(&FileTrace) -> bool,
        F: Fn(FileTrace) -> T,
    {
        traces
            .into_iter()
            .filter(predicate)
            .map(transformer)
            .collect()
    }

    /// Reduce file processing results to summary statistics
    pub fn reduce_processing_results<T, E>(
        results: Vec<Result<T, E>>,
    ) -> (Vec<T>, Vec<E>) {
        results.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut successes, mut errors), result| {
                match result {
                    Ok(value) => successes.push(value),
                    Err(error) => errors.push(error),
                }
                (successes, errors)
            },
        )
    }
}