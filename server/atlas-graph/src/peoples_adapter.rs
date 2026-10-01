//! PeopleGroup nodes from three sources -- imported groups, curated nation seeds, and the curated
//! reclassified person records, which keep their raw slug under the PeopleGroup kind -- plus the
//! curated NamedAfter rows, emitted only where the named eponym resolves to a real Person node.

use std::collections::BTreeSet;

use atlas_core::data::ScriptureGroundSeed;
use atlas_graph_types::edge::{Ground, Justification, MentionedEntity, Mentions, NamedAfter, Namesake};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{NodeKind, PeopleGroupId, PersonId, PlaceId, PolityId};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, TextLocus, VerseRef};

use crate::pipeline::BuildCtx;

/// One provenance tag per PeopleGroup source, so a card's provenance always names which of the three
/// a node came from.
pub const PROVENANCE_THEOGRAPHIC: &str = "theographic-people-groups";
pub const PROVENANCE_CURATED_SEED: &str = "curated-people-groups";
pub const PROVENANCE_RECLASSIFIED: &str = "theographic-people-reclassified";
pub const PROVENANCE_NAMED_AFTER: &str = "curated-named-after";

/// The reclassified slugs as a lookup set: the ONE shared view this module and `person_adapter` both
/// read, so the partition of `atlas.people` cannot drift between them. Curated data, never a
/// hardcoded list.
pub fn reclassified_person_slugs(atlas: &atlas_core::data::AtlasData) -> BTreeSet<String> {
    atlas.people_group_reclassify.iter().map(|r| r.person_slug.clone()).collect()
}

fn verse_locus(vref: &str) -> Option<TextLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    let vr = VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse };
    Some(TextLocus::from(BibleLocus::whole(vr)))
}

/// `pub(crate)` because the fulfillment adapter parses its curated Scripture grounds with this exact
/// parser rather than a third copy of it.
pub(crate) fn ground_locus(vref: &str) -> Option<BibleLocus> {
    atlas_core::refs::VerseId::parse_canonical(vref).ok().map(|vid| vid.locus())
}

/// `to` defaults to `from`, a single-verse ground. `None` on an unparseable verse ref or an inverted
/// range, so a curated typo folds into the caller's omission accounting instead of panicking.
pub(crate) fn ground_range(g: &ScriptureGroundSeed) -> Option<BibleLocusRange> {
    let from = ground_locus(&g.from)?;
    let to = match &g.to {
        Some(t) => ground_locus(t)?,
        None => from.clone(),
    };
    BibleLocusRange::new(from, to).ok()
}

#[derive(Debug, Clone, Default)]
pub struct PeoplesAdapterStats {
    pub theographic_group_nodes: usize,
    pub curated_seed_nodes: usize,
    pub reclassified_nodes: usize,
    pub reclassified_mentions_rows: usize,
    /// Mentions rows built from imported groups' own `verse_links`: general code, any group that
    /// carries verses.
    pub theographic_mentions_rows: usize,
    pub named_after_rows: usize,
    /// `(namesake_id, reason)`: every curated `named_after` row this adapter declined to build.
    pub named_after_omitted: Vec<(String, String)>,
}

