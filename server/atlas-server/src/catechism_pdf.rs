//! CATECH-V1: `GET /api/catechism/print` -- a real PDF of a catechism item,
//! or of a whole chief part, for sending home with the kids.
//!
//! WHY SERVER-SIDE (owner decision): the browser's own print dialog would
//! have been cheaper, but it prints what is on SCREEN -- one open item, at
//! whatever scroll position, with the app chrome to fight. The ask was to
//! print "at any given level... and all (or potentially filtered set of)
//! child nodes," which is a different document each time and mostly content
//! the screen is not showing. That is a document-generation job, so it lives
//! here, and the output is a file you can attach to an email to a parent
//! without explaining how to un-print a sidebar.
//!
//! FONTS: PDF's 14 built-in fonts (Times here) are used deliberately -- they
//! require no font file to be embedded, vendored, or licensed, so this
//! endpoint adds no binary assets to the repo and no attribution obligation
//! to LICENSES.md. The cost is that built-in fonts are WinAnsi-encoded, which
//! is why `sanitize` below folds the curated text's typographic characters
//! (em dashes, curly quotes) down to their ASCII equivalents rather than
//! emitting bytes the viewer would render as mojibake.
//!
//! LAYOUT: deliberately plain -- A4, one column, generous margins, no
//! images. Line breaking is an APPROXIMATION (see `wrap`): built-in font
//! metrics are not exposed by the pdf crate, so this estimates the average
//! glyph width as a fraction of the font size. The estimate is deliberately
//! conservative (it wraps slightly early rather than slightly late), because
//! a line that wraps early looks like a design choice and a line that
//! overruns the margin looks like a bug.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use atlas_core::data::{AtlasData, CatechismItem, CatechismPart};
use printpdf::*;
use serde::Deserialize;

use crate::error::ApiError;

/// A4, and the margins the body text lives inside.
const PAGE_W_MM: f32 = 210.0;
const PAGE_H_MM: f32 = 297.0;
const MARGIN_MM: f32 = 20.0;
const BOTTOM_MM: f32 = 18.0;

/// Point sizes. One scale, used everywhere, so a printout reads as one
/// document rather than a pile of independently-styled blocks.
const SIZE_TITLE: f32 = 18.0;
const SIZE_HEADING: f32 = 13.0;
const SIZE_LABEL: f32 = 8.5;
const SIZE_BODY: f32 = 11.0;

/// Average glyph width as a fraction of font size, for Times. Real Times
/// lowercase averages ~0.46-0.50em across ordinary English prose; 0.52 is
/// used here so estimation error wraps EARLY (inside the margin) rather than
/// late (past it). See this module's own header.
const AVG_GLYPH_EM: f32 = 0.52;

#[derive(Debug, Deserialize)]
pub struct PrintQuery {
    /// `item` (this item alone) or `part` (the whole chief part containing
    /// it -- the "all child nodes" case). Anything else is a 400 rather than
    /// a silent default, so a typo in a bookmarked print link is visible.
    #[serde(default)]
    pub scope: Option<String>,
    /// The item id the print is anchored at. Required in BOTH scopes: even
    /// `scope=part` is addressed by an item, because that is what the page
    /// has in hand when the teacher presses the button.
    #[serde(default)]
    pub item: Option<String>,
    /// Optional reflection-tier filter, matching the on-screen filter, so
    /// "print what I am looking at" is literally true.
    #[serde(default)]
    pub tier: Option<String>,
    /// `luther` (default) prints only the item's own embedded proof texts;
    /// `all` additionally prints every question-level proof verse. Default is
    /// deliberately the small one: an item can carry over a thousand
    /// question-level verses (creed-1 does), which is a research surface, not
    /// something to hand a nine-year-old.
    #[serde(default)]
    pub verses: Option<String>,
}

