//! PG-1a ("People groups & eponymy: the data half" -- batch-pg1a-brief.md,
//! owner orders 2026-08-23: "we need a way to distinguish between the
//! names of the twelve tribes and the people theyre named after and other
//! sorts of cases" / "pull in Peoples or Nations info so I can find out
//! who the ammonites are or whatever"): PeopleGroup nodes from THREE
//! sources (controller decision 1) + curated NamedAfter eponymy rows
//! (decision 3), the SAME two-phase NORMALIZE/MERGE-ALIAS shape
//! `person_adapter.rs`/`place_adapter.rs` already established.
//!
//! THREE PeopleGroup SOURCES, ONE node table:
//! (a) the 23 Theographic `peopleGroups.json` records (`ctx.atlas.
//!     people_groups`) -- NODES only; membership (`members`/`events_dev`)
//!     is NOT imported (decision 1a).
//! (b) six CURATED nation seeds (`ctx.atlas.people_group_seeds`, decision
//!     1b): Ammonites, Moabites, Edomites, Philistines, Amalekites,
//!     Canaanites.
//! (c) NINE Theographic PERSON records RECLASSIFIED as PeopleGroup
//!     (`ctx.atlas.people_group_reclassify`, decision 1c, the closed
//!     nine-slug curated list): Amorite/Arkite/Arvadite/Girgasite/
//!     Hamathite/Hivite/Jebusite/Sinite/Zemarite. Each keeps its EXISTING
//!     raw slug as its PeopleGroupId (same string, PeopleGroup kind
//!     instead of Person -- `AnyNodeId`'s own `{kind, raw}` shape makes
//!     the two ids distinct node-store keys even though the raw string is
//!     identical); its EXISTING resolved `verse_links` become
//!     `Mentions(PeopleGroup)` rows here (decision 1c: "the only
//!     per-locus group attestations the source actually ships").
//!     `person_adapter.rs` is the complementary half: [`reclassified_person_slugs`]
//!     is the ONE shared view both adapters read, so `ctx.atlas.people`
//!     partitions between "becomes a Person node" and "becomes a
//!     PeopleGroup node" without the two ever drifting out of sync -- no
//!     record is ever built as BOTH (a kind is a fact, not a choice a
//!     client makes at read time).
//!
//! NO INVENTED PER-LOCUS SENSES (decision 2), QUALIFIED by the PG-1B rider
//! (batch-edge1a-brief.md decision 0): (a)/(b) above get NODES ONLY by
//! DEFAULT -- no source attests which loci mean the tribe/nation vs. the
//! man/land for most of (a)'s own 23 records, and NONE of (b)'s six
//! curated seeds carry any per-locus data at all (the JDG 1:2 problem
//! stays an OPEN OWNER QUESTION for those, ledgered in progress.md, not
//! shipped by default). The PG-1a review CORRECTED that premise for a
//! real subset of (a): 2 of the 23 Theographic `peopleGroups.json` records
//! (Tribe of Judah, Nation of Israel) DO carry a genuine, reciprocally-
//! linked `verses` field (`atlas_etl::people_groups::parse_people_groups`
//! now resolves it into `PeopleGroup.verse_links`) -- these 13 loci ARE
//! source-attested, so `merge_alias` below builds real Mentions rows for
//! them, general code over ANY (a)-source group carrying a non-empty
//! `verse_links` (not a hardcoded two-row special case: a future
//! Theographic data refresh adding a `verses` field to a THIRD group would
//! be picked up automatically, no code change). (b)'s curated seeds still
//! carry zero per-locus data and so still build NO mentions rows -- that
//! half of decision 2 is unchanged. The Sin-guard principle stands: links
//! point where data attests, never where a string guesses.
//!
//! NAMEDAFTER (decision 3): curated `ctx.atlas.named_after_seeds` rows,
//! each naming a namesake (PeopleGroup this batch, always) + an eponym
//! `PersonId` + a real `Justification` (prose + >=1 `Ground::Scripture`).
//! Built here in NORMALIZE, AFTER `person_adapter::normalize` has already
//! run in the SAME pass (`pipeline.rs`'s own `NormalizePass::run` call
//! order) -- curated ids are otherwise already fully resolved at ETL time
//! (no OTHER pass's OWN output is needed), the same NORMALIZE-eligibility
//! `pipeline.rs`'s own doc comment states for `event_world::populate`'s
//! witness-derived rows; the one real dependency is on person_adapter's
//! own node-building having already run, so the eponym-existence check
//! below can ask the GRAPH itself, literally, rather than re-deriving
//! "would person_adapter have built this" from the raw source a second
//! time. Philistines/Amalekites/Canaanites (decision 3's own
//! conditional): a row is emitted ONLY when the named eponym slug
//! resolves to a REAL Person node in the built graph -- never a forced
//! edge the graph can't ground; `PeoplesAdapterStats::named_after_omitted`
//! names every curated row skipped this way, with its own reason.

