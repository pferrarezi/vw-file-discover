use crate::database::DbPool;
use crate::models::{FileTrace, FvwArqDiarioExt};
use anyhow::Result;
use sqlx::Row;

/// Functional repository functions for FvwArqDiarioExt
pub mod arq_vw_ext {
    use super::*;

    /// Fetch all revendas from the database
    pub async fn get_revendas(pool: &DbPool) -> Result<Vec<FvwArqDiarioExt>> {
        let rows = sqlx::query(
            "SELECT empresa, revenda, extensao, dn, pasta_input, pasta_output FROM fvw_arq_diarios_ext"
        )
        .fetch_all(pool)
        .await?;

        let revendas = rows
            .into_iter()
            .map(|row| -> Result<FvwArqDiarioExt> {
                Ok(FvwArqDiarioExt {
                    empresa: row.try_get("empresa")?,
                    revenda: row.try_get("revenda")?,
                    extensao: row.try_get::<Option<String>, _>("extensao")?.unwrap_or_default(),
                    dn: row.try_get("dn")?,
                    pasta_input: row.try_get::<Option<String>, _>("pasta_input")?.unwrap_or_default(),
                    pasta_output: row.try_get::<Option<String>, _>("pasta_output")?.unwrap_or_default(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(revendas)
    }
}

/// Functional repository functions for FileTrace
pub mod file_trace {
    use super::*;

    /// Save multiple file traces to database (insert on conflict do nothing)
    /// Pure functional approach - takes pool and data, returns Result
    pub async fn save_batch(pool: &DbPool, file_traces: &[FileTrace]) -> Result<u64> {
        if file_traces.is_empty() {
            return Ok(0);
        }

        let mut query_builder = sqlx::QueryBuilder::new(
            r#"
            INSERT INTO fvw_file_trace
                (name, path, hash, size_bytes, size_mb, total_lines,
                 created_at, modified_at, processed_at,
                 status_fvw, status_fnt, status_fa4, dn)
            "#,
        );

        query_builder.push_values(file_traces, |mut b, file_trace| {
            b.push_bind(&file_trace.name)
                .push_bind(&file_trace.path)
                .push_bind(&file_trace.hash)
                .push_bind(file_trace.size_bytes)
                .push_bind(file_trace.size_mb)
                .push_bind(file_trace.total_lines)
                .push_bind(file_trace.created_at)
                .push_bind(file_trace.modified_at)
                .push_bind(file_trace.processed_at)
                .push_bind(file_trace.status_fvw as i32)
                .push_bind(file_trace.status_fnt as i32)
                .push_bind(file_trace.status_fa4 as i32)
                .push_bind(file_trace.dn);
        });

        query_builder.push(" ON CONFLICT (hash) DO NOTHING");

        let result = query_builder.build().execute(pool).await?;

        Ok(result.rows_affected())
    }

    /// Get file traces by status - functional approach
    pub async fn get_by_status(
        pool: &DbPool,
        status_fvw: Option<i32>,
        status_fnt: Option<i32>,
        status_fa4: Option<i32>,
    ) -> Result<Vec<FileTrace>> {
        let mut query = sqlx::QueryBuilder::new(
            "SELECT id, name, path, hash, size_bytes, size_mb, total_lines, created_at, modified_at, processed_at, status_fvw, status_fnt, status_fa4, dn FROM fvw_file_trace WHERE 1=1"
        );

        if let Some(status) = status_fvw {
            query.push(" AND status_fvw = ").push_bind(status);
        }
        if let Some(status) = status_fnt {
            query.push(" AND status_fnt = ").push_bind(status);
        }
        if let Some(status) = status_fa4 {
            query.push(" AND status_fa4 = ").push_bind(status);
        }

        let rows = query.build().fetch_all(pool).await?;

        let file_traces = rows
            .into_iter()
            .map(|row| {
                Ok(FileTrace {
                    id: Some(row.get("id")),
                    name: row.get("name"),
                    path: row.get("path"),
                    hash: row.get("hash"),
                    size_bytes: row.get("size_bytes"),
                    size_mb: row.get("size_mb"),
                    total_lines: row.get("total_lines"),
                    created_at: row.get("created_at"),
                    modified_at: row.get("modified_at"),
                    processed_at: row.get("processed_at"),
                    status_fvw: row.get("status_fvw"),
                    status_fnt: row.get("status_fnt"),
                    status_fa4: row.get("status_fa4"),
                    dn: row.get("dn"),
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?;

        Ok(file_traces)
    }
}