pub async fn catechism_print(
    State(data): State<Arc<AtlasData>>,
    Query(q): Query<PrintQuery>,
) -> Result<Response, ApiError> {
    let item_id = q.item.as_deref().unwrap_or_default();
    if item_id.is_empty() {
        return Err(ApiError::bad_request("print requires ?item={catechism item id}"));
    }
    let scope = q.scope.as_deref().unwrap_or("item");
    if scope != "item" && scope != "part" {
        return Err(ApiError::bad_request("scope must be 'item' or 'part'"));
    }
    let tier = match q.tier.as_deref() {
        None | Some("") | Some("all") => None,
        Some(t) if matches!(t, "child" | "youth" | "adult") => Some(t.to_string()),
        Some(_) => return Err(ApiError::bad_request("tier must be child, youth, adult, or all")),
    };
    let all_verses = matches!(q.verses.as_deref(), Some("all"));

    let (part, item) = data.catechism_item_by_id(item_id).ok_or_else(|| ApiError::not_found("catechism item"))?;

    let items: Vec<&CatechismItem> = if scope == "part" { part.items.iter().collect() } else { vec![item] };

    let doc_title = if scope == "part" { part.title.clone() } else { item.name.clone() };
    let bytes = render(&doc_title, part, &items, tier.as_deref(), all_verses, &data);

    // `inline` (not `attachment`): pressing Print should show the PDF, and
    // the browser's own viewer already offers Save and Print from there.
    let filename = format!("{}.pdf", slug(&doc_title));
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_DISPOSITION, format!("inline; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response())
}

/// The document builder. Walks items in curated order, emitting a heading
/// block and then whichever sections that item actually has -- the same
/// conditional-presence rule the UI follows (no empty "Where is this
/// written?" heading on an item that has none).
fn render(
    doc_title: &str,
    part: &CatechismPart,
    items: &[&CatechismItem],
    tier: Option<&str>,
    all_verses: bool,
    data: &AtlasData,
) -> Vec<u8> {
    let mut doc = PdfDocument::new(doc_title);
    let mut pages = Pages::new();

    pages.title(doc_title, &part.title, tier);

    // In `scope=item` the document title IS the item name, so repeating it as
    // a heading directly underneath just wastes the top of the page. In
    // `scope=part` the title is the part, and every item still needs its own
    // heading to be findable.
    let heading_per_item = items.len() > 1;

    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            pages.gap(6.0);
        }
        if heading_per_item {
            pages.heading(&item.name);
        }

        if let Some(text) = &item.text {
            pages.body_italic(text);
            pages.gap(2.0);
        }

        pages.label(&item.explanation_heading);
        for paragraph in item.explanation.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
            pages.body(paragraph);
            pages.gap(1.5);
        }

        if let Some(w) = &item.where_written {
            pages.gap(2.0);
            pages.label("Where is this written?");
            pages.body(w);
        }

        // Reflection prompts, in curated order, tier-filtered to match the
        // screen. A tier with no prompts on this item prints the heading
        // and says so -- silence would read as "this item has none at all."
        let prompts: Vec<_> =
            item.reflection.iter().filter(|r| tier.is_none_or(|t| r.tier.label() == t)).collect();
        pages.gap(3.0);
        let reflection_label = match tier {
            Some(t) => format!("Reflection questions ({t})"),
            None => "Reflection questions".to_string(),
        };
        pages.label(&reflection_label);
        if prompts.is_empty() {
            pages.body_italic("None at this level yet.");
        } else {
            for (n, r) in prompts.iter().enumerate() {
                // The per-prompt tier tag earns its space only when the list
                // actually mixes tiers; under a filter the heading already
                // said which tier this is.
                let line = if tier.is_some() {
                    format!("{}. {}", n + 1, r.prompt)
                } else {
                    format!("{}. [{}] {}", n + 1, r.tier.label(), r.prompt)
                };
                pages.body(&line);
                pages.gap(1.0);
            }
        }

        // Scripture. Luther's own embedded citations always; the
        // question-level ones only on request (see `PrintQuery::verses`).
        let mut refs: Vec<(String, Option<String>)> = item.verses.iter().map(|v| (v.clone(), None)).collect();
        if all_verses {
            for question in &item.questions {
                for v in &question.verses {
                    refs.push((v.clone(), Some(question.title.clone())));
                }
            }
        }
        if !refs.is_empty() {
            pages.gap(3.0);
            pages.label("The Scriptures");
            for (vref, question) in &refs {
                if let Some(text) = data.verses.get(vref) {
                    match question {
                        Some(q) => pages.body(&format!("{vref} ({q}) -- {text}")),
                        None => pages.body(&format!("{vref} -- {text}")),
                    }
                    pages.gap(0.8);
                }
            }
        }

        if !item.media.is_empty() {
            pages.gap(3.0);
            pages.label("Songs");
            for m in &item.media {
                pages.body(&format!("{} ({}) -- {}", m.title, m.kind.label(), m.url));
            }
        }
    }

    doc.with_pages(pages.finish()).save(&PdfSaveOptions::default(), &mut Vec::new())
}

