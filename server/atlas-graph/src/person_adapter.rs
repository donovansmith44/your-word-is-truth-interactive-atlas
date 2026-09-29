//! Theographic persons -> `Person` nodes and `mentions` rows. A record curated as RECLASSIFIED gets
//! no Person node: `peoples_adapter` builds a PeopleGroup node under the same raw slug instead, and
//! because a kind is a fact, no record is ever built as both.

use atlas_core::data::AtlasData;
use atlas_graph_types::edge::{Mentions, MentionedEntity, ParentOf, Participates, Partners};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{EventId, NodeKind, PersonId};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef};

use crate::pipeline::BuildCtx;

/// The provenance tag every Person node and every Person `mentions` row carries: one constant, not
/// two independently-typed literals.
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
            first_year: p.first_year,
            last_year: p.last_year,
            eternal: p.eternal,
            eternal_grounds: p.eternal_grounds.clone(),
        },
        provenance: ProvenanceId::from(PROVENANCE),
    }
}

#[derive(Debug, Clone, Default)]
pub struct PersonAdapterStats {
    pub person_nodes: usize,
    pub mentions_rows: usize,
    /// Kinship and participation rows, and the links skipped because their other end is not a Person
    /// node -- a reclassified people group -- or, for the timeline, not an Event node.
    pub parent_of_rows: usize,
    pub partners_rows: usize,
    pub participates_rows: usize,
    pub kin_links_skipped: usize,
    pub timeline_links_skipped: usize,
}

pub fn normalize(ctx: &mut BuildCtx) -> PersonAdapterStats {
    let mut stats = PersonAdapterStats::default();
    let reclassified = crate::peoples_adapter::reclassified_person_slugs(ctx.atlas);
    for p in &ctx.atlas.people {
        if reclassified.contains(&p.id) {
            continue;
        }
        let node = person_node(p);
        ctx.graph.nodes.insert(node.id.clone(), node);
        stats.person_nodes += 1;
    }
    stats
}

