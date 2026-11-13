//! Re-embed migrations when files are added or changed, including in release builds.
//! This follows SQLx's recommended approach for stable Rust
//! [SQLx migrate](https://docs.rs/sqlx/0.9.0/sqlx/macro.migrate.html)

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
