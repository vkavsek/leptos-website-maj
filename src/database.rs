//! Application SQL and shared SQLite access for HTTP and local administration.

use crate::{auth::Administrator, routes::shows::Event};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::{error::Error, path::Path, time::Duration};
use tower_sessions::ExpiredDeletion;
use tower_sessions_sqlx_store::SqliteStore;

/// Owns application database access; clones share the same connection pool.
/// The pool stays private so callers use named operations with fixed SQL and bound values.
#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl DatabaseManager {
    /// Opens the prepared database, then initializes application and session schemas.
    ///
    /// Call storage validation first, with process umask 0077 already set. Missing
    /// files are errors: silently creating another database would hide a wrong path.
    /// Every pooled connection uses the same durability and contention settings.
    /// Migrations are embedded in the binary so releases need no source files.
    pub async fn initialize(data_dir: &Path) -> Result<Self, Box<dyn Error>> {
        let database = Self::open(data_dir).await?;
        let setup = async {
            sqlx::migrate!("./migrations").run(&database.pool).await?;
            database.session_store().migrate().await?;
            Ok::<_, Box<dyn Error>>(())
        }
        .await;

        if let Err(error) = setup {
            database.close().await;
            return Err(error);
        }
        Ok(database)
    }

    /// Opens an existing file without applying migrations, for pre-upgrade backups.
    pub async fn open(data_dir: &Path) -> sqlx::Result<Self> {
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
        Ok(Self { pool })
    }

    /// Closes the shared pool after serving or the operator command has finished.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Uses the same pool for the session library's persistence adapter.
    pub(crate) fn session_store(&self) -> SqliteStore {
        SqliteStore::new(self.pool.clone())
    }

    /// Explicitly approves an identity and returns its stable local administrator ID.
    ///
    /// A new row starts enabled with the schema's default generation of zero. The
    /// unique `(issuer, subject)` constraint makes a repeat grant update that same row:
    /// `excluded.label` is the proposed insert's label, and `RETURNING id` supplies
    /// the ID for either the insert or update through `query_scalar`.
    ///
    /// The CASE examines the existing row's enabled flag before the update. Keeping
    /// an enabled user's generation makes repeat grants preserve current sessions.
    /// Re-enabling a disabled user advances the generation again, so sessions issued
    /// before revocation cannot become valid again. Arguments are bound separately
    /// from SQL, including the display label.
    ///
    /// 9223372036854775807 is the maximum signed 64-bit integer. Adding one would
    /// promote SQLite's value to REAL and lose the integer generation guarantee.
    /// The -1 sentinel instead violates CHECK(session_generation >= 0), causing the
    /// statement to fail atomically rather than wrap, reset, or reuse a generation.
    pub async fn grant_administrator(
        &self,
        issuer: &str,
        subject: &str,
        label: &str,
    ) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO administrators (issuer, subject, label, enabled)
             VALUES (?, ?, ?, 1)
             ON CONFLICT(issuer, subject) DO UPDATE SET
                 label = excluded.label,
                 enabled = 1,
                 session_generation = CASE
                     WHEN administrators.enabled = 1
                         THEN administrators.session_generation
                     WHEN administrators.session_generation < 9223372036854775807
                         THEN administrators.session_generation + 1
                     ELSE -1
                 END
             RETURNING id",
        )
        .bind(issuer)
        .bind(subject)
        .bind(label)
        .fetch_one(&self.pool)
        .await
    }

    /// Disables the matching identity and advances its generation in one statement.
    ///
    /// Subsequent authorization checks reject its sessions through both the disabled
    /// flag and generation mismatch. Even an already disabled row advances; unknown
    /// identities return false. The same maximum-integer/-1 constraint guard as
    /// `grant_administrator` makes exhaustion an error, leaving the row unchanged on failure.
    pub async fn revoke_administrator(&self, issuer: &str, subject: &str) -> sqlx::Result<bool> {
        Ok(sqlx::query(
            "UPDATE administrators SET
                 enabled = 0,
                 session_generation = CASE
                     WHEN session_generation < 9223372036854775807
                         THEN session_generation + 1
                     ELSE -1
                 END
             WHERE issuer = ?
               AND subject = ?",
        )
        .bind(issuer)
        .bind(subject)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    /// Invalidates one administrator's sessions without changing their approval.
    ///
    /// Logout must advance this durable generation before flushing its session:
    /// another request can recreate the deleted session row from cached data.
    /// The existing maximum-integer/-1 guard makes exhaustion fail atomically;
    /// a missing administrator returns false instead of reporting successful revocation.
    pub async fn revoke_administrator_sessions(&self, id: i64) -> sqlx::Result<bool> {
        Ok(sqlx::query(
            "UPDATE administrators SET
                 session_generation = CASE
                     WHEN session_generation < 9223372036854775807
                         THEN session_generation + 1
                     ELSE -1
                 END
             WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    /// Invalidates all current sessions while preserving identity approvals.
    ///
    /// The UPDATE has no WHERE clause: every administrator's generation advances,
    /// including disabled identities. Its maximum-integer/-1 guard rejects exhausted
    /// generations instead of allowing SQLite to convert them to REAL.
    ///
    /// Generation changes and deletion of browser sessions/login transactions share
    /// one transaction. Any failure rolls the whole operation back; successful commit
    /// also prevents pending stored login transactions from completing. Enabled
    /// administrators can subsequently authenticate again and receive fresh sessions.
    pub async fn revoke_all_sessions(&self) -> sqlx::Result<()> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "UPDATE administrators SET
                 session_generation = CASE
                     WHEN session_generation < 9223372036854775807
                         THEN session_generation + 1
                     ELSE -1
                 END",
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query("DELETE FROM tower_sessions")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM login_transactions")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await
    }

    /// Lists explicit identity approvals in stable local-ID order for operator output.
    pub async fn administrators(&self) -> sqlx::Result<Vec<Administrator>> {
        sqlx::query_as(
            "SELECT id, issuer, subject, label, enabled, session_generation
             FROM administrators
             ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Finds an explicitly enabled identity before creating an administrator session.
    pub async fn enabled_administrator(
        &self,
        issuer: &str,
        subject: &str,
    ) -> sqlx::Result<Option<Administrator>> {
        sqlx::query_as(
            "SELECT id, issuer, subject, label, enabled, session_generation
             FROM administrators
             WHERE issuer = ?
               AND subject = ?
               AND enabled = 1",
        )
        .bind(issuer)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
    }

    /// Reloads approval and generation so guards can reject revoked sessions immediately.
    pub async fn administrator(&self, id: i64) -> sqlx::Result<Option<Administrator>> {
        sqlx::query_as(
            "SELECT id, issuer, subject, label, enabled, session_generation
             FROM administrators
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Reserves a five-minute login record; false means the browser or global limit is full.
    /// Callers hash state/browser bindings and generate protocol secrets before calling.
    pub async fn try_insert_login_transaction(
        &self,
        state_hash: &str,
        browser_hash: &str,
        pkce_verifier: &str,
        nonce: &str,
    ) -> sqlx::Result<bool> {
        // Serialize expiry cleanup, capacity checks, and insertion across pool connections.
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query(
            "DELETE FROM login_transactions
             WHERE expires_at <= unixepoch()",
        )
        .execute(&mut *transaction)
        .await?;

        let (total, browser): (i64, i64) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(browser_hash = ?), 0)
             FROM login_transactions",
        )
        .bind(browser_hash)
        .fetch_one(&mut *transaction)
        .await?;

        if total >= 128 || browser >= 3 {
            return Ok(false);
        }

        sqlx::query(
            "INSERT INTO login_transactions (state_hash, browser_hash, pkce_verifier, nonce, expires_at)
             VALUES (?, ?, ?, ?, unixepoch() + 300)",
        )
        .bind(state_hash)
        .bind(browser_hash)
        .bind(pkce_verifier)
        .bind(nonce)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(true)
    }

    /// Consumes matching, unexpired state once, returning its verifier and nonce.
    pub async fn consume_login_transaction(
        &self,
        state_hash: &str,
        browser_hash: &str,
    ) -> sqlx::Result<Option<(String, String)>> {
        sqlx::query_as(
            "DELETE FROM login_transactions
             WHERE state_hash = ?
               AND browser_hash = ?
               AND expires_at > unixepoch()
             RETURNING pkce_verifier, nonce",
        )
        .bind(state_hash)
        .bind(browser_hash)
        .fetch_optional(&self.pool)
        .await
    }

    /// Removes abandoned protocol state when the browser session could not be saved.
    pub async fn delete_login_transaction(&self, state_hash: &str) -> sqlx::Result<()> {
        sqlx::query(
            "DELETE FROM login_transactions
             WHERE state_hash = ?",
        )
        .bind(state_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Purges expired protocol state and sessions; guards enforce expiry independently.
    pub async fn cleanup_expired_auth(&self) -> Result<(), Box<dyn Error>> {
        sqlx::query(
            "DELETE FROM login_transactions
             WHERE expires_at <= unixepoch()",
        )
        .execute(&self.pool)
        .await?;
        self.session_store().delete_expired().await?;
        Ok(())
    }

    /// Rejects import when either the completion marker or any catalog content exists.
    pub async fn content_is_initialized(&self) -> sqlx::Result<bool> {
        let occupied: i64 = sqlx::query_scalar(
            "SELECT
                 (SELECT count(*) FROM content_import)
                 + (SELECT count(*) FROM shows)
                 + (SELECT count(*) FROM videos)
                 + (SELECT count(*) FROM gallery_images)",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(occupied != 0)
    }

    /// Queries editable content on every request; ties retain their insertion order.
    pub async fn shows(&self) -> sqlx::Result<Vec<Show>> {
        sqlx::query_as(
            "SELECT id, date, name, club, location
             FROM shows
             ORDER BY date DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Adds a previously validated calendar date and optional text fields.
    pub async fn add_show(
        &self,
        date: &str,
        name: Option<&str>,
        club: Option<&str>,
        location: Option<&str>,
    ) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO shows (date, name, club, location)
             VALUES (?, ?, ?, ?)
             RETURNING id",
        )
        .bind(date)
        .bind(name)
        .bind(club)
        .bind(location)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn remove_show(&self, id: i64) -> sqlx::Result<bool> {
        Ok(sqlx::query("DELETE FROM shows WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            == 1)
    }

    pub async fn videos(&self) -> sqlx::Result<Vec<Video>> {
        sqlx::query_as("SELECT id, video_id FROM videos ORDER BY id")
            .fetch_all(&self.pool)
            .await
    }

    /// The unique constraint rejects duplicate videos, including concurrent submissions.
    pub async fn add_video(&self, video_id: &str) -> sqlx::Result<i64> {
        sqlx::query_scalar("INSERT INTO videos (video_id) VALUES (?) RETURNING id")
            .bind(video_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn remove_video(&self, id: i64) -> sqlx::Result<bool> {
        Ok(sqlx::query("DELETE FROM videos WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            == 1)
    }

    pub async fn gallery(&self) -> sqlx::Result<Vec<GalleryImage>> {
        sqlx::query_as(
            "SELECT id, public_name, original_file, thumbnail_file
             FROM gallery_images ORDER BY public_name DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Only published names resolve; filesystem-only orphans are invisible to visitors.
    pub async fn gallery_image(&self, name: &str) -> sqlx::Result<Option<GalleryImage>> {
        sqlx::query_as(
            "SELECT id, public_name, original_file, thumbnail_file
             FROM gallery_images WHERE public_name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    /// Publish both completed files before committing the row. On ambiguous commit
    /// failure keep the files: an orphan is safer than a committed broken reference.
    pub async fn add_gallery(
        &self,
        name: &str,
        original: &str,
        thumbnail: &str,
        publish: impl FnOnce() -> std::io::Result<()>,
    ) -> sqlx::Result<i64> {
        let mut transaction = self.pool.begin().await?;
        let id = sqlx::query_scalar(
            "INSERT INTO gallery_images (public_name, original_file, thumbnail_file)
             VALUES (?, ?, ?) RETURNING id",
        )
        .bind(name)
        .bind(original)
        .bind(thumbnail)
        .fetch_one(&mut *transaction)
        .await?;
        publish()?;
        transaction.commit().await?;
        Ok(id)
    }

    /// Unpublish first and return exactly the files that may now be cleaned up.
    pub async fn remove_gallery(&self, id: i64) -> sqlx::Result<Option<GalleryImage>> {
        sqlx::query_as(
            "DELETE FROM gallery_images WHERE id = ?
             RETURNING id, public_name, original_file, thumbnail_file",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Inserts validated records, publishes their files, and only then commits the catalog.
    /// The caller holds the exclusive storage lock. Publication failure must remove its
    /// new links; returning an error here drops the transaction and rolls back its rows.
    pub async fn import_content(
        &self,
        shows: &[Event],
        video_ids: &[String],
        photos: &[(String, String, String)],
        publish: impl FnOnce() -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        // The unique singleton also protects against callers that bypass the operator lock.
        sqlx::query(
            "INSERT INTO content_import (id, completed_at)
             VALUES (1, unixepoch())",
        )
        .execute(&mut *transaction)
        .await?;
        // Keep shows, video IDs, and image references in the same uncommitted catalog update.
        for show in shows {
            sqlx::query(
                "INSERT INTO shows (date, name, club, location)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(
                show.get_date()
                    .ok_or("Missing show date")?
                    .format("%Y-%m-%d")
                    .to_string(),
            )
            .bind(&show.name)
            .bind(&show.club)
            .bind(&show.location)
            .execute(&mut *transaction)
            .await?;
        }
        for id in video_ids {
            sqlx::query(
                "INSERT INTO videos (video_id)
                 VALUES (?)",
            )
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        }
        for (name, original, thumbnail) in photos {
            sqlx::query(
                "INSERT INTO gallery_images (public_name, original_file, thumbnail_file)
                 VALUES (?, ?, ?)",
            )
            .bind(name)
            .bind(original)
            .bind(thumbnail)
            .execute(&mut *transaction)
            .await?;
        }
        publish()?;
        // On ambiguous commit failure, retain files: orphans are safer than broken committed rows.
        transaction.commit().await?;
        Ok(())
    }

    /// Writes an SQLite snapshot into the caller's privately reserved empty file.
    pub async fn snapshot_into(&self, destination: &Path) -> Result<(), Box<dyn Error>> {
        sqlx::query("VACUUM INTO ?")
            .bind(destination.to_str().ok_or("Backup path must be UTF-8")?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Lists files that backups must include; pre-upgrade schemas may lack the gallery table.
    pub async fn gallery_references(&self) -> sqlx::Result<Vec<(String, String)>> {
        let has_gallery: i64 = sqlx::query_scalar(
            "SELECT count(*)
             FROM sqlite_master
             WHERE type = 'table'
               AND name = 'gallery_images'",
        )
        .fetch_one(&self.pool)
        .await?;
        if has_gallery == 0 {
            return Ok(Vec::new());
        }
        sqlx::query_as(
            "SELECT original_file, thumbnail_file
             FROM gallery_images",
        )
        .fetch_all(&self.pool)
        .await
    }
}

/// A show with a non-reusable local ID and an ISO calendar date.
#[derive(sqlx::FromRow)]
pub struct Show {
    pub id: i64,
    pub date: String,
    pub name: Option<String>,
    pub club: Option<String>,
    pub location: Option<String>,
}

/// Only a validated YouTube identifier is stored; embed URLs are built by the server.
#[derive(sqlx::FromRow)]
pub struct Video {
    pub id: i64,
    pub video_id: String,
}

/// Published image pair; filesystem names are private and never supplied by a URL.
#[derive(sqlx::FromRow)]
pub struct GalleryImage {
    pub id: i64,
    pub public_name: String,
    pub original_file: String,
    pub thumbnail_file: String,
}

#[cfg(test)]
impl DatabaseManager {
    /// Allows independent fixture setup and assertions without exposing runtime SQL access.
    pub(crate) fn test_pool(&self) -> &SqlitePool {
        &self.pool
    }
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

    #[tokio::test]
    async fn content_migration_preserves_records_and_never_reuses_deleted_ids() {
        let fixture = Fixture::new();
        let database = DatabaseManager::open(&fixture.0).await.unwrap();
        sqlx::migrate!("./migrations")
            .run_to(202611130002, &database.pool)
            .await
            .unwrap();
        let show = database
            .add_show("2024-02-29", Some("Existing"), None, None)
            .await
            .unwrap();
        let video = database.add_video("3uh_Jc1prRQ").await.unwrap();
        let photo = database
            .add_gallery("old.jpg", "old.jpg", "old.webp", || Ok(()))
            .await
            .unwrap();
        sqlx::migrate!("./migrations")
            .run(&database.pool)
            .await
            .unwrap();
        assert_eq!(
            database.shows().await.unwrap()[0].name.as_deref(),
            Some("Existing")
        );
        assert_eq!(database.videos().await.unwrap()[0].id, video);
        assert_eq!(database.gallery().await.unwrap()[0].id, photo);
        database.remove_show(show).await.unwrap();
        database.remove_video(video).await.unwrap();
        database.remove_gallery(photo).await.unwrap();
        assert!(
            database
                .add_show("2026-01-01", None, None, None)
                .await
                .unwrap()
                > show
        );
        assert!(database.add_video("3uh_Jc1prRQ").await.unwrap() > video);
        assert!(
            database
                .add_gallery("old.jpg", "new.jpg", "new.webp", || Ok(()))
                .await
                .unwrap()
                > photo
        );
        assert!(!database.remove_show(show).await.unwrap());
        assert!(!database.remove_video(video).await.unwrap());
        assert!(database.remove_gallery(photo).await.unwrap().is_none());
        assert!(database
            .add_gallery("failure.jpg", "failure.jpg", "failure.webp", || Err(
                std::io::Error::other("publication failed")
            ))
            .await
            .is_err());
        assert!(database
            .gallery_image("failure.jpg")
            .await
            .unwrap()
            .is_none());
        database.close().await;
    }

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
        let database = DatabaseManager::initialize(&fixture.0).await.unwrap();
        let mut connections = Vec::new();
        for _ in 0..4 {
            let mut connection = database.pool.acquire().await.unwrap();
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
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM administrators",
            )
            .fetch_one(&database.pool)
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM tower_sessions",
            )
            .fetch_one(&database.pool)
            .await
            .unwrap(),
            0
        );
        sqlx::query(
            "INSERT INTO administrators (issuer, subject, label)
             VALUES ('https://accounts.google.com', 'test-subject', 'test')",
        )
        .execute(&database.pool)
        .await
        .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT enabled
                 FROM administrators",
            )
            .fetch_one(&database.pool)
            .await
            .unwrap(),
            0
        );
        assert!(sqlx::query(
            "INSERT INTO administrators (issuer, subject, label)
             VALUES ('https://accounts.google.com', 'test-subject', 'duplicate')",
        )
        .execute(&database.pool)
        .await
        .is_err());
        assert!(sqlx::query(
            "UPDATE administrators SET
                 enabled = 2",
        )
        .execute(&database.pool)
        .await
        .is_err());
        assert!(sqlx::query(
            "UPDATE administrators SET
                 session_generation = -1",
        )
        .execute(&database.pool)
        .await
        .is_err());
        database.close().await;
        let reopened = DatabaseManager::initialize(&fixture.0).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT label
                 FROM administrators",
            )
            .fetch_one(&reopened.pool)
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
        assert!(DatabaseManager::initialize(&fixture.0).await.is_err());
        assert!(!file.exists());
        fs::write(&file, b"not a SQLite database").unwrap();
        assert!(DatabaseManager::initialize(&fixture.0).await.is_err());
        assert_eq!(fs::read(file).unwrap(), b"not a SQLite database");
    }

    #[tokio::test]
    async fn migration_checksum_mismatch_aborts_initialization() {
        let fixture = Fixture::new();
        let database = DatabaseManager::initialize(&fixture.0).await.unwrap();
        sqlx::query(
            "UPDATE _sqlx_migrations SET
                 checksum = X'00'",
        )
        .execute(&database.pool)
        .await
        .unwrap();
        database.close().await;
        let error = DatabaseManager::initialize(&fixture.0).await.err().unwrap();
        assert!(error.to_string().contains("modified"), "{error}");
    }

    #[tokio::test]
    async fn write_contention_times_out_while_readers_continue() {
        let fixture = Fixture::new();
        let database = DatabaseManager::initialize(&fixture.0).await.unwrap();
        let mut writer = database.pool.acquire().await.unwrap();
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *writer)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM administrators",
            )
            .fetch_one(&database.pool)
            .await
            .unwrap(),
            0
        );
        let started = std::time::Instant::now();
        let error = sqlx::query(
            "INSERT INTO administrators (issuer, subject, label)
             VALUES ('issuer', 'subject', 'test')",
        )
        .execute(&database.pool)
        .await
        .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("5")
        );
        assert!(started.elapsed() >= Duration::from_millis(4500));
        sqlx::query("ROLLBACK").execute(&mut *writer).await.unwrap();
        drop(writer);
        database.close().await;
    }

    #[tokio::test]
    async fn cloned_managers_enforce_login_limits_under_concurrent_reservations() {
        for browser_limited in [true, false] {
            let (_root, database) = crate::test_support::database().await;
            if !browser_limited {
                // Leave one global slot so simultaneous browsers must compete for it.
                sqlx::query(
                    "WITH RECURSIVE n(x) AS (
                         SELECT 1
                         UNION ALL
                         SELECT x + 1
                         FROM n
                         WHERE x < 127
                     )
                     INSERT INTO login_transactions
                     SELECT 'seed-' || x, 'seed-' || x, 'verifier', 'nonce', unixepoch() + 300
                     FROM n",
                )
                .execute(database.test_pool())
                .await
                .unwrap();
            }

            // Start all reservations together using distinct handles to the shared pool.
            let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(4));
            let mut tasks = tokio::task::JoinSet::new();
            for attempt in 0..4 {
                let database = database.clone();
                let barrier = barrier.clone();
                tasks.spawn(async move {
                    let browser = if browser_limited {
                        "shared-browser".to_string()
                    } else {
                        format!("browser-{attempt}")
                    };
                    barrier.wait().await;
                    database
                        .try_insert_login_transaction(
                            &format!("attempt-{attempt}"),
                            &browser,
                            "verifier",
                            "nonce",
                        )
                        .await
                });
            }
            let mut accepted = 0;
            while let Some(result) = tasks.join_next().await {
                accepted += usize::from(result.unwrap().unwrap());
            }
            assert_eq!(accepted, if browser_limited { 3 } else { 1 });
            let total: i64 = sqlx::query_scalar(
                "SELECT count(*)
                 FROM login_transactions",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap();
            assert_eq!(total, if browser_limited { 3 } else { 128 });
            database.close().await;
        }
    }
}
