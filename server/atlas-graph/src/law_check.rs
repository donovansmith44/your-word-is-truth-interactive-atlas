//! The pipeline's LAW-CHECK stage (pipeline.rs's own stage mapping): a
//! generic, cross-adapter REFERENTIAL-INTEGRITY law, alongside the KJV
//! adapter's own bijection/reconstruction fidelity law (fidelity.rs).
//!
//! Scope, disclosed: checks every NODE-TYPED endpoint of every authored
//! row -- Event/Narrative/Anchor/Place ids on `attests`/`succession`/
//! `dated_by`/`located_at`, (M-C2, folding M-C review M-1) `mentions.entity`
//! (`MentionedEntity`) and `catechism.item` (`CatechismItemId`), and
//! (PG-1a) `named_after`'s own `namesake` (`Namesake::{PeopleGroup, Place,
//! Polity}`) and `eponym` (`PersonId`) -- resolves to a real node in the
//! built graph. This is now every node-typed endpoint this crate's own
//! adapters emit; the
//! previous omission of the latter two was a genuine scope-disclosure gap
//! (M-C review's own words: "safe today only because place_adapter/
//! catechism_adapter build their rows and their corresponding nodes from
//! the identical source iteration... but that invariant isn't enforced by
//! this law, only coincidentally true"), now closed for real rather than
//! merely documented. TextLocus-typed endpoints (`attests.attestation`,
//! `mentions.locus`, `cross_refs.from`/`.to`, `catechism.locus`) are STILL
//! NOT checked here -- resolving a locus to its own TextUnit node id
//! requires `graph_types::graph::Graph`'s own PRIVATE `text_node` helper
//! (confirmed: not part of that module's public surface), and every
//! adapter that EMITS a TextLocus-shaped row already derives it directly
//! from a real, just-built TextUnit node id (kjv_adapter's own
//! `verse_node_id`/`dot_ref` round trip) or a citation the KJV fidelity
//! law itself already proves exists -- a second, weaker re-check here
//! would not catch anything the adapters' own construction doesn't
//! already guarantee by shape (unlike the node-typed endpoints above,
//! which name an id typed once at authoring time with no such structural
//! guarantee).
use std::collections::BTreeSet;

use atlas_graph_types::edge::MentionedEntity;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;

#[derive(Debug)]
pub struct DanglingReference {
    pub relation: &'static str,
    pub field: &'static str,
    pub missing: AnyNodeId,
}

impl std::fmt::Display for DanglingReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{} names {:?}, which has no node in the built graph", self.relation, self.field, self.missing)
    }
}
impl std::error::Error for DanglingReference {}

