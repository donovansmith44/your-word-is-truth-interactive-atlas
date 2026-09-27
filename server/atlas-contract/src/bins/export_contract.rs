//! Writes every published contract document (spec D8). `--check` writes
//! nothing and exits non-zero when a committed document has drifted, which
//! is what a gate runs.

fn main() {
    let check = std::env::args().any(|a| a == "--check");
    let mut stale = Vec::new();
    for (path, contents) in atlas_contract::document::generated_files() {
        let current = std::fs::read_to_string(&path).ok();
        if current.as_deref() == Some(contents.as_str()) {
            continue;
        }
        if check {
            stale.push(path.display().to_string());
        } else {
            std::fs::write(&path, contents).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
            println!("wrote {}", path.display());
        }
    }
    if check && !stale.is_empty() {
        eprintln!("stale generated documents: {stale:?}");
        std::process::exit(1);
    }
}
