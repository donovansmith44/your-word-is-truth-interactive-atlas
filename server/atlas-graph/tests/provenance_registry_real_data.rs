//! Batch PROV-1: THE RESOLUTION LAW, over the real committed corpus.
//!
//! OWNER ORDER 1 (verbatim): "one thing we definitely need for EVERY PIECE
//! OF DATA is the source from which it came. openbible, etc."
//!
//! This file is that order as CODE rather than as prose: every distinct
//! `ProvenanceId` carried by any node or any authored/imported row of the
//! real graph must resolve to exactly one entry in the source REGISTRY
//! (`data/curated/sources.toml` -> `data/compiled/sources.json`, served at
//! `GET /api/sources`). An id that resolves to nothing is a LOUD failure
//! here, at build time, so the browser can never be handed a provenance it
//! cannot name -- the fail-loud law, applied to attribution.
//!
//! UNLIKE every other `_real_data.rs` file in this crate, this one loads
//! the COMPILED ARTIFACT rather than rebuilding from raw sources -- see
//! `real_graph` below for the measured reason.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// THE ARTIFACT ITSELF -- `data/compiled/graph.bin`, deserialized, not a
/// partial rebuild.
///
/// This deliberately does NOT use the `build_graph_from_sources_with_eras`
/// construction every other `_real_data.rs` file in this crate uses, and
/// the difference is load-bearing rather than stylistic. That builder takes
/// only the KJV + xrefs raw sources, so the graph it returns carries NONE of
/// the optional corpora the shipped artifact does -- the Book of Concord,
/// Kretzmann's commentary, the red-letter annotation, the parallel editions.
/// Measured, not assumed: five declared provenance kinds (`concord`,
/// `concord-sc-overlap`, `kretzmann`, `red-letter`, `brainfuel`) are absent
/// from that builder's output and present in the artifact.
///
/// The owner's order is "EVERY PIECE OF DATA," and the artifact is what the
/// server actually serves -- so the artifact is what this law sweeps.
fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        // DB-5: the committed sections read back (sqlite::reload).
        atlas_graph::sqlite::reload::committed_graph(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")).expect("the committed sections read back (run atlas-graph-compile first)").0
    })
}

fn registry() -> atlas_core::sources::SourcesDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/sources.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()));
    serde_json::from_str(&json).expect("data/compiled/sources.json must parse as a SourcesDocument")
}

/// THE SWEEP. One entry per provenance-bearing row family of
/// `atlas_graph_types::graph::Graph`, plus `nodes` -- the family name is
/// the `Graph` FIELD NAME verbatim, so a reader can check completeness by
/// reading the struct beside this function.
///
/// COMPLETENESS is not asserted by eyeball: `the_sweep_covers_every_
/// provenance_bearing_row_family` below counts the `pub provenance:`
/// declarations in graph-types' own source text and reconciles that count
/// against this map, so a future row family added to the contract fails
/// HERE rather than silently escaping attribution.
fn provenance_by_family(g: &atlas_graph_types::graph::Graph) -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut out: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    macro_rules! sweep {
        ($field:ident) => {
            out.entry(stringify!($field)).or_default().extend(g.$field.iter().map(|r| r.provenance.clone()));
        };
    }
    out.entry("nodes").or_default().extend(g.nodes.values().map(|n| n.provenance.clone()));
    sweep!(contains_bible);
    sweep!(contains_concord);
    sweep!(attests);
    sweep!(succession);
    sweep!(canon_succession);
    sweep!(dated_by);
    sweep!(located_at);
    sweep!(fulfills);
    sweep!(typology);
    sweep!(named_after);
    sweep!(catechism);
    sweep!(comments_on);
    sweep!(spoken_by);
    sweep!(spoken_at);
    sweep!(mentions);
    sweep!(cross_refs);
    sweep!(quotes);
    sweep!(confesses);
    sweep!(corresponds_bible);
    sweep!(temporal_adjacency);
    sweep!(analogue);
    sweep!(occurs);
    sweep!(parent_of);
    sweep!(partners);
    sweep!(participates);
    out
}

fn distinct_provenance_ids(g: &atlas_graph_types::graph::Graph) -> BTreeSet<String> {
    provenance_by_family(g).into_values().flatten().collect()
}

