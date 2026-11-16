//! Content validation, offline import and coordinated backups.
//! Import/backup callers hold the exclusive storage lock; online edits use database transactions.

pub(crate) mod gallery;

use crate::database::DatabaseManager;
use crate::routes::{media::YoutubeUrl, shows::parse_shows};
use crate::storage;
use image::{ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const JSON_BYTES: u64 = 2 * 1024 * 1024;

/// Safe errors for editor responses; never expose filesystem or database diagnostics.
#[derive(Debug)]
pub(crate) enum ContentError {
    Invalid(&'static str),
    Conflict,
    TooLarge,
    Busy,
    Unavailable,
}

impl From<sqlx::Error> for ContentError {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            Self::Conflict
        } else {
            Self::Unavailable
        }
    }
}

impl From<std::io::Error> for ContentError {
    fn from(_: std::io::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u8,
    pub files: BTreeMap<String, String>,
}

/// Imports trusted bundled files once; rejects links, malformed data, and an existing catalog.
/// Files are fully staged before publication. Failure before commit removes only new links.
pub async fn import(
    database: &DatabaseManager,
    data_dir: &Path,
    source: &Path,
) -> Result<(), Box<dyn Error>> {
    let source = source.canonicalize()?;
    if source.starts_with(data_dir) || data_dir.starts_with(&source) {
        return Err("Import source and private storage must not overlap".into());
    }

    storage::reject_public_symlinks(&source)?;
    if database.content_is_initialized().await? {
        return Err("Content is already initialized; import will not overwrite it".into());
    }
    // Validate source records before staging files or writing any catalog rows.
    let shows = parse_shows(std::str::from_utf8(&read_file(
        &source.join("data_json/shows.json"),
        JSON_BYTES,
    )?)?)?;
    let videos: Vec<YoutubeUrl> = serde_json::from_slice(&read_file(
        &source.join("data_json/yt_videos.json"),
        JSON_BYTES,
    )?)?;

    if shows.get_asc().len() > 10000 || videos.len() > 1000 {
        return Err("Content record limit exceeded".into());
    }
    let video_ids = videos
        .iter()
        .map(|video| youtube_id(&video.url))
        .collect::<Result<Vec<_>, _>>()?;
    // Prepare validated image pairs under private, opaque filenames before publication.
    let staging = tempfile::Builder::new()
        .prefix("import-")
        .permissions({
            use std::os::unix::fs::PermissionsExt;
            fs::Permissions::from_mode(0o700)
        })
        .tempdir_in(data_dir.join("staging"))?;
    let mut photos = Vec::new();
    for entry in fs::read_dir(source.join("img/about_pics"))? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("Gallery imports require regular files".into());
        }
        if photos.len() >= 1000 {
            return Err("Gallery import limit exceeded".into());
        }

        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Gallery filenames must be UTF-8")?;
        validate_filename(&name)?;

        let original = read_file(&entry.path(), IMAGE_BYTES)?;
        let thumbnail = read_file(
            &source
                .join("img/about_thumbnails")
                .join(format!("{name}.webp")),
            IMAGE_BYTES,
        )?;

        let format = validate_image(&original, false)?;
        validate_image(&thumbnail, true)?;
        let token = oauth2::CsrfToken::new_random().secret().clone();
        let extension = format
            .extensions_str()
            .first()
            .ok_or("Unsupported image extension")?;
        let original_fname = format!("{token}.{extension}");
        let thumbnail_fname = format!("{token}.webp");
        // Sync both staged files before recording the pair for later publication.
        for (stored, bytes) in [(&original_fname, &original), (&thumbnail_fname, &thumbnail)] {
            let mut file = private_file(&staging.path().join(stored))?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        photos.push((name, original_fname, thumbnail_fname));
    }
    photos.sort_by(|a, b| a.0.cmp(&b.0));