/// Every node-typed endpoint of every authored row resolves to a real
/// node. Fail-loud on the FIRST dangling reference found (named precisely
/// -- which relation, which field, which missing id), matching this
/// crate's own fail-loud convention for boundary laws (`fidelity.rs`).
pub fn every_authored_edge_resolves(graph: &Graph) -> Result<(), DanglingReference> {
    let has = |id: &AnyNodeId| graph.nodes.contains_key(id);
    let check = |relation: &'static str, field: &'static str, id: AnyNodeId| -> Result<(), DanglingReference> {
        if has(&id) {
            Ok(())
        } else {
            Err(DanglingReference { relation, field, missing: id })
        }
    };

    for row in &graph.attests {
        check("attests", "event", row.event.erase())?;
    }
    for row in &graph.located_at {
        check("located_at", "event", row.event.erase())?;
        check("located_at", "place", row.place.erase())?;
    }
    for row in &graph.succession {
        let seen: BTreeSet<&atlas_graph_types::id::EventId> = row.chain.iter().collect();
        for eid in seen {
            check("succession", "chain", eid.erase())?;
        }
    }
    for row in &graph.dated_by {
        check("dated_by", "event", row.event.erase())?;
        use atlas_graph_types::chrono::ChronoTarget;
        // ChronoTarget::Era is DELIBERATELY excluded: `event_world::
        // choose_placement`'s own degenerate fallback (reachable only when
        // the anchor table is empty -- never true of real data, but true
        // of `AtlasData::demo_fixture()` and several unit fixtures across
        // this workspace) mints a synthetic `"undetermined-basis-{id}"`
        // era id BY DESIGN, documented at that call site as intentionally
        // unresolvable ("EraOnly has no real Era node to resolve against,
        // honestly... this event's own 'why this date?' explorability
        // degrades to 'undetermined basis'"). Flagging that as a dangling
        // reference would turn a disclosed, deliberate honesty gap into a
        // build failure -- the opposite of this law's own purpose. A
        // REAL (curated, non-synthetic) EraOnly placement's target -- once
        // Era nodes materialize -- is exactly the kind of reference this
        // law WOULD want to check; that's a real Era adapter's own concern,
        // not this generic pass's.
        let target = match row.placement.target() {
            ChronoTarget::Anchor(a) => Some(a.erase()),
            ChronoTarget::Prior(p) => Some(p.erase()),
            ChronoTarget::Era(_) => None,
        };
        if let Some(target) = target {
            check("dated_by", "target", target)?;
        }
    }
    // M-D3 (owner ruling R2): the `named` relation (and its own
    // `named.place` dangling-reference check, formerly here) retired whole
    // -- a place's aliases are `NodePayload::Place::aliases`'s own payload
    // field, plain strings with no node reference of their own to dangle,
    // so there is nothing left for a check at this shape to verify (the
    // vacuous branch of the ruling: "re-points... or deletes it if
    // vacuous").
    // M-C2 (folded M-C review M-1): the two node-typed endpoints this
    // law's own module doc comment previously disclosed as unchecked --
    // now real checks, not just a documented coincidence.
    for row in &graph.mentions {
        let id = match &row.entity {
            MentionedEntity::Place(p) => p.erase(),
            MentionedEntity::Person(p) => p.erase(),
            MentionedEntity::PeopleGroup(g) => g.erase(),
            // ATTEST-1: the widened sense -- a verse that references an
            // event without narrating it. Same check, one more variant.
            MentionedEntity::Event(e) => e.erase(),
        };
        check("mentions", "entity", id)?;
    }
    for row in &graph.catechism {
        check("catechism", "item", row.item.erase())?;
    }
    // KRETZ-1: `comments_on`'s own node-typed endpoint (`item`) -- the SAME
    // class of check `catechism`'s own `item` gets immediately above. The
    // `on` field (a `BibleLocusRange`) is NOT checked here, matching this
    // law's own disclosed scope (module doc comment): every adapter that
    // emits a TextLocus/BibleLocusRange-shaped row already derives it
    // directly from a real, just-built TextUnit position, and `kretzmann_
    // adapter::normalize` is no exception (`VerseRef { book, chapter, verse
    // }` built straight from the SAME parsed unit the KJV TextUnit nodes
    // themselves are keyed by).
    for row in &graph.comments_on {
        check("comments_on", "item", row.item.erase())?;
    }
    // RED-1: `spoken_by`'s own node-typed endpoint (`speaker`) and
    // `spoken_at`'s own (`place`) -- the SAME class of check `comments_on`'s
    // own `item` gets immediately above. `locus` (a `BibleLocusRange`) is
    // NOT checked here, matching this law's own disclosed scope (module
    // doc comment): `red_letter_adapter::normalize` derives every locus
    // directly from real, just-built TextUnit positions, the same
    // discipline `kretzmann_adapter::normalize` already follows.
    for row in &graph.spoken_by {
        check("spoken_by", "speaker", row.speaker.erase())?;
    }
    for row in &graph.spoken_at {
        check("spoken_at", "place", row.place.erase())?;
    }
    // PG-1a: `named_after`'s two node-typed endpoints (`namesake`/`eponym`)
    // -- newly authored this batch, closing the SAME class of gap M-C2's
    // own `mentions`/`catechism` extension above closed first (this law's
    // own scope grows with every new node-typed authored relation, per its
    // own module doc comment).
    for row in &graph.named_after {
        let namesake_id = match &row.namesake {
            atlas_graph_types::edge::Namesake::PeopleGroup(g) => g.erase(),
            atlas_graph_types::edge::Namesake::Place(p) => p.erase(),
            atlas_graph_types::edge::Namesake::Polity(p) => p.erase(),
        };
        check("named_after", "namesake", namesake_id)?;
        check("named_after", "eponym", row.eponym.erase())?;
    }
    // NODE-1 fix round 1 (review M-1, closing the gap this law's own
    // growth rule names -- and the SAME gap `contains_concord` had
    // carried since CORP-2a): every `Contains` row's `container`
    // endpoint, and (NODE1-ROWS-1) the `Container` child endpoint, must
    // name real nodes. The `Loci` content is NOT checked here, matching
    // this law's own disclosed scope for TextLocus-shaped fields (module
    // doc comment).
    for row in &graph.contains_bible {
        check("contains_bible", "container", row.container.erase())?;
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            check("contains_bible", "content.container", child.erase())?;
        }
    }
    for row in &graph.contains_concord {
        check("contains_concord", "container", row.container.erase())?;
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            check("contains_concord", "content.container", child.erase())?;
        }
    }
    // NODE1-ROWS-1: the pairwise canon steps -- both endpoints node-typed.
    for row in &graph.canon_succession {
        check("canon_succession", "prior", row.prior.erase())?;
        check("canon_succession", "next", row.next.erase())?;
    }
    // ATTEST-1: the symmetric analogue rows -- both endpoints node-typed
    // (the SAME class of check `canon_succession` gets immediately above;
    // this law's own scope grows with every new node-typed authored
    // relation, per its own module doc comment).
    for row in &graph.analogue {
        check("analogue", "a", row.a.erase())?;
        check("analogue", "b", row.b.erase())?;
    }

    Ok(())
}

/// NODE1-ROWS-1 (owner recursion addendum: "are nodes all recursively
/// defined? they should be." -- recursion is edge-carried, and the
/// container-containment rows must form a FOREST): every container that
/// appears as a `ContainerContent::Container` CHILD across BOTH corpora's
/// `Contains` tables has at most ONE parent (single-parent), and
/// following parents never returns to a visited container (acyclic).
/// Runs as part of `LawCheckPass` -- a violation is a fail-loud BUILD
/// failure, never shipped data.
pub fn container_containment_is_a_forest(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;
    // Every Container-child row of both corpora, as (relation, parent,
    // child) borrows off the graph.
    let mut edges: Vec<(&'static str, &str, &str)> = Vec::new();
    for row in &graph.contains_bible {
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            edges.push(("contains_bible", row.container.0.as_str(), child.0.as_str()));
        }
    }
    for row in &graph.contains_concord {
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            edges.push(("contains_concord", row.container.0.as_str(), child.0.as_str()));
        }
    }

    // child -> parent (single-parent enforced as the map is built).
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    for (relation, container, child) in edges {
        if let Some(existing) = parent.insert(child, container) {
            if existing != container {
                return Err(format!(
                    "container containment is not single-parent: '{child}' is a Container child of BOTH '{existing}' and '{container}' ({relation})"
                ));
            }
            // The same (parent, child) row twice is a duplicate edge --
            // also a defect, and cheaper to name here than to let the
            // index silently double.
            return Err(format!(
                "duplicate container-containment row: '{container}' ⊃ '{child}' appears more than once ({relation})"
            ));
        }
    }
    // Acyclicity: with single-parent already enforced, every walk up the
    // parent map either terminates at a root or revisits -- a revisit is
    // a cycle. Each chain is walked once with a local visited set (the
    // map is small: one entry per child container).
    for start in parent.keys() {
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut cur: &str = start;
        seen.insert(cur);
        while let Some(next) = parent.get(cur) {
            if !seen.insert(next) {
                return Err(format!(
                    "container containment has a CYCLE reachable from '{start}' (revisited '{next}') -- containment must be a forest"
                ));
            }
            cur = next;
        }
    }
    Ok(())
}

