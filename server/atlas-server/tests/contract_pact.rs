//! Batch CDC-1: THE PROVIDER HALF OF THE GATE.
//!
//! The consumer-driven contract law says the CONSUMER writes the
//! expectations and the PROVIDER runs them and breaks when it violates
//! them. `contracts/runner` is what runs them. This file is what makes
//! running them cheap enough to do before every push.
//!
//! # Why a recorded pact at all
//!
//! The expectations must be executable PRE-PUSH, LOCALLY, and FAST, with
//! no escape hatch. The obvious shape -- start a server, point the runner
//! at it -- is not available here: 8080 is the owner's live app and 8090
//! is doubly reserved, and a gate that needs a process on a port is a gate
//! that fails for reasons unrelated to any contract. A gate that cries
//! wolf gets disabled, and a disabled gate is worse than none.
//!
//! So the evidence is recorded instead of re-served, and the recording is
//! itself gated:
//!
//!   * THIS test regenerates the pact from the REAL committed graph
//!     through the REAL axum `Router`, in-process, and fails if the result
//!     differs by one byte from the committed copy. That is PROVIDER
//!     drift: our answers changed.
//!   * `contract-runner run --replay contracts/pacts <suite>` executes
//!     every published expectation against that pact. That is EXPECTATION
//!     drift: what a consumer reads changed.
//!
//! Neither half can pass alone, and neither is advisory. A pact compared
//! only against a pact would prove nothing; a pact regenerated from the
//! live graph in the same gate proves exactly as much as a live run,
//! minus the socket.
//!
//! # Why it cannot silently under-cover
//!
//! The recorder does not carry a hand-maintained list of what to record.
//! It READS THE FEATURE FILES and records exactly the request keys they
//! ask for. Adding `When I GET /api/something-new` to any feature file
//! therefore adds a pact entry here; and if this test has not been re-run,
//! the runner fails with "no pact entry for ...", naming the key. An
//! expectation can never quietly outrun the evidence behind it.
//!
//! In-process via `tower::ServiceExt::oneshot`, the same idiom
//! `tests/aqc_cucumber.rs` and `tests/graph_api.rs` already use -- no
//! socket is bound, so this test is safe under `cargo test --workspace`
//! and cannot collide with anything on any port.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use atlas_core::data::AtlasData;
use atlas_core::sources::SourcesDocument;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{RelationId, SymRelationId};
use atlas_graph_types::id::NodeKind;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Map, Value};
use tower::ServiceExt;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The real committed graph's own Router -- built ONCE, then cloned per
/// request (axum's `Router` is an `Arc` internally). Same "one real
/// compile, shared across every call site" discipline
/// `tests/aqc_cucumber.rs::app` already established.
///
/// PACT FIDELITY, and the trap this function walks around: a recorder that
/// assembles the app DIFFERENTLY from production records the wrong
/// evidence, and does so silently. The first draft here called
/// `app::build(..., None)`, which supplies `SourcesDocument::default()` --
/// so `GET /api/sources` recorded `{"categories":[],"provenances":[],
/// "sources":[]}` while the real server, which loads
/// `data/compiled/sources.json` and calls `build_with_sources`
/// (`src/main.rs:196-201`), serves 5 categories, 18 sources and 26
/// provenances. The suite would have gone green against an empty registry.
/// It was caught because the projection demanded an array and got an
/// object, which is luck; the structural answer is to build the app the
/// way `main.rs` builds it, which is what this now does.
fn app() -> axum::Router {
    static ROUTER: OnceLock<axum::Router> = OnceLock::new();
    ROUTER
        .get_or_init(|| {
            let data_dir = repo_root().join("data");
            let raw_dir = data_dir.join("raw");
            let compiled = atlas_etl::compile::compile(&raw_dir, &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first");
            let data: AtlasData = compiled.data;
            let graph = GraphService::build(&raw_dir, &data)
                .expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law");

            // Exactly what src/main.rs does, for exactly its reason.
            let sources_path = data_dir.join("compiled/sources.json");
            let sources_json = std::fs::read_to_string(&sources_path).unwrap_or_else(|e| {
                panic!("reading {} ({e}) -- run `cargo run -p atlas-etl --bin gen_sources` from server/ first", sources_path.display())
            });
            let sources: SourcesDocument = serde_json::from_str(&sources_json)
                .unwrap_or_else(|e| panic!("parsing {}: {e}", sources_path.display()));
            assert!(
                !sources.sources.is_empty(),
                "the recorded pact must reflect the REAL provider: an empty source registry means this recorder is assembling the app differently from src/main.rs"
            );

            atlas_server::app::build_with_sources(
                std::sync::Arc::new(data),
                std::sync::Arc::new(graph),
                std::sync::Arc::new(sources),
                None,
            )
        })
        .clone()
}

// ---------------------------------------------------------------------
// THE GRAPH'S DECLARED VOCABULARY
// ---------------------------------------------------------------------

/// ONE definition site for the node-kind list, which is the whole point.
///
/// The macro's single list generates BOTH the published names AND an
/// exhaustive `match` over `NodeKind`. Adding a variant to graph-types'
/// `kind_tags!` manifest therefore fails to compile HERE, rather than
/// silently omitting the new kind from the vocabulary the contract suite
/// checks every other transport against. That is the same
/// unrepresentable-drift discipline `relations!` itself uses, applied to
/// the one list `relations!` does not generate.
///
/// The names are `format!("{:?}", ..)` because that is literally what the
/// wire does (`graph_handlers.rs:152`, `kind: format!("{:?}",
/// node_id.kind)`) -- this publishes the vocabulary the server actually
/// speaks, not a prettier one we would like it to speak.
macro_rules! node_kind_manifest {
    ($($v:ident),+ $(,)?) => {{
        #[allow(dead_code)]
        fn exhaustive(k: NodeKind) {
            match k { $(NodeKind::$v => ()),+ }
        }
        vec![$(format!("{:?}", NodeKind::$v)),+]
    }};
}

fn graph_vocabulary() -> Value {
    let node_kinds = node_kind_manifest![
        TextUnit,
        Container,
        Event,
        Narrative,
        Place,
        Person,
        Anchor,
        Era,
        Polity,
        CatechismItem,
        Source,
        Translation,
        PeopleGroup,
        CommentaryItem,
    ];

    // These two DO come straight from the macro-generated tables -- there
    // is no second list to drift from.
    let relations: Vec<Value> = RelationId::ALL
        .iter()
        .map(|r| json!({ "name": format!("{r:?}"), "forward": r.forward_label(), "inverse": r.inverse_label() }))
        .collect();
    let symmetric: Vec<Value> = SymRelationId::ALL
        .iter()
        .map(|s| json!({ "name": format!("{s:?}"), "label": s.label() }))
        .collect();

    json!({ "node_kinds": node_kinds, "relations": relations, "symmetric": symmetric })
}

// ---------------------------------------------------------------------
// READING THE CORPUS'S OWN REQUEST KEYS
// ---------------------------------------------------------------------

/// Every `.feature` file under `contracts/`, recursively.
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

/// A step body like `GET /api/eras as first` binds its answer under a
/// name; the KEY is the part before ` as <name>`.
///
/// Split on the FIRST ` as `, not the last, because that is what the
/// Haskell side does and the two must agree exactly: `capUntil @UrlPath
/// " as "` breaks on the first occurrence, and a remainder that is not a
/// bare binding name fails `BindName`'s parse, so the step falls through
/// to the un-bound `capRest @UrlPath` overload which takes the line whole.
/// Splitting on the LAST occurrence would make this recorder key
/// `/api/x as well` where the runner asks for `/api/x as well as more` --
/// the two languages silently disagreeing about one line, which is the
/// exact failure this pair of implementations has to avoid. Caught by
/// `request_keys_are_read_out_of_step_lines_exactly`, not by inspection.
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

/// The request key a step line asks for, if it asks for one. Mirrors
/// `contracts/runner/src/Steps.hs`'s `When` definitions exactly -- that is
/// a real coupling between two languages, and it is deliberately kept to
/// this one small function so it is auditable in one place. A step shape
/// added there without being added here shows up immediately as the
/// runner's own "no pact entry for ..." failure, naming the key.
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

fn corpus_request_keys() -> Vec<String> {
    let contracts = repo_root().join("contracts");
    let mut files = Vec::new();
    feature_files(&contracts, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no .feature files under {} -- the recorder derives its work from the corpus, so an empty corpus is a broken gate, not an empty one",
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

// ---------------------------------------------------------------------
// RECORDING
// ---------------------------------------------------------------------

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

/// A published export is NOT recorded into the pact -- the runner reads
/// the committed file directly (`--exports data/exports`).
///
/// The pact exists to freeze evidence that is otherwise only obtainable
/// from a running server. An export is already a committed artifact that
/// every reviewer sees in the diff, so recording it would store the same
/// bytes twice and churn the pact on every recompile for no added
/// guarantee. Measured, not assumed: the three exports were 741 KB of an
/// otherwise 50 KB pact.
///
/// What is still owed here is that the artifact the corpus NAMES actually
/// exists and parses, so a feature file cannot name an export into being.
fn verify_export_is_readable(name: &str) {
    let p = repo_root().join("data/exports").join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!("a feature file asks for the '{name}' export, but {} cannot be read: {e}", p.display())
    });
    serde_json::from_str::<Value>(&raw)
        .unwrap_or_else(|e| panic!("the published export {} is not JSON: {e}", p.display()));
}

/// The pact this crate is responsible for: every juncture reachable from
/// the Router, the published exports, and the graph's own vocabulary.
///
/// bibex's junctures are NOT here. Its binary belongs to `atlas-cli`, and
/// `CARGO_BIN_EXE_bibex` is only defined for that package's own tests --
/// so the CLI records its own fragment next to this one
/// (`server/atlas-cli/tests/contract_pact_cli.rs`), and the runner merges
/// every `*.json` in `contracts/pacts/`. One recorder per crate that owns
/// a transport, rather than one recorder reaching across crate boundaries.
async fn build_pact() -> Value {
    let mut entries = Map::new();
    let mut cli_keys: Vec<Value> = Vec::new();

    for key in corpus_request_keys() {
        let entry = if let Some(path) = key.strip_prefix("GET ") {
            record_http(path).await
        } else if let Some(name) = key.strip_prefix("export ") {
            // Served from the committed artifact, not from the pact.
            verify_export_is_readable(name);
            continue;
        } else if key == "graph vocabulary" {
            json!({ "status": 200, "body": graph_vocabulary() })
        } else if key.starts_with("bibex ") {
            // atlas-cli's fragment owns these, but the KEY EXTRACTION rule
            // lives here, in one place, so the two recorders cannot come to
            // disagree about what a step line means. This list is what the
            // CLI recorder works from.
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

/// Regenerate the pact from the live graph and compare it to the committed
/// copy, byte for byte.
///
/// `ATLAS_BLESS_PACT=1` re-records instead of comparing. That is a
/// RE-RECORD switch, not a gate escape hatch, and the distinction is the
/// whole point: it changes what the evidence SAYS, it does not change
/// whether the gate RUNS. After blessing, the pact is a committed file
/// that a reviewer reads in the diff, and every expectation still executes
/// against it. `--bless` on map-generator's own runner is the same idea
/// and the same precedent.
#[tokio::test(flavor = "multi_thread")]
async fn the_recorded_pact_still_matches_the_live_graph() {
    let pact = build_pact().await;
    let rendered = render(&pact);
    let path = pact_path();

    if std::env::var("ATLAS_BLESS_PACT").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().expect("the pact has a parent directory"))
            .expect("the pact directory must be creatable");
        std::fs::write(&path, &rendered).expect("the pact must be writable");
        eprintln!("ATLAS_BLESS_PACT=1: re-recorded {} ({} bytes)", path.display(), rendered.len());
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no recorded pact at {}.\n  Generate it once with:\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact",
            path.display()
        )
    });

    if committed == rendered {
        return;
    }

    // Name the first key that moved, rather than dumping two documents.
    let old: Value = serde_json::from_str(&committed).expect("the committed pact must be JSON");
    let first_change = first_differing_key(&old, &pact);
    panic!(
        "the provider drifted from the recorded pact.\n  {first_change}\n\
         \n  If this change is DELIBERATE, it is a contract change: bump the affected\n\
         suite's VERSION, add a CHANGELOG entry, and re-record with\n\
         ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact\n\
         The semver gate classifies the bump from the diff and will refuse one that\n\
         is too small."
    );
}