/// A running cursor over a growing list of pages.
///
/// POSITIONING, and the trap in it: PDF's `Td` operator -- which is what
/// `Op::SetTextCursor` emits -- is RELATIVE. It moves to the start of a new
/// line offset from the CURRENT line's origin, not to an absolute point on
/// the page. Feeding it absolute coordinates therefore works exactly once
/// (the first line lands correctly) and then accumulates, throwing every
/// later line off the page -- which is precisely what the first version of
/// this module did: a one-line PDF with a page of white space under it.
///
/// So this tracks `y_mm` as the absolute position it WANTS (page-break
/// decisions need to reason absolutely), and emits the DIFFERENCE from the
/// last line actually drawn. `x` stays 0 in every relative move because
/// every line in this document shares one left margin.
struct Pages {
    finished: Vec<PdfPage>,
    ops: Vec<Op>,
    /// Where the next line should go, in absolute page coordinates.
    y_mm: f32,
    /// Where the last emitted line actually went, or None if no line has
    /// been drawn on this page yet (the next one is placed absolutely).
    last_drawn_y_mm: Option<f32>,
}

impl Pages {
    fn new() -> Self {
        Self {
            finished: Vec::new(),
            ops: Self::open_page(),
            y_mm: PAGE_H_MM - MARGIN_MM,
            last_drawn_y_mm: None,
        }
    }

    fn open_page() -> Vec<Op> {
        vec![Op::SaveGraphicsState, Op::StartTextSection]
    }

    fn break_page(&mut self) {
        let mut ops = std::mem::replace(&mut self.ops, Self::open_page());
        ops.push(Op::EndTextSection);
        ops.push(Op::RestoreGraphicsState);
        self.finished.push(PdfPage::new(Mm(PAGE_W_MM), Mm(PAGE_H_MM), ops));
        self.y_mm = PAGE_H_MM - MARGIN_MM;
        self.last_drawn_y_mm = None;
    }

    fn finish(mut self) -> Vec<PdfPage> {
        self.ops.push(Op::EndTextSection);
        self.ops.push(Op::RestoreGraphicsState);
        self.finished.push(PdfPage::new(Mm(PAGE_W_MM), Mm(PAGE_H_MM), self.ops));
        self.finished
    }

    fn gap(&mut self, mm: f32) {
        self.y_mm -= mm;
    }

    /// Move the text cursor to `self.y_mm`, absolutely for the first line on
    /// a page and relatively thereafter (see this struct's own doc comment).
    fn place_cursor(&mut self) {
        let pos = match self.last_drawn_y_mm {
            None => Point::new(Mm(MARGIN_MM), Mm(self.y_mm)),
            Some(last) => Point::new(Mm(0.0), Mm(self.y_mm - last)),
        };
        self.ops.push(Op::SetTextCursor { pos });
        self.last_drawn_y_mm = Some(self.y_mm);
    }

    /// One wrapped block of text in one font/size, breaking pages as needed.
    fn block(&mut self, text: &str, font: BuiltinFont, size_pt: f32, leading_mm: f32) {
        for line in wrap(&sanitize(text), size_pt) {
            if self.y_mm < BOTTOM_MM {
                self.break_page();
            }
            self.ops.push(Op::SetFont { font: PdfFontHandle::Builtin(font), size: Pt(size_pt) });
            self.place_cursor();
            self.ops.push(Op::ShowText { items: vec![TextItem::Text(line)] });
            self.y_mm -= leading_mm;
        }
    }

