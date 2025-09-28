pub mod file_copy;
pub mod file_discovery;

pub use file_copy::{
    copy_files_for_revendas, extract_file_extensions, create_copy_mappings,
    FileCopyConfig, FileCopyReport, CopiedFile, CopyError
};
pub use file_discovery::{
    discover_and_register_files, extract_output_directories, extract_unique_extensions,
    FileDiscoveryConfig, FileDiscoveryReport
};