fn first_differing_key(old: &Value, new: &Value) -> String {
    let empty = Map::new();
    let o = old.get("entries").and_then(Value::as_object).unwrap_or(&empty);
    let n = new.get("entries").and_then(Value::as_object).unwrap_or(&empty);
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

/// The key-extraction rule is the one place two languages have to agree
/// about what a step line means, so it gets its own falsifiable test
/// rather than being trusted because it is short.
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
    // A `Then` is not a request, and prose is not a request.
    assert_eq!(request_key("    Then the response equals fixture \"x\""), None);
    assert_eq!(request_key("  Our parse_eras reads four fields per era"), None);
    // ` as ` inside a path is not a binding: only a BARE trailing name is.
    assert_eq!(
        request_key("    When I GET /api/xrefs/JHN.3.16 as well as more").as_deref(),
        Some("GET /api/xrefs/JHN.3.16 as well as more")
    );
}

/// The vocabulary is the root of every other promise in the suite, so
/// "did we actually publish a vocabulary" is checked here rather than
/// only implied by a fixture comparison downstream.
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

    // A forward and an inverse label are never the same string; if they
    // were, the union the runner checks against would silently lose one.
    for r in RelationId::ALL {
        assert_ne!(r.forward_label(), r.inverse_label(), "{r:?} has a degenerate label pair");
    }
}
