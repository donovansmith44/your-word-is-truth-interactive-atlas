use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use atlas_graph_types::edge::{RelationId, SymRelationId};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Map, Value};
use tower::ServiceExt;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn app() -> axum::Router {
    static ROUTER: OnceLock<axum::Router> = OnceLock::new();
    ROUTER
        .get_or_init(|| {
            atlas_contract::load::load_from_data_dir(&repo_root().join("data/compiled"))
                .expect("data/compiled must load exactly as the server loads it")
                .into_router(None)
        })
        .clone()
}

fn graph_vocabulary() -> Value {
    serde_json::from_str(&atlas_contract::document::graph_vocabulary_json()).expect("the published vocabulary is valid JSON")
}

fn feature_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            feature_files(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("feature") {
            out.push(p);
        }
    }
}

fn strip_binding(body: &str) -> &str {
    match body.find(" as ") {
        Some(i) => {
            let tail = &body[i + 4..];
            let bare = !tail.is_empty()
                && tail.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            if bare {
                &body[..i]
            } else {
                body
            }
        }
        None => body,
    }
}

fn request_key(line: &str) -> Option<String> {
    let s = line.trim();
    let body = s
        .strip_prefix("When I ")
        .or_else(|| s.strip_prefix("And I "))?;
    let body = strip_binding(body);

    if let Some(path) = body.strip_prefix("GET ") {
        return Some(format!("GET {}", path.trim()));
    }
    if let Some(args) = body.strip_prefix("run bibex ") {
        return Some(format!("bibex {}", args.trim()));
    }
    if let Some(rest) = body.strip_prefix("read the ") {
        if let Some(name) = rest.strip_suffix(" export") {
            return Some(format!("export {}", name.trim()));
        }
    }
    if body == "read the graph's declared vocabulary" {
        return Some("graph vocabulary".to_string());
    }
    None
}

fn provided_suites() -> Vec<String> {
    let registry = repo_root().join("contracts/SUITES");
    let text = std::fs::read_to_string(&registry).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\n  The recorder derives WHICH suites it provides from the registry; \
             without it, it would have to guess, and a hardcoded guess is what review M-R2-4 is about.",
            registry.display()
        )
    });
    let mut suites: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(dir), Some(harness)) = (parts.next(), parts.next()) else {
            continue;
        };
        if harness != "contract-runner" {
            continue;
        }
        suites.push(dir.strip_prefix("contracts/").unwrap_or(dir).to_string());
    }
    suites.sort();
    assert!(
        !suites.is_empty(),
        "no suite in {} is registered 'contract-runner', so this recorder would answer nothing. \
         A recorder with no scope is a broken gate, not a passing one.",
        registry.display()
    );
    suites
}

fn corpus_request_keys() -> Vec<String> {
    let contracts = repo_root().join("contracts");
    let provided = provided_suites();
    let mut files = Vec::new();
    for suite in &provided {
        feature_files(&contracts.join(suite), &mut files);
    }
    files.sort();
    assert!(
        !files.is_empty(),
        "no .feature files under {} for any of {provided:?} -- the recorder derives its work from the corpus, so an empty corpus is a broken gate, not an empty one",
        contracts.display()
    );

    let mut keys: Vec<String> = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("cannot read {}: {e}", f.display()));
        for line in src.lines() {
            if let Some(k) = request_key(line) {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
    }
    keys.sort();
    keys
}

async fn record_http(path: &str) -> Value {
    let response = app()
        .oneshot(Request::builder().uri(path).body(Body::empty()).expect("a request must build"))
        .await
        .expect("the router must answer");
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.expect("a body must collect").to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!("{path} did not answer with JSON ({e}) -- every juncture this suite pins is a JSON surface")
    });
    json!({ "status": status, "body": body })
}

fn verify_export_is_readable(name: &str) {
    let p = repo_root().join("data/exports").join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!("a feature file asks for the '{name}' export, but {} cannot be read: {e}", p.display())
    });
    serde_json::from_str::<Value>(&raw)
        .unwrap_or_else(|e| panic!("the published export {} is not JSON: {e}", p.display()));
}