pub fn normalize(ctx: &mut BuildCtx) -> PeoplesAdapterStats {
    let mut stats = PeoplesAdapterStats::default();

    // Source (a): nodes only -- membership is not imported.
    for g in &ctx.atlas.people_groups {
        let id = PeopleGroupId::new(g.id.clone()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::PeopleGroup { label: g.label.clone(), description: None }, provenance: ProvenanceId::from(PROVENANCE_THEOGRAPHIC) },
        );
        stats.theographic_group_nodes += 1;
    }

    // Source (b): nodes only -- a curated seed carries no per-locus data.
    for g in &ctx.atlas.people_group_seeds {
        let id = PeopleGroupId::new(g.id.clone()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::PeopleGroup { label: g.label.clone(), description: None }, provenance: ProvenanceId::from(PROVENANCE_CURATED_SEED) },
        );
        stats.curated_seed_nodes += 1;
    }

    // Source (c): the PeopleGroup node keeps the Person record's own raw slug. `person_adapter`
    // already ran in this pass and built no Person node for these ids.
    for r in &ctx.atlas.people_group_reclassify {
        let Some(p) = ctx.atlas.people.iter().find(|p| p.id == r.person_slug) else { continue };
        let id = PeopleGroupId::new(p.id.clone()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::PeopleGroup { label: p.name.clone(), description: None }, provenance: ProvenanceId::from(PROVENANCE_RECLASSIFIED) },
        );
        stats.reclassified_nodes += 1;
    }

    for row in &ctx.atlas.named_after_seeds {
        // The eponym must resolve to a real Person NODE, checked against the graph rather than the
        // raw source list: a curated row naming a reclassified slug would wrongly pass a source-only
        // check, since that raw record still exists.
        let eponym_id = PersonId::new(row.eponym.clone());
        let eponym_node_exists = ctx.graph.nodes.get(&eponym_id.erase()).is_some_and(|n| n.id.kind == NodeKind::Person);
        if !eponym_node_exists {
            stats.named_after_omitted.push((row.namesake_id.clone(), format!("eponym person '{}' has no Person node in the built graph", row.eponym)));
            continue;
        }

        let namesake = match row.namesake_kind.as_str() {
            "people_group" => Namesake::PeopleGroup(PeopleGroupId::new(row.namesake_id.clone())),
            "place" => Namesake::Place(PlaceId::new(row.namesake_id.clone())),
            "polity" => Namesake::Polity(PolityId::new(row.namesake_id.clone())),
            other => {
                stats.named_after_omitted.push((row.namesake_id.clone(), format!("unknown namesake_kind '{other}' (expected people_group/place/polity)")));
                continue;
            }
        };

        let mut grounds: BTreeSet<Ground> = BTreeSet::new();
        let mut all_parsed = true;
        for g in &row.grounds {
            match ground_range(g) {
                Some(range) => {
                    grounds.insert(Ground::Scripture(range));
                }
                None => {
                    all_parsed = false;
                    break;
                }
            }
        }
        if !all_parsed {
            stats.named_after_omitted.push((row.namesake_id.clone(), "one or more curated scripture ground(s) failed to parse (bad verse ref, or an inverted range)".to_string()));
            continue;
        }
        if grounds.is_empty() {
            stats.named_after_omitted.push((row.namesake_id.clone(), "no scripture ground(s) at all -- a Justification needs at least one".to_string()));
            continue;
        }

        ctx.graph.named_after.push(NamedAfter {
            namesake,
            eponym: eponym_id,
            provenance: ProvenanceId::from(PROVENANCE_NAMED_AFTER),
            justification: Justification { text: row.text.clone(), grounds },
        });
        stats.named_after_rows += 1;
    }

    stats
}

