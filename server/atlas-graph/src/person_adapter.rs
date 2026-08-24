//! Batch P -- THE EXTENSIBILITY PROOF (design doc §8/§9: "Person is
//! planned... enters as Theographic adapter output with zero machinery
//! changes"). Theographic PERSONS -> `Person` nodes + `mentions` rows, via
//! the SAME two-phase shape `place_adapter.rs` established at M-C:
//! NORMALIZE builds nodes (this module's own `normalize`, mirroring
//! `event_world::place_node`/`populate_nodes_and_direct_rows`'s own node
//! construction); MERGE/ALIAS builds `mentions` rows (this module's own
//! `merge_alias`, mirroring `place_adapter::merge_alias`'s own mentions
//! half almost line for line).
//!
//! NO `named` COUNTERPART, disclosed: Person carries no KJV-alias merge
//! table this batch (nothing in the brief asked for one, and this batch's
//! own scoping pass found no evidence of a curated person-naming file
//! anywhere in `data/curated/`) -- `also_called` (Theographic's own
//! alternate-name list) rides the PAYLOAD instead, the identical "payload,
//! not a new relation kind" shape Place's own KJV aliases and Polity's own
//! border data already use (see `graph_types::node::NodePayload::Person`'s
//! own doc comment).
//!
//! ALSO NO GENEALOGY/PLACE EDGES, disclosed (fix round 1, R-P2 -- a real
//! disclosure gap the first draft of this module left silent): the raw
//! source (`people.json`) carries `father`/`mother`/`children`/`siblings`/
//! `partners` (foreign-key ids resolving to other Person records in the
//! same file -- the identical shape `verses` uses for mentions) plus
//! `birthPlace`/`deathPlace`/`memberOf`/`timeline`. None of it is read by
//! `atlas_etl::people::parse_people`, so none of it reaches this adapter.
//! Not an oversight -- a genealogy/family relation, or a Person-to-Place
//! `birthplace`/`deathplace` edge mirroring `located-at`'s own shape, is a
//! NEW RELATION KIND (design types doc §3's own relation-manifest law:
//! "adding a relation = one manifest row + its Row type + a compiler rule
//! + a display-policy row" -- a `graph-types` change needing owner
//! approval, not an adapter-only decision), correctly outside this
//! batch's own card+mentions scope. Ledgered for the owner: this is the
//! SAME extensibility-proof shape `mentions` already demonstrates --
//! zero new sourcing needed, only an adapter, whenever it is wanted.
//!
//! SOURCE: `AtlasData.people` (`atlas_etl::people::parse_people`, reading
//! Theographic's own `people.json`+`verses.json`) -- populated by
//! `atlas_etl::compile::compile` as a `#[serde(skip)]` sidecar field, the
//! SAME status `polities`/`catechism`/etc. already have. UNLIKE
//! `places`/`events`, there is no `people.json` compiled artifact and never
//! will be: Person was born AFTER the graph became the one artifact (design
//! doc P1), so it goes straight from raw source into the graph with no
//! intermediate JSON stage ever standing, let alone retiring later -- the
//! deletion event's own discipline applied PROSPECTIVELY.
//!
//! BOUNDARY FIDELITY LAW (design doc P3; batch-p-brief.md requirement 1,
//! "counts + referential integrity, fail-loud"): [`check_person_fidelity`]
//! below, run unconditionally at pipeline LAW-CHECK time (`pipeline.rs`'s
//! own `LawCheckPass`). Two real, adapter-specific COUNTS checks --
//! (a) BIJECTION: every source person record becomes exactly one Person
//! node, id-exact (mirrors the KJV adapter's own bijection law in miniature
//! -- `fidelity.rs`'s "every source verse <-> exactly one Verse node");
//! (b) MENTIONS COMPLETENESS: every person's own resolved `verse_links`
//! becomes exactly that many `mentions` rows, no silent drop. REFERENTIAL
//! INTEGRITY is deliberately NOT re-implemented here -- `law_check::
//! every_authored_edge_resolves` already checks `mentions.entity` for BOTH
//! `MentionedEntity` variants (confirmed reading that function fresh, M-C2
//! vintage, entirely unaware this batch was coming), so this adapter's own
//! referential integrity comes free from the EXISTING generic law -- a real
//! thesis data point, disclosed rather than duplicated.

