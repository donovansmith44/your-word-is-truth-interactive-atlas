//! `bibex verse <ref>` -- text + red-letter marks + attached
//! places/persons/events. See CONTRACT.md's own "bibex verse" section.
//! Ref decoded via `graph_wire::decode_node_id("text-unit:" + ref)` -- the
//! SAME locus grammar `/api/text`/`/api/node` accept on the wire, reused
//! verbatim rather than hand-parsed (R2/R1).

use atlas_core::data::AtlasData;
use atlas_core::history::resolve_display_name;
use atlas_graph::window;
use atlas_graph::GraphService;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_server::graph_wire::{decode_node_id, encode_node_id};

use crate::error::CliError;

/// BIBEX-1 addendum (ticket 2, ruling 1, "IDS EVERYWHERE"): one
/// associated-entity row -- a wire-encoded node id (`graph_wire::
/// encode_node_id`, the SAME grammar `bibex node <id>` decodes, so "see it
/// -> use it" always closes) plus its display label.
struct Attached {
    id: String,
    label: String,
}

fn attached(kind: NodeKind, raw: &str, label: impl Into<String>) -> Attached {
    Attached { id: encode_node_id(&AnyNodeId { kind, raw: raw.to_string() }), label: label.into() }
}

/// Plain-mode rendering of one section's own list: `"name [id]"` pairs,
/// comma-joined, or the literal `(none)` -- the SAME empty-result
/// discipline `CONTRACT.md`'s own "bibex verse" section already
/// establishes, now with the addendum's own bracketed-id suffix on each
/// entry (CONTRACT.md's own "ID discoverability" section fixes this exact
/// convention).
fn render_section(items: &[Attached]) -> String {
    if items.is_empty() {
        "(none)".to_string()
    } else {
        items.iter().map(|a| format!("{} [{}]", a.label, a.id)).collect::<Vec<_>>().join(", ")
    }
}

fn attached_to_json(items: &[Attached]) -> serde_json::Value {
    serde_json::Value::Array(items.iter().map(|a| serde_json::json!({"id": a.id, "label": a.label})).collect())
}

/// Renders any red-letter spans (`(start, end)` byte offsets into `text`)
/// as inline `[...]` brackets -- CONTRACT.md's own "red-letter marks shown
/// inline" wording. Spans are non-overlapping and sorted (the same
/// invariant `red_letter_spans.rs` itself establishes for the compiled
/// table), so a single left-to-right pass suffices.
pub(crate) fn mark_red_letter(text: &str, spans: &[(usize, usize)]) -> String {
    if spans.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + spans.len() * 2);
    let mut pos = 0usize;
    for &(start, end) in spans {
        if start > pos && start <= text.len() {
            out.push_str(&text[pos..start]);
        }
        let end = end.min(text.len());
        let start = start.min(end);
        out.push('[');
        out.push_str(&text[start..end]);
        out.push(']');
        pos = end;
    }
    if pos < text.len() {
        out.push_str(&text[pos..]);
    }
    out
}

/// The Bible half's own resolved shape, shared by `run` (plain) and
/// `run_json` -- ONE resolution, TWO renderings, `node.rs`'s/`edges.rs`'s
/// own discipline.
struct ResolvedKjvVerse {
    sref: String,
    text: String,
    spans: Vec<(usize, usize)>,
    places: Vec<Attached>,
    persons: Vec<Attached>,
    events: Vec<Attached>,
    passages: Vec<Attached>,
}