/// Reclassified persons' `verse_links` become `Mentions(PeopleGroup)` rows, and so do any imported
/// group's own `verse_links` -- general code over the whole list, never a hardcoded pair of names.
/// The curated nation seeds build no mentions rows at all: they carry no per-locus data.
pub fn merge_alias(ctx: &mut BuildCtx) -> PeoplesAdapterStats {
    let mut stats = PeoplesAdapterStats::default();
    let reclass = reclassified_person_slugs(ctx.atlas);
    for p in &ctx.atlas.people {
        if !reclass.contains(&p.id) {
            continue;
        }
        let group_id = PeopleGroupId::new(p.id.clone());
        for vref in &p.verse_links {
            let Some(locus) = verse_locus(vref) else { continue };
            ctx.graph.mentions.push(Mentions { locus, entity: MentionedEntity::PeopleGroup(group_id.clone()), provenance: ProvenanceId::from(PROVENANCE_RECLASSIFIED) });
            stats.reclassified_mentions_rows += 1;
        }
    }

    for g in &ctx.atlas.people_groups {
        if g.verse_links.is_empty() {
            continue;
        }
        let group_id = PeopleGroupId::new(g.id.clone());
        for vref in &g.verse_links {
            let Some(locus) = verse_locus(vref) else { continue };
            ctx.graph.mentions.push(Mentions { locus, entity: MentionedEntity::PeopleGroup(group_id.clone()), provenance: ProvenanceId::from(PROVENANCE_THEOGRAPHIC) });
            stats.theographic_mentions_rows += 1;
        }
    }

    stats
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeoplesFidelityViolation(pub String);

impl std::fmt::Display for PeoplesFidelityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PG-1a peoples adapter fidelity violation: {}", self.0)
    }
}
impl std::error::Error for PeoplesFidelityViolation {}

/// The boundary fidelity law: every source record of all three sources becomes exactly one
/// PeopleGroup node by exact id and a reclassified slug carries no Person node, the total count
/// matches, and the mentions rows are complete. Fail-loud on the first violation, named precisely.
pub fn check_peoples_fidelity(atlas: &atlas_core::data::AtlasData, graph: &Graph) -> Result<(), PeoplesFidelityViolation> {
    for g in &atlas.people_groups {
        let id = PeopleGroupId::new(g.id.clone()).erase();
        let Some(node) = graph.nodes.get(&id) else {
            return Err(PeoplesFidelityViolation(format!("bijection: Theographic people-group '{}' ({}) has no PeopleGroup node in the built graph", g.id, g.label)));
        };
        if node.id.kind != NodeKind::PeopleGroup {
            return Err(PeoplesFidelityViolation(format!("bijection: id '{}' resolves to a {:?} node, not PeopleGroup", g.id, node.id.kind)));
        }
    }
    for g in &atlas.people_group_seeds {
        let id = PeopleGroupId::new(g.id.clone()).erase();
        let Some(node) = graph.nodes.get(&id) else {
            return Err(PeoplesFidelityViolation(format!("bijection: curated nation seed '{}' ({}) has no PeopleGroup node in the built graph", g.id, g.label)));
        };
        if node.id.kind != NodeKind::PeopleGroup {
            return Err(PeoplesFidelityViolation(format!("bijection: id '{}' resolves to a {:?} node, not PeopleGroup", g.id, node.id.kind)));
        }
    }
    for r in &atlas.people_group_reclassify {
        let id = PeopleGroupId::new(r.person_slug.clone()).erase();
        let Some(node) = graph.nodes.get(&id) else {
            return Err(PeoplesFidelityViolation(format!("bijection: reclassified slug '{}' has no PeopleGroup node in the built graph", r.person_slug)));
        };
        if node.id.kind != NodeKind::PeopleGroup {
            return Err(PeoplesFidelityViolation(format!("bijection: reclassified id '{}' resolves to a {:?} node, not PeopleGroup", r.person_slug, node.id.kind)));
        }
        let person_id = PersonId::new(r.person_slug.clone()).erase();
        if graph.nodes.contains_key(&person_id) {
            return Err(PeoplesFidelityViolation(format!("reclassified slug '{}' carries BOTH a Person node and a PeopleGroup node -- must be exactly one kind", r.person_slug)));
        }
    }

    // The total count catches a stray or duplicate insert that the per-source loops -- each of which
    // only ever checks that its own records are all present -- would miss.
    let expected_total = atlas.people_groups.len() + atlas.people_group_seeds.len() + atlas.people_group_reclassify.len();
    let actual_total = graph.nodes.values().filter(|n| n.id.kind == NodeKind::PeopleGroup).count();
    if actual_total != expected_total {
        return Err(PeoplesFidelityViolation(format!(
            "bijection: expected exactly {expected_total} PeopleGroup node(s) ({} Theographic + {} curated seed(s) + {} reclassified) but the built graph carries {actual_total}",
            atlas.people_groups.len(),
            atlas.people_group_seeds.len(),
            atlas.people_group_reclassify.len()
        )));
    }

    // Mentions completeness for the reclassified subset, counted fresh over `graph.mentions`'s own
    // row table rather than through the derived index.
    for r in &atlas.people_group_reclassify {
        let Some(p) = atlas.people.iter().find(|p| p.id == r.person_slug) else {
            return Err(PeoplesFidelityViolation(format!("reclassified slug '{}' names no record in the compiled Theographic person set at all", r.person_slug)));
        };
        let expected = p.verse_links.len();
        let actual = graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(g) if g.0 == r.person_slug)).count();
        if actual != expected {
            return Err(PeoplesFidelityViolation(format!(
                "mentions completeness: reclassified group '{}' has {} resolved verse_link(s) but {} PeopleGroup mentions row(s) in the built graph",
                r.person_slug, expected, actual
            )));
        }
    }

    // The same discipline over imported groups' `verse_links`, as general code, so a future refresh
    // that gives a third group verses is caught here rather than silently under-served.
    for g in &atlas.people_groups {
        let expected = g.verse_links.len();
        let actual = graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(pg) if pg.0 == g.id)).count();
        if actual != expected {
            return Err(PeoplesFidelityViolation(format!(
                "mentions completeness: Theographic group '{}' ({}) has {} resolved verse_link(s) but {} PeopleGroup mentions row(s) in the built graph",
                g.id, g.label, expected, actual
            )));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAfterGroundingViolation(pub String);

impl std::fmt::Display for NamedAfterGroundingViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PG-1a named-after grounding violation: {}", self.0)
    }
}
impl std::error::Error for NamedAfterGroundingViolation {}

