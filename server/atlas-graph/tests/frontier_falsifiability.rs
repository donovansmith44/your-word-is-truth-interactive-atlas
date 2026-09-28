use std::path::Path;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile")
                .data
        })
        .clone()
}

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
        let atlas = real_atlas_data();
        let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
        let concord_corpus = atlas_etl::concord::read_all(&dir.join("concord")).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first");
        let sc_overlap_text = std::fs::read_to_string(dir.parent().unwrap().join("curated/concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
        let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).expect("concord-sc-overlap.toml must parse");
        let concord_bundle = atlas_graph::concord_adapter::ConcordBundle { corpus: concord_corpus, sc_overlap };
        let (_, kjv_verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
        let kretzmann_corpus = atlas_etl::kretzmann::read_all(&dir.join("kretzmann"), &kjv_verses).expect("data/raw/kretzmann must exist -- run data/fetch-raw.ps1 first");

        let (mut graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(
            &kjv_json,
            &xrefs_tsv,
            &atlas,
            &atlas.eras,
            Some(&brainfuel),
            Some(&concord_bundle),
            Some(&kretzmann_corpus),
        )
        .expect("the real committed sources must build");
        graph.build_indexes();
        graph
    })
}

fn relation_row_count(graph: &atlas_graph_types::graph::Graph, kind: atlas_graph_types::edge::EdgeKind) -> usize {
    use atlas_graph_types::edge::{EdgeKind, RelationId as R, SymRelationId as S};
    match kind {
        EdgeKind::Directed(R::Cites, _) => graph.cross_refs.len(),
        EdgeKind::Directed(R::Attests, _) => graph.attests.len(),
        EdgeKind::Directed(R::Mentions, _) => graph.mentions.len(),
        EdgeKind::Directed(R::Succession, _) => graph.succession.len() + graph.canon_succession.len(),
        EdgeKind::Directed(R::DatedBy, _) => graph.dated_by.len(),
        EdgeKind::Directed(R::LocatedAt, _) => graph.located_at.len(),
        EdgeKind::Directed(R::CommentsOn, _) => graph.comments_on.len(),
        EdgeKind::Directed(R::Contains, _) => graph.contains_bible.len(),
        EdgeKind::Symmetric(S::CatechismLink) => graph.catechism.len(),
        EdgeKind::Symmetric(S::TemporalAdjacency) => graph.temporal_adjacency.len(),
        EdgeKind::Symmetric(S::Analogue) => graph.analogue.len(),
        EdgeKind::Directed(R::ParentOf, _) => graph.parent_of.len(),
        EdgeKind::Symmetric(S::Partners) => graph.partners.len(),
        EdgeKind::Directed(R::Participates, _) => graph.participates.len(),
        other => panic!(
            "frontier_falsifiability.rs's relation_row_count has no mapping for {other:?} -- \
             a new Capability::edges() cell names a relation this sweep doesn't know how to \
             count; add an arm here before trusting the falsifiability sweep for it"
        ),
    }
}

#[test]
fn every_capability_has_real_rows_in_the_compiled_artifact() {
    use atlas_graph_types::frontier::Capability;

    let graph = real_graph();
    for cap in Capability::ALL {
        for edge in cap.edges() {
            let n = relation_row_count(graph, edge.kind);
            assert!(
                n > 0,
                "capability {cap:?}'s edge {:?} has ZERO rows in the real compiled graph -- \
                 the unity bridge is decorative here (design review B1's exact defect: the \
                 original `Parallel` cell named a relation with zero producers and zero \
                 consumers anywhere in the repo)",
                edge.kind
            );
        }
    }
}

#[test]
fn members_capability_has_real_bible_contains_rows_since_node_1() {
    let graph = real_graph();
    assert!(
        !graph.contains_bible.is_empty(),
        "NODE-1's own Bible-corpus Contains rows are missing from the compiled graph -- \
         Capability::Members would be as decorative as the original Parallel cell; re-gate it \
         honestly if this ever regresses to zero"
    );
}

#[test]
fn analogues_capability_has_the_owners_own_charter_row() {
    let graph = real_graph();
    assert!(
        !graph.analogue.is_empty(),
        "Capability::Analogues has zero rows in the real compiled graph -- the cell is decorative; \
         gate it honestly if this ever regresses to zero"
    );
    let has_leper_pair = graph.analogue.iter().any(|r| {
        let (a, b) = (r.a.0.as_str(), r.b.0.as_str());
        (a == "rob_leper_healed" && b == "mat_leper_healed") || (a == "mat_leper_healed" && b == "rob_leper_healed")
    });
    assert!(
        has_leper_pair,
        "the owner's own Analogue charter case is missing: \"A leper healed; a great popular \
         excitement is given a parallel where there shouldn't be from Mat.8.1-4; another leprosy \
         story.\" rob_leper_healed <-> mat_leper_healed must be an Analogue row"
    );
}

#[test]
fn the_espousal_is_a_mention_only_event_with_no_attests_rows() {
    use atlas_graph_types::edge::MentionedEntity;
    let graph = real_graph();
    let attests: Vec<&str> = graph.attests.iter().filter(|r| r.event.0 == "theo-249").map(|r| r.provenance.as_str()).collect();
    assert!(
        attests.is_empty(),
        "theo-249 (Espousal of Mary) still has {} Attests row(s) -- LUK.1.27 and MAT.1.18 both \
         MENTION the espousal while narrating something else; neither is an account of it",
        attests.len()
    );
    let mentions = graph
        .mentions
        .iter()
        .filter(|r| matches!(&r.entity, MentionedEntity::Event(e) if e.0 == "theo-249"))
        .count();
    assert_eq!(
        mentions, 2,
        "theo-249 must carry exactly its two retyped mentions (LUK.1.27, MAT.1.18) -- TOTAL \
         CAPTURE means the facts changed type, not that they were dropped"
    );
}

#[test]
fn succession_has_real_rows_in_both_of_its_row_implementations() {
    let graph = real_graph();
    assert!(
        !graph.succession.is_empty(),
        "event-narrative Succession has zero rows in the real compiled graph -- \
         Capability::Chronology would be as decorative as the original Parallel cell; gate \
         Chronology honestly if this ever regresses to zero"
    );
    assert!(
        !graph.canon_succession.is_empty(),
        "canon Succession (chapter/book prev-next steps) has zero rows in the real compiled \
         graph -- the Chapter/Book Chronology cell would be decorative; gate it honestly if \
         this ever regresses to zero"
    );
}
