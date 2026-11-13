//! Prepare private storage without opening SQLite or publishing uploaded files.
//!
//! Startup separates the administration data directory from public asset trees,
//! creates missing layout entries, and validates existing entries before serving
//! requests. Existing data and permissions are preserved; unsafe entries cause
//! startup to fail instead of being repaired implicitly.
//!
//! These checks do not lock filesystem paths. Parent directories and public assets
//! must remain under trusted control so entries cannot be replaced after validation.

use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(not(unix))]
/// Refuses startup without changing storage when Unix permission checks are unavailable.
pub fn prepare(_data_dir: &Path, _public_roots: &[&Path]) -> io::Result<PathBuf> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Private storage requires Unix filesystem permissions",
    ))
}

#[cfg(unix)]
/// Creates the private layout and returns its canonical root, preserving existing files.
///
/// `data_dir` must be absolute, contain no `..` components, and neither contain nor
/// sit inside a public root. Public roots may be relative to the working directory.
/// Both configured and resolved paths are compared, including existing root aliases.
/// Existing public trees are scanned for symlinks before any storage is created.
/// Public roots must remain trusted and immutable after this startup validation.
///
/// Creates the root, `gallery`, `gallery/originals`, `gallery/thumbnails`, and
/// `staging` with mode `0700`, plus an empty `admin.sqlite3` with mode `0600`.
/// This reserves the database file but does not initialize SQLite or its schema.
/// The caller should also set umask `0077` before starting threads so files later
/// created by libraries inherit private defaults.
///
/// Managed entries must belong to the effective user. Child directory symlinks,
/// database symlinks, and database hard links are rejected. Existing SQLite
/// companion files receive the same ownership, mode, and link checks as the
/// database. Gallery and staging directories must share the root's filesystem.
///
/// Invalid layouts or filesystem errors abort preparation. Existing entries are
/// never removed, truncated, or given new permissions. Failure can leave newly
/// created private entries in place; preparation does not roll back those entries.
pub fn prepare(data_dir: &Path, public_roots: &[&Path]) -> io::Result<PathBuf> {
    use std::{
        fs,
        os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    };

    if !data_dir.is_absolute()
        || data_dir
            .components()
            .any(|part| part == std::path::Component::ParentDir)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "MAJ_DATA_DIR must be absolute without parent traversal",
        ));
    }
    let resolved = resolve_missing(data_dir)?;
    // Ensure data_dir and public_roots don't overlap & there is no symlinks in canonicalized public_roots
    for root in public_roots {
        let root = std::path::absolute(root)?;
        let canonical_root = resolve_missing(&root)?;
        // Check both spellings so aliases cannot hide overlap with a public directory.
        for data in [data_dir, resolved.as_path()] {
            for public in [&root, &canonical_root] {
                if data.starts_with(public) || public.starts_with(data) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "MAJ_DATA_DIR and public asset directories must not overlap",
                    ));
                }
            }
        }
        if canonical_root.exists() {
            reject_public_symlinks(&canonical_root)?;
        }
    }

    // Existing paths are validated, never chmodded or truncated implicitly.
    for relative in [
        "",
        "gallery",
        "gallery/originals",
        "gallery/thumbnails",
        "staging",
    ] {
        let path = resolved.join(relative);
        match fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(&path)?;
            }
            Err(error) => return Err(error),
        }
        check_private(&path, true)?;
        // Cross-device layouts would prevent moving staged uploads into the gallery by rename.
        if fs::metadata(&path)?.dev() != fs::metadata(&resolved)?.dev() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Gallery and staging must be on the same filesystem as MAJ_DATA_DIR",
            ));
        }
    }

    let database = resolved.join("admin.sqlite3");
    // Reserve a new database file without opening or truncating an existing one.
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&database)
    {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    check_private(&database, false)?;
    // SQLite owns these companions; inspect existing files without creating or modifying them.
    for suffix in ["-wal", "-shm", "-journal"] {
        let path = resolved.join(format!("admin.sqlite3{suffix}"));
        match fs::symlink_metadata(&path) {
            Ok(_) => check_private(&path, false)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(resolved)
}

#[cfg(unix)]
/// Resolves existing path components so roots can be compared before storage is created.
///
/// Unlike `canonicalize`, this also accepts paths whose final components do not
/// exist. On `NotFound`, it saves the final name and recursively resolves the
/// parent. Once an existing ancestor is canonicalized (including its symlinks),
/// the saved names are appended as the recursion unwinds. The missing suffix
/// remains unresolved; this function creates no files or directories.
///
/// For example, if `/srv/alias` points to `/mnt/data` and `maj/gallery` does not
/// exist there, `/srv/alias/maj/gallery` resolves to `/mnt/data/maj/gallery`.
///
/// Callers supply absolute paths. Errors other than `NotFound` propagate rather
/// than being treated as missing directories. If no parent exists, the original
/// error is returned; a missing final name produces `InvalidInput`.
/// Path resolution alone does not validate ownership, permissions, or file types.
fn resolve_missing(path: &Path) -> io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // Canonicalize the nearest existing ancestor, then rebuild the missing suffix without creating it.
            let parent = path.parent().ok_or(error)?;
            let name = path.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid storage path")
            })?;
            Ok(resolve_missing(parent)?.join(name))
        }
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
/// Checks one managed entry's type, owner, and Unix permissions without changing it.
///
/// `directory` selects a directory with mode `0700` or a regular file with mode
/// `0600`. Ownership must match the process's effective user. Metadata is read
/// without following the final component, so a symlink cannot pass as its target.
/// Files must have exactly one hard link to prevent another name referring to the
/// same private contents. Only this entry is inspected; callers validate parents.
///
/// Unexpected ownership, permissions, types, or link counts produce
/// `PermissionDenied`; errors while reading metadata propagate unchanged.
fn check_private(path: &Path, directory: bool) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path)?;
    // SAFETY: geteuid has no arguments or preconditions.
    let owner = unsafe { libc::geteuid() };
    let mode = if directory { 0o700 } else { 0o600 };
    if metadata.uid() != owner
        || metadata.mode() & 0o777 != mode
        || (directory && !metadata.is_dir())
        || (!directory && (!metadata.is_file() || metadata.nlink() != 1))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} must be an owned, private {} with mode {mode:o}; links are not allowed",
                path.display(),
                if directory { "directory" } else { "file" }
            ),
        ));
    }
    Ok(())
}