/// ATTEST-1's L4 companion gate: an `Analogue` row asserts that its two
/// ends are DISTINCT events (see `edge::Analogue`'s own doc comment, which
/// is the law: "distinct events whose accounts are similar in form or
/// content — NEVER two accounts of one event"). A self-loop asserts the
/// opposite of what the relation means, so it is a fail-loud build
/// failure, not a silently-tolerated no-op. Duplicate rows for the same
/// unordered pair are caught here too -- the symmetric index would
/// silently double the edge, exactly the defect the container-containment
/// forest gate's own duplicate branch exists to name.
pub fn analogue_rows_join_two_distinct_events(graph: &Graph) -> Result<(), String> {
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for row in &graph.analogue {
        let (a, b) = (row.a.0.as_str(), row.b.0.as_str());
        if a == b {
            return Err(format!(
                "analogue row joins '{a}' to ITSELF -- an Analogue asserts two DISTINCT events; two accounts of ONE event are Attests rows on that one event"
            ));
        }
        let key = if a <= b { (a, b) } else { (b, a) };
        if !seen.insert(key) {
            return Err(format!(
                "duplicate analogue row: '{}' <-> '{}' appears more than once (the symmetric index would double the edge)",
                key.0, key.1
            ));
        }
    }
    Ok(())
}

