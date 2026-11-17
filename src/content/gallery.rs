//! Decode uploads in private staging, publish complete pairs, and serve catalogued files only.

use super::{private_file, ContentError};
use crate::{database::DatabaseManager, server_config::ServerConfig};
use axum::{
    extract::Path,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use http::{header, Request, StatusCode};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::{
    fs,
    io::Cursor,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub(crate) const UPLOAD_BYTES: usize = 10 * 1024 * 1024;
const PIXELS: u64 = 12_000_000;

/// RAII staging keeps failed/cancelled preparation private and removes temporary files.
pub(crate) struct PreparedImage {
    pub name: String,
    pub original: String,
    pub thumbnail: String,
    staging: tempfile::TempDir,
}

impl PreparedImage {
    /// Atomic no-replace links work across directories on the same validated volume.
    /// Remove only links created here if publication fails before the database commit.
    pub(crate) fn publish(&self, root: &FsPath) -> std::io::Result<()> {
        let mut published = Vec::new();
        let result = (|| {
            for (directory, name) in [
                ("originals", &self.original),
                ("thumbnails", &self.thumbnail),
            ] {
                let destination = root.join("gallery").join(directory).join(name);
                fs::hard_link(self.staging.path().join(name), &destination)?;
                published.push(destination);
            }
            for directory in ["gallery/originals", "gallery/thumbnails"] {
                fs::File::open(root.join(directory))?.sync_all()?;
            }
            Ok(())
        })();

        if result.is_err() {
            for path in published {
                let _ = fs::remove_file(path);
            }
        }
        result
    }
}

/// Called on a blocking worker under the single processing permit, never a Tokio IO thread.
/// Inspect actual bytes, enforce dimensions before decoding, apply orientation and re-encode
/// pixels only. Metadata and appended payloads are not copied to the published files.
pub(crate) fn prepare(
    root: &FsPath,
    filename: &str,
    bytes: Vec<u8>,
) -> Result<PreparedImage, ContentError> {
    if bytes.len() > UPLOAD_BYTES {
        return Err(ContentError::Invalid("Photos must be at most 10 MiB."));
    }

    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let format = reader
        .format()
        .ok_or(ContentError::Invalid("Choose a JPEG, PNG or WebP photo."))?;

    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
    ) {
        return Err(ContentError::Invalid("Choose a JPEG, PNG or WebP photo."));
    }

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(6000);
    limits.max_image_height = Some(6000);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| ContentError::Invalid("The photo is damaged or exceeds the image limits."))?;

    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > PIXELS {
        return Err(ContentError::Invalid(
            "Photos must be at most 12 megapixels and 6000 pixels per side.",
        ));
    }

    let orientation = decoder
        .orientation()
        .map_err(|_| ContentError::Invalid("The photo metadata is invalid."))?;
    let image = DynamicImage::from_decoder(decoder).map_err(|_| {
        ContentError::Invalid("The photo could not be decoded within the image limits.")
    })?;

    // Keep JPEGs compact; normalize other formats to 8-bit PNG to preserve transparency.
    let output = if format == ImageFormat::Jpeg {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Png
    };

    let extension = if output == ImageFormat::Jpeg {
        "jpg"
    } else {
        "png"
    };
    let name = normalized_name(filename, extension)?;
    let mut image = if output == ImageFormat::Jpeg {
        DynamicImage::ImageRgb8(image.into_rgb8())
    } else {
        DynamicImage::ImageRgba8(image.into_rgba8())
    };
    // Normalize first: rotating a 16-bit image would otherwise duplicate its larger pixel buffer.
    image.apply_orientation(orientation);

    let staging = tempfile::Builder::new()
        .prefix("upload-")
        .permissions({
            use std::os::unix::fs::PermissionsExt;
            fs::Permissions::from_mode(0o700)
        })
        .tempdir_in(root.join("staging"))?;

    let token = crate::encode_hex(oauth2::CsrfToken::new_random().secret().as_bytes());
    let prepared = PreparedImage {
        name,
        original: format!("{token}.{extension}"),
        thumbnail: format!("{token}.webp"),
        staging,
    };

    let mut original = private_file(&prepared.staging.path().join(&prepared.original))?;
    image
        .write_to(&mut original, output)
        .map_err(|_| ContentError::Unavailable)?;
    original.sync_all()?;

    let thumbnail = image.resize_to_fill(480, 270, image::imageops::FilterType::Triangle);
    let mut file = private_file(&prepared.staging.path().join(&prepared.thumbnail))?;
    thumbnail
        .write_to(&mut file, ImageFormat::WebP)
        .map_err(|_| ContentError::Unavailable)?;
    file.sync_all()?;

    Ok(prepared)
}