/// THE RESOLUTION RULE, in one line, mirrored EXACTLY by the client's own
/// `ProvenanceResolver.Resolve` (client/Explore/ProvenanceResolver.cs) and
/// by `atlas_core::sources::split_provenance_id`, which is the shared
/// implementation both the server and this law call.
///
/// A provenance id is `kind` or `kind/locator`: the KIND is the registry
/// key, the remainder (when present) is the row's own locator -- which is
/// precisely the `{source, locator}` shape
/// `atlas_graph_types::ingest::Provenance` has always declared. It is not
/// a convenience: `kretzmann_adapter` mints one id PER commentary unit
/// (`format!("{KRETZMANN_PROVENANCE_KIND}/{slug}/{n}")`), so an
/// exact-match-only registry would need tens of thousands of curated rows
/// to say one true thing.
fn kind_of(id: &str) -> &str {
    atlas_core::sources::split_provenance_id(id).0
}

fn distinct_provenance_kinds(g: &atlas_graph_types::graph::Graph) -> BTreeSet<String> {
    distinct_provenance_ids(g).iter().map(|id| kind_of(id).to_string()).collect()
}

/// THE LAW ITSELF (owner order 1). Every distinct provenance id in the
/// real artifact resolves to exactly one registry entry -- never zero
/// (a silent blank / a fabricated label at the UI), never two (an
/// ambiguous attribution).
#[test]
fn every_distinct_provenance_id_in_the_artifact_resolves_to_a_registry_source() {
    let g = real_graph();
    let doc = registry();
    let source_ids: BTreeSet<&str> = doc.sources.iter().map(|s| s.id.as_str()).collect();

    let mut unresolved: Vec<String> = Vec::new();
    let mut dangling: Vec<String> = Vec::new();
    let mut duplicated: Vec<String> = Vec::new();

    for kind in distinct_provenance_kinds(g) {
        let hits: Vec<&atlas_core::sources::ProvenanceEntry> = doc.provenances.iter().filter(|p| p.id == kind).collect();
        match hits.len() {
            0 => unresolved.push(kind.clone()),
            1 => {
                if !source_ids.contains(hits[0].source.as_str()) {
                    dangling.push(format!("{kind} -> source '{}' (no such source entry)", hits[0].source));
                }
            }
            _ => duplicated.push(kind.clone()),
        }
    }

    assert!(
        unresolved.is_empty() && dangling.is_empty() && duplicated.is_empty(),
        "THE RESOLUTION LAW FAILED -- every piece of data must name its source.\n\
         unresolved (no [[provenance]] row in data/curated/sources.toml): {unresolved:?}\n\
         dangling (row names a source id that does not exist): {dangling:?}\n\
         duplicated (more than one [[provenance]] row claims this id): {duplicated:?}"
    );
}

/// The other direction, so the registry cannot rot: every declared
/// `[[provenance]]` row is actually INHABITED by the real artifact. A row
/// for an id nothing carries is dead curation -- exactly the drift
/// `validate_against_licenses` already forbids in the LICENSES.md
/// direction, applied to this new table.
#[test]
fn every_declared_provenance_row_is_inhabited_by_the_real_artifact() {
    let g = real_graph();
    let doc = registry();
    let carried = distinct_provenance_kinds(g);

    let orphaned: Vec<&str> = doc.provenances.iter().map(|p| p.id.as_str()).filter(|id| !carried.contains(*id)).collect();
    assert!(
        orphaned.is_empty(),
        "these [[provenance]] rows are declared in data/curated/sources.toml but no node or row in the real \
         artifact carries them -- delete them or fix the id: {orphaned:?}"
    );
}