async fn build_pact() -> Value {
    let mut entries = Map::new();
    let mut cli_keys: Vec<Value> = Vec::new();

    for key in corpus_request_keys() {
        let entry = if let Some(path) = key.strip_prefix("GET ") {
            record_http(path).await
        } else if let Some(name) = key.strip_prefix("export ") {
            verify_export_is_readable(name);
            continue;
        } else if key == "graph vocabulary" {
            json!({ "status": 200, "body": graph_vocabulary() })
        } else if key.starts_with("bibex ") {
            cli_keys.push(Value::String(key));
            continue;
        } else {
            panic!("the corpus asks for request key '{key}', which this recorder does not know how to answer -- teach `request_key` about it, or fix the step");
        };
        entries.insert(key, entry);
    }

    assert!(
        !entries.is_empty(),
        "recorded nothing -- a pact with no entries would let every expectation fail with 'no pact entry', which reads like a corpus problem and is actually a recorder problem"
    );

    json!({ "entries": Value::Object(entries), "cli_keys": Value::Array(cli_keys) })
}

fn pact_path() -> PathBuf {
    repo_root().join("contracts/pacts/http.json")
}

fn render(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("a pact must serialize");
    s.push('\n');
    s
}

#[tokio::test(flavor = "multi_thread")]
async fn the_recorded_pact_still_matches_the_live_graph() {
    let pact = build_pact().await;
    let rendered = render(&pact);
    let path = pact_path();

    if std::env::var("ATLAS_BLESS_PACT").as_deref() == Ok("1") {
        if let Ok(committed) = std::fs::read_to_string(&path) {
            if let Ok(old) = serde_json::from_str::<Value>(&committed) {
                let empty = Map::new();
                let o = old.get("entries").and_then(Value::as_object).unwrap_or(&empty);
                let n = pact.get("entries").and_then(Value::as_object).unwrap_or(&empty);
                let lost: Vec<&String> = o.keys().filter(|k| !n.contains_key(*k)).collect();
                if !lost.is_empty() {
                    panic!(
                        "REFUSING TO RE-RECORD: the committed pact answers {} juncture(s) this run \
                         does not ask about.\n  first lost: {:?}\n\
                         \n  The recorder derives its questions from the .feature corpus, so a corpus \
                         that shrank -- renamed, moved, emptied, or resolved away in a merge -- makes \
                         this test look like provider drift. It is not drift. Re-recording here would \
                         write a SMALLER pact and turn the gate green over expectations nobody is \
                         asking any more.\n  \
                         Restore the corpus, or remove those expectations deliberately: the semver \
                         gate classifies a removal MAJOR, and on a RECEIVED suite refuses it outright.",
                        lost.len(),
                        lost.iter().take(3).collect::<Vec<_>>()
                    );
                }
            }
        }
        std::fs::create_dir_all(path.parent().expect("the pact has a parent directory"))
            .expect("the pact directory must be creatable");
        std::fs::write(&path, &rendered).expect("the pact must be writable");
        panic!(
            "ATLAS_BLESS_PACT=1: RE-RECORDED {} ({} bytes) -- failing on purpose.\n  \
             Blessing rewrites the evidence, so it must never be able to report a passing gate.\n  \
             Review the diff, then re-run WITHOUT ATLAS_BLESS_PACT to verify against it.",
            path.display(),
            rendered.len()
        );
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no recorded pact at {}.\n  Generate it once with:\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-contract --test contract_pact",
            path.display()
        )
    });

    if committed == rendered {
        return;
    }

    let old: Value = serde_json::from_str(&committed).expect("the committed pact must be JSON");
    let first_change = first_differing_key(&old, &pact);
    panic!(
        "the provider drifted from the recorded pact.\n  {first_change}\n\
         \n  If this change is DELIBERATE, it is a contract change: bump the affected\n\
         suite's VERSION, add a CHANGELOG entry, and re-record with\n\
         ATLAS_BLESS_PACT=1 cargo test -p atlas-contract --test contract_pact\n\
         The semver gate classifies the bump from the diff and will refuse one that\n\
         is too small."
    );
}

