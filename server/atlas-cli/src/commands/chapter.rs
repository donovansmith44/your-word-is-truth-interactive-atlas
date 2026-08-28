//! `atlas chapter <ref>` -- every verse in a KJV chapter, one line each.
//! Ref parsed via `atlas_core::refs::ScriptureRef::parse` (the shared
//! locus parser, not a hand-rolled one -- R2). Concord is deliberately
//! NOT accepted here (CONTRACT.md's own "atlas chapter" section: a
//! Concord article's own paragraph count varies too widely for a
//! server-derived chapter span to mean anything consistent --
//! `graph_handlers::text_window` rejects the identical combination for
//! the same reason, and this command inherits that disclosed scope
//! limit rather than inventing its own answer).

use atlas_core::refs::ScriptureRef;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;

use crate::error::CliError;

pub fn run(graph: &GraphService, ref_raw: &str) -> Result<String, CliError> {
    let (book, chapter) = match ScriptureRef::parse(ref_raw) {
        Ok(ScriptureRef::Chapter { book, chapter }) => (book, chapter),
        _ => {
            return Err(CliError::bad_ref(
                format!("'{ref_raw}' is not a valid chapter reference"),
                "expected BOOK.CHAPTER (e.g. GEN.1) -- not a bare book, a verse, or a Concord citation",
                "drop any verse number, or check the book code",
            ))
        }
    };

    let (start, n) = graph.chapter_span(book.0, chapter).ok_or_else(|| {
        CliError::not_found(
            format!("no chapter '{ref_raw}'"),
            "the reference parsed fine but this graph has no such book/chapter combination",
            "check the chapter number is within that book's real length",
        )
    })?;

    let snap = graph.snapshot();
    let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward);

    let mut out = String::new();
    for id in &ids {
        if let Some((b, c, v)) = atlas_graph::kjv_adapter::decode_text_unit(id) {
            let sref = atlas_graph::kjv_adapter::dot_ref(b, c, v);
            let text = window::render(&snap, id).unwrap_or_default();
            let spans = graph.red_letter_spans.get(&sref).cloned().unwrap_or_default();
            out.push_str(&format!("{sref}  {}\n", super::verse::mark_red_letter(&text, &spans)));
        }
    }
    Ok(out)
}
