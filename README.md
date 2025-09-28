# VW File Discover - Rust Version

A functional, idiomatic Rust rewrite of the original C# VW File Discover application. This application processes and tracks files using functional programming principles, avoiding object-oriented patterns in favor of pure functions and immutable data structures.

## Architecture

This Rust version follows functional programming principles:

- **Pure Functions**: All core business logic is implemented as pure functions without side effects
- **Immutable Data**: All data structures are immutable by default
- **Functional Composition**: Complex operations are built by composing smaller, focused functions
- **Result Types**: Comprehensive error handling using `Result<T, E>` types
- **No OOP Patterns**: Avoids classes, inheritance, and mutable state typical of OO design

## Project Structure

```
src/
├── main.rs              # Application entry point with functional composition
├── lib.rs               # Library exports and common types
├── crypto/              # Cryptographic functions (AES-GCM decryption)
│   ├── mod.rs
│   └── aes_gcm.rs
├── database/            # Database connection and repository functions
│   ├── mod.rs
│   ├── connection.rs
│   └── repositories.rs
├── models/              # Data models and processing functions
│   ├── mod.rs
│   ├── file_trace.rs
│   └── fvw_arq_diario_ext.rs
├── services/            # Business logic as pure functions
│   ├── mod.rs
│   ├── file_copy.rs
│   └── file_discovery.rs
└── utils/               # Utility functions for file operations
    ├── mod.rs
    └── file_operations.rs
```

## Dependencies

- **tokio**: Async runtime for non-blocking I/O operations
- **sqlx**: Type-safe SQL toolkit (replaces Dapper from C# version)
- **serde**: Serialization/deserialization
- **tracing**: Structured logging (replaces Serilog)
- **aes-gcm**: AES-GCM encryption/decryption
- **anyhow**: Error handling
- **clap**: Command-line argument parsing

## Key Functional Programming Features

### Pure Functions

All core logic is implemented as pure functions:

```rust
// Pure function for file processing
pub fn process_file_one_pass(file_path: &Path) -> Result<FileProcessingResult>

// Pure function for creating copy mappings
pub fn create_copy_mappings(revendas: &[FvwArqDiarioExt]) -> Vec<(String, String)>
```

### Function Composition

Complex operations are built by composing simpler functions:

```rust
pub async fn copy_files_for_revendas(pool: &DbPool, config: FileCopyConfig) -> Result<FileCopyReport> {
    let revendas = get_revendas(pool).await?;
    let extensions = extract_file_extensions(&revendas);
    let mappings = create_copy_mappings(&revendas);
    let results = copy_files_batch(&mappings, &extensions, config.days_back, config.overwrite)?;
    Ok(create_copy_report(results))
}
```

### Immutable Data Structures

All data structures are immutable by default:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrace {
    pub id: Option<i32>,
    pub hash: String,
    pub name: String,
    // ... other immutable fields
}
```

### Higher-Order Functions

Functions that operate on other functions:

```rust
pub fn create_file_filter(
    extensions: Vec<String>,
    modified_since: Option<DateTime<Utc>>,
) -> impl Fn(&Path) -> bool {
    move |path: &Path| -> bool {
        matches_extensions(path, &extensions) &&
        matches_modification_date(path, modified_since).unwrap_or(false)
    }
}
```

## Setup

1. **Install Rust**: Follow instructions at https://rustup.rs/

2. **Clone and setup**:

   ```bash
   cd rust-version
   cp .env.example .env
   # Edit .env with your actual configuration
   ```

3. **Install dependencies**:

   ```bash
   cargo build
   ```

4. **Run the application**:
   ```bash
   cargo run
   ```

## Usage

The application supports various command-line options:

```bash
# Basic usage
cargo run

# With custom configuration
cargo run -- --log-level debug --days-back 30 --batch-size 500

# Skip certain phases
cargo run -- --skip-copy          # Skip file copying
cargo run -- --skip-discovery     # Skip file discovery
```

### Command Line Options

- `--log-level`: Set logging level (trace, debug, info, warn, error)
- `--days-back`: Number of days back to look for files (default: 15)
- `--batch-size`: Batch size for database operations (default: 1000)
- `--skip-copy`: Skip the file copying phase
- `--skip-discovery`: Skip the file discovery phase

## Configuration

### Environment Variables

Create a `.env` file based on `.env.example`:

```bash
SECRET_KEY1=base64_encoded_secret_key
PG_API_CONNECTION=encrypted_postgresql_connection_string
RUST_LOG=info
```

### Database

The application uses PostgreSQL with sqlx for type-safe database operations. Ensure your database has the required tables:

- `fvw_arq_diarios_ext`: Configuration for file processing
- `fvw_file_trace`: File tracking and metadata

## Key Differences from C# Version

1. **No Classes**: Replaced C# classes with Rust modules containing pure functions
2. **No Inheritance**: Used trait composition instead of class inheritance
3. **Immutable by Default**: All data structures are immutable unless explicitly made mutable
4. **Functional Error Handling**: Uses `Result<T, E>` instead of exceptions
5. **Pure Functions**: Business logic separated from side effects
6. **Function Composition**: Complex operations built from simple function combinations

## Performance Considerations

- Uses `tokio::task::spawn_blocking` for CPU-intensive file processing
- Implements batched database operations to reduce connection overhead
- Uses memory-efficient streaming for large file processing
- Employs functional composition to enable easy parallelization

## Testing

Run tests with:

```bash
cargo test
```

The functional approach makes unit testing straightforward since pure functions are easy to test in isolation.

## Logging

Uses structured logging with tracing:

```bash
# Different log levels
RUST_LOG=debug cargo run
RUST_LOG=trace cargo run
```

## Development

The functional approach makes the codebase:

- **Easier to test**: Pure functions are deterministic and side-effect-free
- **Easier to reason about**: No hidden state mutations
- **More composable**: Functions can be easily combined and reused
- **More maintainable**: Clear separation of concerns and data flow