use atlas_core::data::AtlasData;
use atlas_graph_types::edge::{Mentions, MentionedEntity};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{NodeKind, PersonId};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef};

use crate::pipeline::BuildCtx;

/// Provenance tag every Person node and every Person-entity `mentions` row
/// carries -- one constant, not two independently-typed literals.
pub const PROVENANCE: &str = "theographic-people";

fn verse_locus(vref: &str) -> Option<TextLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    let vr = VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse };
    Some(TextLocus::from(BibleLocus::whole(vr)))
}

fn person_node(p: &atlas_core::data::Person) -> Node {
    let id = PersonId::new(p.id.clone()).erase();
    Node {
        id,
        payload: NodePayload::Person {
            label: p.name.clone(),
            gender: p.gender.clone(),
            birth_year: p.birth_year,
            death_year: p.death_year,
            also_called: p.also_called.clone(),
            description: None,
        },
        provenance: ProvenanceId::from(PROVENANCE),
    }
}

#[derive(Debug, Clone, Default)]
pub struct PersonAdapterStats {
    pub person_nodes: usize,
    pub mentions_rows: usize,
}

/// NORMALIZE: one Person node per source record -- called from
/// `pipeline::NormalizePass`, mirrors `event_world::normalize`'s own node
/// construction for Event/Narrative/Place/etc.
pub fn normalize(ctx: &mut BuildCtx) -> PersonAdapterStats {
    let mut stats = PersonAdapterStats::default();
    for p in &ctx.atlas.people {
        let node = person_node(p);
        ctx.graph.nodes.insert(node.id.clone(), node);
        stats.person_nodes += 1;
    }
    stats
}

/// MERGE/ALIAS: `Person.verse_links` -> `mentions` rows -- mirrors
/// `place_adapter::merge_alias`'s own mentions half exactly, substituting
/// `MentionedEntity::Person` for `MentionedEntity::Place`. `verse_links` is
/// already resolved AND canon-sorted at ETL time (`atlas_etl::people::
/// parse_people`'s own doc comment) -- this loop preserves that order row
/// for row, which is what makes the graph's own `mentioned-in` inverse
/// frontier canon-ordered by construction (BiIndex's own inverse
/// projection preserves row-insertion order; see `graph.rs::build_indexes`).
pub fn merge_alias(ctx: &mut BuildCtx) -> PersonAdapterStats {
    let mut stats = PersonAdapterStats::default();
    for p in &ctx.atlas.people {
        let person_id = PersonId::new(p.id.clone());
        for vref in &p.verse_links {
            let Some(locus) = verse_locus(vref) else { continue };
            ctx.graph.mentions.push(Mentions {
                locus,
                entity: MentionedEntity::Person(person_id.clone()),
                provenance: ProvenanceId::from(PROVENANCE),
            });
            stats.mentions_rows += 1;
        }
    }
    stats
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonFidelityViolation(pub String);

impl std::fmt::Display for PersonFidelityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Theographic person adapter fidelity violation: {}", self.0)
    }
}
impl std::error::Error for PersonFidelityViolation {}

