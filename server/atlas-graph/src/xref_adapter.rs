//! The cross-references source adapter: turns openbible.info's raw
//! cross-references TSV into `cites` (TextUnit -> TextUnit) rows -- the
//! first relation proving the generic edge path end-to-end (controller
//! ruling 3).
//!
//! Reuses `atlas_etl::xrefs::parse` + `atlas_etl::xrefs::filter_missing_first_verse`
//! verbatim (the SAME raw-TSV parsing/filtering pipeline `atlas-etl`'s own
//! binary runs to produce `data/compiled/cross-refs.json`) as this adapter's
//! reader -- mirroring the KJV adapter's reuse (`kjv_adapter`'s own doc
//! comment). No parsing logic is forked or duplicated.
//!
//! DISCLOSED DECISION 1: a cross-ref TARGET may be a same-chapter or
//! cross-book/chapter SPAN (`"COL.1.16-19"`, `"MAT.5.3-MAT.6.2"`), not a
//! single verse -- graph-types' `CrossRef` row is `TextLocus -> TextLocus`
//! (a single locus each; design doc §4's edge-kind table: "verse-level
//! today, loci by design"). This adapter targets the span's FIRST verse,
//! mirroring `atlas_server::handlers::first_verse_of_target`/
//! `atlas_core::xrefs::target_span`'s own pre-existing "preview" convention
//! exactly -- not a new precedent.
//!
//! DISCLOSED DECISION 2: graph-types' `CrossRef.votes` is `u32` (a shape the
//! M-A brief holds fixed, per the "types are owner-approved" law).
//! openbible.info's own vote column is signed, and a small fraction of real
//! rows (1,241 of 344,799 in the committed raw TSV, ~0.36%) carry negative
//! votes. A negative vote cannot be cast into `u32` without corrupting its
//! sign (clamping to 0 or taking the magnitude would both misstate the
//! source), so those rows are DROPPED at the adapter boundary -- counted,
//! never silent -- consistent with "where no assertion or derivation
//! speaks, no edge exists" (design doc P2).

use std::collections::HashMap;

use atlas_core::data::CrossRef as CoreCrossRef;
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef as GVerseRef};

/// One `cites`-ready row, lowered from a raw (from, target, votes) triple.
pub struct XrefRow {
    pub from: TextLocus,
    pub to: TextLocus,
    pub votes: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct XrefAdapterStats {
    /// Disclosed decision 2 above.
    pub dropped_negative_votes: usize,
}

/// Extracts a cross-ref TARGET's first verse -- mirrors
/// `atlas_server::handlers::first_verse_of_target` / `atlas_core::xrefs::target_span`'s
/// three-shape parse (single verse / same-chapter range / cross-chapter
/// range) over an ALREADY-CANONICALIZED target string (3-letter codes,
/// `atlas_etl::xrefs::parse`'s own output shape) -- duplicated rather than
/// imported because those two live in atlas-server/atlas-core respectively
/// and this is the same small, stable, three-branch parse already
/// duplicated twice in this codebase for analogous reasons.
fn first_verse_of_target(target: &str) -> Option<VerseId> {
    if let Ok(v) = VerseId::parse_canonical(target) {
        return Some(v);
    }
    if let Ok(ScriptureRef::Passage { book, chapter, from_verse, .. }) = ScriptureRef::parse(target) {
        return Some(VerseId { book, chapter, verse: from_verse });
    }
    let (left, _right) = target.split_once('-')?;
    VerseId::parse_canonical(left).ok()
}

fn text_locus(v: VerseId) -> TextLocus {
    let vr = GVerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse };
    TextLocus::from(BibleLocus::whole(vr))
}

