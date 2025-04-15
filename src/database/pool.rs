use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::{
    fs::{OpenOptions, create_dir_all},
    path::Path,
};

/// Establishes a connection pool to the SQLite database
pub async fn create_sqlite_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    ensure_sqlite_file_exists(database_url)?;

    let pool = SqlitePoolOptions::new().connect(database_url).await?;

    Ok(pool)
}

/// Ensures that the SQLite database file exists and creates parent directories if necessary
fn ensure_sqlite_file_exists(database_url: &str) -> std::io::Result<()> {
    // Skip the sqlite: prefix if present
    let path = database_url.trim_start_matches("sqlite:");

    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            create_dir_all(parent)?;
        }
    }

    // Create empty file if it doesn't exist
    if !Path::new(path).exists() {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        drop(file);
    }

    Ok(())
}
