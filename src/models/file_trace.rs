use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use anyhow::{Context, Result};

/// File trace status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileTraceStatus {
    Pending = 0,
    Processing = 1,
    Processed = 2,
    Error = 3,
    Banned = 4,
}

impl From<i32> for FileTraceStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => FileTraceStatus::Pending,
            1 => FileTraceStatus::Processing,
            2 => FileTraceStatus::Processed,
            3 => FileTraceStatus::Error,
            4 => FileTraceStatus::Banned,
            _ => FileTraceStatus::Pending,
        }
    }
}

/// File trace model - immutable struct following functional principles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrace {
    pub id: Option<i32>,
    pub hash: String,
    pub name: String,
    pub path: String,
    pub size_bytes: i64,
    pub size_mb: f64,
    pub total_lines: i32,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub processed_at: DateTime<Utc>,
    pub status_fvw: i32,
    pub status_fnt: i32,
    pub status_fa4: i32,
    pub dn: i32,
}

impl FileTrace {
    /// Create a new FileTrace with default values
    pub fn new(
        name: String,
        path: String,
        hash: String,
        size_bytes: i64,
        total_lines: i32,
        created_at: DateTime<Utc>,
        modified_at: DateTime<Utc>,
        dn: i32,
    ) -> Self {
        Self {
            id: None,
            hash,
            name,
            path,
            size_bytes,
            size_mb: (size_bytes as f64) / (1024.0 * 1024.0),
            total_lines,
            created_at,
            modified_at,
            processed_at: Utc::now(),
            status_fvw: FileTraceStatus::Pending as i32,
            status_fnt: FileTraceStatus::Pending as i32,
            status_fa4: FileTraceStatus::Pending as i32,
            dn,
        }
    }
}

/// File processing result containing hash, DN, and line count
#[derive(Debug)]
pub struct FileProcessingResult {
    pub hash: String,
    pub dn: i32,
    pub total_lines: i32,
}

/// Pure functional approach to create FileTrace from file path
pub fn create_file_trace_from_path<P: AsRef<Path>>(file_path: P) -> Result<FileTrace> {
    let path = file_path.as_ref();
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for: {:?}", path))?;
    
    let processing_result = process_file_one_pass(path)?;
    
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let path_str = path.to_string_lossy().to_string();
    
    let created_at = metadata_to_datetime(metadata.created().ok());
    let modified_at = metadata_to_datetime(metadata.modified().ok());
    
    Ok(FileTrace::new(
        name,
        path_str,
        processing_result.hash,
        metadata.len() as i64,
        processing_result.total_lines,
        created_at,
        modified_at,
        processing_result.dn,
    ))
}

/// Process file in one pass to get hash, DN from first line, and line count
/// Pure function with no side effects
pub fn process_file_one_pass<P: AsRef<Path>>(file_path: P) -> Result<FileProcessingResult> {
    let mut file = File::open(file_path.as_ref())
        .with_context(|| format!("Failed to open file: {:?}", file_path.as_ref()))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 128 * 1024]; // 128KB buffer
    let mut total_lines = 0;
    let mut first_line = String::new();
    let mut first_line_read = false;
    let mut line_buffer = Vec::new();
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .context("Failed to read from file")?;
        
        if bytes_read == 0 {
            break;
        }
        
        // Update hash
        hasher.update(&buffer[..bytes_read]);
        
        // Process bytes for line counting and first line extraction
        for &byte in &buffer[..bytes_read] {
            if !first_line_read {
                if byte == b'\n' {
                    first_line = String::from_utf8_lossy(&line_buffer).trim_end_matches('\r').to_string();
                    first_line_read = true;
                    line_buffer.clear();
                } else {
                    line_buffer.push(byte);
                }
            }
            
            if byte == b'\n' {
                total_lines += 1;
            }
        }
    }
    
    // Handle case where file doesn't end with newline
    if !line_buffer.is_empty() && !first_line_read {
        first_line = String::from_utf8_lossy(&line_buffer).trim_end_matches('\r').to_string();
        total_lines = total_lines.max(1);
    }
    
    let hash = format!("{:x}", hasher.finalize());
    let dn = extract_dn_from_fhi_first_line(&first_line);
    
    Ok(FileProcessingResult {
        hash,
        dn,
        total_lines,
    })
}

/// Extract DN from FHI first line (positions 39-44, 0-based)
/// Pure function
fn extract_dn_from_fhi_first_line(first_line: &str) -> i32 {
    if first_line.starts_with("FHI") && first_line.len() >= 45 {
        first_line
            .chars()
            .skip(39)
            .take(5)
            .collect::<String>()
            .parse()
            .unwrap_or(0)
    } else {
        0
    }
}

/// Convert system time to UTC DateTime
fn metadata_to_datetime(system_time: Option<std::time::SystemTime>) -> DateTime<Utc> {
    system_time
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| DateTime::from_timestamp_millis(d.as_millis() as i64).unwrap_or_default())
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dn_from_fhi_first_line() {
        let line = "FHI12345678901234567890123456789012345678901234567890";
        let dn = extract_dn_from_fhi_first_line(line);
        // Characters at positions 39-43 (0-based) are "78901"
        assert_eq!(dn, 78901);
    }

    #[test]
    fn test_extract_dn_from_non_fhi_line() {
        let line = "ABC12345678901234567890123456789012345678901234567890";
        let dn = extract_dn_from_fhi_first_line(line);
        assert_eq!(dn, 0);
    }

    #[test]
    fn test_extract_dn_from_short_line() {
        let line = "FHI123";
        let dn = extract_dn_from_fhi_first_line(line);
        assert_eq!(dn, 0);
    }
}