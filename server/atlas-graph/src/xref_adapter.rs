//! openbible.info's raw cross-references TSV lowered into `cites` rows. A target may be a span, and
//! this adapter cites the span's FIRST verse. A row carrying NEGATIVE votes is dropped and counted:
//! `CrossRef.votes` is unsigned, and clamping or taking the magnitude would misstate the source.

use std::collections::HashMap;

use atlas_core::data::CrossRef as CoreCrossRef;
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef as GVerseRef};

/// One `cites`-ready row. `to_last`/`target_display` carry the honest resolution of a span target,
/// while `to` alone stays the graph's own edge endpoint.
pub struct XrefRow {
    pub from: TextLocus,
    pub to: TextLocus,
    pub to_last: Option<TextLocus>,
    pub target_display: String,
    pub votes: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct XrefAdapterStats {
    pub dropped_negative_votes: usize,
}

/// A target's own (first, last) verse endpoints, over an ALREADY-CANONICALIZED target string: the
/// three shapes are a single verse, a same-chapter range and a cross-chapter range. Duplicated
/// rather than imported, since the analogous parses live in atlas-server and atlas-core.
fn target_span(target: &str) -> Option<(VerseId, VerseId)> {
    if let Ok(v) = VerseId::parse_canonical(target) {
        return Some((v, v));
    }
    if let Ok(ScriptureRef::Passage { book, chapter, from_verse, to_verse }) = ScriptureRef::parse(target) {
        return Some((VerseId { book, chapter, verse: from_verse }, VerseId { book, chapter, verse: to_verse }));
    }
    let (left, right) = target.split_once('-')?;
    let lv = VerseId::parse_canonical(left).ok()?;
    let rv = VerseId::parse_canonical(right).ok()?;
    Some((lv, rv))
}

fn text_locus(v: VerseId) -> TextLocus {
    let vr = GVerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse };
    TextLocus::from(BibleLocus::whole(vr))
}

/// "First verse of target exists" is checked against the SAME dot-ref map the KJV adapter just
/// built, so it agrees with the graph this row joins rather than a stale artifact. Order is
/// deterministic: by `from` key, then that key's own votes-descending order.
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
            let Some((to_v, last_v)) = target_span(target) else { continue };
            let to_last = if last_v != to_v { Some(text_locus(last_v)) } else { None };
            rows.push(XrefRow {
                from: text_locus(from_v),
                to: text_locus(to_v),
                to_last,
                target_display: target.clone(),
                votes: *votes as u32,
            });
        }
    }
    Ok((rows, stats))
}

/// Runs after `kjv_adapter::normalize` within the same pass: xref rows need the KJV nodes' own
/// dot-ref-keyed text to exist. The map checked against is rebuilt by walking the just-normalized
/// nodes, which is exactly the filtered set that adapter inserted.
pub fn normalize(ctx: &mut crate::pipeline::BuildCtx) -> anyhow::Result<()> {
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
                    let text = crate::kjv_adapter::kjv_text(ctx.graph.nodes.get(id)?)?;
                    Some((crate::kjv_adapter::dot_ref(b, c, v), text.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let (xref_rows, xref_stats) = read_xrefs_ordered(ctx.xrefs_tsv, &verses_by_ref)?;
    for row in &xref_rows {
        ctx.graph.cross_refs.push(atlas_graph_types::edge::CrossRef {
            from: row.from.clone(),
            to: row.to.clone(),
            to_last: row.to_last.clone(),
            target_display: row.target_display.clone(),
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
        for key in ["GEN.1.1", "JOB.26.13", "COL.1.16", "PSA.124.8", "MAT.5.3", "REV.22.21", "ROM.16.23"] {
            v.insert(key.to_string(), format!("text of {key}"));
        }
        v
    }

    #[test]
    fn self_reference_and_unparseable_target_are_dropped_upstream() {
        let (rows, _stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        let gen_targets: Vec<_> =
            rows.iter().filter(|r| r.from == text_locus(VerseId::parse_canonical("GEN.1.1").unwrap())).collect();
        assert_eq!(gen_targets.len(), 3, "JOB.26.13, COL.1.16 (first verse of the range), PSA.124.8");
    }

    #[test]
    fn range_target_resolves_to_its_first_verse() {
        let (rows, _stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        let col_row = rows
            .iter()
            .find(|r| r.from == text_locus(VerseId::parse_canonical("COL.1.16").unwrap()))
            .expect("Col.1.16 -> Col.1.16-Col.1.19 must survive");
        assert_eq!(col_row.to, text_locus(VerseId::parse_canonical("COL.1.16").unwrap()));
        assert_eq!(
            col_row.to_last,
            Some(text_locus(VerseId::parse_canonical("COL.1.19").unwrap())),
            "a same-chapter range target's own to_last must be its real LAST verse, not absent"
        );
        assert_eq!(
            col_row.target_display, "COL.1.16-19",
            "target_display must be the compressed canonical citation string, never re-synthesized from to/to_last"
        );
    }

    #[test]
    fn cross_chapter_range_target_carries_its_own_full_to_last_and_display_string() {
        let (rows, _stats) = read_xrefs_ordered(SAMPLE_TSV, &sample_verses()).unwrap();
        let matt_row = rows
            .iter()
            .find(|r| r.from == text_locus(VerseId::parse_canonical("MAT.5.3").unwrap()))
            .expect("Matt.5.3 -> Matt.5.3-Matt.6.2 must survive");
        assert_eq!(matt_row.to, text_locus(VerseId::parse_canonical("MAT.5.3").unwrap()));
        assert_eq!(
            matt_row.to_last,
            Some(text_locus(VerseId::parse_canonical("MAT.6.2").unwrap())),
            "a cross-chapter range target's own to_last must be its real LAST verse"
        );
        assert_eq!(matt_row.target_display, "MAT.5.3-MAT.6.2", "cross-chapter form never compresses");
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
        assert_eq!(job_row.to_last, None, "a single-verse target has no LAST verse distinct from its own to");
        assert_eq!(job_row.target_display, "JOB.26.13");
    }
}
