//! `decompress_verified` under contention: many callers materialising one section into one cache
//! path from cold, at once, and what the cache holds afterwards.

use std::path::{Path, PathBuf};
use std::sync::Barrier;

use atlas_graph::sqlite::blob::{compress_file, decompress_verified, sha256_hex_of_file};

const CONTENDERS: usize = 8;
const ATTEMPTS: usize = 5;
const SECTION_BYTES: usize = 4 << 20;
const SECTION_FILE: &str = "section.sqlite";

struct Fixture {
    blob: PathBuf,
    transport_sha256: String,
    section_sha256: String,
    cache: PathBuf,
    dst: PathBuf,
}

fn fixture(name: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("blob-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join(SECTION_FILE);
    std::fs::write(&src, section_bytes()).unwrap();
    let blob = dir.join("section.sqlite.zst");
    let (transport_sha256, _) = compress_file(&src, &blob).unwrap();
    let cache = dir.join("cache");
    Fixture {
        blob,
        transport_sha256,
        section_sha256: sha256_hex_of_file(&src).unwrap(),
        dst: cache.join(SECTION_FILE),
        cache,
    }
}

fn section_bytes() -> Vec<u8> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..SECTION_BYTES)
        .map(|_| {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (state >> 56) as u8
        })
        .collect()
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn every_contender_materialises_the_whole_section_and_exactly_one_file_remains() {
    // Arrange
    let f = fixture("race");
    let expected_outcomes = vec![Ok(SECTION_BYTES as u64); CONTENDERS];
    let expected_files = vec![SECTION_FILE.to_string()];
    for attempt in 0..ATTEMPTS {
        let _ = std::fs::remove_dir_all(&f.cache);
        let start = Barrier::new(CONTENDERS);
        // Act
        let outcomes: Vec<Result<u64, String>> = std::thread::scope(|s| {
            let contenders: Vec<_> = (0..CONTENDERS)
                .map(|_| {
                    s.spawn(|| {
                        start.wait();
                        decompress_verified(&f.blob, &f.transport_sha256, &f.dst).map_err(|e| e.0)
                    })
                })
                .collect();
            contenders.into_iter().map(|c| c.join().unwrap()).collect()
        });
        // Assert
        assert_eq!(outcomes, expected_outcomes, "attempt {attempt}: every call sees the whole section");
        assert_eq!(files_in(&f.cache), expected_files, "attempt {attempt}: one section, no tmp");
        assert_eq!(sha256_hex_of_file(&f.dst).unwrap(), f.section_sha256, "attempt {attempt}: the section verifies");
    }
}

#[cfg(windows)]
mod refused_rename {
    use super::*;
    use std::fs::File;
    use std::os::windows::fs::OpenOptionsExt;

    /// SQLite's share mode for an open section: readers and writers may join, nobody may delete or
    /// replace the file underneath it.
    const SQLITE_SHARE_MODE: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    const REFUSED: &str = "Access is denied. (os error 5)";

    fn hold_open(path: &Path) -> File {
        std::fs::OpenOptions::new().read(true).share_mode(SQLITE_SHARE_MODE).open(path).unwrap()
    }

    #[test]
    fn a_loser_whose_rename_a_held_open_winner_refuses_returns_the_winners_size() {
        // Arrange
        let f = fixture("held-same");
        decompress_verified(&f.blob, &f.transport_sha256, &f.dst).unwrap();
        let _winner = hold_open(&f.dst);
        // Act
        let outcome = decompress_verified(&f.blob, &f.transport_sha256, &f.dst).map_err(|e| e.0);
        // Assert
        assert_eq!(outcome, Ok(SECTION_BYTES as u64));
        assert_eq!(files_in(&f.cache), vec![SECTION_FILE.to_string()]);
    }

    #[test]
    fn a_held_open_file_with_other_bytes_is_named_and_not_accepted() {
        // Arrange
        let f = fixture("held-other");
        std::fs::create_dir_all(&f.cache).unwrap();
        std::fs::write(&f.dst, b"not the section").unwrap();
        let _other = hold_open(&f.dst);
        // Act
        let outcome = decompress_verified(&f.blob, &f.transport_sha256, &f.dst).map_err(|e| e.0);
        // Assert
        assert_eq!(outcome, Err(format!("cannot replace {} ({REFUSED}): it holds a different file", f.dst.display())));
        assert_eq!(files_in(&f.cache), vec![SECTION_FILE.to_string()]);
    }

    #[test]
    fn a_destination_that_cannot_be_read_is_named_with_both_refusals() {
        // Arrange
        let f = fixture("dst-dir");
        std::fs::create_dir_all(&f.dst).unwrap();
        // Act
        let outcome = decompress_verified(&f.blob, &f.transport_sha256, &f.dst).map_err(|e| e.0);
        // Assert
        assert_eq!(outcome, Err(format!("cannot replace {} ({REFUSED}): io: {REFUSED}", f.dst.display())));
        assert_eq!(files_in(&f.cache), vec![SECTION_FILE.to_string()]);
    }
}
