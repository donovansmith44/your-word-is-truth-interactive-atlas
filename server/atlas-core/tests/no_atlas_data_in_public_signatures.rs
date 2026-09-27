//! No public function in `atlas-core` may take an `AtlasData` parameter: its
//! `events`/`places`/`narratives` are empty on every serving path, so the way to pass
//! is to delete the reader or move it onto `SceneSource`, never to extend `ALLOWED`.

use std::path::{Path, PathBuf};

/// Set equality, not containment: an entry that is deleted must also leave this list,
/// so the list cannot rot into a blanket exemption. Both entries are oracles, never
/// reached by a serving response.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "chronology.rs",
        "era_boundary_violations",
        "ETL/report-time check over the freshly COMPILED data (atlas-etl fills the vecs); never reached by a serving response",
    ),
    (
        "narrative.rs",
        "global_timeline_position",
        "test-only LOCKSTEP ORACLE for atlas-graph/tests/timeline_equivalence.rs (see narrative.rs's own header); the live timeline half is served from GraphService::temporal_neighbors instead",
    ),
];

/// The parameter list is taken by counting parentheses, which handles generics and
/// closure parameters without a grammar. Return types are deliberately not scanned:
/// a constructor of an `AtlasData` is not a reader of a serving one.
fn atlas_data_taking_public_fns(src: &str) -> Vec<(String, String)> {
    const HEADS: [&str; 2] = ["pub fn ", "pub(crate) fn "];
    let mut out = Vec::new();
    for head in HEADS {
        let mut from = 0usize;
        while let Some(rel) = src[from..].find(head) {
            let at = from + rel;
            from = at + head.len();
            // `pub` must start the item, not be the tail of an identifier.
            if at > 0 && src[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let rest = &src[at + head.len()..];
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.is_empty() {
                continue;
            }
            let Some(open) = rest.find('(') else { continue };
            let bytes = rest.as_bytes();
            let mut depth = 0i32;
            let mut close = None;
            for (i, b) in bytes.iter().enumerate().skip(open) {
                match b {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(close) = close else { continue };
            let params = &rest[open + 1..close];
            if params.contains("AtlasData") {
                out.push((name, params.split_whitespace().collect::<Vec<_>>().join(" ")));
            }
        }
    }
    out
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_public_atlas_data_taking_function_in_atlas_core_is_on_the_closed_list() {
    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rs_files(&src_root, &mut files);
    files.sort();
    assert!(
        files.len() >= 10,
        "the scan found only {} sources under {} -- it has lost its target and would pass vacuously",
        files.len(),
        src_root.display()
    );

    let mut found: Vec<(String, String, String)> = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
        let stem = file.file_name().unwrap().to_string_lossy().into_owned();
        for (name, params) in atlas_data_taking_public_fns(&text) {
            found.push((stem.clone(), name, params));
        }
    }
    found.sort();
    for (file, name, params) in &found {
        println!("atlas-core public fn taking AtlasData: {file}::{name}({params})");
    }

    let found_keys: Vec<(String, String)> = found.iter().map(|(f, n, _)| (f.clone(), n.clone())).collect();
    let mut allowed_keys: Vec<(String, String)> =
        ALLOWED.iter().map(|(f, n, _)| ((*f).to_string(), (*n).to_string())).collect();
    allowed_keys.sort();

    let unexpected: Vec<&(String, String)> = found_keys.iter().filter(|k| !allowed_keys.contains(k)).collect();
    assert!(
        unexpected.is_empty(),
        "NEW public fn(s) in atlas-core take an AtlasData: {unexpected:?}\n\
         AtlasData.events/.places/.narratives are EMPTY on every serving path -- a function that can \
         be handed one can be handed an empty one, which is how OVERLAY-1-HOTFIX-1's regression shipped.\n\
         Fix it by deleting the reader or by taking `&dyn SceneSource` instead (the move \
         `narrative::adjacent_event` made). Do NOT add it to ALLOWED unless it is a genuine \
         non-serving oracle, and then say why in the entry itself."
    );

    let stale: Vec<&(String, String)> = allowed_keys.iter().filter(|k| !found_keys.contains(k)).collect();
    assert!(
        stale.is_empty(),
        "ALLOWED names {stale:?}, which no longer exist -- a closed list must be exact, or it rots \
         into a blanket exemption. Delete the entry."
    );

    assert_eq!(found_keys.len(), ALLOWED.len(), "set equality: {found_keys:?} vs {allowed_keys:?}");
    for (_, _, why) in ALLOWED {
        assert!(why.len() > 40, "every ALLOWED entry must carry a real reason, not a shrug: {why:?}");
    }
}

#[test]
fn the_signature_scan_finds_a_planted_reader_and_ignores_the_correct_shape() {
    let planted = r#"
        pub fn would_be_a_new_reader(d: &AtlasData, event_id: &str) -> Option<String> { None }
        pub(crate) fn by_arc(d: &std::sync::Arc<AtlasData>) -> usize { 0 }
        pub fn by_value(d: AtlasData) -> usize { 0 }
        pub fn generic_reader<T: Clone>(d: &AtlasData, t: T) -> T { t }
        pub fn with_a_closure_param(d: &AtlasData, f: impl Fn(&str) -> bool) -> bool { false }
    "#;
    let mut names: Vec<String> = atlas_data_taking_public_fns(planted).into_iter().map(|(n, _)| n).collect();
    names.sort();
    assert_eq!(
        names,
        vec!["by_arc", "by_value", "generic_reader", "with_a_closure_param", "would_be_a_new_reader"],
        "the scan missed a planted AtlasData-taking signature -- it would pass silently forever"
    );

    let correct = r#"
        pub fn adjacent_event(src: &dyn SceneSource, event_id: &str) -> Option<NarrativeAdjacentEvent> { None }
        pub fn compose_time_scene(d: &dyn SceneSource, w: TimeRange) -> Scene { todo!() }
        pub fn demo_fixture() -> AtlasData { todo!() }
        fn private_reader(d: &AtlasData) -> usize { 0 }
        pub fn mentions_atlas_data_only_in_its_return(x: u32) -> AtlasData { todo!() }
    "#;
    let found = atlas_data_taking_public_fns(correct);
    assert!(
        found.is_empty(),
        "the scan fired on a CORRECT shape (a SceneSource parameter, a private fn, or an AtlasData RETURN): {found:?}"
    );
}
