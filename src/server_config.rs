//! Server-only runtime configuration; credentials never enter hydration data.

use http::Uri;
use secrecy::SecretString;
use std::{env, net::IpAddr, path::PathBuf};

pub struct ServerConfig {
    pub development: bool,
    pub app_origin: String,
    pub data_dir: PathBuf,
    pub google: Option<GoogleCredentials>,
    pub porkmail_pwd: Option<SecretString>,
}

pub struct GoogleCredentials {
    pub client_id: String,
    pub client_secret: SecretString,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::parse(
            read_env("APP_ENVIRONMENT")?.as_deref(),
            required("MAJ_APP_ORIGIN", read_env("MAJ_APP_ORIGIN")?)?,
            required("MAJ_DATA_DIR", read_env("MAJ_DATA_DIR")?)?,
            read_env("GOOGLE_CLIENT_ID")?,
            read_env("GOOGLE_CLIENT_SECRET")?,
            env::var("PORKMAIL_PWD").ok(),
        )
    }

    pub fn google_callback(&self) -> String {
        format!("{}/auth/google/callback", self.app_origin)
    }

    fn parse(
        environment: Option<&str>,
        app_origin: String,
        data_dir: String,
        client_id: Option<String>,
        client_secret: Option<String>,
        porkmail_pwd: Option<String>,
    ) -> Result<Self, String> {
        let development = match environment {
            Some("development") => true,
            Some("production") => false,
            None => cfg!(debug_assertions),
            _ => return Err("APP_ENVIRONMENT must be development or production".into()),
        };
        let uri: Uri = app_origin
            .parse()
            .map_err(|_| "MAJ_APP_ORIGIN must be an HTTP(S) origin without a path")?;
        let authority = uri.authority().ok_or("MAJ_APP_ORIGIN needs a hostname")?;
        let scheme = uri.scheme_str().unwrap_or_default();
        let host = authority.host();
        let valid_authority = authority.as_str() == host
            || authority.port().is_some_and(|port| {
                port.as_u16() > 0 && authority.as_str() == format!("{host}:{port}")
            });

        let loopback = host.eq_ignore_ascii_case("localhost")
            || host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        if !valid_authority
            || authority.as_str().contains('@')
            || app_origin != format!("{scheme}://{authority}")
            || !matches!(scheme, "http" | "https")
        {
            return Err("MAJ_APP_ORIGIN must be an HTTP(S) origin without credentials, path, query, fragment, or invalid port".into());
        }
        if development && !loopback {
            return Err("MAJ_APP_ORIGIN must use loopback in development".into());
        }
        if !development && (scheme != "https" || loopback) {
            return Err("MAJ_APP_ORIGIN must use a public HTTPS hostname in production".into());
        }

        let data_dir = PathBuf::from(data_dir);
        if !data_dir.is_absolute() {
            return Err("MAJ_DATA_DIR must be an absolute path".into());
        }
        let google = match (client_id, client_secret) {
            (None, None) if development => None,
            (client_id, client_secret) => {
                let client_id = required("GOOGLE_CLIENT_ID", client_id)?;
                if !client_id.ends_with(".apps.googleusercontent.com") {
                    return Err(
                        "GOOGLE_CLIENT_ID must be an OAuth client ID, not a project ID".into(),
                    );
                }
                let client_secret = required("GOOGLE_CLIENT_SECRET", client_secret)?;
                Some(GoogleCredentials {
                    client_id,
                    client_secret: client_secret.into(),
                })
            }
        };
        Ok(Self {
            development,
            app_origin,
            data_dir,
            google,
            porkmail_pwd: porkmail_pwd
                .filter(|password| !password.is_empty())
                .map(SecretString::from),
        })
    }
}

fn read_env(name: &str) -> Result<Option<String>, String> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(format!("{name} must be valid UTF-8")),
    }
}