/// ATTEST-1's L2, THE ATTESTATION-EXCLUSIVITY LAW (owner-signed, and
/// owner-signed FAIL-LOUD -- "the softer warning option was declined"):
///
/// > no verse belongs to the `Attests` set of two distinct events.
///
/// WHY it is a law and not a lint. The founding diagnosis (owner, verbatim):
/// "I'm seeing a fundamental error. The Espousal of Mary event has parallel
/// accounts Mat.1.18 + Luke.1.27, and that's a distinct event from The Angel
/// Gabriel Announces Jesus'... which has Luk1.26-36; Even worse, the
/// appearance of Gabriel to Zacharias is BETWEEN the espousal of mary and
/// the announcement of Gabriel to Mary." LUK.1.27 sat in the `Attests` set
/// of BOTH `theo-249` and `rob_annunciation_mary`, and because `Attests`
/// is what the `Parallels` capability walks (verse -> event -> that event's
/// OTHER witnesses, `frontier.rs`), a shared verse silently fabricates a
/// parallel between two events that are not parallel at all. The same
/// shape produced the leper false-parallel the owner reported separately.
/// `Analogue` and `Mentions` rows are EXEMPT by construction -- they are
/// not in this table; only `Attests` partitions.
///
/// HOW it fails loud without asserting a corpus-wide fix nobody has made
/// yet. A green law over a corpus with 500+ real collisions would be a
/// lie, and silently retyping 500+ collisions to make it green would be
/// exactly the bulk guessing the batch charter forbids. So the law is
/// stated against a DECLARED inventory, the same shape
/// `atlas_core::event_merge`'s own `EVENT_MERGE_PAIRS`/
/// `EVENT_DISTINCT_PAIRS` already use for the sibling duplicate-identity
/// question:
///
///   * an UNDECLARED collision -- any event pair sharing an attestation
///     that is not in `attestation_pending::PENDING` -- is a BUILD
///     FAILURE. New data can never introduce a new shared attestation
///     quietly, which is the property that actually protects a reader.
///   * a DRIFTED declaration -- a declared pair now sharing a different
///     number of verses than declared -- is also a build failure. A count
///     that is merely "close" is a count nobody can trust.
///
/// The STALE direction (a declared pair that no longer collides at all)
/// is checked by `attestation_inventory_has_no_stale_rows` below, NOT
/// here, and the split is deliberate: this function runs on EVERY graph
/// build, including the many small synthetic fixtures across this
/// workspace, and every one of those legitimately collides on nothing, so
/// checking staleness here would make the entire inventory "stale" against
/// a three-node test graph. Staleness is a claim about the REAL corpus, so
/// it is asserted where the real corpus is
/// (`tests/attestation_exclusivity_real_data.rs`).
///
/// KNOWN LIMIT, stated so nobody has to rediscover it (ATTEST-1 fix round
/// 1, review finding L-2): a declared row pins the shared-verse COUNT, not
/// the shared-verse SET. A future change that removes one shared verse
/// between an already-declared pair and adds a different one leaves the
/// count unchanged and passes here silently. That is tolerable only because
/// the pair is already queued for an owner ruling either way; if the queue
/// ever outlives that assumption, the row should carry a hash of the shared
/// verse set rather than its cardinality.
///
/// The inventory itself is the OWNER'S CURATION QUEUE, compiled: every row
/// carries the mechanically-derived structural class (`Containment` when
/// one event's attestation set contains the other's -- the dominant real
/// shape, a Theographic mega-span such as `theo-443` "Holy Week" or
/// `theo-217` "Prophecies of Isaiah" against the fine-grained curated
/// pericopes inside it -- `Overlap` otherwise) and NOTHING ELSE. The
/// SEMANTIC call (shared-account error / mention misfiled as account /
/// same event needing a merge / genuine containment wanting an event
/// containment relation this vocabulary does not yet have) is deliberately
/// NOT guessed here; it belongs to the owner, and `batch-attest1-report.md`
/// carries the full table.
pub fn attestation_is_exclusive(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;

    // verse position -> the distinct events attesting it.
    // `event_world::populate_nodes_and_direct_rows` emits ONE Attests row
    // per verse (`from == to`, a single-verse range -- see its own
    // `verse_to_range` doc comment), so the range's own `from` unit IS the
    // attested verse; no range expansion is needed or would be honest here.
    let mut by_verse: BTreeMap<(u8, u16, u16), BTreeSet<&str>> = BTreeMap::new();
    for row in &graph.attests {
        let v = &row.attestation.from.unit;
        by_verse.entry((v.book, v.chapter, v.verse)).or_default().insert(row.event.0.as_str());
    }

    // Observed collisions, folded to unordered event pairs with a count of
    // the verses they actually share.
    let mut observed: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for events in by_verse.values() {
        if events.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = events.iter().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                *observed.entry((ids[i], ids[j])).or_insert(0) += 1;
            }
        }
    }

    let declared: BTreeMap<(&str, &str), usize> = crate::attestation_pending::PENDING
        .iter()
        .map(|p| ((p.a, p.b), p.shared_verses))
        .collect();

    for (pair, count) in &observed {
        match declared.get(pair) {
            None => {
                return Err(format!(
                    "ATTESTATION EXCLUSIVITY (L2) VIOLATED: '{}' and '{}' both attest {} shared verse(s), and this pair is NOT in the declared pending inventory. An event's Attests edges carry ONLY narrative accounts: if one side merely REFERENCES the event, author a Mentions row instead; if the two are similar-but-distinct, author an Analogue row; if they are one event, merge them. Silently sharing an attestation fabricates a parallel account.",
                    pair.0, pair.1, count
                ));
            }
            Some(declared_count) if declared_count != count => {
                return Err(format!(
                    "ATTESTATION EXCLUSIVITY (L2) INVENTORY DRIFT: '{}' and '{}' now share {} verse(s), but attestation_pending::PENDING declares {}. Re-pin the row (and record the shrink) rather than letting the inventory drift.",
                    pair.0, pair.1, count, declared_count
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// L2's other direction, asserted over the REAL corpus only (see
/// `attestation_is_exclusive`'s own doc comment for why it is not part of
/// the per-build pass): a DECLARED pair that no longer collides at all is
/// a failure. The curation queue can only shrink deliberately, with the
/// shrink recorded -- a queue that keeps resolved entries stops being a
/// queue, and a resolved entry left in place would also silently license a
/// future regression to re-introduce the very collision it names.
pub fn attestation_inventory_has_no_stale_rows(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;

    let mut by_verse: BTreeMap<(u8, u16, u16), BTreeSet<&str>> = BTreeMap::new();
    for row in &graph.attests {
        let v = &row.attestation.from.unit;
        by_verse.entry((v.book, v.chapter, v.verse)).or_default().insert(row.event.0.as_str());
    }
    let mut observed: BTreeSet<(&str, &str)> = BTreeSet::new();
    for events in by_verse.values() {
        if events.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = events.iter().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                observed.insert((ids[i], ids[j]));
            }
        }
    }
    for p in crate::attestation_pending::PENDING {
        if !observed.contains(&(p.a, p.b)) {
            return Err(format!(
                "ATTESTATION EXCLUSIVITY (L2) STALE DECLARATION: '{}' and '{}' are declared as sharing {} verse(s) but no longer collide at all. Delete the row and record the shrink.",
                p.a, p.b, p.shared_verses
            ));
        }
    }
    Ok(())
}

/// NODE1-ROWS-1's standing index≡rows conformance law ("indexes derive
/// FROM rows, never the reverse"): rebuilding the indexes from THIS
/// graph's own row tables alone reproduces the indexes the graph is
/// actually serving, entry for entry, in order. Catches any future step
/// that writes into `graph.indexes`/`graph.symmetric_indexes` outside
/// `Graph::build_indexes` + `event_world::add_justified_by` (the one
/// row-derived post-step) -- the exact class the retired NODE-1
/// derived-entry merge belonged to.
///
/// CALL SITES, stated exactly (Batch ATTEST-1, closing NODE-1 review
/// NEW-1: this comment used to claim coverage the law's single call site
/// did not deliver). It runs over BOTH paths a real graph can arrive by,
/// and neither is a per-build pass (that would double every build's index
/// cost for a property only a code change can break):
///   * FROM SOURCES -- `tests/bible_containers_real_data.rs`, over the
///     real committed corpus as `build_graph_from_sources_with_eras`
///     produces it;
///   * FROM ARTIFACT -- `tests/artifact_conformance.rs`, over the graph
///     decoded from real written artifact bytes and indexed by exactly
///     the sequence `GraphService::from_artifact` runs at server startup.
/// That second call site is the one NEW-1 found missing; it was the path
/// nine of the retired derived-merge's own call sites lived on.
pub fn indexes_derive_exactly_from_rows(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;
    // Clone the row tables + nodes into a fresh graph (indexes are a pure
    // function of exactly these), rebuild, and compare.
    let mut fresh = Graph::default();
    fresh.nodes = graph.nodes.clone();
    fresh.contains_bible = graph.contains_bible.clone();
    fresh.contains_concord = graph.contains_concord.clone();
    fresh.attests = graph.attests.clone();
    fresh.succession = graph.succession.clone();
    fresh.canon_succession = graph.canon_succession.clone();
    fresh.dated_by = graph.dated_by.clone();
    fresh.located_at = graph.located_at.clone();
    fresh.fulfills = graph.fulfills.clone();
    fresh.typology = graph.typology.clone();
    fresh.named_after = graph.named_after.clone();
    fresh.catechism = graph.catechism.clone();
    fresh.comments_on = graph.comments_on.clone();
    fresh.spoken_by = graph.spoken_by.clone();
    fresh.spoken_at = graph.spoken_at.clone();
    fresh.mentions = graph.mentions.clone();
    fresh.cross_refs = graph.cross_refs.clone();
    fresh.quotes = graph.quotes.clone();
    fresh.confesses = graph.confesses.clone();
    fresh.corresponds_bible = graph.corresponds_bible.clone();
    fresh.temporal_adjacency = graph.temporal_adjacency.clone();
    fresh.analogue = graph.analogue.clone();
    fresh.build_indexes();
    crate::event_world::add_justified_by(&mut fresh);

    let compare = |name: &str, served: &BTreeMap<atlas_graph_types::id::Position, Vec<(atlas_graph_types::edge::EdgeId, atlas_graph_types::id::Position, atlas_graph_types::explore::EdgeMeta)>>, rebuilt: &BTreeMap<atlas_graph_types::id::Position, Vec<(atlas_graph_types::edge::EdgeId, atlas_graph_types::id::Position, atlas_graph_types::explore::EdgeMeta)>>| -> Result<(), String> {
        if served != rebuilt {
            return Err(format!("{name}: the served index diverges from a pure rebuild from rows -- something wrote into the indexes outside build_indexes/add_justified_by"));
        }
        Ok(())
    };
    for (rel, served) in &graph.indexes {
        let rebuilt = fresh.indexes.get(rel).ok_or_else(|| format!("served index for {rel:?} has no row-derived counterpart at all"))?;
        compare(&format!("indexes[{rel:?}].fwd"), &served.fwd, &rebuilt.fwd)?;
        compare(&format!("indexes[{rel:?}].inv"), &served.inv, &rebuilt.inv)?;
    }
    if graph.indexes.len() != fresh.indexes.len() {
        return Err("the rebuilt graph carries a relation index the served graph lacks".into());
    }
    for (rel, served) in &graph.symmetric_indexes {
        let rebuilt = fresh.symmetric_indexes.get(rel).ok_or_else(|| format!("served symmetric index for {rel:?} has no row-derived counterpart at all"))?;
        compare(&format!("symmetric_indexes[{rel:?}].fwd"), &served.fwd, &rebuilt.fwd)?;
    }
    if graph.symmetric_indexes.len() != fresh.symmetric_indexes.len() {
        return Err("the rebuilt graph carries a symmetric index the served graph lacks".into());
    }
    Ok(())
}

// M-D3 (owner ruling R1): `payload_years_match_resolved_placements` (the
// M-C2 "verified-cache law," which verified `NodePayload::Event.from_year`
// never diverged from its own resolved `dated_by` placement) RETIRES here,
// WITH the payload fields it existed to police -- `NodePayload::Event` no
// longer carries a from_year/to_year/order_key mirror at all (the crate
// patch this batch applies), so there is no longer a second, independently-
// computed copy for this law to cross-check; deletion is the stronger fix
// (a law with nothing left to guard is not a law, it is dead weight). The
// deleted code (struct, fn, and its own tests) is recoverable from git
// history at the commit immediately preceding this one -- see the batch
// report for the exact SHA, the P7 way (no runtime flag).

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::edge::{Justification, LocatedAt};
    use atlas_graph_types::id::{EventId, PlaceId};
    use atlas_graph_types::node::{Node, NodePayload};

    #[test]
    fn green_on_an_empty_graph() {
        let graph = Graph::default();
        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_located_at_row_names_a_place_with_no_node() {
        let mut graph = Graph::default();
        graph.located_at.push(LocatedAt {
            event: EventId::new("e1"),
            place: PlaceId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        // No Event/Place nodes inserted at all -- both endpoints dangle;
        // the first-checked field (`event`) is what the error names.
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling reference");
        assert_eq!(err.relation, "located_at");
        assert_eq!(err.field, "event");
    }

    fn locus() -> atlas_graph_types::text::TextLocus {
        atlas_graph_types::text::TextLocus {
            at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: 0, chapter: 1, verse: 1 }),
            span: None,
        }
    }

    // M-C2 (folded M-C review M-1): the two node-typed endpoints this
    // law previously left unchecked -- red-then-green, mirroring the
    // `located_at` case above.
    #[test]
    fn red_when_a_mentions_row_names_a_place_with_no_node() {
        let mut graph = Graph::default();
        graph.mentions.push(atlas_graph_types::edge::Mentions {
            locus: locus(),
            entity: MentionedEntity::Place(atlas_graph_types::id::PlaceId::new("nowhere")),
            provenance: "test".into(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling mentions.entity reference");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }

    #[test]
    fn red_when_a_catechism_link_row_names_an_item_with_no_node() {
        let mut graph = Graph::default();
        graph.catechism.push(atlas_graph_types::edge::CatechismLink {
            locus: locus(),
            item: atlas_graph_types::id::CatechismItemId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling catechism.item reference");
        assert_eq!(err.relation, "catechism");
        assert_eq!(err.field, "item");
    }

    /// KRETZ-1: the SAME red-then-green shape as `catechism` immediately
    /// above, for `comments_on`'s own node-typed `item` endpoint.
    #[test]
    fn red_when_a_comments_on_row_names_an_item_with_no_node() {
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};
        let mut graph = Graph::default();
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })).unwrap();
        graph.comments_on.push(atlas_graph_types::edge::CommentsOn {
            item: atlas_graph_types::id::CommentaryItemId::new("nowhere"),
            on: range,
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling comments_on.item reference");
        assert_eq!(err.relation, "comments_on");
        assert_eq!(err.field, "item");
    }

    #[test]
    fn green_when_a_comments_on_row_resolves_to_a_real_commentary_item_node() {
        use atlas_graph_types::id::{CommentaryItemId, NodeKind, SourceId};
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};

        let mut graph = Graph::default();
        let item_id = CommentaryItemId::new("kretzmann/0.1.0").erase();
        assert_eq!(item_id.kind, NodeKind::CommentaryItem);
        graph.nodes.insert(item_id.clone(), Node { id: item_id.clone(), payload: NodePayload::CommentaryItem { work: SourceId::new("kretzmann-popular-commentary"), heading: None, text: "prose".into() }, provenance: "test".into() });
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })).unwrap();
        graph.comments_on.push(atlas_graph_types::edge::CommentsOn { item: CommentaryItemId::new("kretzmann/0.1.0"), on: range, provenance: "test".into(), justification: Justification::default() });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    /// RED-1: the SAME red-then-green shape as `comments_on` above, for
    /// `spoken_by`'s own node-typed `speaker` endpoint.
    #[test]
    fn red_when_a_spoken_by_row_names_a_speaker_with_no_node() {
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};
        let mut graph = Graph::default();
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_by.push(atlas_graph_types::edge::SpokenBy {
            locus: range,
            speaker: atlas_graph_types::id::PersonId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling spoken_by.speaker reference");
        assert_eq!(err.relation, "spoken_by");
        assert_eq!(err.field, "speaker");
    }

    #[test]
    fn green_when_a_spoken_by_row_resolves_to_a_real_person_node() {
        use atlas_graph_types::id::{NodeKind, PersonId};
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};

        let mut graph = Graph::default();
        let person_id = PersonId::new("jesus_905").erase();
        assert_eq!(person_id.kind, NodeKind::Person);
        graph.nodes.insert(person_id.clone(), Node { id: person_id, payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: "test".into() });
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_by.push(atlas_graph_types::edge::SpokenBy { locus: range, speaker: PersonId::new("jesus_905"), provenance: "test".into(), justification: Justification::default() });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    /// RED-1: the SAME red-then-green shape, for `spoken_at`'s own
    /// node-typed `place` endpoint.
    #[test]
    fn red_when_a_spoken_at_row_names_a_place_with_no_node() {
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};
        let mut graph = Graph::default();
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_at.push(atlas_graph_types::edge::SpokenAt {
            locus: range,
            place: atlas_graph_types::id::PlaceId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling spoken_at.place reference");
        assert_eq!(err.relation, "spoken_at");
        assert_eq!(err.field, "place");
    }

    #[test]
    fn green_when_a_spoken_at_row_resolves_to_a_real_place_node() {
        use atlas_graph_types::id::PlaceId;
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};

        let mut graph = Graph::default();
        let place_id = PlaceId::new("capernaum").erase();
        graph.nodes.insert(place_id.clone(), Node { id: place_id, payload: NodePayload::Place { canonical: "Capernaum".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() });
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_at.push(atlas_graph_types::edge::SpokenAt { locus: range, place: PlaceId::new("capernaum"), provenance: "test".into(), justification: Justification::default() });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_named_after_row_names_a_namesake_with_no_node() {
        let mut graph = Graph::default();
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(atlas_graph_types::id::PeopleGroupId::new("nowhere")),
            eponym: atlas_graph_types::id::PersonId::new("also-nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        // Both endpoints dangle; the first-checked field (`namesake`) is
        // what the error names, the same "first field wins" convention
        // `red_when_a_located_at_row_names_a_place_with_no_node` proves.
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling named_after.namesake reference");
        assert_eq!(err.relation, "named_after");
        assert_eq!(err.field, "namesake");
    }

    #[test]
    fn red_when_a_named_after_row_names_an_eponym_with_no_node() {
        use atlas_graph_types::id::PeopleGroupId;
        use atlas_graph_types::node::{Node, NodePayload};

        let mut graph = Graph::default();
        let group_id = PeopleGroupId::new("ammonites").erase();
        graph.nodes.insert(group_id.clone(), Node { id: group_id, payload: NodePayload::PeopleGroup { label: "Ammonites".into(), description: None }, provenance: "test".into() });
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(PeopleGroupId::new("ammonites")),
            eponym: atlas_graph_types::id::PersonId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("the namesake resolves; the eponym must still be caught");
        assert_eq!(err.relation, "named_after");
        assert_eq!(err.field, "eponym");
    }

    #[test]
    fn green_when_mentions_catechism_and_named_after_rows_resolve_to_real_nodes() {
        use atlas_graph_types::id::{NodeKind, PeopleGroupId, PersonId, PlaceId};
        use atlas_graph_types::node::{Node, NodePayload};

        let mut graph = Graph::default();
        let place_id = PlaceId::new("hebron").erase();
        graph.nodes.insert(
            place_id.clone(),
            Node { id: place_id.clone(), payload: NodePayload::Place { canonical: "Hebron".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() },
        );
        graph.mentions.push(atlas_graph_types::edge::Mentions { locus: locus(), entity: MentionedEntity::Place(PlaceId::new("hebron")), provenance: "test".into() });

        let item_id = atlas_graph_types::id::CatechismItemId::new("commandment-1").erase();
        assert_eq!(item_id.kind, NodeKind::CatechismItem);
        graph.nodes.insert(item_id.clone(), Node { id: item_id.clone(), payload: NodePayload::CatechismItem { label: "The First Commandment".into() }, provenance: "test".into() });
        graph.catechism.push(atlas_graph_types::edge::CatechismLink {
            locus: locus(),
            item: atlas_graph_types::id::CatechismItemId::new("commandment-1"),
            provenance: "test".into(),
            justification: Justification::default(),
        });

        let group_id = PeopleGroupId::new("ammonites").erase();
        graph.nodes.insert(group_id.clone(), Node { id: group_id, payload: NodePayload::PeopleGroup { label: "Ammonites".into(), description: None }, provenance: "test".into() });
        let person_id = PersonId::new("ben-ammi_451").erase();
        graph.nodes.insert(person_id.clone(), Node { id: person_id, payload: NodePayload::Person { label: "Ben-ammi".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: "test".into() });
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(PeopleGroupId::new("ammonites")),
            eponym: PersonId::new("ben-ammi_451"),
            provenance: "test".into(),
            justification: Justification::default(),
        });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    // M-D3: the former "verified-cache law" tests (`one_dated_event` +
    // four `payload_years_match_resolved_placements` cases) are deleted
    // alongside the function itself -- see this file's own retirement note
    // above.

    // -----------------------------------------------------------------
    // NODE-1 fix round 1: the grown referential checks (review M-1) and
    // the NODE1-ROWS-1 forest law, red/green pairs (the same discipline
    // every prior law extension here followed).
    // -----------------------------------------------------------------

    fn container_node(graph: &mut Graph, raw: &str) -> atlas_graph_types::id::ContainerNodeId {
        let id = atlas_graph_types::id::ContainerNodeId::new(raw);
        graph.nodes.insert(
            id.erase(),
            Node { id: id.erase(), payload: NodePayload::Container { title: raw.to_string() }, provenance: "test".into() },
        );
        id
    }

    fn child_row(parent: &atlas_graph_types::id::ContainerNodeId, child: &atlas_graph_types::id::ContainerNodeId) -> atlas_graph_types::edge::Contains<atlas_graph_types::text::BibleTag> {
        atlas_graph_types::edge::Contains {
            container: parent.clone(),
            content: atlas_graph_types::edge::ContainerContent::Container(child.clone()),
            provenance: "test".into(),
            justification: Justification::default(),
        }
    }

    #[test]
    fn red_when_a_contains_row_names_a_container_with_no_node() {
        let mut graph = Graph::default();
        // Neither the container nor the child node exists.
        graph.contains_bible.push(child_row(&atlas_graph_types::id::ContainerNodeId::new("bible-book-GEN"), &atlas_graph_types::id::ContainerNodeId::new("bible-chapter-GEN-1")));
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling container");
        assert_eq!(err.relation, "contains_bible");
        assert_eq!(err.field, "container");
    }

    #[test]
    fn red_when_a_container_child_dangles_and_when_a_canon_step_dangles() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        graph.contains_bible.push(child_row(&book, &atlas_graph_types::id::ContainerNodeId::new("bible-chapter-GEN-99")));
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling child");
        assert_eq!(err.relation, "contains_bible");
        assert_eq!(err.field, "content.container");

        let mut graph2 = Graph::default();
        let ch1 = container_node(&mut graph2, "bible-chapter-GEN-1");
        graph2.canon_succession.push(atlas_graph_types::edge::CanonSuccession {
            prior: ch1,
            next: atlas_graph_types::id::ContainerNodeId::new("bible-chapter-GEN-2"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err2 = every_authored_edge_resolves(&graph2).expect_err("must catch the dangling next");
        assert_eq!(err2.relation, "canon_succession");
        assert_eq!(err2.field, "next");
    }

    #[test]
    fn forest_law_green_on_a_real_forest() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        let ch1 = container_node(&mut graph, "bible-chapter-GEN-1");
        let ch2 = container_node(&mut graph, "bible-chapter-GEN-2");
        graph.contains_bible.push(child_row(&book, &ch1));
        graph.contains_bible.push(child_row(&book, &ch2));
        assert!(container_containment_is_a_forest(&graph).is_ok());
    }

    #[test]
    fn forest_law_red_on_two_parents() {
        let mut graph = Graph::default();
        let gen = container_node(&mut graph, "bible-book-GEN");
        let exo = container_node(&mut graph, "bible-book-EXO");
        let ch = container_node(&mut graph, "bible-chapter-GEN-1");
        graph.contains_bible.push(child_row(&gen, &ch));
        graph.contains_bible.push(child_row(&exo, &ch));
        let err = container_containment_is_a_forest(&graph).expect_err("two parents must be caught");
        assert!(err.contains("not single-parent"), "{err}");
    }

    #[test]
    fn forest_law_red_on_a_duplicate_row_and_on_a_cycle() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        let ch = container_node(&mut graph, "bible-chapter-GEN-1");
        graph.contains_bible.push(child_row(&book, &ch));
        graph.contains_bible.push(child_row(&book, &ch));
        let err = container_containment_is_a_forest(&graph).expect_err("a duplicate row must be caught");
        assert!(err.contains("duplicate"), "{err}");

        let mut graph2 = Graph::default();
        let a = container_node(&mut graph2, "a");
        let b = container_node(&mut graph2, "b");
        let c = container_node(&mut graph2, "c");
        graph2.contains_bible.push(child_row(&a, &b));
        graph2.contains_bible.push(child_row(&b, &c));
        graph2.contains_bible.push(child_row(&c, &a));
        let err2 = container_containment_is_a_forest(&graph2).expect_err("a cycle must be caught");
        assert!(err2.contains("CYCLE"), "{err2}");
    }

    // -----------------------------------------------------------------
    // ATTEST-1: the attestation-exclusivity law (L2) and the Analogue
    // distinctness gate (L4), red/green pairs -- the same per-field
    // discipline every prior law extension in this file followed.
    // -----------------------------------------------------------------

    fn event_node(graph: &mut Graph, raw: &str) -> EventId {
        let id = EventId::new(raw);
        graph.nodes.insert(
            id.erase(),
            Node {
                id: id.erase(),
                payload: NodePayload::Event { label: raw.to_string(), kind: "event".into(), verses: vec![], witnesses: vec![], robertson_section: None, acts_section: None, atlas_section: None, kjv_superscription: None, ref_note: None },
                provenance: "test".into(),
            },
        );
        id
    }

    fn attests(event: &EventId, book: u8, chapter: u16, verse: u16) -> atlas_graph_types::edge::Attests {
        use atlas_graph_types::text::{BibleLocusRange, Locus, VerseRef};
        let l = Locus::whole(VerseRef { book, chapter, verse });
        atlas_graph_types::edge::Attests {
            event: event.clone(),
            attestation: BibleLocusRange::new(l.clone(), l).unwrap(),
            provenance: "test".into(),
            justification: Justification::default(),
        }
    }

    /// GREEN: two events attesting DIFFERENT verses partition cleanly --
    /// the law's own happy path, and the shape every corrected pair is
    /// meant to reach.
    #[test]
    fn attestation_exclusivity_green_when_no_verse_is_shared() {
        let mut graph = Graph::default();
        let a = event_node(&mut graph, "rob_leper_healed");
        let b = event_node(&mut graph, "mat_leper_healed");
        graph.attests.push(attests(&a, 40, 1, 40));
        graph.attests.push(attests(&b, 39, 8, 1));
        assert!(attestation_is_exclusive(&graph).is_ok());
    }

    /// RED: the founding case's own SHAPE -- one verse in two events'
    /// Attests sets, with the pair undeclared. This is the failure the
    /// owner reported (LUK.1.27 attested to both the espousal and the
    /// annunciation), stated as the law that now catches it.
    #[test]
    fn attestation_exclusivity_red_on_an_undeclared_shared_verse() {
        let mut graph = Graph::default();
        let espousal = event_node(&mut graph, "theo-249");
        let annunciation = event_node(&mut graph, "rob_annunciation_mary");
        // LUK 1:27 -- book 41 (Luke), chapter 1, verse 27.
        graph.attests.push(attests(&espousal, 41, 1, 27));
        graph.attests.push(attests(&annunciation, 41, 1, 27));
        let err = attestation_is_exclusive(&graph).expect_err("a shared attestation must fail the build");
        assert!(err.contains("ATTESTATION EXCLUSIVITY (L2) VIOLATED"), "{err}");
        assert!(err.contains("theo-249") && err.contains("rob_annunciation_mary"), "the error must name BOTH events: {err}");
    }

    /// RED: a DECLARED pair that no longer collides is a failure of the
    /// REAL-corpus half of the law -- the curation queue can only shrink
    /// deliberately, with the shrink recorded. Uses a real row from the
    /// shipped inventory so the test cannot rot into asserting nothing.
    ///
    /// GREEN half, stated as the complement: the per-build pass
    /// (`attestation_is_exclusive`) must NOT fail on the same empty graph
    /// -- every small synthetic fixture in this workspace collides on
    /// nothing, and treating that as staleness would red every pipeline
    /// test in the repo. That split is the reason the two functions exist
    /// separately at all, so it is pinned here rather than only described.
    #[test]
    fn stale_declarations_fail_the_real_corpus_law_but_never_the_per_build_pass() {
        let first = crate::attestation_pending::PENDING.first().expect("the shipped inventory is non-empty -- if it ever empties, L2 is fully satisfied and this module should be deleted with it");
        // An EMPTY graph collides on nothing, so every declared row is
        // stale against it by construction.
        let err = attestation_inventory_has_no_stale_rows(&Graph::default()).expect_err("a declared pair that no longer collides must fail");
        assert!(err.contains("STALE DECLARATION"), "{err}");
        assert!(err.contains(first.a), "the error must name the stale pair: {err}");

        assert!(
            attestation_is_exclusive(&Graph::default()).is_ok(),
            "the PER-BUILD pass must stay green on a collision-free fixture graph -- otherwise every pipeline test in this workspace reds on an inventory that is about the real corpus, not about them"
        );
    }

    /// RED: an inventory whose declared verse count has drifted from the
    /// truth fails too -- a count that is merely "close" is a count
    /// nobody can trust.
    #[test]
    fn attestation_exclusivity_red_on_inventory_drift() {
        let row = crate::attestation_pending::PENDING.first().expect("the shipped inventory is non-empty");
        let mut graph = Graph::default();
        let a = event_node(&mut graph, row.a);
        let b = event_node(&mut graph, row.b);
        // Exactly ONE shared verse -- the real inventory row declares its
        // own true count, which is never 1 for a real mega-span overlap;
        // if it ever were, the guard below keeps this test honest.
        assert_ne!(row.shared_verses, 1, "pick a different row: this test needs a declared count that differs from 1");
        graph.attests.push(attests(&a, 40, 1, 1));
        graph.attests.push(attests(&b, 40, 1, 1));
        let err = attestation_is_exclusive(&graph).expect_err("a drifted count must fail the build");
        assert!(err.contains("INVENTORY DRIFT"), "{err}");
    }

    /// L4's companion gate: an Analogue joins two DISTINCT events, once.
    #[test]
    fn analogue_gate_green_then_red_on_a_self_loop_and_on_a_duplicate() {
        use atlas_graph_types::edge::Analogue;
        let mut graph = Graph::default();
        let a = event_node(&mut graph, "rob_leper_healed");
        let b = event_node(&mut graph, "mat_leper_healed");
        graph.analogue.push(Analogue { a: a.clone(), b: b.clone(), provenance: "test".into() });
        assert!(analogue_rows_join_two_distinct_events(&graph).is_ok(), "two distinct events is the whole point");

        let mut loops = Graph::default();
        let s = event_node(&mut loops, "rob_leper_healed");
        loops.analogue.push(Analogue { a: s.clone(), b: s, provenance: "test".into() });
        let err = analogue_rows_join_two_distinct_events(&loops).expect_err("a self-loop must be caught");
        assert!(err.contains("ITSELF"), "{err}");

        // The SAME unordered pair twice, written from opposite ends --
        // still a duplicate; `entry_id_symmetric` would give both rows the
        // same id and the index would double the edge.
        let mut dupes = Graph::default();
        let x = event_node(&mut dupes, "rob_leper_healed");
        let y = event_node(&mut dupes, "mat_leper_healed");
        dupes.analogue.push(Analogue { a: x.clone(), b: y.clone(), provenance: "test".into() });
        dupes.analogue.push(Analogue { a: y, b: x, provenance: "test".into() });
        let err2 = analogue_rows_join_two_distinct_events(&dupes).expect_err("a duplicate pair must be caught");
        assert!(err2.contains("duplicate"), "{err2}");
    }

    /// The referential half, for both ATTEST-1 endpoints: an `Analogue`
    /// naming an event with no node, and a `Mentions` row whose widened
    /// `Event` entity dangles.
    #[test]
    fn red_when_an_analogue_or_an_event_mention_names_a_missing_node() {
        use atlas_graph_types::edge::Analogue;
        let mut graph = Graph::default();
        let a = event_node(&mut graph, "rob_leper_healed");
        graph.analogue.push(Analogue { a, b: EventId::new("nowhere"), provenance: "test".into() });
        let err = every_authored_edge_resolves(&graph).expect_err("the dangling analogue end must be caught");
        assert_eq!(err.relation, "analogue");
        assert_eq!(err.field, "b");

        let mut graph2 = Graph::default();
        graph2.mentions.push(atlas_graph_types::edge::Mentions {
            locus: locus(),
            entity: MentionedEntity::Event(EventId::new("nowhere")),
            provenance: "test".into(),
        });
        let err2 = every_authored_edge_resolves(&graph2).expect_err("the dangling mentions.entity Event must be caught");
        assert_eq!(err2.relation, "mentions");
        assert_eq!(err2.field, "entity");
    }

    /// NODE1-ROWS-1's index≡rows conformance law, red case: a graph whose
    /// indexes were tampered with after build (the retired derived-merge
    /// class) fails; the same graph untampered passes.
    #[test]
    fn indexes_derive_exactly_from_rows_catches_a_post_build_write() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        let ch = container_node(&mut graph, "bible-chapter-GEN-1");
        graph.contains_bible.push(child_row(&book, &ch));
        graph.build_indexes();
        assert!(indexes_derive_exactly_from_rows(&graph).is_ok(), "an untampered graph must pass");

        // Tamper: append one extra entry the rows do not back.
        use atlas_graph_types::edge::{at, entry_id, RelationId};
        let s = at(&book.erase());
        let o = at(&ch.erase());
        let eid = entry_id(RelationId::Succession, &s, &o);
        graph
            .indexes
            .entry(RelationId::Succession)
            .or_default()
            .fwd
            .entry(s)
            .or_default()
            .push((eid, o, atlas_graph_types::explore::EdgeMeta::None));
        assert!(indexes_derive_exactly_from_rows(&graph).is_err(), "a post-build index write must be caught");
    }
}