use std::collections::BTreeSet;

use atlas_core::data::ScriptureGroundSeed;
use atlas_graph_types::edge::{Ground, Justification, MentionedEntity, Mentions, NamedAfter, Namesake};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{NodeKind, PeopleGroupId, PersonId, PlaceId, PolityId};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, TextLocus, VerseRef};

use crate::pipeline::BuildCtx;

/// Provenance tags -- one per PeopleGroup source, so a card's own
/// provenance string always names WHICH of the three this batch's own
/// controller decision 1 ships a node came from.
pub const PROVENANCE_THEOGRAPHIC: &str = "theographic-people-groups";
pub const PROVENANCE_CURATED_SEED: &str = "curated-people-groups";
pub const PROVENANCE_RECLASSIFIED: &str = "theographic-people-reclassified";
pub const PROVENANCE_NAMED_AFTER: &str = "curated-named-after";

/// The nine reclassified slugs (decision 1c), as a lookup set -- the ONE
/// shared view `person_adapter.rs` (to EXCLUDE these ids from its own
/// Person-node/mentions construction) and this module (to build their
/// PeopleGroup nodes/mentions) both read, so the partition of `atlas.
/// people` can never drift between the two call sites. Reads curated
/// data (`atlas.people_group_reclassify`), never a hardcoded list of its
/// own (decision 1c: "the reclassification list is CURATED DATA... not
/// code constants").
pub fn reclassified_person_slugs(atlas: &atlas_core::data::AtlasData) -> BTreeSet<String> {
    atlas.people_group_reclassify.iter().map(|r| r.person_slug.clone()).collect()
}

fn verse_locus(vref: &str) -> Option<TextLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    let vr = VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse };
    Some(TextLocus::from(BibleLocus::whole(vr)))
}

/// `pub(crate)`: EDGE-1a's own `fulfillment_adapter.rs` reuses this exact
/// verse-ref-string -> `BibleLocus` parser for `FulfillmentSeed`/
/// `TypologySeed`'s own `ScriptureGroundSeed`-shaped endpoints, rather than
/// duplicating the `VerseId::parse_canonical` call a third time -- unlike
/// `people.rs`/`people_groups.rs`'s own deliberately-independent
/// `verse_osis_by_id` copies (different reason: those two run at different
/// ETL pipeline points over different raw sources), this is the exact same
/// small, pure, source-agnostic string parser both adapters need, with
/// nothing to keep independent.
pub(crate) fn ground_locus(vref: &str) -> Option<BibleLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    Some(BibleLocus::whole(VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse }))
}

/// Parses one curated `{from, to?}` ground row into a real
/// `BibleLocusRange` -- `to` defaults to `from` (a single-verse ground);
/// `None` on an unparseable verse ref or an inverted range (`to < from`),
/// letting the caller fold that into its own omission/skip accounting
/// rather than panicking on a curated-data typo. `pub(crate)`: see
/// `ground_locus`'s own doc comment immediately above.
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
    /// PG-1B rider: Mentions rows built from source (a) Theographic
    /// groups' own `verse_links` (general code -- any group with verses;
    /// 13 in the real committed data, across the 2 of 23 groups that carry
    /// any).
    pub theographic_mentions_rows: usize,
    pub named_after_rows: usize,
    /// `(namesake_id, reason)` -- every curated `[[named_after]]` row this
    /// adapter declined to build, with why (decision 3's own "report which
    /// were omitted and why").
    pub named_after_omitted: Vec<(String, String)>,
}

