use crate::database::DbPool;
use crate::models::FvwArqDiarioExt;
use crate::utils::{copy_files_batch, CopyResult};
use anyhow::Result;
use std::path::Path;
use tracing::{info, warn, error};

/// Configuration for file copying operations
#[derive(Debug, Clone)]
pub struct FileCopyConfig {
    pub days_back: i64,
    pub overwrite: bool,
}

impl Default for FileCopyConfig {
    fn default() -> Self {
        Self {
            days_back: 15,
            overwrite: false,
        }
    }
}

/// Pure function to extract file extensions from revendas
pub fn extract_file_extensions(revendas: &[FvwArqDiarioExt]) -> Vec<String> {
    revendas
        .iter()
        .map(|revenda| &revenda.extensao)
        .filter(|ext| !ext.is_empty())
        .cloned()
        .collect()
}

/// Pure function to create copy path mappings from revendas
pub fn create_copy_mappings(revendas: &[FvwArqDiarioExt]) -> Vec<(String, String)> {
    revendas
        .iter()
        .map(|revenda| (revenda.pasta_input.clone(), revenda.pasta_output.clone()))
        .filter(|(input, output)| !input.is_empty() && !output.is_empty())
        .collect()
}

/// Main file copy operation - functional composition
pub async fn copy_files_for_revendas(
    pool: &DbPool,
    config: FileCopyConfig,
) -> Result<FileCopyReport> {
    info!("Starting file copy process...");

    // Get revendas data
    let revendas = crate::database::arq_vw_ext::get_revendas(pool).await?;
    
    if revendas.is_empty() {
        warn!("No revendas found in database");
        return Ok(FileCopyReport::empty());
    }

    // Extract configuration data functionally
    let extensions = extract_file_extensions(&revendas);
    let mappings = create_copy_mappings(&revendas);

    info!("Found {} revendas with {} unique extensions", revendas.len(), extensions.len());
    info!("Processing {} directory mappings", mappings.len());

    // Convert string pairs to Path references for the copy function
    let path_mappings: Vec<(&Path, &Path)> = mappings
        .iter()
        .map(|(input, output)| (Path::new(input), Path::new(output)))
        .collect();

    // Perform batch copy operation
    let copy_results = copy_files_batch(
        &path_mappings,
        &extensions,
        Some(config.days_back),
        config.overwrite,
    )?;

    // Create report from results
    let report = create_copy_report(copy_results);
    
    info!(
        "File copy completed. Success: {}, Skipped: {}, Errors: {}",
        report.successful_copies,
        report.skipped_files,
        report.errors.len()
    );

    Ok(report)
}

/// Create a comprehensive report from copy results
fn create_copy_report(results: Vec<CopyResult>) -> FileCopyReport {
    let mut successful_copies = 0;
    let mut skipped_files = 0;
    let mut errors = Vec::new();
    let mut copied_files = Vec::new();

    for result in results {
        match result {
            CopyResult::Success { source, destination } => {
                successful_copies += 1;
                copied_files.push(CopiedFile {
                    source: source.to_string_lossy().to_string(),
                    destination: destination.to_string_lossy().to_string(),
                });
            }
            CopyResult::Skipped { source, destination, reason } => {
                skipped_files += 1;
                // Optionally log skipped files
                tracing::debug!(
                    "Skipped copying {} to {}: {}",
                    source.display(),
                    destination.display(),
                    reason
                );
            }
            CopyResult::Error { source, destination, error } => {
                let error_info = CopyError {
                    source: source.to_string_lossy().to_string(),
                    destination: destination.to_string_lossy().to_string(),
                    error: error.clone(),
                };
                errors.push(error_info);
                error!(
                    "Failed to copy {} to {}: {}",
                    source.display(),
                    destination.display(),
                    error
                );
            }
        }
    }

    FileCopyReport {
        successful_copies,
        skipped_files,
        copied_files,
        errors,
    }
}

/// Report structure for file copy operations
#[derive(Debug, Clone)]
pub struct FileCopyReport {
    pub successful_copies: usize,
    pub skipped_files: usize,
    pub copied_files: Vec<CopiedFile>,
    pub errors: Vec<CopyError>,
}

impl FileCopyReport {
    pub fn empty() -> Self {
        Self {
            successful_copies: 0,
            skipped_files: 0,
            copied_files: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn total_processed(&self) -> usize {
        self.successful_copies + self.skipped_files + self.errors.len()
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_processed();
        if total == 0 {
            0.0
        } else {
            self.successful_copies as f64 / total as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct CopiedFile {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone)]
pub struct CopyError {
    pub source: String,
    pub destination: String,
    pub error: String,
}

/// Functional utility functions for file copy operations
pub mod functional {
    use super::*;

    /// Higher-order function to create a copy operation filter
    pub fn create_copy_filter(
        allowed_extensions: Vec<String>,
    ) -> impl Fn(&FvwArqDiarioExt) -> bool {
        move |revenda: &FvwArqDiarioExt| -> bool {
            allowed_extensions.contains(&revenda.extensao)
        }
    }

    /// Map revendas to their respective configurations
    pub fn map_revendas_to_configs<F, T>(revendas: Vec<FvwArqDiarioExt>, mapper: F) -> Vec<T>
    where
        F: Fn(FvwArqDiarioExt) -> T,
    {
        revendas.into_iter().map(mapper).collect()
    }

    /// Reduce copy results to summary statistics
    pub fn reduce_copy_results(results: Vec<CopyResult>) -> (usize, usize, usize) {
        results.into_iter().fold((0, 0, 0), |(success, skip, error), result| {
            match result {
                CopyResult::Success { .. } => (success + 1, skip, error),
                CopyResult::Skipped { .. } => (success, skip + 1, error),
                CopyResult::Error { .. } => (success, skip, error + 1),
            }
        })
    }
}