fn resolve_kjv(graph: &GraphService, data: &AtlasData, ref_raw: &str, text_id: &AnyNodeId, book: u8, chapter: u16, verse: u16) -> Result<ResolvedKjvVerse, CliError> {
    let sref = atlas_graph::kjv_adapter::dot_ref(book, chapter, verse);
    let snap = graph.snapshot();
    let text = window::render(&snap, text_id).ok_or_else(|| {
        CliError::not_found(format!("no text for '{ref_raw}'"), "the reference parsed fine but this graph has no verse with that book/chapter/verse", "check the verse number is within that chapter's real length")
    })?;
    let spans = graph.red_letter_spans.get(&sref).cloned().unwrap_or_default();

    let places: Vec<Attached> = data
        .places_for_verse(&sref)
        .iter()
        .filter_map(|pid| data.place_by_id(pid).map(|p| (pid, p)))
        .map(|(pid, p)| {
            let name = resolve_display_name(&p.name, data.place_history_for(&p.id), None, data.place_name_alias_for(&p.id));
            attached(NodeKind::Place, pid, name)
        })
        .collect();

    let persons: Vec<Attached> = graph.persons_by_verse.get(&sref).map(|v| v.iter().map(|(pid, label)| attached(NodeKind::Person, pid, label.clone())).collect()).unwrap_or_default();

    // Batch PERI-1 (PRESENTATION CATEGORY LAW -- owner, verbatim: "NUN is
    // not an event. fix this error and others like it"): SPLIT by
    // `Event::kind`, which was already on every row -- `Events:` keeps only
    // `kind == "event"` (a real, dated/placed passage); `Passages:` is
    // `kind == "general"` (a dateless pericope/literary-structure passage --
    // a Psalm acrostic stanza, an epistle outline pericope, the owner's own
    // PSA.119.105/GAL.1.8 repros). Each independently empty when its own
    // kind has zero entries -- the SAME empty-result discipline as before,
    // now applied per-kind. See CONTRACT.md's own `bibex verse` section.
    let all_events: Vec<&atlas_core::data::Event> = data.events_for_verse(&sref).iter().filter_map(|eid| data.event_by_id(eid)).collect();
    let events: Vec<Attached> = all_events.iter().filter(|e| e.kind == "event").map(|e| attached(NodeKind::Event, &e.id, e.label.clone())).collect();
    let passages: Vec<Attached> = all_events.iter().filter(|e| e.kind == "general").map(|e| attached(NodeKind::Event, &e.id, e.label.clone())).collect();

    Ok(ResolvedKjvVerse { sref, text, spans, places, persons, events, passages })
}

pub fn run(graph: &GraphService, data: &AtlasData, ref_raw: &str) -> Result<String, CliError> {
    let wire = format!("text-unit:{ref_raw}");
    let text_id = decode_node_id(&wire).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{ref_raw}' is not a valid verse/Concord reference"),
            "expected BOOK.CHAPTER.VERSE (e.g. GEN.1.1) or \"BoC PART.ARTICLE.PARAGRAPH\"",
            "check the book code and the dot-separated parts",
        )
    })?;

    if let Some((book, chapter, verse)) = atlas_graph::kjv_adapter::decode_text_unit(&text_id) {
        let v = resolve_kjv(graph, data, ref_raw, &text_id, book, chapter, verse)?;
        let marked = mark_red_letter(&v.text, &v.spans);

        let mut out = String::new();
        out.push_str(&format!("{}  {marked}\n\n", v.sref));
        out.push_str("Places:  ");
        out.push_str(&render_section(&v.places));
        out.push('\n');
        out.push_str("Persons: ");
        out.push_str(&render_section(&v.persons));
        out.push('\n');
        out.push_str("Events:  ");
        out.push_str(&render_section(&v.events));
        out.push('\n');
        out.push_str("Passages: ");
        out.push_str(&render_section(&v.passages));
        out.push('\n');

        Ok(out)
    } else if let Some((part, article, paragraph)) = atlas_graph::concord_adapter::decode_text_unit(&text_id) {
        let snap = graph.snapshot();
        let text = window::render_layer(&snap, &text_id, atlas_graph::concord_adapter::CONCORD_TRANSLATION).ok_or_else(|| {
            CliError::not_found(
                format!("no text for 'BoC {part}.{article}.{paragraph}'"),
                "the reference parsed fine but this graph has no Concord paragraph at that part/article/paragraph",
                "check the part/article/paragraph numbers",
            )
        })?;
        let mut out = String::new();
        out.push_str(&format!("BoC {part}.{article}.{paragraph}  {text}\n\n"));
        out.push_str("Places/Persons/Events/Passages: not tracked for the Book of Concord\n");
        Ok(out)
    } else {
        // decode_node_id succeeded (it's a well-shaped text-unit id) but
        // neither adapter recognizes it -- structurally unreachable given
        // decode_node_id's own two arms, kept as a named, honest error
        // rather than a panic (fail-loud even on a path this graph's own
        // grammar should never actually produce).
        Err(CliError::bad_ref(
            format!("'{ref_raw}' did not resolve to a Bible or Concord locus"),
            "the id decoded as a text-unit but matched neither adapter",
            "check the reference against CONTRACT.md's own grammar",
        ))
    }
}