/// NORMALIZE: PeopleGroup nodes from all three sources, plus the curated
/// NamedAfter rows -- module doc comment above has the full ordering/
/// eponym-existence reasoning. Called from `pipeline::NormalizePass`,
/// AFTER `person_adapter::normalize`.
pub fn normalize(ctx: &mut BuildCtx) -> PeoplesAdapterStats {
    let mut stats = PeoplesAdapterStats::default();

    // (a) Theographic groups -- nodes only (decision 1a).
    for g in &ctx.atlas.people_groups {
        let id = PeopleGroupId::new(g.id.clone()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::PeopleGroup { label: g.label.clone(), description: None }, provenance: ProvenanceId::from(PROVENANCE_THEOGRAPHIC) },
        );
        stats.theographic_group_nodes += 1;
    }

    // (b) curated nation seeds -- nodes only (decision 1b).
    for g in &ctx.atlas.people_group_seeds {
        let id = PeopleGroupId::new(g.id.clone()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::PeopleGroup { label: g.label.clone(), description: None }, provenance: ProvenanceId::from(PROVENANCE_CURATED_SEED) },
        );
        stats.curated_seed_nodes += 1;
    }

    // (c) reclassified persons -- PeopleGroup nodes, SAME raw slug as the
    // Person record they re-home (decision 1c). `person_adapter::normalize`
    // (already run, earlier in this SAME pass) never built a Person node
    // for these ids -- see that module's own doc comment.
    for r in &ctx.atlas.people_group_reclassify {
        let Some(p) = ctx.atlas.people.iter().find(|p| p.id == r.person_slug) else { continue };
        let id = PeopleGroupId::new(p.id.clone()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::PeopleGroup { label: p.name.clone(), description: None }, provenance: ProvenanceId::from(PROVENANCE_RECLASSIFIED) },
        );
        stats.reclassified_nodes += 1;
    }

    // NamedAfter eponymy rows (decision 3).
    for row in &ctx.atlas.named_after_seeds {
        // "add NamedAfter ONLY where the eponym person exists as a node"
        // (decision 3, verbatim) -- checked against the GRAPH itself
        // (person_adapter::normalize already ran earlier in this pass),
        // not the raw source list: this is both the literal reading of
        // the decision AND the more robust check -- a curated row naming
        // a RECLASSIFIED slug as its own eponym (never true in this
        // batch's own seed data, but a real future-mistake class) would
        // wrongly pass an `atlas.people`-only check (the raw record is
        // still there) while correctly failing THIS one (no Person node
        // exists for it any more).
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

/// MERGE/ALIAS: reclassified persons' `verse_links` -> `Mentions(PeopleGroup)`
/// rows -- mirrors `person_adapter::merge_alias` almost line for line,
/// substituting `MentionedEntity::PeopleGroup` for `MentionedEntity::
/// Person` and reading only the reclassified subset of `ctx.atlas.people`.
/// PG-1B rider: source (a) Theographic groups carrying a non-empty
/// `verse_links` ALSO build real Mentions rows now (module doc comment's
/// own "QUALIFIED by the PG-1B rider" paragraph has the full reasoning) --
/// general code over the WHOLE `ctx.atlas.people_groups` list, not a
/// hardcoded Tribe-of-Judah/Nation-of-Israel special case. Source (b)
/// (curated nation seeds) still builds NO mentions rows at all (decision
/// 2's other half, unchanged -- those carry no per-locus data of any kind).
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

    // PG-1B rider (decision 0): source (a) groups, general code -- ANY
    // Theographic peopleGroups.json record whose OWN `verse_links` is
    // non-empty (today: Tribe of Judah, 1; Nation of Israel, 12 -- see
    // `atlas_etl::people_groups::parse_people_groups`'s own resolution).
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

/// THE BOUNDARY FIDELITY LAW (module doc comment has the full design):
/// bijection over all three PeopleGroup sources (every source record ->
/// exactly one PeopleGroup node, by exact id; a reclassified slug carries
/// NO Person node) + a total-count check (catches a stray/duplicate
/// insert the per-source loops alone would miss) + mentions completeness
/// for the reclassified subset -- the SAME shape `person_adapter::
/// check_person_fidelity` already established. Fail-loud on the FIRST
/// violation found, named precisely.
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

    // TOTAL COUNT: catches a stray/duplicate PeopleGroup insert the three
    // per-source loops above (each only ever checking "this source's own
    // records are ALL present") would not by themselves -- the same
    // "too few AND too many" discipline `person_adapter::
    // check_person_fidelity`'s own sibling tests establish.
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

    // MENTIONS COMPLETENESS (reclassified subset only -- (a)/(b) sources
    // carry no mentions by design, decision 2): per-reclassified-person,
    // resolved `verse_links` count == PeopleGroup-mentions rows actually
    // carrying that id as entity, exactly. A fresh count over `graph.
    // mentions`'s own row table, not graph-index-derived -- the same
    // "count what actually got built" discipline `check_person_fidelity`
    // already follows.
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

    // PG-1B rider: the SAME mentions-completeness discipline, over source
    // (a) Theographic groups' own `verse_links` -- general code (every
    // `atlas.people_groups` record, not just the two known verse-bearing
    // ones today), so a future Theographic refresh adding a `verses` field
    // to a THIRD group is caught here too, not silently under-served.
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

/// Brief decision 3/requirement 7 ("each with a real `Justification {
/// text, grounds: [Ground::Scripture(...)] }`"; "every NamedAfter row's
/// justification carries at least one Scripture ground -- a law-shaped
/// test over the table"): a FRESH check over the built graph's own
/// `named_after` table, independent of how `normalize` above constructed
/// it (the same "check the built graph, don't just trust the adapter"
/// discipline `check_peoples_fidelity`/`check_person_fidelity` already
/// follow) -- every row must carry >=1 `Ground::Scripture` in its own
/// `justification.grounds` (an `Anchor`/`Source`-only or empty-grounds
/// NamedAfter row is exactly the "distinction labeled but not actually
/// grounded" shape this batch exists to avoid).
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
        Person { id: id.into(), name: name.into(), gender: None, birth_year: None, death_year: None, also_called: vec![], verse_links: verses.iter().map(|s| s.to_string()).collect(), dict_text: None }
    }

    fn ground(from: &str) -> ScriptureGroundSeed {
        ScriptureGroundSeed { from: from.into(), to: None }
    }

    fn ctx_with<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData) -> BuildCtx<'a> {
        BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas)
    }

    // --- normalize: three sources -------------------------------------------

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
        // person_adapter deliberately NOT run here (this test proves
        // peoples_adapter's own reclassified-node construction in
        // isolation) -- the partition itself is proven by the dedicated
        // pipeline-level real-data test in this module below.
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

    // --- merge_alias: reclassified mentions only ----------------------------

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
        // Decision 2 ("NO invented per-locus senses"), as QUALIFIED by the
        // PG-1B rider (module doc comment above): source (a) groups with
        // an EMPTY verse_links (the overwhelming majority -- 21 of 23 real
        // records) still build NO mentions rows, and source (b) curated
        // seeds NEVER carry verse_links at all (no such field on
        // `PeopleGroupSeed`) -- so this remains true for both, just no
        // longer vacuously-by-construction for (a) the way it was pre-rider.
        let atlas = atlas_with(vec![], vec![PeopleGroup { id: "tribe-of-judah".into(), label: "Tribe of Judah".into(), verse_links: vec![] }], vec![PeopleGroupSeed { id: "ammonites".into(), label: "Ammonites".into() }], vec![], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.reclassified_mentions_rows, 0);
        assert_eq!(stats.theographic_mentions_rows, 0);
        assert!(ctx.graph.mentions.is_empty());
    }

    /// PG-1B rider (batch-edge1a-brief.md decision 0): "own test (count +
    /// a spot locus)". Synthetic fixture proving the GENERAL code path
    /// (any source-(a) group with a non-empty `verse_links`, not a
    /// hardcoded Tribe-of-Judah/Nation-of-Israel special case) -- uses a
    /// DIFFERENT group id/label than the real data on purpose, so this
    /// test cannot pass by accident if the adapter secretly hardcoded the
    /// two real names. `real_committed_data_resolves_exactly_the_two_verse_bearing_groups`
    /// (`atlas_etl::people_groups`'s own test) plus
    /// `pg1b_real_data_yields_exactly_13_mentions_rows_at_the_reported_loci`
    /// (`tests/peoples_real_data.rs`) cover the REAL 13-loci claim end to
    /// end; this one proves the mechanism in isolation.
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

        // Spot locus: Tribe of Judah -> PRO.25.1, the exact real-data
        // pairing (module doc comment: "if Tribe of Judah's one verse is
        // JDG 1:2, say so loudly" -- it is NOT).
        let judah_locus = ctx
            .graph
            .mentions
            .iter()
            .find(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(g) if g.0 == "tribe-of-judah"))
            .map(|row| row.locus.clone())
            .expect("a tribe-of-judah mention must exist");
        assert_eq!(judah_locus, verse_locus("PRO.25.1").unwrap());
    }

    // --- normalize: NamedAfter --------------------------------------------

    fn person_node(ctx: &mut BuildCtx, slug: &str, label: &str) {
        let id = PersonId::new(slug).erase();
        ctx.graph.nodes.insert(id.clone(), Node { id, payload: NodePayload::Person { label: label.into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: ProvenanceId::from("test") });
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
        // Deliberately NO person_node("casluhim_nowhere", ...) call.

        let stats = normalize(&mut ctx);
        assert_eq!(stats.named_after_rows, 0);
        assert_eq!(stats.named_after_omitted.len(), 1);
        assert_eq!(stats.named_after_omitted[0].0, "philistines");
        assert!(stats.named_after_omitted[0].1.contains("casluhim_nowhere"), "{}", stats.named_after_omitted[0].1);
        assert!(ctx.graph.named_after.is_empty());
    }

    #[test]
    fn a_reclassified_slug_can_never_satisfy_a_named_after_eponym_check() {
        // The "checked against the GRAPH, not the raw source list" design
        // decision (module doc comment above): a reclassified person has
        // NO Person node (person_adapter's own exclusion), so a
        // (hypothetical, never true of this batch's own real seed data)
        // named_after row naming a reclassified slug as its eponym must be
        // omitted, even though the RAW `atlas.people` record still exists.
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
        // person_adapter never ran (isolation, per this module's own test
        // convention above) -- so no Person node for jebusite_748 exists
        // regardless, which is exactly the state a real build reaches too.

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
        // Edomites' own real curated shape: GEN 36:8-9 (a range) PLUS
        // GEN 25:30 (a second, single-verse ground) on the SAME row.
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

    // --- fidelity ------------------------------------------------------------

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
        let ctx = ctx_with(&canon, &verses, &atlas); // normalize() never called
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the missing node");
        assert!(err.0.contains("bijection"), "{}", err.0);
    }

    #[test]
    fn fidelity_catches_a_reclassified_slug_carrying_both_kinds_at_once() {
        let atlas = atlas_with(vec![person("jebusite_748", "Jebusite", &[])], vec![], vec![], vec![PeopleGroupReclassify { person_slug: "jebusite_748".into(), reason: "test".into() }], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx); // builds the PeopleGroup node correctly
        // Simulate a regression: a Person node ALSO exists for this slug
        // (e.g. a future person_adapter change forgetting the exclusion).
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
        // merge_alias deliberately NOT called -- simulates a silent drop.
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the incomplete mentions rows");
        assert!(err.0.contains("mentions completeness"), "{}", err.0);
    }

    /// PG-1B rider: the SAME mentions-completeness law, over source (a)'s
    /// own `verse_links` -- proves the new fidelity loop actually fires,
    /// not just the pre-existing reclassified-subset one.
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
        // merge_alias deliberately NOT called -- simulates a silent drop.
        let err = check_peoples_fidelity(&atlas, &ctx.graph).expect_err("must catch the incomplete mentions rows");
        assert!(err.0.contains("mentions completeness"), "{}", err.0);
        assert!(err.0.contains("tribe-of-judah"), "{}", err.0);
    }

    #[test]
    fn referential_integrity_of_peoplegroup_mentions_is_already_covered_by_the_generic_law() {
        let mut graph = Graph::default();
        graph.mentions.push(Mentions { locus: verse_locus("GEN.1.1").unwrap(), entity: MentionedEntity::PeopleGroup(PeopleGroupId::new("nowhere")), provenance: ProvenanceId::from("test") });
        let err = crate::law_check::every_authored_edge_resolves(&graph).expect_err("the EXISTING generic law must catch this -- no new code needed");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }

    // --- NamedAfter grounding law --------------------------------------------

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