#[cfg(unix)]
/// Recursively rejects symlink entries below an existing, already resolved public root.
///
/// Entry types are inspected without following links, including dangling links
/// whose targets could become private storage later. Only actual directories are
/// traversed. A link produces `InvalidInput`; directory enumeration errors
/// propagate so an unreadable subtree cannot silently escape validation.
fn reject_public_symlinks(root: &Path) -> io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Public assets must not contain symbolic links: {}",
                    entry.path().display()
                ),
            ));
        }
        if kind.is_dir() {
            reject_public_symlinks(&entry.path())?;
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{symlink, DirBuilderExt, OpenOptionsExt, PermissionsExt},
        sync::atomic::{AtomicU64, Ordering},
    };

    /// Gives each test separate public/private trees and removes them on drop.
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "maj-storage-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            fs::create_dir(root.join("site")).unwrap();
            Self(root)
        }
        fn prepare(&self) -> io::Result<PathBuf> {
            prepare(
                &self.0.join("data"),
                &[&self.0.join("site"), &self.0.join("public")],
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Cleanup must not cause a second panic when a test is already unwinding.
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_private_layout_and_preserves_existing_data() {
        let fixture = Fixture::new();
        let data = fixture.prepare().unwrap();
        for name in [
            "",
            "gallery",
            "gallery/originals",
            "gallery/thumbnails",
            "staging",
        ] {
            assert_eq!(
                fs::metadata(data.join(name)).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let database = data.join("admin.sqlite3");
        assert_eq!(
            fs::metadata(&database).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::metadata(&database).unwrap().len(), 0);
        fs::write(&database, b"existing database bytes").unwrap();
        fs::write(
            data.join("gallery/originals/existing.jpg"),
            b"existing photo",
        )
        .unwrap();
        fixture.prepare().unwrap();
        assert_eq!(fs::read(database).unwrap(), b"existing database bytes");
        assert_eq!(
            fs::read(data.join("gallery/originals/existing.jpg")).unwrap(),
            b"existing photo"
        );
    }

    #[test]
    fn rejects_overlapping_and_aliased_public_paths_before_creating_files() {
        let fixture = Fixture::new();
        let site = fixture.0.join("site");
        for data in [
            site.clone(),
            site.join("private"),
            fixture.0.clone(),
            fixture.0.join("public/private"),
        ] {
            assert!(prepare(&data, &[&site, &fixture.0.join("public")]).is_err());
        }
        symlink(&site, fixture.0.join("alias")).unwrap();
        assert!(prepare(&fixture.0.join("alias/private"), &[&site]).is_err());
        assert!(!site.join("private").exists());
        assert!(!fixture.0.join("public").exists());
        assert!(prepare(&fixture.0.join("data/../site/private"), &[&site]).is_err());
    }

    #[test]
    fn rejects_public_symlinks_including_dangling_links_to_future_storage() {
        let fixture = Fixture::new();
        symlink(fixture.0.join("data"), fixture.0.join("site/private")).unwrap();
        assert!(fixture.prepare().is_err());
        assert!(!fixture.0.join("data").exists());
    }

    #[test]
    fn rejects_insecure_existing_permissions_without_changing_them() {
        let fixture = Fixture::new();
        let data = fixture.prepare().unwrap();
        for (path, public_mode, private_mode) in [
            (data.clone(), 0o755, 0o700),
            (data.join("staging"), 0o755, 0o700),
            (data.join("admin.sqlite3"), 0o644, 0o600),
        ] {
            fs::set_permissions(&path, fs::Permissions::from_mode(public_mode)).unwrap();
            assert!(fixture.prepare().is_err());
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                public_mode
            );
            fs::set_permissions(path, fs::Permissions::from_mode(private_mode)).unwrap();
        }
    }

    #[test]
    fn rejects_private_symlinks_hardlinks_and_unsafe_companion_files() {
        let fixture = Fixture::new();
        let data = fixture.prepare().unwrap();
        fs::remove_dir(data.join("staging")).unwrap();
        symlink(fixture.0.join("site"), data.join("staging")).unwrap();
        assert!(fixture.prepare().is_err());
        fs::remove_file(data.join("staging")).unwrap();
        fixture.prepare().unwrap();
        fs::hard_link(data.join("admin.sqlite3"), fixture.0.join("database-link")).unwrap();
        assert!(fixture.prepare().is_err());
        fs::remove_file(fixture.0.join("database-link")).unwrap();
        let companion = data.join("admin.sqlite3-wal");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&companion)
            .unwrap();
        fixture.prepare().unwrap();
        fs::set_permissions(&companion, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(fixture.prepare().is_err());
        fs::remove_file(companion).unwrap();
        fs::remove_file(data.join("admin.sqlite3")).unwrap();
        symlink(fixture.0.join("site/secret"), data.join("admin.sqlite3")).unwrap();
        assert!(fixture.prepare().is_err());
        assert!(!fixture.0.join("site/secret").exists());
    }
}