    database
        .import_content(shows.get_asc(), &video_ids, &photos, || {
            // Publish files before committing their rows so catalog readers never see incomplete pairs.
            let mut published = Vec::new();
            let publish = (|| -> Result<(), Box<dyn Error>> {
                for (_, original, thumbnail) in &photos {
                    for (directory, file) in [("originals", original), ("thumbnails", thumbnail)] {
                        let destination = data_dir.join("gallery").join(directory).join(file);
                        // Hard-link creation is atomic and refuses any existing destination.
                        fs::hard_link(staging.path().join(file), &destination)?;
                        published.push(destination);
                    }
                }
                for directory in ["gallery/originals", "gallery/thumbnails"] {
                    fs::File::open(data_dir.join(directory))?.sync_all()?;
                }
                Ok(())
            })();
            if let Err(error) = publish {
                // Before commit, only this attempt's new links need removal; SQL rolls back on return.
                for path in published {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
            Ok(())
        })
        .await?;

    println!(
        "Imported {} shows, {} videos, {} photos",
        shows.get_asc().len(),
        video_ids.len(),
        photos.len()
    );
    Ok(())
}

/// Creates an offline, complete database/gallery snapshot without running migrations.
/// A directory without COMPLETE is incomplete and must never be restored.
pub async fn backup(
    database: &DatabaseManager,
    data_dir: &Path,
    output: &Path,
    public_roots: &[PathBuf],
) -> Result<(), Box<dyn Error>> {
    let output = validate_backup_destination(output, data_dir, public_roots)?;
    // Parent directories are operator-provisioned; never change their ownership or mode.
    create_private_dir(&output)?;
    create_private_dir(&output.join("gallery"))?;

    let mut files = BTreeMap::new();
    let snapshot = output.join("admin.sqlite3");
    // VACUUM INTO accepts an empty file; reserve it privately even under a permissive caller umask.
    private_file(&snapshot)?;
    database.snapshot_into(&snapshot).await?;
    storage::check_private(&snapshot, false)?;
    files.insert("admin.sqlite3".into(), file_hash(&snapshot)?);
    fs::File::open(&snapshot)?.sync_all()?;

    // Copy both gallery trees while the caller's maintenance lock keeps writers stopped.
    for directory in ["originals", "thumbnails"] {
        create_private_dir(&output.join("gallery").join(directory))?;
        for entry in fs::read_dir(data_dir.join("gallery").join(directory))? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "Backup filename must be UTF-8")?;
            validate_filename(&name)?;
            let relative = format!("gallery/{directory}/{name}");
            let checksum = copy_file(&entry.path(), &output.join(&relative))?;
            files.insert(relative, checksum);
        }
        fs::File::open(output.join("gallery").join(directory))?.sync_all()?;
    }
    // Every database reference must have a matching copied file before completion.
    for (original, thumbnail) in database.gallery_references().await? {
        validate_filename(&original)?;
        validate_filename(&thumbnail)?;
        if !files.contains_key(&format!("gallery/originals/{original}"))
            || !files.contains_key(&format!("gallery/thumbnails/{thumbnail}"))
        {
            return Err("A referenced gallery file is missing; backup is incomplete".into());
        }
    }
    // Record checksums only after validating the snapshot and its gallery references.
    let manifest = serde_json::to_vec_pretty(&BackupManifest { version: 1, files })?;
    let mut file = private_file(&output.join("manifest.json"))?;
    file.write_all(&manifest)?;
    file.sync_all()?;
    fs::File::open(output.join("gallery"))?.sync_all()?;

    // Write the completion marker last, after the backup files and manifest have been synced.
    let mut complete = private_file(&output.join("COMPLETE"))?;
    complete
        .write_all(format!("{}\n", crate::encode_hex(&Sha256::digest(&manifest))).as_bytes())?;
    complete.sync_all()?;
    fs::File::open(&output)?.sync_all()?;
    fs::File::open(output.parent().ok_or("Backup directory needs a parent")?)?.sync_all()?;
    println!("Backup complete");
    Ok(())
}