/// A FRESH check over the built graph's own `named_after` table: every row must carry at least one
/// `Ground::Scripture`, since an anchor-only or empty-grounds row is the "labeled but not grounded"
/// shape this table exists to avoid.
pub fn every_named_after_row_has_a_scripture_ground(graph: &Graph) -> Result<(), NamedAfterGroundingViolation> {
    for row in &graph.named_after {
        let has_scripture_ground = row.justification.grounds.iter().any(|g| matches!(g, Ground::Scripture(_)));
        if !has_scripture_ground {
            return Err(NamedAfterGroundingViolation(format!(
                "named_after row (eponym '{}') carries no Ground::Scripture in its own justification -- {} ground(s) total",
                row.eponym.0,
                row.justification.grounds.len()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, NamedAfterSeed, PeopleGroup, PeopleGroupReclassify, PeopleGroupSeed, Person};
    use std::collections::HashMap;

    fn atlas_with(
        people: Vec<Person>,
        people_groups: Vec<PeopleGroup>,
        people_group_seeds: Vec<PeopleGroupSeed>,
        people_group_reclassify: Vec<PeopleGroupReclassify>,
        named_after_seeds: Vec<NamedAfterSeed>,
    ) -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.people = people;
        d.people_groups = people_groups;
        d.people_group_seeds = people_group_seeds;
        d.people_group_reclassify = people_group_reclassify;
        d.named_after_seeds = named_after_seeds;
        d
    }

    fn person(id: &str, name: &str, verses: &[&str]) -> Person {
        Person { id: id.into(), name: name.into(), gender: None, birth_year: None, death_year: None, also_called: vec![], verse_links: verses.iter().map(|s| s.to_string()).collect(), dict_text: None, ..Default::default() }
    }

    fn ground(from: &str) -> ScriptureGroundSeed {
        ScriptureGroundSeed { from: from.into(), to: None }
    }

    fn ctx_with<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData) -> BuildCtx<'a> {
        BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas)
    }

    #[test]
    fn normalize_builds_one_node_per_theographic_group_and_curated_seed() {
        let atlas = atlas_with(
            vec![],
            vec![PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec![] }],
            vec![PeopleGroupSeed { id: "ammonites".into(), label: "Ammonites".into() }],
            vec![],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.theographic_group_nodes, 1);
        assert_eq!(stats.curated_seed_nodes, 1);

        let tribe = ctx.graph.nodes.get(&PeopleGroupId::new("tribe-of-judah").erase()).expect("Theographic group node must exist");
        match &tribe.payload {
            NodePayload::PeopleGroup { label, description } => {
                assert_eq!(label, "Tribe of Judah");
                assert!(description.is_none());
            }
            other => panic!("expected PeopleGroup, got {other:?}"),
        }
        assert_eq!(tribe.provenance, PROVENANCE_THEOGRAPHIC);

        let ammon = ctx.graph.nodes.get(&PeopleGroupId::new("ammonites").erase()).expect("curated seed node must exist");
        assert_eq!(ammon.provenance, PROVENANCE_CURATED_SEED);
    }

    #[test]
    fn normalize_reclassifies_a_person_into_a_peoplegroup_node_never_both() {
        let atlas = atlas_with(
            vec![person("jebusite_748", "Jebusite", &["GEN.10.16"])],
            vec![],
            vec![],
            vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "Gen-10 gentilic collective".into() }],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.reclassified_nodes, 1);

        let group_id = PeopleGroupId::new("jebusite_748").erase();
        let node = ctx.graph.nodes.get(&group_id).expect("reclassified PeopleGroup node must exist under the SAME raw slug");
        assert_eq!(node.id.kind, NodeKind::PeopleGroup);
        assert_eq!(node.provenance, PROVENANCE_RECLASSIFIED);
        match &node.payload {
            NodePayload::PeopleGroup { label, .. } => assert_eq!(label, "Jebusite"),
            other => panic!("expected PeopleGroup, got {other:?}"),
        }

        let person_id = PersonId::new("jebusite_748").erase();
        assert!(ctx.graph.nodes.get(&person_id).is_none(), "peoples_adapter itself never builds a Person node -- person_adapter's own exclusion is what keeps this true in a real build");
    }

    #[test]
    fn a_reclassify_row_naming_no_real_person_record_is_skipped_not_panicked_on() {
        let atlas = atlas_with(vec![], vec![], vec![], vec![PeopleGroupReclassify { person_slug: "ghost_1".into(), reason: "test".into() }], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.reclassified_nodes, 0);
    }

    #[test]
    fn merge_alias_builds_peoplegroup_mentions_only_for_reclassified_persons() {
        let atlas = atlas_with(
            vec![person("jebusite_748", "Jebusite", &["GEN.10.16", "1CH.1.14"]), person("aaron_1", "Aaron", &["EXO.4.14"])],
            vec![],
            vec![],
            vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.reclassified_mentions_rows, 2, "only the RECLASSIFIED person's own two verse_links become mentions rows -- Aaron's own verse is untouched by this adapter");
        for row in &ctx.graph.mentions {
            match &row.entity {
                MentionedEntity::PeopleGroup(g) => assert_eq!(g.0, "jebusite_748"),
                other => panic!("expected only PeopleGroup mentions from this adapter, got {other:?}"),
            }
        }
    }

    #[test]
    fn merge_alias_builds_no_mentions_for_a_verseless_theographic_group_or_any_curated_seed() {
        let atlas = atlas_with(vec![], vec![PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec![] }], vec![PeopleGroupSeed { id: "ammonites".into(), label: "Ammonites".into() }], vec![], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.reclassified_mentions_rows, 0);
        assert_eq!(stats.theographic_mentions_rows, 0);
        assert!(ctx.graph.mentions.is_empty());
    }

    #[test]
    fn merge_alias_builds_mentions_for_any_theographic_group_carrying_verse_links() {
        let atlas = atlas_with(
            vec![],
            vec![
                PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec!["PRO.25.1".into()] },
                PeopleGroup { id: "some-other-group".into(), label: "Some Other Group".into(), verse_links: vec!["GEN.1.1".into(), "GEN.1.2".into()] },
                PeopleGroup { id: "no-verses-group".into(), label: "No Verses Group".into(), verse_links: vec![] },
            ],
            vec![],
            vec![],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.theographic_mentions_rows, 3, "1 (tribe-of-judah) + 2 (some-other-group) + 0 (no-verses-group)");
        assert_eq!(stats.reclassified_mentions_rows, 0);
        assert_eq!(ctx.graph.mentions.len(), 3);

        for row in &ctx.graph.mentions {
            assert_eq!(row.provenance, PROVENANCE_THEOGRAPHIC);
            match &row.entity {
                MentionedEntity::PeopleGroup(g) => assert!(g.0 == "tribe-of-judah" || g.0 == "some-other-group", "unexpected entity: {}", g.0),
                other => panic!("expected only PeopleGroup mentions from this adapter, got {other:?}"),
            }
        }

        let judah_locus = ctx
            .graph
            .mentions
            .iter()
            .find(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(g) if g.0 == "tribe-of-judah"))
            .map(|row| row.locus.clone())
            .expect("a tribe-of-judah mention must exist");
        assert_eq!(judah_locus, verse_locus("PRO.25.1").unwrap());
    }

    fn person_node(ctx: &mut BuildCtx, slug: &str, label: &str) {
        let id = PersonId::new(slug).erase();
        ctx.graph.nodes.insert(id.clone(), Node { id, payload: NodePayload::Person { label: label.into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: ProvenanceId::from("test") });
    }

    #[test]
    fn named_after_row_builds_when_the_eponym_person_node_exists() {
        let atlas = atlas_with(
            vec![],
            vec![],
            vec![PeopleGroupSeed { id: "ammonites".into(), label: "Ammonites".into() }],
            vec![],
            vec![NamedAfterSeed { namesake_kind: "people_group".into(), namesake_id: "ammonites".into(), eponym: "ben-ammi_451".into(), text: Some("test text".into()), grounds: vec![ground("GEN.19.38")] }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        person_node(&mut ctx, "ben-ammi_451", "Ben-ammi");

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 1);
        assert!(stats.named_after_omitted.is_empty());

        let row = &ctx.graph.named_after[0];
        assert_eq!(row.eponym.0, "ben-ammi_451");
        match &row.namesake {
            Namesake::PeopleGroup(g) => assert_eq!(g.0, "ammonites"),
            other => panic!("expected Namesake::PeopleGroup, got {other:?}"),
        }
        assert_eq!(row.justification.text.as_deref(), Some("test text"));
        assert_eq!(row.justification.grounds.len(), 1);
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(_)));
    }

    #[test]
    fn named_after_row_is_omitted_and_reported_when_the_eponym_has_no_person_node() {
        let atlas = atlas_with(
            vec![],
            vec![],
            vec![PeopleGroupSeed { id: "philistines".into(), label: "Philistines".into() }],
            vec![],
            vec![NamedAfterSeed { namesake_kind: "people_group".into(), namesake_id: "philistines".into(), eponym: "casluhim_nowhere".into(), text: None, grounds: vec![ground("GEN.10.14")] }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 0);
        assert_eq!(stats.named_after_omitted.len(), 1);
        assert_eq!(stats.named_after_omitted[0].0, "philistines");
        assert!(stats.named_after_omitted[0].1.contains("casluhim_nowhere"), "{}", stats.named_after_omitted[0].1);
        assert!(ctx.graph.named_after.is_empty());
    }

    #[test]
    fn a_reclassified_slug_can_never_satisfy_a_named_after_eponym_check() {
        let atlas = atlas_with(
            vec![person("jebusite_748", "Jebusite", &[])],
            vec![],
            vec![PeopleGroupSeed { id: "somegroup".into(), label: "Somegroup".into() }],
            vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }],
            vec![NamedAfterSeed { namesake_kind: "people_group".into(), namesake_id: "somegroup".into(), eponym: "jebusite_748".into(), text: None, grounds: vec![ground("GEN.10.16")] }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 0);
        assert_eq!(stats.named_after_omitted.len(), 1);
    }

    #[test]
    fn named_after_row_is_omitted_when_a_ground_is_unparseable() {
        let atlas = atlas_with(
            vec![],
            vec![],
            vec![PeopleGroupSeed { id: "somegroup".into(), label: "Somegroup".into() }],
            vec![],
            vec![NamedAfterSeed { namesake_kind: "people_group".into(), namesake_id: "somegroup".into(), eponym: "eponym_1".into(), text: None, grounds: vec![ground("not-a-verse")] }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        person_node(&mut ctx, "eponym_1", "Eponym");

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 0);
        assert_eq!(stats.named_after_omitted.len(), 1);
    }

    #[test]
    fn named_after_row_supports_a_two_ground_multi_range_justification() {
        let atlas = atlas_with(
            vec![],
            vec![],
            vec![PeopleGroupSeed { id: "edomites".into(), label: "Edomites".into() }],
            vec![],
            vec![NamedAfterSeed {
                namesake_kind: "people_group".into(),
                namesake_id: "edomites".into(),
                eponym: "esau_1216".into(),
                text: None,
                grounds: vec![ScriptureGroundSeed { from: "GEN.36.8".into(), to: Some("GEN.36.9".into()) }, ground("GEN.25.30")],
            }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        person_node(&mut ctx, "esau_1216", "Esau");

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 1);
        assert_eq!(ctx.graph.named_after[0].justification.grounds.len(), 2, "both grounds must survive onto the row's own justification");
    }

    #[test]
    fn namesake_kind_place_and_polity_are_accepted_for_schema_completeness() {
        let atlas = atlas_with(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![
                NamedAfterSeed { namesake_kind: "place".into(), namesake_id: "some-place".into(), eponym: "e1".into(), text: None, grounds: vec![ground("GEN.1.1")] },
                NamedAfterSeed { namesake_kind: "polity".into(), namesake_id: "some-polity".into(), eponym: "e2".into(), text: None, grounds: vec![ground("GEN.1.1")] },
            ],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        person_node(&mut ctx, "e1", "E1");
        person_node(&mut ctx, "e2", "E2");

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 2);
        assert!(matches!(ctx.graph.named_after[0].namesake, Namesake::Place(_)));
        assert!(matches!(ctx.graph.named_after[1].namesake, Namesake::Polity(_)));
    }

    #[test]
    fn fidelity_is_green_over_a_clean_three_source_build() {
        let atlas = atlas_with(
            vec![person("jebusite_748", "Jebusite", &["GEN.10.16"])],
            vec![PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec![] }],
            vec![PeopleGroupSeed { id: "ammonites".into(), label: "Ammonites".into() }],
            vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx);
        merge_alias(&mut ctx);
        assert!(check_peoples_fidelity(&atlas, &ctx.graph).is_ok());
    }

    #[test]
    fn fidelity_catches_a_missing_theographic_group_node() {
        let atlas = atlas_with(vec![], vec![PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec![] }], vec![], vec![], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let ctx = ctx_with(&canon, &verses, &atlas);
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the missing node");
        assert!(err.0.contains("bijection"), "{}", err.0);
    }

    #[test]
    fn fidelity_catches_a_reclassified_slug_carrying_both_kinds_at_once() {
        let atlas = atlas_with(vec![person("jebusite_748", "Jebusite", &[])], vec![], vec![], vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx);
        person_node(&mut ctx, "jebusite_748", "Jebusite");

        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the dual-kind regression");
        assert!(err.0.contains("BOTH"), "{}", err.0);
    }

    #[test]
    fn fidelity_catches_an_extra_stray_peoplegroup_node() {
        let atlas = atlas_with(vec![], vec![], vec![], vec![], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stray_id = PeopleGroupId::new("ghost").erase();
        ctx.graph.nodes.insert(stray_id.clone(), Node { id: stray_id, payload: NodePayload::PeopleGroup { label: "Ghost".into(), description: None }, provenance: ProvenanceId::from("test") });
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the extra node");
        assert!(err.0.contains("bijection"), "{}", err.0);
    }

    #[test]
    fn fidelity_catches_a_mentions_completeness_violation() {
        let atlas = atlas_with(vec![person("jebusite_748", "Jebusite", &["GEN.10.16", "1CH.1.14"])], vec![], vec![], vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx);
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the incomplete mentions rows");
        assert!(err.0.contains("mentions completeness"), "{}", err.0);
    }

    #[test]
    fn fidelity_catches_a_theographic_group_mentions_completeness_violation() {
        let atlas = atlas_with(
            vec![],
            vec![PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec!["PRO.25.1".into()] }],
            vec![],
            vec![],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx);
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the incomplete mentions rows");
        assert!(err.0.contains("mentions completeness"), "{}", err.0);
        assert!(err.0.contains("tribe-of-judah"), "{}", err.0);
    }

    #[test]
    fn referential_integrity_of_peoplegroup_mentions_is_already_covered_by_the_generic_law() {
        let mut graph = Graph::default();
        graph.mentions.push(Mentions { locus: verse_locus("GEN.1.1").unwrap(), entity: MentionedEntity::PeopleGroup(PeopleGroupId::new("nowhere")), provenance: ProvenanceId::from("test") });
        let err = crate::law_check::every_row_reference_resolves(&graph).expect_err("the EXISTING generic law must catch this -- no new code needed");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }

    #[test]
    fn every_named_after_row_has_a_scripture_ground_is_green_when_true() {
        let mut graph = Graph::default();
        graph.named_after.push(NamedAfter {
            namesake: Namesake::PeopleGroup(PeopleGroupId::new("ammonites")),
            eponym: PersonId::new("ben-ammi_451"),
            provenance: ProvenanceId::from("test"),
            justification: Justification { text: None, grounds: BTreeSet::from([Ground::Scripture(ground_range(&ground("GEN.19.38")).unwrap())]) },
        });
        assert!(every_named_after_row_has_a_scripture_ground(&graph).is_ok());
    }

    #[test]
    fn every_named_after_row_has_a_scripture_ground_catches_an_empty_grounds_row() {
        let mut graph = Graph::default();
        graph.named_after.push(NamedAfter { namesake: Namesake::PeopleGroup(PeopleGroupId::new("x")), eponym: PersonId::new("y"), provenance: ProvenanceId::from("test"), justification: Justification::default() });
        let err = every_named_after_row_has_a_scripture_ground(&graph).expect_err("must catch a row with zero grounds");
        assert!(err.0.contains("y"), "{}", err.0);
    }

    #[test]
    fn every_named_after_row_has_a_scripture_ground_catches_a_non_scripture_only_row() {
        let mut graph = Graph::default();
        graph.named_after.push(NamedAfter {
            namesake: Namesake::PeopleGroup(PeopleGroupId::new("x")),
            eponym: PersonId::new("y"),
            provenance: ProvenanceId::from("test"),
            justification: Justification { text: None, grounds: BTreeSet::from([Ground::Source(atlas_graph_types::id::SourceId::new("some-source"))]) },
        });
        let err = every_named_after_row_has_a_scripture_ground(&graph).expect_err("a Source-only ground must not satisfy this law");
        assert!(err.0.contains("y"), "{}", err.0);
    }
}
