-- Create table for file processing configuration
CREATE TABLE IF NOT EXISTS fvw_arq_diarios_ext (
    id SERIAL PRIMARY KEY,
    empresa INTEGER NOT NULL,
    revenda INTEGER NOT NULL,
    extensao VARCHAR(10) NOT NULL,
    dn INTEGER NOT NULL DEFAULT 0,
    pasta_input TEXT NOT NULL,
    pasta_output TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create table for file tracking
CREATE TABLE IF NOT EXISTS fvw_file_trace (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    path TEXT NOT NULL,
    hash VARCHAR(64) NOT NULL UNIQUE, -- SHA-256 hash
    size_bytes BIGINT NOT NULL DEFAULT 0,
    size_mb DECIMAL(10,2) NOT NULL DEFAULT 0.0,
    total_lines INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    modified_at TIMESTAMP WITH TIME ZONE NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    status_fvw INTEGER NOT NULL DEFAULT 0,
    status_fnt INTEGER NOT NULL DEFAULT 0,
    status_fa4 INTEGER NOT NULL DEFAULT 0,
    dn INTEGER NOT NULL DEFAULT 0
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_fvw_file_trace_hash ON fvw_file_trace(hash);
CREATE INDEX IF NOT EXISTS idx_fvw_file_trace_status_fvw ON fvw_file_trace(status_fvw);
CREATE INDEX IF NOT EXISTS idx_fvw_file_trace_status_fnt ON fvw_file_trace(status_fnt);
CREATE INDEX IF NOT EXISTS idx_fvw_file_trace_status_fa4 ON fvw_file_trace(status_fa4);
CREATE INDEX IF NOT EXISTS idx_fvw_file_trace_dn ON fvw_file_trace(dn);
CREATE INDEX IF NOT EXISTS idx_fvw_file_trace_processed_at ON fvw_file_trace(processed_at);

-- Add unique constraints
ALTER TABLE fvw_arq_diarios_ext 
ADD CONSTRAINT unique_empresa_revenda_ext 
UNIQUE (empresa, revenda, extensao);

-- Comments for documentation
COMMENT ON TABLE fvw_arq_diarios_ext IS 'Configuration table for file processing by revenda';
COMMENT ON TABLE fvw_file_trace IS 'File tracking and metadata storage';
COMMENT ON COLUMN fvw_file_trace.hash IS 'SHA-256 hash of file content';
COMMENT ON COLUMN fvw_file_trace.status_fvw IS 'Processing status for FVW system (0=pending, 1=processing, 2=processed, 3=error, 4=banned)';
COMMENT ON COLUMN fvw_file_trace.status_fnt IS 'Processing status for FNT system';
COMMENT ON COLUMN fvw_file_trace.status_fa4 IS 'Processing status for FA4 system';
COMMENT ON COLUMN fvw_file_trace.dn IS 'DN extracted from FHI first line (positions 39-44)';