/// `verse_links` is already resolved and canon-sorted at ETL time, and this loop preserves that order
/// row for row, which is what makes the `mentioned-in` inverse frontier canon-ordered by
/// construction.
pub fn merge_alias(ctx: &mut BuildCtx) -> PersonAdapterStats {
    let mut stats = PersonAdapterStats::default();
    let reclassified = crate::peoples_adapter::reclassified_person_slugs(ctx.atlas);
    for p in &ctx.atlas.people {
        if reclassified.contains(&p.id) {
            continue;
        }
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

    // Theographic states each parent link from BOTH ends, so the pairs are collected into ONE ordered
    // set and minted once; partners likewise, ordered a < b. A link whose other end is not a node of
    // the right kind is skipped and counted, never left dangling.
    use std::collections::BTreeSet;
    let is_person = |id: &str| !reclassified.contains(id) && ctx.graph.nodes.contains_key(&PersonId::new(id.to_string()).erase());
    let mut parent_child: BTreeSet<(String, String)> = BTreeSet::new();
    let mut partner_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut participation: Vec<(String, String)> = Vec::new();
    for p in &ctx.atlas.people {
        if reclassified.contains(&p.id) {
            continue;
        }
        for parent in p.father.iter().chain(p.mother.iter()) {
            if is_person(parent) {
                parent_child.insert((parent.clone(), p.id.clone()));
            } else {
                stats.kin_links_skipped += 1;
            }
        }
        for child in &p.children {
            if is_person(child) {
                parent_child.insert((p.id.clone(), child.clone()));
            } else {
                stats.kin_links_skipped += 1;
            }
        }
        for partner in &p.partners {
            if is_person(partner) && *partner != p.id {
                let (a, b) = if p.id < *partner { (p.id.clone(), partner.clone()) } else { (partner.clone(), p.id.clone()) };
                partner_pairs.insert((a, b));
            } else {
                stats.kin_links_skipped += 1;
            }
        }
        for event in &p.timeline {
            if ctx.graph.nodes.contains_key(&EventId::new(event.clone()).erase()) {
                participation.push((p.id.clone(), event.clone()));
            } else {
                stats.timeline_links_skipped += 1;
            }
        }
    }
    for (parent, child) in parent_child {
        ctx.graph.parent_of.push(ParentOf { parent: PersonId::new(parent), child: PersonId::new(child), provenance: ProvenanceId::from(PROVENANCE) });
        stats.parent_of_rows += 1;
    }
    for (a, b) in partner_pairs {
        ctx.graph.partners.push(Partners { a: PersonId::new(a), b: PersonId::new(b), provenance: ProvenanceId::from(PROVENANCE) });
        stats.partners_rows += 1;
    }
    for (person, event) in participation {
        ctx.graph.participates.push(Participates { person: PersonId::new(person), event: EventId::new(event), provenance: ProvenanceId::from(PROVENANCE) });
        stats.participates_rows += 1;
    }
    eprintln!(
        "D5 PERSON KIN/PARTICIPATION: {} parent-of row(s), {} partner-of row(s), {} participates-in row(s); {} kin link(s) and {} timeline link(s) skipped (other end not a Person / Event node)",
        stats.parent_of_rows, stats.partners_rows, stats.participates_rows, stats.kin_links_skipped, stats.timeline_links_skipped
    );
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

/// The boundary fidelity law: every source person becomes exactly one graph node by exact id, and
/// every person's resolved `verse_links` count equals its own `mentions` row count. The bijection is
/// over every source person EXCEPT the curated reclassified slugs, whose complement law is elsewhere.
pub fn check_person_fidelity(atlas: &AtlasData, graph: &Graph) -> Result<(), PersonFidelityViolation> {
    let reclassified = crate::peoples_adapter::reclassified_person_slugs(atlas);

    // Every non-reclassified source id must resolve to a real node of the right kind, which catches a
    // silent id COLLISION that a bare count comparison would pass.
    let person_node_count = graph.nodes.values().filter(|n| n.id.kind == NodeKind::Person).count();
    let expected_person_count = atlas.people.len() - reclassified.len();
    if person_node_count != expected_person_count {
        return Err(PersonFidelityViolation(format!(
            "bijection: {} source person record(s) minus {} reclassified (PG-1a) = {expected_person_count} expected, but {} Person node(s) in the built graph",
            atlas.people.len(),
            reclassified.len(),
            person_node_count
        )));
    }

    for p in &atlas.people {
        if reclassified.contains(&p.id) {
            continue;
        }
        let node_id = PersonId::new(p.id.clone()).erase();
        let Some(node) = graph.nodes.get(&node_id) else {
            return Err(PersonFidelityViolation(format!("bijection: source person '{}' has no Person node in the built graph", p.id)));
        };
        if node.id.kind != NodeKind::Person {
            return Err(PersonFidelityViolation(format!("bijection: id '{}' resolves to a {:?} node, not Person", p.id, node.id.kind)));
        }
    }

    // A fresh count over `graph.mentions`'s own row table rather than the derived index: counting
    // through the index would only check `merge_alias` against itself.
    for p in &atlas.people {
        if reclassified.contains(&p.id) {
            continue;
        }
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
            ..Default::default()
        }
    }

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
            ..Default::default()
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
            NodePayload::Person { label, gender, birth_year, death_year, also_called, description, .. } => {
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
        let ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let err = check_person_fidelity(&atlas, &ctx.graph).expect_err("must catch the missing node");
        assert!(err.0.contains("bijection"), "unexpected message: {}", err.0);
    }

    #[test]
    fn fidelity_catches_an_extra_person_node_bijection_violation() {
        let atlas = atlas_with_people(vec![]);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
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
        let mut graph = Graph::default();
        graph.mentions.push(Mentions {
            locus: verse_locus("GEN.1.1").unwrap(),
            entity: MentionedEntity::Person(PersonId::new("nowhere")),
            provenance: ProvenanceId::from(PROVENANCE),
        });
        let err = crate::law_check::every_row_reference_resolves(&graph).expect_err("the EXISTING generic law must catch this -- no new code needed");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }

    #[test]
    fn normalize_excludes_a_reclassified_slug_from_person_node_construction() {
        let mut atlas = atlas_with_people(vec![person("jebusite_748", "Jebusite", &["GEN.10.16"]), person("aaron_1", "Aaron", &[])]);
        atlas.people_group_reclassify = vec![atlas_core::data::PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }];
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.person_nodes, 1, "only Aaron -- Jebusite is excluded");
        assert!(ctx.graph.nodes.get(&PersonId::new("jebusite_748").erase()).is_none(), "a reclassified slug must get NO Person node");
        assert!(ctx.graph.nodes.get(&PersonId::new("aaron_1").erase()).is_some(), "a non-reclassified person is unaffected");
    }

    #[test]
    fn merge_alias_excludes_a_reclassified_slugs_verse_links_from_person_mentions() {
        let mut atlas = atlas_with_people(vec![person("jebusite_748", "Jebusite", &["GEN.10.16", "1CH.1.14"]), person("aaron_1", "Aaron", &["EXO.4.14"])]);
        atlas.people_group_reclassify = vec![atlas_core::data::PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }];
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.mentions_rows, 1, "only Aaron's own verse -- Jebusite's two verse_links are excluded here");
        for row in &ctx.graph.mentions {
            match &row.entity {
                MentionedEntity::Person(p) => assert_eq!(p.0, "aaron_1"),
                other => panic!("unexpected entity {other:?}"),
            }
        }
    }

    #[test]
    fn fidelity_bijection_accounts_for_reclassified_exclusions() {
        let mut atlas = atlas_with_people(vec![person("jebusite_748", "Jebusite", &["GEN.10.16"]), person("aaron_1", "Aaron", &[])]);
        atlas.people_group_reclassify = vec![atlas_core::data::PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }];
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);
        merge_alias(&mut ctx);
        assert!(check_person_fidelity(&atlas, &ctx.graph).is_ok(), "one Person node (Aaron) for two source records, one reclassified -- must be green, not a false bijection failure");
    }
}
