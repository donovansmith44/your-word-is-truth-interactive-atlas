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
/// PACT FIDELITY: a recorder that assembles the app DIFFERENTLY from
/// production records the wrong evidence, and does so silently. CDC-1 did
/// exactly that twice, and both times the contract suite stayed GREEN.
///
/// **This function is one line, and that is the fix** (fix round 1, review
/// C-3). It used to hand-copy `src/main.rs`'s default startup path;
/// `main.rs` and this recorder now call the SAME constructor and reach the
/// Router through the SAME door, so there is no second copy to drift from.
/// Divergence is not *detected*, it is unrepresentable. See
/// `atlas_server::load`'s header for the two bugs that motivated it.
///
/// `static_dir: None` is the one deliberate difference and it cannot reach
/// an `/api` handler -- it only mounts the published client's static files.
fn app() -> axum::Router {
    static ROUTER: OnceLock<axum::Router> = OnceLock::new();
    ROUTER
        .get_or_init(|| {
            atlas_server::load::load_from_data_dir(&repo_root().join("data/compiled"))
                .expect("data/compiled must load exactly as the server loads it")
                .into_router(None)
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
        LexiconEntry,
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

    // THE ARTIFACT FORMAT WALL (fix round 1, review M-5).
    //
    // `artifact::load` refuses any graph.bin whose format_version is not
    // exactly this number, so a bump refuses every holder of an older
    // artifact outright -- the sharpest break this repo can make, and the
    // one the gate could not see. Published here, from the same constant
    // `load` enforces, so a 13 -> 14 bump now moves the vocabulary fixture
    // and the semver gate classifies it MAJOR like any other removed
    // guarantee.
    // DB-5: the artifact is gone; the served identity is the manifest's
    // (spec 9): its schema and every section's `PRAGMA user_version` --
    // an unknown one is refused at open, exactly as an old graph.bin was.
    let manifest_schema = atlas_graph::sqlite::manifest::MANIFEST_SCHEMA;
    let section_schema_version = atlas_graph::sections::SECTION_SCHEMA_VERSION;

    json!({
        "manifest_schema": manifest_schema,
        "section_schema_version": section_schema_version,
        "node_kinds": node_kinds,
        "relations": relations,
        "symmetric": symmetric,
    })
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

/// The suites for which THE ATLAS IS THE PROVIDER, and therefore the only
/// suites this recorder may answer.
///
/// This list is not tidiness -- it is a correctness boundary, and leaving
/// it out was a real bug the gate caught on its first full run. Two suites
/// can name the same path and mean different servers:
/// `contracts/map-api-consumer` is the atlas speaking as a CONSUMER of
/// map-generator's API, and its `When I GET /api/scene` addresses
/// map-generator's scene endpoint. Scanning the whole `contracts/` tree
/// made this recorder answer that key with OUR `/api/scene` -- recording
/// one provider's answer under another provider's question, silently, in a
/// pact that would have looked perfectly well-formed.
///
/// So the rule is stated positively: record only where we are the
/// provider. `atlas-edge` qualifies (it is map-generator's expectations OF
/// US); `map-api-consumer` does not, and is run by map-generator against
/// its own server.
/// DERIVED, not written down (fix round 3, review M-R2-4).
///
/// This used to be `const PROVIDED_SUITES: &[&str] = &["atlas-graph-contract",
/// "atlas-edge"]` -- a literal array, which is the exact shape that produced
/// C-2 in round 1 (a suite excluded from every tag check) and C-NEW-2 in
/// round 2 (a suite excluded by one registry word). Leg 3 turned out to be
/// the strongest single defence in the gate -- it is what refused three of
/// the round-2 probes when nothing else did -- and its scope was a
/// hardcoded list. A second received suite added tomorrow would be detected
/// by the gate, classified by the registry, executed by leg 4, and
/// **silently uncovered here**, so hiding its corpus would move no pact and
/// none of leg 3's objections would fire.
///
/// So the scope is read from `contracts/SUITES`: every row registered
/// `contract-runner` is a suite for which WE are the provider, which is the
/// same fact the gate derives. An unreadable or empty registry is a broken
/// recorder, not an empty one, and says so.
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
/// guarantee.
///
/// Measured on disk, with the method stated so the numbers can be checked:
/// the three published exports are 428,479 + 689,530 + 17,290 = **1,135,299
/// bytes (1,109 KiB)**, against a committed `http.json` of **175,656 bytes
/// (172 KiB)**. Recording them would multiply the pact several-fold and
/// churn all of it on every recompile.
///
/// (Fix round 1, review L-1a. The earlier version of this sentence said
/// "741 KB of an otherwise 50 KB pact". Those came from a compact
/// `json.dumps` of the entries while the pact itself is pretty-printed, so
/// they corresponded to nothing anybody could measure on disk -- the
/// argument was right and the numbers were not comparable. The claim was
/// the problem, not the conclusion.)
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
/// # `ATLAS_BLESS_PACT=1` RE-RECORDS **AND THEN FAILS** (fix round 1, C-1)
///
/// Last round this branch wrote the file and `return`ed, so the test passed
/// unconditionally. The review found the consequence: any shell that
/// already exports the variable -- a leftover from a legitimate re-record,
/// a line in a profile, a CI variable -- makes leg 3 of the contract gate
/// pass forever while silently rewriting the evidence underneath it. The
/// defence offered was that this is "a re-record switch, not an escape
/// hatch"; that was a claim about how people would behave, not about what
/// the code does, and the brief's rule is that an escape hatch on a gate is
/// the gate's death.
///
/// So blessing is now **incapable of producing a green test**. It writes
/// the pact and panics. A re-record therefore requires a human to read the
/// diff and re-run without the variable, which is exactly the workflow the
/// old comment merely *described*.
///
/// This closes the bypass at the source. `scripts/contract-gate.sh` closes
/// it twice more -- it strips the variable from leg 3's environment, and it
/// asserts afterwards that `contracts/pacts` is unchanged in the working
/// tree, which catches every present and future re-record path in one line.
#[tokio::test(flavor = "multi_thread")]
async fn the_recorded_pact_still_matches_the_live_graph() {
    let pact = build_pact().await;
    let rendered = render(&pact);
    let path = pact_path();

    if std::env::var("ATLAS_BLESS_PACT").as_deref() == Ok("1") {
        // A SHRINKING JUNCTURE SET IS NOT BLESSABLE (fix round 3, C-R2-3).
        //
        // This is the laundering path the round-2 review walked end to end.
        // Rename a received suite's `.feature` files away: the corpus
        // shrinks, so `corpus_request_keys()` shrinks, so the pact this test
        // builds shrinks -- and the test fails with "the provider drifted
        // from the recorded pact", whose remedy the gate PRINTS and this
        // file PRINTED: re-record with ATLAS_BLESS_PACT=1. Following our own
        // instructions took contracts/pacts/http.json from 175,656 bytes to
        // 34,257 -- 80% of the recorded evidence gone -- and turned the
        // whole gate green.
        //
        // The message named the wrong event and prescribed the wrong cure.
        // Losing a juncture is not drift; it is a corpus that stopped
        // asking. So blessing refuses BEFORE it writes, and there is no
        // variable that makes it write anyway: the fix for a missing
        // juncture is to put the question back, or to remove it deliberately
        // in a diff a human reads.
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
    // LOST JUNCTURES FIRST, and named as what they are (fix round 3,
    // C-R2-3). This loop used to iterate only the NEW keys, so a corpus
    // that stopped asking four questions produced the generic "the provider
    // drifted" headline and the re-record remedy underneath it -- the exact
    // sentence the reviewer followed to shrink the pact by 80% and turn the
    // gate green. A missing question is not a drifting answer, and the two
    // must not share a message.
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

/// THE FIDELITY REGRESSION TEST (fix round 1, review C-3).
///
/// Sharing `atlas_server::load` with `main.rs` makes an assembly divergence
/// unrepresentable, which is the real fix. This is the belt to that braces:
/// a law that fails if the assembled app is HOLLOW on any of the three
/// surfaces whose emptiness has actually bitten this project.
///
/// Why it earns its place even after the structural fix: `load.rs` is one
/// function, but it is still a function someone can edit. Dropping
/// `.finish()` from it, or handing back a default `SourcesDocument`, would
/// once again make whole endpoints answer with 200 and nothing in them --
/// and every fixture would re-bless cleanly to the hollow answers, because a
/// projection over an empty array is a perfectly well-formed empty array.
/// That is precisely how fidelity bug 2 stayed green.
///
/// Each assertion below names a surface that was ACTUALLY dark at some point
/// in this batch, not a hypothetical:
///
///   * `/api/sources` -> empty registry (fidelity bug 1, caught by luck)
///   * `/api/catechism/{sref}` -> `[]` for EVERY reference (fidelity bug 2,
///     which shipped green and was found only incidentally)
///   * `/api/xrefs/{sref}` -> the other derived-index surface of the same
///     class, included because it would fail the same way and had no guard
///
/// Falsifiability, verified rather than assumed: reverting `app()` to the
/// pre-fix hand-rolled assembly (no `.finish()`) makes the catechism
/// assertion fail, and building with `SourcesDocument::default()` makes the
/// sources assertion fail.
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

    // MAT.28.19 is the most-cited verse in the compiled catechism. The point
    // is not the exact count -- it is that a derived index which is empty
    // answers 200 with `[]` and looks perfectly healthy.
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