/// Delete the catalog row before calling this. Cleanup failure leaves a private orphan.
pub(crate) async fn cleanup(root: &FsPath, photo: &crate::database::GalleryImage) {
    for (directory, name) in [
        ("originals", &photo.original_file),
        ("thumbnails", &photo.thumbnail_file),
    ] {
        if let Some(path) = stored_path(root, directory, name) {
            if tokio::fs::remove_file(path).await.is_err() {
                tracing::warn!(
                    image_id = photo.id,
                    "Unpublished gallery file cleanup failed"
                );
            }
        }
    }
}

pub(crate) fn routes() -> Router<leptos::config::LeptosOptions> {
    Router::new()
        .route("/img/about_pics/{*name}", get(original))
        .route("/img/about_thumbnails/{*name}", get(thumbnail))
}

fn normalized_name(filename: &str, extension: &str) -> Result<String, ContentError> {
    // A filename is a label, never a client-selected path; preserve Unicode letters.
    if filename.len() > 180 || filename.contains(['/', '\\']) {
        return Err(ContentError::Invalid(
            "Choose a filename without paths, at most 180 bytes.",
        ));
    }

    let source = FsPath::new(filename);
    let allowed = source
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp"
            )
        });

    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("")
        .to_lowercase()
        .replace(' ', "_");
    if !allowed
        || stem.is_empty()
        || stem.len() > 180
        || !stem
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ContentError::Invalid(
            "Use letters, numbers, spaces, underscores or hyphens in the photo name.",
        ));
    }

    Ok(format!("{stem}.{extension}"))
}

