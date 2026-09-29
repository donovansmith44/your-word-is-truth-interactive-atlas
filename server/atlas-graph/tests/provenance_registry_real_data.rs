use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        atlas_graph::sqlite::reload::committed_graph(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")).expect("the committed sections read back (run atlas-graph-compile first)").0
    })
}

fn registry() -> atlas_core::sources::SourcesDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/sources.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()));
    serde_json::from_str(&json).expect("data/compiled/sources.json must parse as a SourcesDocument")
}

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
    sweep!(authored);
    sweep!(shown);
    sweep!(map_succession);
    out
}

fn distinct_provenance_ids(g: &atlas_graph_types::graph::Graph) -> BTreeSet<String> {
    provenance_by_family(g).into_values().flatten().collect()
}

fn kind_of(id: &str) -> &str {
    atlas_core::sources::split_provenance_id(id).0
}

fn distinct_provenance_kinds(g: &atlas_graph_types::graph::Graph) -> BTreeSet<String> {
    distinct_provenance_ids(g).iter().map(|id| kind_of(id).to_string()).collect()
}

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

fn provenance_field_decls_per_file() -> BTreeMap<String, usize> {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../graph-types/src");
    let mut out: BTreeMap<String, usize> = BTreeMap::new();

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
            let n = src.lines().filter(|l| l.trim().starts_with("pub provenance:") && l.trim().ends_with(',')).count();
            if n > 0 {
                out.insert(child_rel, n);
            }
        }
    }
    out
}

#[test]
fn the_sweep_covers_every_provenance_bearing_row_family() {
    let per_file = provenance_field_decls_per_file();

    let expected: BTreeMap<String, usize> = [("chrono.rs", 1usize), ("edge.rs", 26), ("node.rs", 2), ("store.rs", 1)].into_iter().map(|(f, n)| (f.to_string(), n)).collect();
    assert_eq!(
        per_file, expected,
        "graph-types' `pub provenance:` field declarations moved. If a NEW row family appeared, add it to \
         provenance_by_family() AND to ProvenanceIndex::build(), then re-pin here."
    );

    let node_decls = per_file["node.rs"];
    assert_eq!(node_decls, 2, "graph-types/src/node.rs's provenance fields changed ({node_decls} now, 2 pinned: Node + the Card projection)");

    let total_decls: usize = per_file.values().sum();
    let row_structs = total_decls - node_decls - per_file["store.rs"];
    let families = provenance_by_family(real_graph());
    assert_eq!(
        families.len(),
        row_structs + 1 + 1,
        "provenance_by_family() sweeps {} families; graph-types/src declares {total_decls} provenance fields, \
         {row_structs} of them row structs (+1 for the second Contains<C> vector, +1 for nodes). \
         A family is missing from the sweep.",
        families.len()
    );
}

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

#[test]
fn the_distinct_provenance_inventory_of_the_real_artifact_is_pinned() {
    let kinds: Vec<String> = distinct_provenance_kinds(real_graph()).into_iter().collect();
    assert_eq!(kinds, PINNED_INVENTORY.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "the artifact's distinct provenance-kind inventory changed");
}

const PINNED_INVENTORY: &[&str] = &[
    "attestation-corrections",
    "brainfuel",
    "chronology-anchors",
    "chronology-derivation",
    "concord",
    "concord-sc-overlap",
    "curated",
    "curated-books",
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

    assert_eq!(actual["cross_refs"], vec!["openbible.info-cross-references".to_string()]);
    assert_eq!(actual["catechism"], vec!["concord-sc-overlap".to_string(), "curated-catechism".to_string()]);
}

const PINNED_FAMILIES: &[(&str, &[&str])] = &[
    ("analogue", &["attestation-corrections"]),
    ("attests", &["event-witnesses"]),
    ("authored", &["curated-books"]),
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
    ("map_succession", &["curated-eras"]),
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
    ("occurs", &["stepbible-tagnt", "stepbible-tahot"]),
    ("parent_of", &["theographic-people"]),
    ("participates", &["theographic-people"]),
    ("partners", &["theographic-people"]),
    ("shown", &["curated-eras"]),
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