/// Reads and parses the raw TSV (via `atlas_etl::xrefs::parse` +
/// `filter_missing_first_verse`, against `verses` -- the SAME dot-ref-keyed
/// map the KJV adapter just built, so "first verse of target must exist"
/// checks against the SAME graph this row will join, not a stale compiled
/// artifact), then lowers every surviving row into a `cites`-ready triple.
/// Self-references and structurally unparseable rows are already dropped by
/// `atlas_etl::xrefs::parse` itself (counted there, not here); negative-vote
/// rows are dropped here (disclosed decision 2, module doc comment).
/// Deterministic order: by `from` key, then each `from`'s own
/// votes-descending order (`atlas_etl::xrefs::parse`'s own per-from sort).
pub fn read_xrefs_ordered(
    raw_tsv: &str,
    verses: &HashMap<String, String>,
) -> anyhow::Result<(Vec<XrefRow>, XrefAdapterStats)> {
    let (map, _parse_stats) = atlas_etl::xrefs::parse(raw_tsv)?;
    let (map, _dropped_missing_first_verse) = atlas_etl::xrefs::filter_missing_first_verse(map, verses);

    let mut froms: Vec<&String> = map.keys().collect();
    froms.sort();

    let mut rows = Vec::new();
    let mut stats = XrefAdapterStats::default();
    for from_key in froms {
        let Ok(from_v) = VerseId::parse_canonical(from_key) else { continue };
        let Some(refs) = map.get(from_key) else { continue };
        for cr in refs {
            let CoreCrossRef { target, votes } = cr;
            if *votes < 0 {
                stats.dropped_negative_votes += 1;
                continue;
            }
            let Some(to_v) = first_verse_of_target(target) else { continue };
            rows.push(XrefRow { from: text_locus(from_v), to: text_locus(to_v), votes: *votes as u32 });
        }
    }
    Ok((rows, stats))
}

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`, run
/// AFTER `kjv_adapter::normalize` within the same pass -- xref rows need
/// the KJV nodes' own dot-ref-keyed text to exist first): parses
/// `ctx.xrefs_tsv` and lowers surviving rows into `cites`-ready `CrossRef`
/// rows on `ctx.graph`. Mirrors exactly what
/// `build::build_graph_from_canon_and_verses`'s own middle block did
/// before the pipeline restructuring, with ONE deliberate difference: the
/// dot-ref-keyed verse-text map it checks "first verse of target exists"
/// against is rebuilt by walking the JUST-NORMALIZED graph nodes (not a
/// second parameter threaded alongside `ctx.kjv_verses`) -- this is the
/// SAME set of verses `ordered_verses_from_canon` actually inserted (the
/// filtered, canon-walked set, not `ctx.kjv_verses` as originally handed
/// in, which could theoretically carry a stray key no chapter/verse slot
/// ever reached), so behavior is unchanged, not merely similar.
pub fn normalize(ctx: &mut crate::pipeline::BuildCtx) -> anyhow::Result<()> {
    use atlas_graph_types::node::NodePayload;
    use atlas_graph_types::text::TranslationId;

    let verses_by_ref: HashMap<String, String> = ctx
        .graph
        .reading
        .get(crate::kjv_adapter::BIBLE_CORPUS)
        .map(|spine| {
            spine
                .order
                .iter()
                .filter_map(|id| {
                    let (b, c, v) = crate::kjv_adapter::decode_text_unit(id)?;
                    let text = match &ctx.graph.nodes.get(id)?.payload {
                        NodePayload::TextUnit { renderings, .. } => {
                            renderings.get(&TranslationId(crate::kjv_adapter::KJV_TRANSLATION.to_string())).cloned()
                        }
                        _ => None,
                    }?;
                    Some((crate::kjv_adapter::dot_ref(b, c, v), text))
                })
                .collect()
        })
        .unwrap_or_default();

    let (xref_rows, xref_stats) = read_xrefs_ordered(ctx.xrefs_tsv, &verses_by_ref)?;
    for row in &xref_rows {
        ctx.graph.cross_refs.push(atlas_graph_types::edge::CrossRef {
            from: row.from.clone(),
            to: row.to.clone(),
            votes: row.votes,
            provenance: "openbible.info-cross-references".to_string(),
        });
    }
    ctx.stats.cites_rows = xref_rows.len();
    ctx.stats.cites_dropped_negative_votes = xref_stats.dropped_negative_votes;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TSV: &str = "From Verse\tTo Verse\tVotes\t#comment\n\
Gen.1.1\tGen.1.1\t50\n\
Gen.1.1\tJob.26.13\t20\n\
Gen.1.1\tCol.1.16-Col.1.17\t7\n\
Gen.1.1\tPs.124.8\t3\n\
Gen.1.1\tNotARealRef\t15\n\
Col.1.16\tCol.1.16-Col.1.19\t12\n\
Matt.5.3\tMatt.5.3-Matt.6.2\t5\n\
Rev.22.21\tRom.16.23\t-2\n";

    fn sample_verses() -> HashMap<String, String> {
        let mut v = HashMap::new();
        // ROM.16.23 (the negative-vote row's own target) is included
        // deliberately -- it must survive atlas_etl::xrefs::filter_missing_first_verse
        // so the negative-votes test below actually exercises THIS adapter's
        // own disclosed-decision-2 drop, not upstream's unrelated
        // missing-first-verse drop.
        for key in ["GEN.1.1", "JOB.26.13", "COL.1.16", "PSA.124.8", "MAT.5.3", "REV.22.21", "ROM.16.23"] {
            v.insert(key.to_string(), format!("text of {key}"));
        }
        v
    }

    #[test]
    fn self_reference_and_unparseable_target_are_dropped_upstream() {
        let (rows, _stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        // Gen.1.1 -> Gen.1.1 (self) and Gen.1.1 -> NotARealRef (unparseable)
        // are both dropped by atlas_etl::xrefs::parse itself, before this
        // adapter ever sees them.
        let gen_targets: Vec<_> =
            rows.iter().filter(|r| r.from == text_locus(VerseId::parse_canonical("GEN.1.1").unwrap())).collect();
        assert_eq!(gen_targets.len(), 3, "JOB.26.13, COL.1.16 (first verse of the range), PSA.124.8");
    }

    #[test]
    fn range_target_resolves_to_its_first_verse() {
        // Col.1.16 -> Col.1.16-Col.1.19: a range target whose own first verse
        // is the citing verse itself. atlas_etl::xrefs::parse's self-check
        // only fires for a SINGLE-verse target identical to `from` (its own
        // `is_single` guard), so this range survives upstream; this adapter
        // then resolves it to that first verse per disclosed decision 1.
        let (rows, _stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        let col_row = rows
            .iter()
            .find(|r| r.from == text_locus(VerseId::parse_canonical("COL.1.16").unwrap()))
            .expect("Col.1.16 -> Col.1.16-Col.1.19 must survive");
        assert_eq!(col_row.to, text_locus(VerseId::parse_canonical("COL.1.16").unwrap()));
    }

    #[test]
    fn negative_votes_are_dropped_and_counted_disclosed_decision_2() {
        let (rows, stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        assert_eq!(stats.dropped_negative_votes, 1, "Rev.22.21 -> Rom.16.23 carries votes=-2");
        assert!(
            !rows.iter().any(|r| r.from == text_locus(VerseId::parse_canonical("REV.22.21").unwrap())),
            "the negative-vote row must not appear in the output at all"
        );
    }

    #[test]
    fn positive_votes_cast_losslessly() {
        let (rows, _stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        let job_row = rows
            .iter()
            .find(|r| r.to == text_locus(VerseId::parse_canonical("JOB.26.13").unwrap()))
            .expect("Gen.1.1 -> Job.26.13 must survive");
        assert_eq!(job_row.votes, 20);
    }
}
