//! TOOLCHAIN-1 (spec §3.2): the toolchain pin is a law, not a convention.
//!
//! Every content-addressed id in this project was, until the relational
//! artifact cutover, a `DefaultHasher` output -- an algorithm Rust's own
//! docs say "may change between releases." A `rustup update` could move
//! every id with zero data change. These tests make the pin load-bearing:
//! the file must exist, must name the exact version, and the compiler
//! that BUILT THIS TEST must be that version.

use std::path::{Path, PathBuf};
use std::process::Command;

const PINNED: &str = "1.97.1";

fn repo_root() -> PathBuf {
    // graph-types/ sits at the repo root; the pin sits beside it.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn channel_from(toml: &str) -> Option<String> {
    toml.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("channel"))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .map(|v| v.trim().trim_matches('"').to_string())
}

#[test]
fn rust_toolchain_toml_pins_the_exact_version() {
    let path = repo_root().join("rust-toolchain.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist at the repo root: {e}", path.display()));
    let channel = channel_from(&text).expect("rust-toolchain.toml must declare `channel = \"...\"`");
    assert_eq!(
        channel, PINNED,
        "channel must be the exact version {PINNED:?}, never a moving name like \"stable\""
    );
}

#[test]
fn the_compiler_that_built_this_test_is_the_pinned_version() {
    // `CARGO` is the cargo binary that performed this build; its sibling
    // `rustc` is the compiler that built this test binary.
    let rustc = Path::new(env!("CARGO")).with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&rustc)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", rustc.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(
        v.starts_with(&format!("rustc {PINNED} ")),
        "this test was built by {v:?}; the pin requires rustc {PINNED}"
    );
}

#[test]
fn rustup_resolves_the_pin_from_the_repo_root() {
    // The proxy in ~/.cargo/bin resolves the toolchain from the working
    // directory; from the repo root it must land on the pin.
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).expect("a home dir");
    let proxy = Path::new(&home)
        .join(".cargo")
        .join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&proxy)
        .arg("--version")
        .current_dir(repo_root())
        .output()
        .unwrap_or_else(|e| panic!("could not run the rustup proxy {}: {e}", proxy.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(
        v.starts_with(&format!("rustc {PINNED} ")),
        "rustc resolved from the repo root is {v:?}; expected the pin {PINNED}"
    );
}

#[test]
fn channel_parser_reads_the_toml_shape_we_write() {
    assert_eq!(channel_from("[toolchain]\nchannel = \"1.97.1\"\n").as_deref(), Some("1.97.1"));
    assert_eq!(channel_from("[toolchain]\n  channel=\"stable\"\n").as_deref(), Some("stable"));
    assert_eq!(channel_from("[toolchain]\nprofile = \"minimal\"\n"), None);
}
