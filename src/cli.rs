//! Shell-only administration. Identity approval never requires Google credentials.

use crate::database::DatabaseManager;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Portfolio server and private database administration"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Admin(AdminCommand),
}

#[derive(Subcommand)]
pub enum AdminCommand {
    /// Initialize application and session schemas without opening HTTP.
    Migrate,
    /// List explicitly approved Google identities as JSON.
    ListUsers,
    /// Approve a verified Google issuer/subject; never use an email as the subject.
    GrantGoogle {
        #[arg(long, value_parser = google_issuer)]
        issuer: String,
        #[arg(long, value_parser = google_subject)]
        subject: String,
        #[arg(long, value_parser = display_label)]
        label: String,
    },
    /// Disable an identity and invalidate its existing administrator sessions.
    RevokeGoogle {
        #[arg(long, value_parser = google_issuer)]
        issuer: String,
        #[arg(long, value_parser = google_subject)]
        subject: String,
    },
    /// Advance all generations and delete browser sessions and login transactions.
    RevokeAllSessions,
    /// Import bundled content once. Requires the HTTP server to be stopped.
    ImportContent {
        #[arg(long)]
        source: PathBuf,
    },
    /// Snapshot database/gallery into a new directory. Stop the HTTP server first.
    Backup {
        #[arg(long)]
        output: PathBuf,
    },
}

impl AdminCommand {
    pub fn needs_exclusive_storage(&self) -> bool {
        matches!(self, Self::ImportContent { .. } | Self::Backup { .. })
    }
}

/// Executes a validated command; stdout contains only its requested result.
pub async fn run(
    command: AdminCommand,
    database: &DatabaseManager,
    data_dir: &std::path::Path,
    public_roots: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AdminCommand::Migrate => println!("Database migrations and session schema are ready"),
        AdminCommand::ListUsers => {
            println!(
                "{}",
                serde_json::to_string_pretty(&database.administrators().await?)?
            )
        }
        AdminCommand::GrantGoogle {
            issuer,
            subject,
            label,
        } => {
            let id = database
                .grant_administrator(&issuer, &subject, &label)
                .await?;
            println!("Approved administrator {id}");
        }
        AdminCommand::RevokeGoogle { issuer, subject } => {
            if !database.revoke_administrator(&issuer, &subject).await? {
                return Err("No matching approved identity".into());
            }
            println!("Identity disabled; previous administrator sessions invalidated");
        }
        AdminCommand::RevokeAllSessions => {
            database.revoke_all_sessions().await?;
            println!("All sessions invalidated");
        }
        AdminCommand::ImportContent { source } => {
            crate::content::import(database, data_dir, &source).await?
        }
        AdminCommand::Backup { output } => {
            crate::content::backup(database, data_dir, &output, public_roots).await?
        }
    }
    Ok(())
}

fn google_issuer(value: &str) -> Result<String, String> {
    if value == "https://accounts.google.com" {
        Ok(value.into())
    } else {
        Err("Use the verified Google issuer https://accounts.google.com".into())
    }
}

fn google_subject(value: &str) -> Result<String, String> {
    if !value.is_empty() && value.len() <= 255 && value.bytes().all(|c| c.is_ascii_graphic()) {
        Ok(value.into())
    } else {
        Err("Subject must be a verified nonempty ASCII identifier of at most 255 bytes".into())
    }
}

fn display_label(value: &str) -> Result<String, String> {
    if !value.is_empty()
        && value.trim() == value
        && value.chars().count() <= 100
        && !value.chars().any(char::is_control)
    {
        Ok(value.into())
    } else {
        Err("Label must contain 1–100 characters without control characters or surrounding whitespace".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_requires_valid_explicit_subcommands_and_identity_arguments() {
        Cli::command().debug_assert();
        assert!(Cli::try_parse_from(["maj"]).unwrap().command.is_none());
        for args in [
            vec!["maj", "admin"],
            vec!["maj", "admin", "unknown"],
            vec!["maj", "admin", "grant-google"],
            vec!["maj", "admin", "backup"],
            vec!["maj", "admin", "list-users", "--unknown"],
            vec![
                "maj",
                "admin",
                "grant-google",
                "--issuer",
                "https://evil.test",
                "--subject",
                "123",
                "--label",
                "owner",
            ],
            vec![
                "maj",
                "admin",
                "grant-google",
                "--issuer",
                "https://accounts.google.com",
                "--subject",
                "",
                "--label",
                "owner",
            ],
            vec![
                "maj",
                "admin",
                "grant-google",
                "--issuer",
                "https://accounts.google.com",
                "--subject",
                "123",
                "--label",
                "owner\nforged log",
            ],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
        assert!(Cli::try_parse_from([
            "maj",
            "admin",
            "grant-google",
            "--issuer",
            "https://accounts.google.com",
            "--subject",
            "123",
            "--label",
            "owner"
        ])
        .is_ok());
        assert!(
            Cli::try_parse_from(["maj", "admin", "backup", "--output", "a path with spaces"])
                .is_ok()
        );
    }

    #[tokio::test]
    async fn approval_is_explicit_idempotent_and_never_revives_old_generations() {
        let (_root, database) = crate::test_support::database().await;
        assert!(database.administrators().await.unwrap().is_empty());
        let issuer = "https://accounts.google.com";
        let id = database
            .grant_administrator(issuer, "123", "owner")
            .await
            .unwrap();
        assert_eq!(
            database
                .grant_administrator(issuer, "123", "owner")
                .await
                .unwrap(),
            id
        );
        assert_eq!(
            database.administrators().await.unwrap()[0].session_generation,
            0
        );
        assert!(database.revoke_administrator(issuer, "123").await.unwrap());
        assert!(!database.administrators().await.unwrap()[0].enabled);
        database
            .grant_administrator(issuer, "123", "owner")
            .await
            .unwrap();
        assert_eq!(
            database.administrators().await.unwrap()[0].session_generation,
            2
        );
        assert!(!database
            .revoke_administrator(issuer, "missing")
            .await
            .unwrap());
        database
            .grant_administrator(issuer, "second", "second admin")
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tower_sessions (id, data, expiry_date)
             VALUES ('test', X'01', 9999999999)",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO login_transactions
             VALUES ('state', 'browser', 'verifier', 'nonce', 9999999999)",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        database.revoke_all_sessions().await.unwrap();
        let approvals = database.administrators().await.unwrap();
        assert_eq!(approvals.len(), 2);
        assert_eq!(approvals[0].session_generation, 3);
        assert_eq!(approvals[1].session_generation, 1);
        assert!(approvals.iter().all(|user| user.enabled));
        for query in [
            "SELECT count(*)
             FROM tower_sessions",
            "SELECT count(*)
             FROM login_transactions",
        ] {
            assert_eq!(
                sqlx::query_scalar::<_, i64>(query)
                    .fetch_one(database.test_pool())
                    .await
                    .unwrap(),
                0
            );
        }
        // Exhausted generations must fail atomically instead of becoming SQLite REAL values.
        sqlx::query(
            "UPDATE administrators SET
                 session_generation = ?
             WHERE id = ?",
        )
        .bind(i64::MAX)
        .bind(id)
        .execute(database.test_pool())
        .await
        .unwrap();
        assert!(database.revoke_all_sessions().await.is_err());
        assert_eq!(
            database.administrators().await.unwrap()[1].session_generation,
            1
        );
        database.close().await;
    }
}
