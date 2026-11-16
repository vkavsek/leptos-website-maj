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

/// Real import input for regressions that manage content published before sign-in.
pub fn content_source() -> tempfile::TempDir {
    let source = directory();
    for directory in ["data_json", "img/about_pics", "img/about_thumbnails"] {
        std::fs::create_dir_all(source.path().join(directory)).unwrap();
    }
    std::fs::write(source.path().join("data_json/shows.json"),
        r#"[{"date":"29.02.2024","name":"Published chromium"},{"date":"01.01.2025","name":"Published firefox"}]"#).unwrap();
    std::fs::write(
        source.path().join("data_json/yt_videos.json"),
        r#"[{"url":"https://youtu.be/RBwe-c9T5sY"},{"url":"https://youtu.be/73KJzjvW_1Q"}]"#,
    )
    .unwrap();
    for browser in ["chromium", "firefox"] {
        let name = format!("published_{browser}.jpg");
        image::RgbImage::from_pixel(48, 27, image::Rgb([40, 100, 80]))
            .save_with_format(
                source.path().join("img/about_pics").join(&name),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        image::RgbImage::from_pixel(480, 270, image::Rgb([40, 100, 80]))
            .save_with_format(
                source
                    .path()
                    .join("img/about_thumbnails")
                    .join(format!("{name}.webp")),
                image::ImageFormat::WebP,
            )
            .unwrap();
    }
    source
}
