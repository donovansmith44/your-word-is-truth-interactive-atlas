use std::path::{Path, PathBuf};

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn strip_header_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    for (i, b) in bytes.iter().enumerate() {
        match quote {
            Some(q) if *b == q => quote = None,
            Some(_) => {}
            None if *b == b'\'' || *b == b'"' => quote = Some(*b),
            None if *b == b'#' => return line[..i].trim_end(),
            None => {}
        }
    }
    line
}

fn table_entries(toml: &str, table: &str) -> Option<Vec<String>> {
    let exact = format!("[{table}]");
    let dotted = format!("[{table}.");
    let suffix = format!(".{table}]");
    let mut found = false;
    let mut inside = false;
    let mut out = Vec::new();
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            let t = strip_header_comment(t);
            let is_sub =
                (t.starts_with(&dotted) && t.ends_with(']')) || t.ends_with(&suffix);
            inside = t == exact || is_sub;
            if inside {
                found = true;
                if is_sub {
                    out.push(t.to_string());
                }
            }
            continue;
        }
        if inside && !t.is_empty() && !t.starts_with('#') {
            out.push(t.to_string());
        }
    }
    if found {
        Some(out)
    } else {
        None
    }
}

fn expected_dependency_entries() -> Vec<String> {
    vec![
        "serde = { version = \"1\", optional = true, default-features = false }".to_string(),
        "utoipa = { version = \"6.0.0\", optional = true }".to_string(),
    ]
}

#[test]
fn every_dependency_is_optional_so_the_default_build_takes_none() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "dependencies").expect("[dependencies] must exist");
    // Assert
    assert_eq!(
        entries,
        expected_dependency_entries(),
        "[dependencies] must hold exactly serde and utoipa, both `optional = true` -- \
         the default build takes neither"
    );
}

#[test]
fn dev_dependencies_hold_only_serde_json_and_proptest_for_tests() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "dev-dependencies").expect("[dev-dependencies] must exist");
    // Assert
    assert_eq!(
        entries,
        vec!["serde_json = \"1\"".to_string(), "proptest = \"1\"".to_string()],
        "[dev-dependencies] must hold exactly serde_json and proptest -- test-only, never reaches a consumer"
    );
}

#[test]
fn build_dependencies_table_holds_no_entry() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "build-dependencies");
    // Assert
    match entries {
        None => {}
        Some(entries) => assert!(
            entries.is_empty(),
            "[build-dependencies] must be empty; found {entries:?}"
        ),
    }
}

#[test]
fn features_hold_exactly_canon_ids_serde_and_openapi_with_no_default() {
    // Arrange
    let toml = std::fs::read_to_string(manifest_path()).expect("graph-types/Cargo.toml");
    // Act
    let entries = table_entries(&toml, "features").expect("[features] must exist");
    // Assert
    assert_eq!(
        entries,
        vec![
            "canon-ids = []".to_string(),
            "serde = [\"dep:serde\"]".to_string(),
            "openapi = [\"dep:utoipa\"]".to_string(),
        ],
        "[features] must declare exactly canon-ids, serde and openapi, and nothing else"
    );
    assert!(
        !entries.iter().any(|e| e.starts_with("default")),
        "[features] must never declare a `default` feature -- that is what would turn \
         an optional dependency into a real one"
    );
}

#[test]
fn a_table_is_read_as_its_own_entries_and_a_table_that_is_absent_as_nothing() {
    // Arrange
    let manifest = "[package]
name = \"x\"

[dependencies]
serde = \"1\"

[features]
# a comment
canon-ids = []
";
    // Act
    let read = [table_entries(manifest, "dependencies"), table_entries(manifest, "features"), table_entries(manifest, "dev-dependencies")];
    // Assert
    assert_eq!(read, [Some(vec!["serde = \"1\"".to_string()]), Some(vec!["canon-ids = []".to_string()]), None]);
}

#[test]
fn a_dotted_sub_table_cannot_hide_a_dependency() {
    // Arrange
    let dotted = [
        ("dependencies", "[dependencies.serde]
version = \"1\"
"),
        ("dev-dependencies", "[dev-dependencies.proptest]
version = \"1\"
"),
        ("build-dependencies", "[build-dependencies.cc]
version = \"1\"
"),
    ];
    // Act
    let read: Vec<Option<Vec<String>>> = dotted.iter().map(|(table, manifest)| table_entries(manifest, table)).collect();
    // Assert
    assert_eq!(
        read,
        vec![
            Some(vec!["[dependencies.serde]".to_string(), "version = \"1\"".to_string()]),
            Some(vec!["[dev-dependencies.proptest]".to_string(), "version = \"1\"".to_string()]),
            Some(vec!["[build-dependencies.cc]".to_string(), "version = \"1\"".to_string()]),
        ]
    );
}

#[test]
fn a_target_scoped_table_cannot_hide_a_dependency() {
    // Arrange
    let scoped = [
        ("dependencies", "[target.'cfg(windows)'.dependencies]
winapi = \"0.3\"
"),
        ("dependencies", "[target.cfg(unix).dependencies]
libc = \"0.2\"
"),
        ("dev-dependencies", "[target.'cfg(target_os = \"linux\")'.dev-dependencies]
proptest = \"1\"
"),
        ("build-dependencies", "[target.'cfg(windows)'.build-dependencies]
cc = \"1\"
"),
        ("dependencies", "[target.'cfg(windows)'.dev-dependencies]
proptest = \"1\"
"),
    ];
    // Act
    let read: Vec<Option<Vec<String>>> = scoped.iter().map(|(table, manifest)| table_entries(manifest, table)).collect();
    // Assert
    assert_eq!(
        read,
        vec![
            Some(vec!["[target.'cfg(windows)'.dependencies]".to_string(), "winapi = \"0.3\"".to_string()]),
            Some(vec!["[target.cfg(unix).dependencies]".to_string(), "libc = \"0.2\"".to_string()]),
            Some(vec!["[target.'cfg(target_os = \"linux\")'.dev-dependencies]".to_string(), "proptest = \"1\"".to_string()]),
            Some(vec!["[target.'cfg(windows)'.build-dependencies]".to_string(), "cc = \"1\"".to_string()]),
            None,
        ]
    );
}

#[test]
fn a_trailing_comment_on_a_header_does_not_hide_the_table() {
    // Arrange
    let commented = [
        ("dependencies", "[dependencies]  # nothing lives here
serde = \"1\"
"),
        ("features", "[features] # the one switch
canon-ids = []
"),
        ("dependencies", "[dependencies.serde] # pinned
version = \"1\"
"),
        ("dependencies", "[target.'cfg(feature = \"a#b\")'.dependencies]
libc = \"0.2\"
"),
    ];
    // Act
    let read: Vec<Option<Vec<String>>> = commented.iter().map(|(table, manifest)| table_entries(manifest, table)).collect();
    // Assert
    assert_eq!(
        read,
        vec![
            Some(vec!["serde = \"1\"".to_string()]),
            Some(vec!["canon-ids = []".to_string()]),
            Some(vec!["[dependencies.serde]".to_string(), "version = \"1\"".to_string()]),
            Some(vec!["[target.'cfg(feature = \"a#b\")'.dependencies]".to_string(), "libc = \"0.2\"".to_string()]),
        ]
    );
}
