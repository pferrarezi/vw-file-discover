use serde::{Deserialize, Serialize};

/// Configuration for file processing by revenda
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FvwArqDiarioExt {
    pub empresa: i32,
    pub revenda: i32,
    pub extensao: String,
    pub dn: i32,
    pub pasta_input: String,
    pub pasta_output: String,
}

impl FvwArqDiarioExt {
    /// Get the base directory based on build configuration
    pub fn base_directory() -> &'static str {
        if cfg!(debug_assertions) {
            r"C:\arquivos_diarios"
        } else {
            "/srv/arquivos/arquivos_diarios_vw"
        }
    }

    /// Create a new FvwArqDiarioExt
    pub fn new(
        empresa: i32,
        revenda: i32,
        extensao: String,
        dn: i32,
        pasta_input: String,
        pasta_output: String,
    ) -> Self {
        Self {
            empresa,
            revenda,
            extensao,
            dn,
            pasta_input,
            pasta_output,
        }
    }
}