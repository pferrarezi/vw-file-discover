use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use std::fs;
use std::path::{Path, PathBuf};

/// List files in a directory matching given extensions
/// Pure function that returns a Result<Vec<PathBuf>>
pub fn list_files_with_extensions<P: AsRef<Path>>(
    directory: P,
    extensions: &[String],
    modified_since: Option<DateTime<Utc>>,
) -> Result<Vec<PathBuf>> {
    let dir_path = directory.as_ref();
    
    if !dir_path.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(dir_path)
        .with_context(|| format!("Failed to read directory: {:?}", dir_path))?;

    let files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| matches_extensions(path, extensions))
        .filter(|path| matches_modification_date(path, modified_since).unwrap_or(true))
        .collect();

    Ok(files)
}

/// Check if file matches any of the given extensions
/// Pure function
fn matches_extensions(path: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_with_dot = format!(".{}", ext);
            extensions.iter().any(|target_ext| {
                target_ext.eq_ignore_ascii_case(&ext_with_dot) || 
                target_ext.eq_ignore_ascii_case(ext)
            })
        })
        .unwrap_or(false)
}

/// Check if file was modified since the given date
/// Pure function (except for file system access)
fn matches_modification_date(path: &Path, modified_since: Option<DateTime<Utc>>) -> Result<bool> {
    let Some(since) = modified_since else {
        return Ok(true);
    };

    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for: {:?}", path))?;

    let modified = metadata.modified()
        .with_context(|| format!("Failed to get modification time for: {:?}", path))?;

    let modified_datetime = DateTime::from_timestamp(
        modified.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
        0
    ).unwrap_or_default();

    Ok(modified_datetime >= since)
}

/// Copy file from source to destination
/// Pure function (except for file system operations)
pub fn copy_file_safe<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    destination: Q,
    overwrite: bool,
) -> Result<bool> {
    let src_path = source.as_ref();
    let dest_path = destination.as_ref();

    if !src_path.exists() {
        anyhow::bail!("Source file does not exist: {:?}", src_path);
    }

    if dest_path.exists() && !overwrite {
        return Ok(false); // File already exists, skip
    }

    // Create destination directory if it doesn't exist
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::copy(src_path, dest_path)
        .with_context(|| format!("Failed to copy file from {:?} to {:?}", src_path, dest_path))?;

    Ok(true)
}

/// Batch copy files with filtering
/// Functional composition of copy operations
pub fn copy_files_batch<P: AsRef<Path>, Q: AsRef<Path>>(
    mappings: &[(P, Q)],
    extensions: &[String],
    days_back: Option<i64>,
    overwrite: bool,
) -> Result<Vec<CopyResult>> {
    let modified_since = days_back.map(|days| Utc::now() - Duration::days(days));

    let results: Vec<CopyResult> = mappings
        .iter()
        .flat_map(|(source_dir, dest_dir)| {
            copy_files_in_directory(source_dir, dest_dir, extensions, modified_since, overwrite)
                .unwrap_or_else(|e| {
                    vec![CopyResult::Error {
                        source: source_dir.as_ref().to_path_buf(),
                        destination: dest_dir.as_ref().to_path_buf(),
                        error: e.to_string(),
                    }]
                })
        })
        .collect();

    Ok(results)
}

/// Copy all files from source directory to destination directory
fn copy_files_in_directory<P: AsRef<Path>, Q: AsRef<Path>>(
    source_dir: P,
    dest_dir: Q,
    extensions: &[String],
    modified_since: Option<DateTime<Utc>>,
    overwrite: bool,
) -> Result<Vec<CopyResult>> {
    let files = list_files_with_extensions(&source_dir, extensions, modified_since)?;
    
    let results: Vec<CopyResult> = files
        .into_iter()
        .map(|file_path| {
            let file_name = file_path.file_name().unwrap_or_default();
            let dest_path = dest_dir.as_ref().join(file_name);
            
            match copy_file_safe(&file_path, &dest_path, overwrite) {
                Ok(true) => CopyResult::Success {
                    source: file_path,
                    destination: dest_path,
                },
                Ok(false) => CopyResult::Skipped {
                    source: file_path,
                    destination: dest_path,
                    reason: "File already exists".to_string(),
                },
                Err(e) => CopyResult::Error {
                    source: file_path,
                    destination: dest_path,
                    error: e.to_string(),
                },
            }
        })
        .collect();

    Ok(results)
}

/// Result of a file copy operation
#[derive(Debug, Clone)]
pub enum CopyResult {
    Success {
        source: PathBuf,
        destination: PathBuf,
    },
    Skipped {
        source: PathBuf,
        destination: PathBuf,
        reason: String,
    },
    Error {
        source: PathBuf,
        destination: PathBuf,
        error: String,
    },
}

impl CopyResult {
    pub fn is_success(&self) -> bool {
        matches!(self, CopyResult::Success { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CopyResult::Error { .. })
    }
}

/// Functional utilities for file operations
pub mod functional {
    use super::*;

    /// Higher-order function that creates a file filter predicate
    pub fn create_file_filter(
        extensions: Vec<String>,
        modified_since: Option<DateTime<Utc>>,
    ) -> impl Fn(&Path) -> bool {
        move |path: &Path| -> bool {
            matches_extensions(path, &extensions) &&
            matches_modification_date(path, modified_since).unwrap_or(false)
        }
    }

    /// Map over file paths with a transformation function
    pub fn map_files<F, T>(files: Vec<PathBuf>, f: F) -> Vec<T>
    where
        F: Fn(PathBuf) -> T,
    {
        files.into_iter().map(f).collect()
    }

    /// Filter files using a predicate function
    pub fn filter_files<F>(files: Vec<PathBuf>, predicate: F) -> Vec<PathBuf>
    where
        F: Fn(&PathBuf) -> bool,
    {
        files.into_iter().filter(predicate).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_extensions() {
        let path = Path::new("test.txt");
        let extensions = vec![".txt".to_string(), ".log".to_string()];
        assert!(matches_extensions(path, &extensions));

        let extensions = vec![".pdf".to_string()];
        assert!(!matches_extensions(path, &extensions));
    }

    #[test]
    fn test_matches_extensions_case_insensitive() {
        let path = Path::new("test.TXT");
        let extensions = vec![".txt".to_string()];
        assert!(matches_extensions(path, &extensions));
    }
}