/// Validates supported YouTube URLs without fetching user-supplied addresses.
pub fn youtube_id(input: &str) -> Result<String, Box<dyn Error>> {
    if input.len() > 2048 {
        return Err("Video URL is too long".into());
    }
    let url = url::Url::parse(input)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Use an HTTPS YouTube URL without credentials, a custom port, or a fragment".into(),
        );
    }
    let host = url.host_str().unwrap_or_default();

    let parts: Vec<_> = url.path().trim_start_matches('/').split('/').collect();
    // Normalize supported URL shapes to the single video ID stored in the catalog.
    let id = match (host, parts.as_slice()) {
        ("youtu.be", [id]) => (*id).to_string(),
        ("youtube.com" | "www.youtube.com" | "m.youtube.com", ["watch"]) => {
            let ids: Vec<_> = url
                .query_pairs()
                .filter(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned())
                .collect();
            if ids.len() != 1 {
                return Err("A watch URL needs exactly one video ID".into());
            }
            ids[0].clone()
        }
        (
            "youtube.com" | "www.youtube.com" | "m.youtube.com",
            ["embed" | "shorts" | "live", id],
        )
        | ("youtube-nocookie.com" | "www.youtube-nocookie.com", ["embed", id]) => (*id).to_string(),
        _ => return Err("Unsupported YouTube URL".into()),
    };
    if id.len() != 11
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    {
        return Err("Invalid YouTube video ID".into());
    }
    Ok(id)
}

/// Accept a real ISO date from a native date input, retaining the optional show fields.
pub(crate) fn validate_show(
    date: &str,
    name: &str,
    club: &str,
    location: &str,
) -> Result<(), ContentError> {
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| ContentError::Invalid("Choose a real calendar date."))?;
    if parsed.format("%Y-%m-%d").to_string() != date || date.len() != 10 {
        return Err(ContentError::Invalid(
            "Use a date between years 0001 and 9999.",
        ));
    }
    if date.starts_with("0000") {
        return Err(ContentError::Invalid(
            "Use a date between years 0001 and 9999.",
        ));
    }
    if [name, club, location]
        .iter()
        .any(|value| value.len() > 500 || value.chars().any(char::is_control))
    {
        return Err(ContentError::Invalid(
            "Show fields must be at most 500 bytes without control characters.",
        ));
    }
    Ok(())
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err("Invalid or oversized content file".into());
    }
    let mut bytes = Vec::new();
    // Reading one extra byte also detects a file growing after the metadata check.
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("Content file exceeds its byte limit".into());
    }
    Ok(bytes)
}

fn validate_image(bytes: &[u8], thumbnail: bool) -> Result<ImageFormat, Box<dyn Error>> {
    let mut reader = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP)
    ) || (thumbnail && reader.format() != Some(ImageFormat::WebP))
    {
        return Err("Only JPEG/PNG/WebP originals and WebP thumbnails are supported".into());
    }
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    reader.limits(limits);

    // Check dimensions before allocating and decoding the full image.
    let dimensions = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()?
        .into_dimensions()?;
    if u64::from(dimensions.0) * u64::from(dimensions.1) > 40_000_000
        || (thumbnail && dimensions != (480, 270))
    {
        return Err("Image dimensions exceed the limit or thumbnail is not 480x270".into());
    }
    let format = reader.format().ok_or("Unrecognized image format")?;
    reader.decode()?;
    Ok(format)
}

fn private_file(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

fn create_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(path)
}

/// Validate filename
fn validate_filename(name: &str) -> Result<(), Box<dyn Error>> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err("Invalid gallery filename".into());
    }
    Ok(())
}

