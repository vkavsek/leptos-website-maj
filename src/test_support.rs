use crate::{database::DatabaseManager, server_config::ServerConfig};

pub fn directory() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}

pub async fn database() -> (tempfile::TempDir, DatabaseManager) {
    let root = directory();
    crate::storage::prepare(root.path(), &[]).unwrap();
    let database = DatabaseManager::initialize(root.path()).await.unwrap();
    (root, database)
}

pub fn config(root: &std::path::Path) -> ServerConfig {
    ServerConfig {
        development: true,
        app_origin: "http://localhost:8080".into(),
        data_dir: root.into(),
        google: None,
        porkmail_pwd: None,
    }
}
