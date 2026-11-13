//! Shared database initialization for HTTP startup and local migration commands.

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::{error::Error, path::Path, time::Duration};
use tower_sessions_sqlx_store::SqliteStore;

/// Opens the prepared database, then initializes application and session schemas.
///
/// Call storage validation first, with process umask 0077 already set. Missing
/// files are errors: silently creating another database would hide a wrong path.
/// Every pooled connection uses the same durability and contention settings.
/// Migrations are embedded in the binary so releases need no source files.
pub async fn initialize(data_dir: &Path) -> Result<SqlitePool, Box<dyn Error>> {
    let options = SqliteConnectOptions::new()
        .filename(data_dir.join("admin.sqlite3"))
        .create_if_missing(false)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    let setup = async {
        sqlx::migrate!("./migrations").run(&pool).await?;
        SqliteStore::new(pool.clone()).migrate().await?;
        Ok::<_, Box<dyn Error>>(())
    }
    .await;

    if let Err(error) = setup {
        pool.close().await;
        return Err(error);
    }
    Ok(pool)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::DirBuilderExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "maj-db-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            crate::storage::prepare(&root, &[]).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn initializes_private_schemas_and_configures_every_connection() {
        let fixture = Fixture::new();
        let pool = initialize(&fixture.0).await.unwrap();
        let mut connections = Vec::new();
        for _ in 0..4 {
            let mut connection = pool.acquire().await.unwrap();
            for (pragma, expected) in [
                ("PRAGMA foreign_keys", 1),
                ("PRAGMA synchronous", 2),
                ("PRAGMA busy_timeout", 5000),
            ] {
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(pragma)
                        .fetch_one(&mut *connection)
                        .await
                        .unwrap(),
                    expected
                );
            }
            assert_eq!(
                sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
                    .fetch_one(&mut *connection)
                    .await
                    .unwrap(),
                "wal"
            );
            connections.push(connection);
        }
        drop(connections);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM administrators")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tower_sessions")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        sqlx::query("INSERT INTO administrators (issuer, subject, label) VALUES ('https://accounts.google.com', 'test-subject', 'test')").execute(&pool).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT enabled FROM administrators")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert!(sqlx::query("INSERT INTO administrators (issuer, subject, label) VALUES ('https://accounts.google.com', 'test-subject', 'duplicate')").execute(&pool).await.is_err());
        assert!(sqlx::query("UPDATE administrators SET enabled = 2")
            .execute(&pool)
            .await
            .is_err());
        assert!(
            sqlx::query("UPDATE administrators SET session_generation = -1")
                .execute(&pool)
                .await
                .is_err()
        );
        pool.close().await;
        let reopened = initialize(&fixture.0).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT label FROM administrators")
                .fetch_one(&reopened)
                .await
                .unwrap(),
            "test"
        );
        reopened.close().await;
    }

    #[tokio::test]
    async fn missing_or_corrupt_database_fails_without_replacement() {
        let fixture = Fixture::new();
        let file = fixture.0.join("admin.sqlite3");
        fs::remove_file(&file).unwrap();
        assert!(initialize(&fixture.0).await.is_err());
        assert!(!file.exists());
        fs::write(&file, b"not a SQLite database").unwrap();
        assert!(initialize(&fixture.0).await.is_err());
        assert_eq!(fs::read(file).unwrap(), b"not a SQLite database");
    }

    #[tokio::test]
    async fn migration_checksum_mismatch_aborts_initialization() {
        let fixture = Fixture::new();
        let pool = initialize(&fixture.0).await.unwrap();
        sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00'")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let error = initialize(&fixture.0).await.unwrap_err();
        assert!(error.to_string().contains("modified"), "{error}");
    }

    #[tokio::test]
    async fn write_contention_times_out_while_readers_continue() {
        let fixture = Fixture::new();
        let pool = initialize(&fixture.0).await.unwrap();
        let mut writer = pool.acquire().await.unwrap();
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *writer)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM administrators")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        let started = std::time::Instant::now();
        let error = sqlx::query("INSERT INTO administrators (issuer, subject, label) VALUES ('issuer', 'subject', 'test')").execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("5")
        );
        assert!(started.elapsed() >= Duration::from_millis(4500));
        sqlx::query("ROLLBACK").execute(&mut *writer).await.unwrap();
        drop(writer);
        pool.close().await;
    }
}