fn validate_backup_destination(
    output: &Path,
    data_dir: &Path,
    public_roots: &[PathBuf],
) -> Result<PathBuf, Box<dyn Error>> {
    let output = std::path::absolute(output)?;
    if output
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return Err("Backup paths must not contain parent traversal".into());
    }
    // Resolve aliases before checking overlap with public assets and managed data paths.
    let output = storage::resolve_missing(&output)?;
    for root in public_roots {
        let root = storage::resolve_missing(&std::path::absolute(root)?)?;
        if output.starts_with(&root) || root.starts_with(&output) {
            return Err("Backups must stay outside public asset trees".into());
        }
    }
    if data_dir.starts_with(&output)
        || output.starts_with(data_dir.join("gallery"))
        || output.starts_with(data_dir.join("staging"))
    {
        return Err("Backup output must not overlap managed gallery/staging paths".into());
    }
    Ok(output)
}

fn copy_file(source: &Path, destination: &Path) -> Result<String, Box<dyn Error>> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut input = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)?;
    if !input.metadata()?.is_file() {
        return Err("Backup inputs must be regular files".into());
    }

    let mut output = private_file(destination)?;
    let mut hash = Sha256::new();
    // Stream the copy and checksum together without loading an entire file into memory.
    let mut buffer = [0; 65536];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
    }
    output.sync_all()?;
    Ok(crate::encode_hex(&hash.finalize()))
}