    fn title(&mut self, title: &str, part_title: &str, tier: Option<&str>) {
        self.block(part_title, BuiltinFont::Helvetica, SIZE_LABEL, 5.0);
        self.block(title, BuiltinFont::TimesBold, SIZE_TITLE, 9.0);
        if let Some(t) = tier {
            self.block(&format!("Reflection questions filtered to: {t}"), BuiltinFont::TimesItalic, SIZE_LABEL, 5.0);
        }
        self.gap(4.0);
    }

    /// A heading pulls its own page break when there is not enough room left
    /// for it plus a couple of lines under it -- a heading stranded alone at
    /// the foot of a page is the one layout artifact worth spending code on
    /// in a document this plain.
    fn heading(&mut self, text: &str) {
        if self.y_mm < BOTTOM_MM + 25.0 {
            self.break_page();
        }
        self.block(text, BuiltinFont::TimesBold, SIZE_HEADING, 7.0);
    }

    fn label(&mut self, text: &str) {
        self.block(&text.to_uppercase(), BuiltinFont::Helvetica, SIZE_LABEL, 5.0);
    }

    fn body(&mut self, text: &str) {
        self.block(text, BuiltinFont::TimesRoman, SIZE_BODY, 5.4);
    }

    fn body_italic(&mut self, text: &str) {
        self.block(text, BuiltinFont::TimesItalic, SIZE_BODY, 5.4);
    }
}

/// Greedy word wrap to an estimated character budget. See `AVG_GLYPH_EM`.
/// A single word longer than the budget is emitted on its own overlong line
/// rather than hyphenated -- the only things that long here are URLs, and a
/// broken URL is worse than a wide one.
fn wrap(text: &str, size_pt: f32) -> Vec<String> {
    let usable_pt = (PAGE_W_MM - 2.0 * MARGIN_MM) * 72.0 / 25.4;
    let budget = (usable_pt / (size_pt * AVG_GLYPH_EM)).floor().max(8.0) as usize;

    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= budget {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut line));
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Fold the curated text's typographic characters into WinAnsi-safe ASCII.
/// The curated catechism genuinely contains em dashes (every Creed article
/// name) and the Triglotta's own punctuation; emitting them raw through a
/// built-in font would render as mojibake in a viewer.
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{2014}' | '\u{2013}' => '-',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2026}' => '.',
            '\u{00A0}' => ' ',
            other => other,
        })
        .collect()
}

/// Filename-safe slug for the Content-Disposition name.
fn slug(text: &str) -> String {
    let s: String = text
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let trimmed = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if trimmed.is_empty() { "catechism".to_string() } else { trimmed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_respects_the_character_budget() {
        let text = "word ".repeat(80);
        let lines = wrap(text.trim(), SIZE_BODY);
        assert!(lines.len() > 1, "long text must wrap");
        let usable_pt = (PAGE_W_MM - 2.0 * MARGIN_MM) * 72.0 / 25.4;
        let budget = (usable_pt / (SIZE_BODY * AVG_GLYPH_EM)).floor() as usize;
        for line in &lines {
            assert!(line.chars().count() <= budget, "line over budget: {line}");
        }
    }

    #[test]
    fn wrap_keeps_an_overlong_word_intact() {
        let url = "https://example.com/".to_string() + &"a".repeat(200);
        let lines = wrap(&url, SIZE_BODY);
        assert_eq!(lines.len(), 1, "a single long token must not be split");
        assert_eq!(lines[0], url);
    }

    #[test]
    fn sanitize_folds_typographic_punctuation() {
        assert_eq!(sanitize("The First Article \u{2014} Of Creation"), "The First Article - Of Creation");
        assert_eq!(sanitize("\u{201C}for you\u{201D}"), "\"for you\"");
    }

    #[test]
    fn slug_is_filename_safe() {
        assert_eq!(slug("What Does Baptism Give or Profit?"), "what-does-baptism-give-or-profit");
        assert_eq!(slug("???"), "catechism");
    }
}