/// THE BOUNDARY FIDELITY LAW (module doc comment has the full design):
/// bijection (every source person -> exactly one graph node, by exact id)
/// + mentions completeness (every source person's own resolved
/// `verse_links` count matches its own `mentions` row count in the built
/// graph, exactly). Fail-loud on the FIRST violation found, named
/// precisely -- same convention `fidelity.rs`/`law_check.rs` already
/// establish for every other boundary/cross-adapter law in this crate.
pub fn check_person_fidelity(atlas: &AtlasData, graph: &Graph) -> Result<(), PersonFidelityViolation> {
    // BIJECTION: source count == graph Person-node count, AND every source
    // id resolves to a real node of the right kind (catches a silent id
    // COLLISION, not just a coarse count match -- two source records
    // sharing one id would pass a bare-count check but fail this one).
    let person_node_count = graph.nodes.values().filter(|n| n.id.kind == NodeKind::Person).count();
    if person_node_count != atlas.people.len() {
        return Err(PersonFidelityViolation(format!(
            "bijection: {} source person record(s) but {} Person node(s) in the built graph",
            atlas.people.len(),
            person_node_count
        )));
    }

    for p in &atlas.people {
        let node_id = PersonId::new(p.id.clone()).erase();
        let Some(node) = graph.nodes.get(&node_id) else {
            return Err(PersonFidelityViolation(format!("bijection: source person '{}' has no Person node in the built graph", p.id)));
        };
        if node.id.kind != NodeKind::Person {
            return Err(PersonFidelityViolation(format!("bijection: id '{}' resolves to a {:?} node, not Person", p.id, node.id.kind)));
        }
    }

    // MENTIONS COMPLETENESS: per-person, resolved verse_links count ==
    // mentions rows actually carrying that person as entity, exactly (no
    // silent drop between ETL's own resolution and the graph adapter's own
    // row construction). Deliberately NOT graph-index-derived (that would
    // just be checking `merge_alias` against itself) -- a fresh count over
    // `graph.mentions`'s own row table, the same "count what actually got
    // built" discipline the bijection check above already follows.
    for p in &atlas.people {
        let expected = p.verse_links.len();
        let actual = graph
            .mentions
            .iter()
            .filter(|row| matches!(&row.entity, MentionedEntity::Person(pid) if pid.0 == p.id))
            .count();
        if actual != expected {
            return Err(PersonFidelityViolation(format!(
                "mentions completeness: person '{}' has {} resolved verse_link(s) but {} mentions row(s) in the built graph",
                p.id, expected, actual
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, Person};
    use std::collections::HashMap;

    fn atlas_with_people(people: Vec<Person>) -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.people = people;
        d
    }

    fn person(id: &str, name: &str, verses: &[&str]) -> Person {
        Person {
            id: id.into(),
            name: name.into(),
            gender: Some("Male".into()),
            birth_year: None,
            death_year: None,
            also_called: vec![],
            verse_links: verses.iter().map(|s| s.to_string()).collect(),
            dict_text: None,
        }
    }

    // Matches place_adapter.rs's own test shape exactly: canon/verses/xrefs
    // fixtures declared inline in each test (never a helper RETURNING a
    // BuildCtx, which would need its own borrows to outlive the function).

    #[test]
    fn normalize_builds_one_node_per_person_with_the_widened_payload() {
        let atlas = atlas_with_people(vec![Person {
            id: "aaron_1".into(),
            name: "Aaron".into(),
            gender: Some("Male".into()),
            birth_year: Some(-1575),
            death_year: Some(-1452),
            also_called: vec!["Ahron".into()],
            verse_links: vec![],
            dict_text: None,
        }]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.person_nodes, 1);

        let id = PersonId::new("aaron_1").erase();
        let node = ctx.graph.nodes.get(&id).expect("Aaron's own node must exist");
        assert_eq!(node.provenance, PROVENANCE);
        match &node.payload {
            NodePayload::Person { label, gender, birth_year, death_year, also_called, description } => {
                assert_eq!(label, "Aaron");
                assert_eq!(*description, None, "description stays None until the Easton's adapter fills it");
                assert_eq!(gender.as_deref(), Some("Male"));
                assert_eq!(*birth_year, Some(-1575));
                assert_eq!(*death_year, Some(-1452));
                assert_eq!(also_called, &vec!["Ahron".to_string()]);
            }
            other => panic!("expected NodePayload::Person, got {other:?}"),
        }
    }

    #[test]
    fn merge_alias_builds_one_mentions_row_per_verse_link_in_source_order() {
        let atlas = atlas_with_people(vec![person("moses_1", "Moses", &["EXO.2.10", "EXO.3.1", "DEU.34.5"])]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.mentions_rows, 3);
        let loci: Vec<String> = ctx
            .graph
            .mentions
            .iter()
            .filter(|r| matches!(&r.entity, MentionedEntity::Person(p) if p.0 == "moses_1"))
            .map(|r| crate::legacy::locus_dot_ref(&r.locus).unwrap())
            .collect();
        // Row order preserves the SOURCE verse_links order (already
        // canon-sorted at ETL time) -- see this module's own merge_alias doc
        // comment for why order fidelity here is load-bearing, not cosmetic.
        assert_eq!(loci, vec!["EXO.2.10", "EXO.3.1", "DEU.34.5"]);
    }

    #[test]
    fn an_unparseable_verse_link_is_skipped_not_panicked_on() {
        let atlas = atlas_with_people(vec![person("x_1", "X", &["not-a-verse"])]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.mentions_rows, 0);
    }

    #[test]
    fn fidelity_is_green_when_normalize_and_merge_alias_both_ran_cleanly() {
        let atlas = atlas_with_people(vec![person("moses_1", "Moses", &["EXO.2.10", "EXO.3.1"]), person("aaron_1", "Aaron", &["EXO.4.14"])]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);
        merge_alias(&mut ctx);
        assert!(check_person_fidelity(&atlas, &ctx.graph).is_ok());
    }

    #[test]
    fn fidelity_catches_a_missing_person_node_bijection_violation() {
        let atlas = atlas_with_people(vec![person("moses_1", "Moses", &[])]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas); // normalize() never called -- no Person node exists
        let err = check_person_fidelity(&atlas, &ctx.graph).expect_err("must catch the missing node");
        assert!(err.0.contains("bijection"), "unexpected message: {}", err.0);
    }

    #[test]
    fn fidelity_catches_an_extra_person_node_bijection_violation() {
        let atlas = atlas_with_people(vec![]); // zero source people
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        // Insert a Person node with no corresponding source record at all --
        // simulates a hypothetical future regression (a stray/duplicate
        // insert) this law must still catch, not just the "too few" case.
        let stray = person_node(&person("ghost_1", "Ghost", &[]));
        ctx.graph.nodes.insert(stray.id.clone(), stray);
        let err = check_person_fidelity(&atlas, &ctx.graph).expect_err("must catch the extra node");
        assert!(err.0.contains("bijection"), "unexpected message: {}", err.0);
    }

    #[test]
    fn fidelity_catches_a_mentions_completeness_violation() {
        let atlas = atlas_with_people(vec![person("moses_1", "Moses", &["EXO.2.10", "EXO.3.1"])]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);
        // Deliberately runs merge_alias only PARTIALLY (simulating a silent
        // drop): push just one of the two expected mentions rows by hand,
        // never calling the real merge_alias().
        ctx.graph.mentions.push(Mentions {
            locus: verse_locus("EXO.2.10").unwrap(),
            entity: MentionedEntity::Person(PersonId::new("moses_1")),
            provenance: ProvenanceId::from(PROVENANCE),
        });
        let err = check_person_fidelity(&atlas, &ctx.graph).expect_err("must catch the incomplete mentions rows");
        assert!(err.0.contains("mentions completeness"), "unexpected message: {}", err.0);
    }

    #[test]
    fn referential_integrity_of_person_mentions_rows_is_already_covered_by_the_generic_law() {
        // Disclosed thesis point (module doc comment): this adapter writes
        // NO referential-integrity check of its own -- proves
        // law_check::every_authored_edge_resolves already catches a Person
        // mentions row naming a Person with no graph node, exactly the way
        // it already does for Place (M-C2's own red-then-green test).
        let mut graph = Graph::default();
        graph.mentions.push(Mentions {
            locus: verse_locus("GEN.1.1").unwrap(),
            entity: MentionedEntity::Person(PersonId::new("nowhere")),
            provenance: ProvenanceId::from(PROVENANCE),
        });
        let err = crate::law_check::every_authored_edge_resolves(&graph).expect_err("the EXISTING generic law must catch this -- no new code needed");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }
}