fn required(name: &str, value: Option<String>) -> Result<String, String> {
    let value = value.ok_or_else(|| format!("{name} is required"))?;
    if value.is_empty()
        || value.trim() != value
        || value
            .get(..13)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("REPLACE_WITH_"))
        || value.eq_ignore_ascii_case("placeholder")
        || value.eq_ignore_ascii_case("changeme")
        || value.contains("YOUR_DOMAIN")
    {
        return Err(format!(
            "{name} must be nonempty and must not be a placeholder"
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    fn parse(
        environment: &str,
        origin: &str,
        id: Option<&str>,
        secret: Option<&str>,
    ) -> Result<ServerConfig, String> {
        ServerConfig::parse(
            Some(environment),
            origin.into(),
            "/tmp/maj-config-test".into(),
            id.map(str::to_owned),
            secret.map(str::to_owned),
            None,
        )
    }

    #[test]
    fn local_public_development_needs_no_google_credentials() {
        let config = parse("development", "http://localhost:8080", None, None).unwrap();
        assert!(config.google.is_none());
        assert_eq!(
            config.google_callback(),
            "http://localhost:8080/auth/google/callback"
        );
        assert_eq!(config.data_dir, PathBuf::from("/tmp/maj-config-test"));
        assert!(parse("development", "http://[::1]:8080", None, None).is_ok());
    }

    #[test]
    fn smtp_password_is_optional_and_kept_as_a_secret() {
        for password in [None, Some(""), Some("fixture-smtp-password")] {
            let config = ServerConfig::parse(
                Some("development"),
                "http://localhost:8080".into(),
                "/tmp/maj-config-test".into(),
                None,
                None,
                password.map(str::to_owned),
            )
            .unwrap();
            assert_eq!(
                config
                    .porkmail_pwd
                    .as_ref()
                    .map(ExposeSecret::expose_secret),
                password.filter(|password| !password.is_empty())
            );
        }
    }

    #[test]
    fn production_requires_a_complete_pair_and_hides_invalid_values() {
        assert!(parse("production", "https://www.majkavsek.com", None, None).is_err());
        let config = parse(
            "production",
            "https://www.majkavsek.com",
            Some("fixture.apps.googleusercontent.com"),
            Some("fixture-secret"),
        )
        .unwrap();
        let google = config.google.unwrap();
        assert_eq!(google.client_id, "fixture.apps.googleusercontent.com");
        assert_eq!(google.client_secret.expose_secret(), "fixture-secret");
        for (id, secret) in [
            (Some("fixture.apps.googleusercontent.com"), None),
            (None, Some("fixture-secret")),
            (
                Some("maj-webpage-admin-development"),
                Some("fixture-secret"),
            ),
            (
                Some("fixture.apps.googleusercontent.com"),
                Some("REPLACE_WITH_SECRET"),
            ),
            (Some("fixture.apps.googleusercontent.com"), Some("")),
        ] {
            let error = parse("development", "http://localhost:8080", id, secret)
                .err()
                .unwrap();
            assert!(!error.contains("fixture-secret"));
            assert!(!error.contains("REPLACE_WITH_SECRET"));
        }
    }

    #[test]
    fn invalid_origins_paths_and_environments_fail_closed() {
        for origin in [
            "http://example.com",
            "http://localhost.evil.test",
            "http://localhost:8080/",
            "http://localhost:8080/admin",
            "http://localhost:8080?secret=fixture",
            "http://localhost:8080#fixture",
            "http://user:fixture@localhost:8080",
            "http://localhost:99999",
            "http://localhost:0",
            "ftp://localhost",
            "localhost:8080",
        ] {
            assert!(
                parse("development", origin, None, None).is_err(),
                "{origin}"
            );
        }
        assert!(parse("production", "http://www.majkavsek.com", None, None).is_err());
        assert!(parse("production", "https://localhost", None, None).is_err());
        assert!(parse("staging", "http://localhost", None, None).is_err());
        assert!(ServerConfig::parse(
            Some("development"),
            "http://localhost".into(),
            "relative/data".into(),
            None,
            None,
            None
        )
        .is_err());
    }
}
