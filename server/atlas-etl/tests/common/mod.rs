#![allow(dead_code)]
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

pub fn raw_dir() -> PathBuf {
    data_dir().join("raw")
}

pub fn curated_dir() -> PathBuf {
    data_dir().join("curated")
}