/// Every `pub provenance:` FIELD DECLARATION in graph-types' own source
/// text, counted per file, over the WHOLE `graph-types/src` tree --
/// RECURSIVELY, every `.rs` file at every depth. Keys are paths relative
/// to `src`, `/`-separated (`node.rs`, `canon/node.rs`).
///
/// FIX ROUND 1 (review L-2): this used to read three NAMED files
/// (`edge.rs`, `chrono.rs`, `node.rs`) with three exact-string needles. A
/// provenance-bearing row family declared in any of the crate's other
/// source files -- or spelled with a different path to the same type --
/// left all three counts unchanged, so the guard passed and the new family
/// escaped attribution silently: the exact hole the guard exists to close.
/// It now walks the tree and matches the FIELD, not a file plus a
/// verbatim type path.
///
/// (FIX ROUND 2, review L-NEW-4: this narration used to assert a FILE
/// COUNT -- "11", then "13, of which 3 carry declarations" -- and got it
/// wrong twice, inside the very fix whose lesson was "walk it, don't name
/// it". So it no longer counts anything in prose. The guard reads the
/// directory; the pin below reads the guard.)
///
/// (DB-2a Task 3 FIX ROUND 1, ruling R16: the walk is now RECURSIVE.
/// It used to read ONE level and assert every entry `is_file`, on the
/// theory that a flat module directory made a subdirectory a new place
/// declarations could hide. Task 1 added `graph-types/src/canon/`, and the
/// assert did exactly what it was built to do -- it failed loudly. The
/// right answer to "declarations could hide in a subdirectory" is to walk
/// the subdirectory, not to forbid it: every `.rs` file at any depth is
/// read, and the map is keyed by the path RELATIVE to `src` with `/`
/// separators, so a future `canon/rows.rs` declaration would appear under
/// its own name rather than colliding with a top-level `rows.rs`.)
fn provenance_field_decls_per_file() -> BTreeMap<String, usize> {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../graph-types/src");
    let mut out: BTreeMap<String, usize> = BTreeMap::new();

    // Explicit stack rather than recursion: the whole walk stays in one
    // function, so the "every .rs at any depth" claim is readable in one
    // place. `rel` is the path so far, `/`-joined, and is what keys `out`.
    let mut stack: Vec<(PathBuf, String)> = vec![(types_dir.clone(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a readable directory entry").path();
            let name = path.file_name().expect("a named entry").to_string_lossy().into_owned();
            let child_rel = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if path.is_dir() {
                stack.push((path, child_rel));
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
            // The FIELD, however its type is spelled: `ProvenanceId`,
            // `crate::ingest::ProvenanceId`, or any future path to it.
            let n = src.lines().filter(|l| l.trim().starts_with("pub provenance:") && l.trim().ends_with(',')).count();
            if n > 0 {
                out.insert(child_rel, n);
            }
        }
    }
    out
}

/// THE COMPLETENESS GUARD. `provenance_by_family` above is hand-written
/// (graph-types is contract-frozen; adding a reflective accessor there is
/// not this batch's authorization), so a NEW provenance-bearing row family
/// added to the contract later could silently escape the sweep. This test
/// closes that hole from the outside: it counts the `pub provenance:`
/// FIELD DECLARATIONS in graph-types' own source text and reconciles them,
/// by an equation stated so it is literally true as written, against the
/// families swept above.
#[test]
fn the_sweep_covers_every_provenance_bearing_row_family() {
    let per_file = provenance_field_decls_per_file();

    // WHERE the declarations live, pinned by IDENTITY (fix round 1, review
    // L-2) -- so a provenance field appearing in a file that had none, or a
    // file dropping out entirely, fails HERE with its own name in the
    // message rather than passing three unchanged per-file counts.
    // DB-3: `store.rs` carries ONE `pub provenance:` -- `RowRef`'s field, the
    // port's answer to "which row made this edge" (spec 4), not a row family;
    // the sweeps below have nothing to add for it.
    let expected: BTreeMap<String, usize> = [("chrono.rs", 1usize), ("edge.rs", 23), ("node.rs", 2), ("store.rs", 1)].into_iter().map(|(f, n)| (f.to_string(), n)).collect();
    assert_eq!(
        per_file, expected,
        "graph-types' `pub provenance:` field declarations moved. If a NEW row family appeared, add it to \
         provenance_by_family() AND to ProvenanceIndex::build(), then re-pin here."
    );

    let node_decls = per_file["node.rs"];
    // node.rs's two are `Node.provenance` (the storage) and
    // `Card.provenance` (a PROJECTION of it -- `node::card()` copies the
    // node's own value; it stores nothing new). NEITHER is a row family:
    // the sweep's own "nodes" family covers the storage one and the
    // projection is not a second source of ids.
    assert_eq!(node_decls, 2, "graph-types/src/node.rs's provenance fields changed ({node_decls} now, 2 pinned: Node + the Card projection)");

    let total_decls: usize = per_file.values().sum();
    // DB-3: minus `store.rs`'s one (`RowRef.provenance`, a port projection
    // of a row's field, not a row struct).
    let row_structs = total_decls - node_decls - per_file["store.rs"];
    let families = provenance_by_family(real_graph());
    // The two `Contains<C>` vectors (`contains_bible`, `contains_concord`)
    // share ONE struct declaration, so the vector count is one more than
    // the struct count. Stated as one equation, literally true as written:
    //   26 (every `pub provenance:` field in graph-types/src)
    //     - 2 (node.rs's storage field and its Card projection: not rows)
    //     = 24 declared ROW structs
    //     + 1 (the second `Contains<C>` vector)
    //     + 1 (the `nodes` map)
    //     = 26 swept families.
    assert_eq!(
        families.len(),
        row_structs + 1 + 1,
        "provenance_by_family() sweeps {} families; graph-types/src declares {total_decls} provenance fields, \
         {row_structs} of them row structs (+1 for the second Contains<C> vector, +1 for nodes). \
         A family is missing from the sweep.",
        families.len()
    );
}

/// FIX ROUND 1 (review L-3): THE TWO SWEEPS, RECONCILED AGAINST EACH OTHER.
///
/// `ProvenanceIndex::build` (the RUNTIME index the wire reads) and
/// `provenance_by_family` (the TEST sweep the completeness guard checks)
/// were two independent hand-maintained lists of the same 22 families with
/// nothing tying them together. The guard above only ever checked the
/// test's copy, so a family added to the test sweep but not to the runtime
/// index would leave `ProvenanceIndex::by_family` silently returning `[]`
/// for it -- an honest-looking empty that means "no rows" everywhere else
/// in this codebase, which is the worst possible way for a drift to
/// present.
///
/// One assertion removes the drift: the two family key sets must be equal.
/// The guard above then binds BOTH lists, because they are now one claim.
#[test]
fn the_runtime_index_and_the_test_sweep_name_exactly_the_same_families() {
    let g = real_graph();
    let swept: Vec<&str> = provenance_by_family(g).keys().copied().collect();
    let indexed: Vec<&str> = atlas_graph::provenance::ProvenanceIndex::build(g).families();
    assert_eq!(
        swept, indexed,
        "ProvenanceIndex::build (server/atlas-graph/src/provenance.rs) and provenance_by_family (this file) \
         have drifted apart. A family missing from the INDEX serves an empty attribution to the wire; a family \
         missing from the SWEEP escapes the resolution law. Add it to both."
    );
}

/// THE INVENTORY, PINNED. The full distinct-provenance-KIND set of the
/// real artifact, asserted by IDENTITY (not merely by count) -- so a new
/// source entering the corpus cannot slip onto the wire un-curated, and a
/// source LEAVING it cannot go unnoticed either. batch-prov1-report.md
/// carries this same list.
#[test]
fn the_distinct_provenance_inventory_of_the_real_artifact_is_pinned() {
    let kinds: Vec<String> = distinct_provenance_kinds(real_graph()).into_iter().collect();
    assert_eq!(kinds, PINNED_INVENTORY.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "the artifact's distinct provenance-kind inventory changed");
}

/// The `Confidence` vocabulary this crate's own contract declares,
/// reconciled against the string list `atlas-core` validates
/// `sources.toml` with -- the two live in different crates on purpose
/// (atlas-core does not depend on graph-types), so this test is the seam
/// that keeps them from drifting apart.
#[test]
fn the_registry_confidence_vocabulary_matches_the_contracts_own_enum() {
    use atlas_graph_types::ingest::Confidence;
    let from_enum: Vec<String> =
        [Confidence::CanonicalText, Confidence::Curated, Confidence::Imported, Confidence::Derived].iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(
        from_enum,
        atlas_core::sources::CONFIDENCE_VOCABULARY.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "atlas_core::sources::CONFIDENCE_VOCABULARY has drifted from atlas_graph_types::ingest::Confidence"
    );
}

/// THE INVENTORY, PINNED -- the 26 distinct provenance KINDS the committed
/// `data/compiled/graph.bin` actually carries, in the sorted order
/// `distinct_provenance_kinds` produces. batch-prov1-report.md carries the
/// same list with each one's registry mapping.
const PINNED_INVENTORY: &[&str] = &[
    "attestation-corrections",
    "brainfuel",
    "chronology-anchors",
    "chronology-derivation",
    "concord",
    "concord-sc-overlap",
    "curated",
    "curated-catechism",
    "curated-eras",
    "curated-fulfillment",
    "curated-named-after",
    "curated-narratives",
    "curated-people-groups",
    "curated-places",
    "curated-polities",
    "curated-typology",
    "event-witnesses",
    "kjv",
    "kretzmann",
    "openbible.info-cross-references",
    "red-letter",
    "stepbible-tagnt",
    "stepbible-tahot",
    "stepbible-tbesg",
    "theographic",
    "theographic-geocoding",
    "theographic-people",
    "theographic-people-groups",
    "theographic-people-reclassified",
];

/// THE PER-FAMILY MAP, PINNED -- which row families are genuinely
/// multi-sourced and which are not.
///
/// THIS TEST EXISTS BECAUSE THIS BATCH GOT IT WRONG. The first draft of
/// `provenance.rs` (and of three doc comments, and of three commit
/// messages) asserted that `attests` carries both `event-witnesses` and
/// `attestation-corrections` in the real corpus. It does not: ATTEST-1's
/// corrections land on `mentions` and `analogue`, and `attests` is
/// single-sourced. The claim was inferred from reading the adapter rather
/// than measured, which is the exact failure the house rule about grep
/// results names. So the fact is code now, not prose -- a future reader
/// deciding whether a section may safely serve a family value has a
/// checked answer instead of a plausible sentence.
///
/// It also pins the three EMPTY families honestly (`quotes`,
/// `confesses`, `corresponds_bible` have zero rows in the artifact) --
/// an empty set here is a real fact about the corpus, not a sweep bug.
#[test]
fn the_per_family_provenance_map_of_the_real_artifact_is_pinned() {
    let actual: BTreeMap<&'static str, Vec<String>> = provenance_by_family(real_graph())
        .into_iter()
        .map(|(fam, set)| (fam, set.iter().map(|id| kind_of(id).to_string()).collect::<BTreeSet<_>>().into_iter().collect()))
        .collect();

    let expected: BTreeMap<&'static str, Vec<String>> = PINNED_FAMILIES
        .iter()
        .map(|(fam, kinds)| (*fam, kinds.iter().map(|k| k.to_string()).collect()))
        .collect();

    assert_eq!(actual, expected, "the artifact's per-family provenance map changed");

    // The two claims the WIRE actually leans on, stated as assertions
    // rather than left implicit in the table above: `cross_refs` and
    // `catechism` are the only two families `handlers::verse` serves as a
    // FAMILY value, and `by_family` is a true statement about an
    // individual row exactly while the family is single-sourced.
    // `cross_refs` is; `catechism` is NOT -- and that is fine, because the
    // wire carries the whole SET and the affordance renders every entry.
    // What must never happen is a family value collapsing to one id.
    assert_eq!(actual["cross_refs"], vec!["openbible.info-cross-references".to_string()]);
    assert_eq!(actual["catechism"], vec!["concord-sc-overlap".to_string(), "curated-catechism".to_string()]);
}

/// Every provenance-bearing family of the committed artifact, with the
/// distinct provenance KINDS it carries. Sorted both ways.
const PINNED_FAMILIES: &[(&str, &[&str])] = &[
    ("analogue", &["attestation-corrections"]),
    ("attests", &["event-witnesses"]),
    ("canon_succession", &["kjv"]),
    ("catechism", &["concord-sc-overlap", "curated-catechism"]),
    ("comments_on", &["kretzmann"]),
    ("confesses", &[]),
    ("contains_bible", &["kjv"]),
    ("contains_concord", &["concord"]),
    ("corresponds_bible", &[]),
    ("cross_refs", &["openbible.info-cross-references"]),
    ("dated_by", &["chronology-derivation"]),
    ("fulfills", &["curated-fulfillment"]),
    ("located_at", &["curated", "theographic"]),
    (
        "mentions",
        &[
            "attestation-corrections",
            "theographic-geocoding",
            "theographic-people",
            "theographic-people-groups",
            "theographic-people-reclassified",
        ],
    ),
    ("named_after", &["curated-named-after"]),
    // LEX-1: NT rows are STEPBible TAGNT's alignment, OT rows TAHOT's.
    ("occurs", &["stepbible-tagnt", "stepbible-tahot"]),
    // D5: kinship and participation are Theographic's own assertions.
    ("parent_of", &["theographic-people"]),
    ("participates", &["theographic-people"]),
    ("partners", &["theographic-people"]),
    (
        "nodes",
        &[
            "brainfuel",
            "chronology-anchors",
            "concord",
            "curated",
            "curated-catechism",
            "curated-eras",
            "curated-narratives",
            "curated-people-groups",
            "curated-places",
            "curated-polities",
            "kjv",
            "kretzmann",
            "stepbible-tbesg",
            "theographic",
            "theographic-people",
            "theographic-people-groups",
            "theographic-people-reclassified",
        ],
    ),
    ("quotes", &[]),
    ("spoken_at", &["event-witnesses"]),
    ("spoken_by", &["red-letter"]),
    ("succession", &["curated-narratives"]),
    ("temporal_adjacency", &["chronology-derivation"]),
    ("typology", &["curated-typology"]),
];