fn stored_path(root: &FsPath, directory: &str, name: &str) -> Option<PathBuf> {
    (!name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\']))
        .then(|| root.join("gallery").join(directory).join(name))
}

async fn original(
    Path(name): Path<String>,
    Extension(database): Extension<DatabaseManager>,
    Extension(config): Extension<Arc<ServerConfig>>,
    request: Request<axum::body::Body>,
) -> Response {
    serve(&database, &config, &name, false, request).await
}

async fn thumbnail(
    Path(name): Path<String>,
    Extension(database): Extension<DatabaseManager>,
    Extension(config): Extension<Arc<ServerConfig>>,
    request: Request<axum::body::Body>,
) -> Response {
    let Some(name) = name.strip_suffix(".webp") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    serve(&database, &config, name, true, request).await
}

async fn serve(
    database: &DatabaseManager,
    config: &ServerConfig,
    name: &str,
    thumbnail: bool,
    request: Request<axum::body::Body>,
) -> Response {
    let result = async {
        let photo = database
            .gallery_image(name)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .ok_or(StatusCode::NOT_FOUND)?;
        let (directory, file) = if thumbnail {
            ("thumbnails", photo.thumbnail_file)
        } else {
            ("originals", photo.original_file)
        };
        let path = stored_path(&config.data_dir, directory, &file).ok_or(StatusCode::NOT_FOUND)?;
        // Startup validates storage; reject links here as well before serving a known file.
        let metadata = tokio::fs::symlink_metadata(&path)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
        if !metadata.is_file() {
            return Err(StatusCode::NOT_FOUND);
        }
        let response = ServeFile::new(path).oneshot(request).await;
        Ok(response.unwrap().into_response())
    }
    .await;
    let mut response = result.unwrap_or_else(IntoResponse::into_response);
    // Recheck publication on every request, including requests for previously deleted images.
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response
}

// ###################################
// ->     TESTS
// ###################################
#[cfg(test)]
mod tests {
    use super::*;

    fn photo(format: ImageFormat) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            48,
            27,
            image::Rgb([20, 40, 80]),
        ))
        .write_to(&mut bytes, format)
        .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn high_depth_rotation_preserves_normalized_pixels_and_alpha() {
        use image::ImageEncoder;
        let root = crate::test_support::directory();
        crate::storage::prepare(root.path(), &[]).unwrap();
        let source = DynamicImage::ImageRgba16(image::ImageBuffer::from_fn(2, 3, |x, y| {
            image::Rgba([(x * 12000) as u16, (y * 20000) as u16, 40000, 32768])
        }));
        let mut bytes = Vec::new();
        let mut encoder = image::codecs::png::PngEncoder::new(&mut bytes);
        encoder
            .set_exif_metadata(
                b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0".to_vec(),
            )
            .unwrap();
        encoder
            .write_image(source.as_bytes(), 2, 3, source.color().into())
            .unwrap();
        let prepared = prepare(root.path(), "oriented.png", bytes).unwrap();
        let actual = image::open(prepared.staging.path().join(&prepared.original))
            .unwrap()
            .into_rgba8();
        // Conversion is pointwise, so it must produce exactly the same pixels before or after rotation.
        assert_eq!(actual, source.rotate90().into_rgba8());
    }

    #[tokio::test]
    async fn jpeg_orientation_is_applied_before_metadata_is_removed() {
        let (root, database) = crate::test_support::database().await;
        // Minimal Exif IFD: orientation tag 0x0112, SHORT value 6 (rotate 90 degrees).
        let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let jpeg = photo(ImageFormat::Jpeg);
        let mut oriented = jpeg[..2].to_vec();
        oriented.extend_from_slice(b"\xff\xe1");
        oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        oriented.extend_from_slice(exif);
        oriented.extend_from_slice(&jpeg[2..]);
        let prepared = prepare(root.path(), "Portrait.JPG", oriented).unwrap();
        let output = prepared.staging.path().join(&prepared.original);
        assert_eq!(image::image_dimensions(&output).unwrap(), (27, 48));
        let mut decoder = ImageReader::open(output)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .into_decoder()
            .unwrap();
        assert_eq!(
            decoder.orientation().unwrap(),
            image::metadata::Orientation::NoTransforms
        );
        database.close().await;
    }

    #[tokio::test]
    async fn uploads_reencode_all_formats_and_publish_complete_private_pairs() {
        let (root, database) = crate::test_support::database().await;
        for (format, extension, output) in [
            (ImageFormat::Jpeg, "JPEG", "jpg"),
            (ImageFormat::Png, "PNG", "png"),
            (ImageFormat::WebP, "WEBP", "png"),
        ] {
            let mut bytes = photo(format);
            bytes.extend_from_slice(b"PRIVATE-UPLOAD-METADATA");
            let prepared = prepare(
                root.path(),
                &format!("My Photo {extension}.{extension}"),
                bytes,
            )
            .unwrap();
            assert_eq!(
                prepared.name,
                format!("my_photo_{}.{output}", extension.to_lowercase())
            );
            let id = database
                .add_gallery(
                    &prepared.name,
                    &prepared.original,
                    &prepared.thumbnail,
                    || prepared.publish(root.path()),
                )
                .await
                .unwrap();
            let original = root
                .path()
                .join("gallery/originals")
                .join(&prepared.original);
            let thumbnail = root
                .path()
                .join("gallery/thumbnails")
                .join(&prepared.thumbnail);
            assert_eq!(image::image_dimensions(&original).unwrap(), (48, 27));
            assert_eq!(image::image_dimensions(&thumbnail).unwrap(), (480, 270));
            assert!(!fs::read(&original)
                .unwrap()
                .windows(23)
                .any(|bytes| bytes == b"PRIVATE-UPLOAD-METADATA"));
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&original).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let removed = database.remove_gallery(id).await.unwrap().unwrap();
            cleanup(root.path(), &removed).await;
            assert!(!original.exists() && !thumbnail.exists());
        }
        database.close().await;
    }

    #[tokio::test]
    async fn invalid_uploads_and_publication_collisions_leave_no_catalog_or_overwritten_files() {
        let (root, database) = crate::test_support::database().await;
        for name in [
            "../photo.jpg",
            "C:\\photo.jpg",
            ".jpg",
            "photo.svg",
            "photo?.jpg",
            "photo\n.jpg",
        ] {
            assert!(
                prepare(root.path(), name, photo(ImageFormat::Jpeg)).is_err(),
                "{name}"
            );
        }
        for bytes in [b"<svg/>".to_vec(), vec![0; UPLOAD_BYTES + 1], vec![]] {
            assert!(prepare(root.path(), "photo.jpg", bytes).is_err());
        }
        let prepared = prepare(root.path(), "Photo.JPG", photo(ImageFormat::Jpeg)).unwrap();
        let occupied = root
            .path()
            .join("gallery/thumbnails")
            .join(&prepared.thumbnail);
        fs::write(&occupied, b"keep existing file").unwrap();
        assert!(database
            .add_gallery(
                &prepared.name,
                &prepared.original,
                &prepared.thumbnail,
                || prepared.publish(root.path())
            )
            .await
            .is_err());
        assert!(database.gallery().await.unwrap().is_empty());
        assert!(!root
            .path()
            .join("gallery/originals")
            .join(&prepared.original)
            .exists());
        assert_eq!(fs::read(occupied).unwrap(), b"keep existing file");
        drop(prepared);
        assert_eq!(
            fs::read_dir(root.path().join("staging")).unwrap().count(),
            0
        );
        database.close().await;
    }

    #[tokio::test]
    async fn decoded_pixel_limit_is_checked_before_full_decode() {
        let (root, database) = crate::test_support::database().await;
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageLuma8(image::GrayImage::new(6000, 2001))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        assert!(
            matches!(prepare(root.path(), "large.png", bytes.into_inner()), Err(ContentError::Invalid(message)) if message.contains("12 megapixels"))
        );
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageLuma8(image::GrayImage::new(6001, 1))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        assert!(prepare(root.path(), "wide.png", bytes.into_inner()).is_err());
        assert_eq!(
            fs::read_dir(root.path().join("staging")).unwrap().count(),
            0
        );
        database.close().await;
    }

    /// Run explicitly: fixture creation is excluded from each child process's measured peak RSS.
    #[test]
    #[ignore = "Linux memory-budget check; run just test-upload-memory"]
    fn maximum_upload_memory_budget() {
        use image::ImageEncoder;
        use std::process::Command;

        let fixtures = crate::test_support::directory();
        let width = 4000;
        let height = (PIXELS / u64::from(width)) as u32;
        // Raw TIFF orientation 6 rotates 90 degrees; JPEG's APP1 adds the Exif prefix.
        let exif = b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut peaks = Vec::new();
        for name in [
            "oriented.jpg",
            "wide.png",
            "rgba16.png",
            "oriented16.png",
            "alpha.webp",
        ] {
            // Construct only one input at a time; each fixture reaches the pixel or side boundary.
            let image = match name {
                "oriented.jpg" => DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                    width,
                    height,
                    image::Rgb([20, 40, 80]),
                )),
                "rgba16.png" | "oriented16.png" => {
                    DynamicImage::ImageRgba16(image::ImageBuffer::from_pixel(
                        width,
                        height,
                        image::Rgba([0x2000, 0x8000, 0xc000, 0x8000]),
                    ))
                }
                "wide.png" => DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                    6000,
                    (PIXELS / 6000) as u32,
                    image::Rgba([20, 40, 80, 128]),
                )),
                _ => DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                    width,
                    height,
                    image::Rgba([20, 40, 80, 128]),
                )),
            };
            let (source_width, source_height) = (image.width(), image.height());
            let path = fixtures.path().join(name);
            if name == "oriented16.png" {
                let mut encoder =
                    image::codecs::png::PngEncoder::new(fs::File::create(&path).unwrap());
                encoder.set_exif_metadata(exif.to_vec()).unwrap();
                encoder
                    .write_image(
                        image.as_bytes(),
                        source_width,
                        source_height,
                        image.color().into(),
                    )
                    .unwrap();
            } else {
                image.save(&path).unwrap();
            }
            drop(image);
            if name == "oriented.jpg" {
                let jpeg = fs::read(&path).unwrap();
                let mut bytes = jpeg[..2].to_vec();
                bytes.extend_from_slice(b"\xff\xe1");
                bytes.extend_from_slice(&((exif.len() + 8) as u16).to_be_bytes());
                bytes.extend_from_slice(b"Exif\0\0");
                bytes.extend_from_slice(exif);
                bytes.extend_from_slice(&jpeg[2..]);
                fs::write(&path, bytes).unwrap();
            }
            assert!(fs::metadata(&path).unwrap().len() <= UPLOAD_BYTES as u64);
            let rotated = name.starts_with("oriented");
            let report = fixtures.path().join("peak-rss.txt");
            // GNU time measures the fresh child, not Cargo, the test parent, or fixture buffers.
            let output = Command::new("/usr/bin/time")
                .args(["--format=%M", "--output"])
                .arg(&report)
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "content::gallery::tests::upload_memory_probe",
                    "--ignored",
                    "--nocapture",
                ])
                .env("MAJ_UPLOAD_PROBE_FILE", &path)
                .env(
                    "MAJ_UPLOAD_PROBE_WIDTH",
                    if rotated { source_height } else { source_width }.to_string(),
                )
                .env(
                    "MAJ_UPLOAD_PROBE_HEIGHT",
                    if rotated { source_width } else { source_height }.to_string(),
                )
                .output()
                .expect("Install GNU time to run the upload memory check");
            assert!(
                output.status.success(),
                "{name}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let kib: usize = fs::read_to_string(&report).unwrap().trim().parse().unwrap();
            eprintln!(
                "{name}: {kib} KiB peak RSS ({:.1} MiB)",
                kib as f64 / 1024.0
            );
            peaks.push((name, kib));
        }
        // Reserve 64 MiB of a 256 MiB Machine for HTTP/SQLite, input buffering, and other work.
        assert!(
            peaks.iter().all(|(_, kib)| *kib <= 192 * 1024),
            "Upload pipeline exceeds its 192 MiB allowance: {peaks:?}"
        );
    }

    #[test]
    #[ignore = "Child of maximum_upload_memory_budget; requires its private fixture environment"]
    fn upload_memory_probe() {
        let input = PathBuf::from(std::env::var_os("MAJ_UPLOAD_PROBE_FILE").unwrap());
        let width: u32 = std::env::var("MAJ_UPLOAD_PROBE_WIDTH")
            .unwrap()
            .parse()
            .unwrap();
        let height: u32 = std::env::var("MAJ_UPLOAD_PROBE_HEIGHT")
            .unwrap()
            .parse()
            .unwrap();
        let root = crate::test_support::directory();
        crate::storage::prepare(root.path(), &[]).unwrap();
        let prepared = prepare(
            root.path(),
            input.file_name().unwrap().to_str().unwrap(),
            fs::read(&input).unwrap(),
        )
        .unwrap();
        prepared.publish(root.path()).unwrap();
        // Inspect headers without decoding another full-size copy into the measured process.
        let mut decoder = ImageReader::open(
            root.path()
                .join("gallery/originals")
                .join(&prepared.original),
        )
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
        assert_eq!(decoder.dimensions(), (width, height));
        assert!(matches!(
            decoder.color_type(),
            image::ColorType::Rgb8 | image::ColorType::Rgba8
        ));
        assert_eq!(
            decoder.orientation().unwrap(),
            image::metadata::Orientation::NoTransforms
        );
        assert_eq!(
            image::image_dimensions(
                root.path()
                    .join("gallery/thumbnails")
                    .join(&prepared.thumbnail)
            )
            .unwrap(),
            (480, 270)
        );
    }
}