/// BIBEX-1 (--json mode): a KJV verse is `{ref, text, tracked: true,
/// words_of_christ, places, persons, events, passages}` (`words_of_christ`
/// reusing the SAME `{start,end}` shape the AQC corpus/`/api/text` already
/// use; `places`/`persons`/`events`/`passages` each `[{id, label}]`, the
/// addendum's own ticket-2 shape); a Concord locus is the leaner `{ref,
/// text, tracked: false}` -- `tracked` is a NEW field (no established wire
/// precedent covers "this corpus does/doesn't carry these sections" as a
/// single flag), disclosed in CONTRACT.md rather than left for a caller to
/// infer from field absence.
pub fn run_json(graph: &GraphService, data: &AtlasData, ref_raw: &str) -> Result<serde_json::Value, CliError> {
    let wire = format!("text-unit:{ref_raw}");
    let text_id = decode_node_id(&wire).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{ref_raw}' is not a valid verse/Concord reference"),
            "expected BOOK.CHAPTER.VERSE (e.g. GEN.1.1) or \"BoC PART.ARTICLE.PARAGRAPH\"",
            "check the book code and the dot-separated parts",
        )
    })?;

    if let Some((book, chapter, verse)) = atlas_graph::kjv_adapter::decode_text_unit(&text_id) {
        let v = resolve_kjv(graph, data, ref_raw, &text_id, book, chapter, verse)?;
        let woc: Vec<_> = v.spans.iter().map(|&(s, e)| serde_json::json!({"start": s, "end": e})).collect();
        Ok(serde_json::json!({
            "ref": v.sref,
            "text": v.text,
            "tracked": true,
            "words_of_christ": woc,
            "places": attached_to_json(&v.places),
            "persons": attached_to_json(&v.persons),
            "events": attached_to_json(&v.events),
            "passages": attached_to_json(&v.passages),
        }))
    } else if let Some((part, article, paragraph)) = atlas_graph::concord_adapter::decode_text_unit(&text_id) {
        let snap = graph.snapshot();
        let text = window::render_layer(&snap, &text_id, atlas_graph::concord_adapter::CONCORD_TRANSLATION).ok_or_else(|| {
            CliError::not_found(
                format!("no text for 'BoC {part}.{article}.{paragraph}'"),
                "the reference parsed fine but this graph has no Concord paragraph at that part/article/paragraph",
                "check the part/article/paragraph numbers",
            )
        })?;
        Ok(serde_json::json!({"ref": format!("BoC {part}.{article}.{paragraph}"), "text": text, "tracked": false}))
    } else {
        Err(CliError::bad_ref(
            format!("'{ref_raw}' did not resolve to a Bible or Concord locus"),
            "the id decoded as a text-unit but matched neither adapter",
            "check the reference against CONTRACT.md's own grammar",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_red_letter_brackets_a_single_span() {
        assert_eq!(mark_red_letter("Jesus wept.", &[(6, 10)]), "Jesus [wept].");
    }

    #[test]
    fn mark_red_letter_is_identity_with_no_spans() {
        assert_eq!(mark_red_letter("plain text", &[]), "plain text");
    }

    #[test]
    fn mark_red_letter_brackets_multiple_non_overlapping_spans() {
        assert_eq!(mark_red_letter("I am the way.", &[(0, 1), (9, 12)]), "[I] am the [way].");
    }
}