fn first_differing_key(old: &Value, new: &Value) -> String {
    let empty = Map::new();
    let o = old.get("entries").and_then(Value::as_object).unwrap_or(&empty);
    let n = new.get("entries").and_then(Value::as_object).unwrap_or(&empty);
    for k in o.keys() {
        if !n.contains_key(k) {
            return format!(
                "the corpus STOPPED ASKING about juncture '{k}' (it is in the committed pact and not \
                 in this run).\n  That is a shrinking corpus, not provider drift: do NOT re-record. \
                 Restore the .feature file, or remove the expectation deliberately"
            );
        }
    }
    for (k, nv) in n {
        match o.get(k) {
            None => return format!("new juncture recorded: '{k}'"),
            Some(ov) if ov != nv => return format!("juncture '{k}' answers differently than the committed pact"),
            _ => {}
        }
    }
    for k in o.keys() {
        if !n.contains_key(k) {
            return format!("juncture '{k}' is no longer requested by any feature file");
        }
    }
    "the pact differs only in formatting".to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_assembled_app_is_not_hollow_on_any_derived_index() {
    async fn body(path: &str) -> Value {
        let response = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).expect("a request must build"))
            .await
            .expect("the router must answer");
        let bytes = response.into_body().collect().await.expect("a body must collect").to_bytes();
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{path} must answer JSON: {e}"))
    }

    let sources = body("/api/sources").await;
    let n_sources = sources["sources"].as_array().map(Vec::len).unwrap_or(0);
    let n_provenances = sources["provenances"].as_array().map(Vec::len).unwrap_or(0);
    assert!(
        n_sources > 0 && n_provenances > 0,
        "/api/sources answered with an EMPTY registry ({n_sources} sources, {n_provenances} provenances).\n  \
         This is fidelity bug 1: the app was assembled with a default SourcesDocument instead of data/compiled/sources.json.\n  \
         The recorded pact would have pinned the empty answer and the contract suite would have gone green over it."
    );

    let catechism = body("/api/catechism/MAT.28.19").await;
    let n_items = catechism.as_array().map(Vec::len).unwrap_or(0);
    assert!(
        n_items > 0,
        "/api/catechism/MAT.28.19 answered with NO items.\n  \
         This is fidelity bug 2: AtlasData::finish() did not run, so verse_to_catechism is empty and EVERY reference answers [].\n  \
         The endpoint is dark and the response is a valid, blessable, permanently-green empty array."
    );

    let xrefs = body("/api/xrefs/JHN.3.16").await;
    let n_xrefs = xrefs.as_array().map(Vec::len).unwrap_or(0);
    assert!(
        n_xrefs > 0,
        "/api/xrefs/JHN.3.16 answered with NO cross-references -- the same hollow-index class as the two bugs above."
    );
}

#[test]
fn request_keys_are_read_out_of_step_lines_exactly() {
    assert_eq!(request_key("    When I GET /api/eras").as_deref(), Some("GET /api/eras"));
    assert_eq!(request_key("    And I GET /api/eras as first").as_deref(), Some("GET /api/eras"));
    assert_eq!(
        request_key("    When I GET /api/node/Place:hazor-1/edges?kind=site-of as wire").as_deref(),
        Some("GET /api/node/Place:hazor-1/edges?kind=site-of")
    );
    assert_eq!(
        request_key("    And I run bibex edges Place:hazor-1 --kind site-of as cli").as_deref(),
        Some("bibex edges Place:hazor-1 --kind site-of")
    );
    assert_eq!(
        request_key("    When I read the kretzmann-chronology export as k").as_deref(),
        Some("export kretzmann-chronology")
    );
    assert_eq!(
        request_key("    When I read the graph's declared vocabulary").as_deref(),
        Some("graph vocabulary")
    );
    assert_eq!(request_key("    Then the response equals fixture \"x\""), None);
    assert_eq!(request_key("  Our parse_eras reads four fields per era"), None);
    assert_eq!(
        request_key("    When I GET /api/xrefs/JHN.3.16 as well as more").as_deref(),
        Some("GET /api/xrefs/JHN.3.16 as well as more")
    );
}

#[test]
fn the_published_vocabulary_is_drawn_from_the_macros() {
    let v = graph_vocabulary();
    let kinds = v["node_kinds"].as_array().expect("node_kinds is an array");
    let relations = v["relations"].as_array().expect("relations is an array");
    let symmetric = v["symmetric"].as_array().expect("symmetric is an array");

    assert_eq!(
        relations.len(),
        RelationId::ALL.len(),
        "every directed relation in the manifest must be published"
    );
    assert_eq!(
        symmetric.len(),
        SymRelationId::ALL.len(),
        "every symmetric relation in the manifest must be published"
    );
    assert!(kinds.iter().any(|k| k == "Place"), "the published kinds must be the wire's own Debug names");

    for r in RelationId::ALL {
        assert_ne!(r.forward_label(), r.inverse_label(), "{r:?} has a degenerate label pair");
    }
}