fn file_hash(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    // Bound hashing memory independently of the database snapshot's size.
    let mut bytes = [0; 65536];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    Ok(crate::encode_hex(&hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> tempfile::TempDir {
        let source = tempfile::tempdir().unwrap();
        for directory in ["data_json", "img/about_pics", "img/about_thumbnails"] {
            fs::create_dir_all(source.path().join(directory)).unwrap();
        }
        fs::write(
            source.path().join("data_json/shows.json"),
            r#"[{"date":"29.02.2024","name":"Example show","club":null}]"#,
        )
        .unwrap();
        fs::write(
            source.path().join("data_json/yt_videos.json"),
            r#"[{"url":"https://www.youtube-nocookie.com/embed/3uh_Jc1prRQ"}]"#,
        )
        .unwrap();
        image::RgbImage::from_pixel(48, 27, image::Rgb([120, 40, 80]))
            .save_with_format(
                source.path().join("img/about_pics/photo é.jpg"),
                ImageFormat::Jpeg,
            )
            .unwrap();
        image::RgbImage::from_pixel(480, 270, image::Rgb([120, 40, 80]))
            .save_with_format(
                source.path().join("img/about_thumbnails/photo é.jpg.webp"),
                ImageFormat::WebP,
            )
            .unwrap();
        source
    }

    async fn count(database: &DatabaseManager, query: &'static str) -> i64 {
        sqlx::query_scalar(query)
            .fetch_one(database.test_pool())
            .await
            .unwrap()
    }

    #[test]
    fn youtube_validation_accepts_supported_forms_and_rejects_spoofed_urls() {
        for url in [
            "https://youtu.be/3uh_Jc1prRQ",
            "https://www.youtube.com/watch?v=3uh_Jc1prRQ&t=1",
            "https://youtube.com/shorts/3uh_Jc1prRQ",
            "https://www.youtube-nocookie.com/embed/3uh_Jc1prRQ",
        ] {
            assert_eq!(youtube_id(url).unwrap(), "3uh_Jc1prRQ");
        }
        for url in [
            "http://youtu.be/3uh_Jc1prRQ",
            "https://youtube.com.evil.test/embed/3uh_Jc1prRQ",
            "https://user@youtube.com/embed/3uh_Jc1prRQ",
            "https://youtube.com:8443/embed/3uh_Jc1prRQ",
            "https://youtube.com/watch?v=3uh_Jc1prRQ&v=aaRGcLUQ4ss",
            "https://youtu.be/short",
            "https://www.youtube.com/embed/3uh_Jc1prRQ/extra",
            "https://youtu.be/%2Fuh_Jc1prRQ",
            "<iframe src='https://youtube.com'></iframe>",
        ] {
            assert!(youtube_id(url).is_err(), "{url}");
        }
    }

    #[tokio::test]
    async fn import_is_complete_private_and_repeat_safe() {
        use std::os::unix::fs::MetadataExt;
        let (root, database) = crate::test_support::database().await;
        let source = source();
        import(&database, root.path(), source.path()).await.unwrap();
        assert_eq!(
            count(
                &database,
                "SELECT count(*)
                 FROM content_import"
            )
            .await,
            1
        );
        let (date, name, club): (String, String, Option<String>) = sqlx::query_as(
            "SELECT date, name, club
             FROM shows",
        )
        .fetch_one(database.test_pool())
        .await
        .unwrap();
        assert_eq!(
            (date.as_str(), name.as_str(), club),
            ("2024-02-29", "Example show", None)
        );
        let (public, original, thumbnail): (String, String, String) = sqlx::query_as(
            "SELECT public_name, original_file, thumbnail_file
             FROM gallery_images",
        )
        .fetch_one(database.test_pool())
        .await
        .unwrap();
        assert_eq!(public, "photo é.jpg");
        for (directory, file) in [("originals", original), ("thumbnails", thumbnail)] {
            assert!(!file.contains('/'));
            let metadata =
                fs::metadata(root.path().join("gallery").join(directory).join(file)).unwrap();
            assert_eq!(metadata.mode() & 0o777, 0o600);
            assert_eq!(metadata.nlink(), 1);
        }
        assert_eq!(
            fs::read_dir(root.path().join("staging")).unwrap().count(),
            0
        );
        sqlx::query(
            "UPDATE shows SET
                 name = 'edited'",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        assert!(import(&database, root.path(), source.path()).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT name
                 FROM shows",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap(),
            "edited"
        );
        database.close().await;
    }

    #[tokio::test]
    async fn invalid_imports_publish_no_partial_catalog_or_gallery() {
        for invalid in ["date", "thumbnail", "duplicate", "symlink", "oversized"] {
            let (root, database) = crate::test_support::database().await;
            let source = source();
            match invalid {
                "date" => fs::write(source.path().join("data_json/shows.json"), r#"[{"date":"31.02.2026"}]"#).unwrap(),
                "thumbnail" => fs::write(source.path().join("img/about_thumbnails/photo é.jpg.webp"), b"<svg/>").unwrap(),
                "duplicate" => fs::write(source.path().join("data_json/yt_videos.json"), r#"[{"url":"https://youtu.be/3uh_Jc1prRQ"},{"url":"https://youtu.be/3uh_Jc1prRQ"}]"#).unwrap(),
                "symlink" => {
                    fs::remove_file(source.path().join("img/about_pics/photo é.jpg")).unwrap();
                    std::os::unix::fs::symlink(source.path().join("data_json/shows.json"), source.path().join("img/about_pics/photo é.jpg")).unwrap();
                },
                "oversized" => fs::File::create(source.path().join("img/about_pics/photo é.jpg")).unwrap().set_len(IMAGE_BYTES + 1).unwrap(),
                _ => unreachable!(),
            }
            assert!(
                import(&database, root.path(), source.path()).await.is_err(),
                "{invalid}"
            );
            for query in [
                "SELECT count(*)
                 FROM content_import",
                "SELECT count(*)
                 FROM shows",
                "SELECT count(*)
                 FROM videos",
                "SELECT count(*)
                 FROM gallery_images",
            ] {
                assert_eq!(count(&database, query).await, 0, "{invalid}");
            }
            for directory in ["staging", "gallery/originals", "gallery/thumbnails"] {
                assert_eq!(
                    fs::read_dir(root.path().join(directory)).unwrap().count(),
                    0,
                    "{invalid}"
                );
            }
            database.close().await;
        }
    }

    #[tokio::test]
    async fn failed_file_publication_rolls_back_rows_and_removes_new_links() {
        let (root, database) = crate::test_support::database().await;
        let source = source();
        // Simulate storage disappearing after startup validation but before publication.
        fs::remove_dir(root.path().join("gallery/thumbnails")).unwrap();
        assert!(import(&database, root.path(), source.path()).await.is_err());
        assert_eq!(
            count(
                &database,
                "SELECT count(*)
                 FROM content_import"
            )
            .await,
            0
        );
        assert_eq!(
            count(
                &database,
                "SELECT count(*)
                 FROM gallery_images"
            )
            .await,
            0
        );
        assert_eq!(
            fs::read_dir(root.path().join("gallery/originals"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(
            fs::read_dir(root.path().join("staging")).unwrap().count(),
            0
        );
        database.close().await;
    }

    #[test]
    fn backup_destination_rejects_managed_paths_traversal_and_public_aliases() {
        let root = crate::test_support::directory();
        let public = tempfile::tempdir().unwrap();
        let roots = [public.path().to_path_buf()];
        for output in [
            root.path().to_path_buf(),
            root.path().join("gallery/copy"),
            root.path().join("staging/copy"),
            root.path().join("../copy"),
        ] {
            assert!(validate_backup_destination(&output, root.path(), &roots).is_err());
        }
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(public.path(), &alias).unwrap();
        assert!(validate_backup_destination(&alias.join("copy"), root.path(), &roots).is_err());
    }

    #[tokio::test]
    async fn backup_snapshots_and_restores_matching_database_and_gallery() {
        let (root, database) = crate::test_support::database().await;
        let source = source();
        import(&database, root.path(), source.path()).await.unwrap();
        database
            .grant_administrator("https://accounts.google.com", "123", "owner")
            .await
            .unwrap();
        let destination = tempfile::tempdir().unwrap();
        let output = destination.path().join("snapshot");
        backup(
            &database,
            root.path(),
            &output,
            &[source.path().to_path_buf()],
        )
        .await
        .unwrap();
        let bytes = fs::read(output.join("manifest.json")).unwrap();
        assert_eq!(
            fs::read_to_string(output.join("COMPLETE")).unwrap().trim(),
            crate::encode_hex(&Sha256::digest(&bytes))
        );
        let manifest: BackupManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.files.len(), 3);
        for (file, checksum) in &manifest.files {
            assert_eq!(&file_hash(&output.join(file)).unwrap(), checksum);
        }
        assert!(backup(&database, root.path(), &output, &[]).await.is_err());
        assert!(backup(
            &database,
            root.path(),
            &source.path().join("backup"),
            &[source.path().into()]
        )
        .await
        .is_err());
        let restore = crate::test_support::directory();
        crate::storage::prepare(restore.path(), &[]).unwrap();
        for file in manifest.files.keys() {
            fs::copy(output.join(file), restore.path().join(file)).unwrap();
        }
        let restored = DatabaseManager::initialize(restore.path()).await.unwrap();
        assert_eq!(
            count(
                &restored,
                "SELECT count(*)
                 FROM shows"
            )
            .await,
            1
        );
        assert_eq!(
            count(
                &restored,
                "SELECT count(*)
                 FROM gallery_images"
            )
            .await,
            1
        );
        assert_eq!(restored.administrators().await.unwrap()[0].subject, "123");
        restored.revoke_all_sessions().await.unwrap();
        assert_eq!(
            restored.administrators().await.unwrap()[0].session_generation,
            1
        );
        restored.close().await;
        database.close().await;
    }

    #[tokio::test]
    async fn backup_refuses_links_and_missing_references_without_a_completion_marker() {
        for link in [false, true] {
            let (root, database) = crate::test_support::database().await;
            let source = source();
            import(&database, root.path(), source.path()).await.unwrap();
            let original: String = sqlx::query_scalar(
                "SELECT original_file
                 FROM gallery_images",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap();
            let original = root.path().join("gallery/originals").join(original);
            fs::remove_file(&original).unwrap();
            if link {
                std::os::unix::fs::symlink(root.path().join("admin.sqlite3"), original).unwrap();
            }
            let destination = tempfile::tempdir().unwrap();
            let output = destination.path().join("incomplete");
            assert!(backup(&database, root.path(), &output, &[]).await.is_err());
            assert!(!output.join("COMPLETE").exists());
            database.close().await;
        }
    }
}
