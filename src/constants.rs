use std::path::{Path, PathBuf};

pub fn base_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME env var not set");
    Path::new(&home).join(".wp-blockmarkup-mcp")
}

pub fn db_path() -> PathBuf {
    base_dir().join("blocks.db")
}

pub fn cache_dir() -> PathBuf {
    base_dir().join("cache")
}
