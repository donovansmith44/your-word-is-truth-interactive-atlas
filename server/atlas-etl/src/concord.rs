//! The Book of Concord parser: one parser over the ten vendored document-root pages, each of which already
//! carries its articles' full text inline. A paragraph boundary is a marker span carrying the source's own
//! visible label; a group with no usable digit takes the previous number plus one, disclosed, never guessed.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

/// One document's identity: its part number, its vendored filename stem, and its display title. The part
/// numbering is the traditional bound order of a printed edition, with the Formula's two texts as two
/// consecutive parts, since they are two different texts rather than one.
#[derive(Debug, Clone, Copy)]
pub struct ConcordDocSpec {
    pub part: u8,
    pub key: &'static str,
    pub title: &'static str,
}

pub const DOCUMENTS: &[ConcordDocSpec] = &[
    ConcordDocSpec { part: 1, key: "preface", title: "Preface to the Book of Concord" },
    ConcordDocSpec { part: 2, key: "ecumenical-creeds", title: "The Three Ecumenical Creeds" },
    ConcordDocSpec { part: 3, key: "augsburg-confession", title: "The Augsburg Confession" },
    ConcordDocSpec { part: 4, key: "defense", title: "Apology of the Augsburg Confession" },
    ConcordDocSpec { part: 5, key: "smalcald-articles", title: "The Smalcald Articles" },
    ConcordDocSpec { part: 6, key: "power-and-primacy", title: "Treatise on the Power and Primacy of the Pope" },
    ConcordDocSpec { part: 7, key: "small-catechism", title: "The Small Catechism" },
    ConcordDocSpec { part: 8, key: "large-catechism", title: "The Large Catechism" },
    ConcordDocSpec { part: 9, key: "epitome", title: "Formula of Concord: Epitome" },
    ConcordDocSpec { part: 10, key: "solid-declaration", title: "Formula of Concord: Solid Declaration" },
];

/// Two sections of the Small Catechism's root page are modern site furniture -- a PDF-availability blurb and a
/// purchase link -- genuinely headed, but not part of the confessional text: excluded by name and counted.
fn is_skipped_article(doc_key: &str, slug: &str) -> bool {
    doc_key == "small-catechism" && (slug == "/small-catechism/prefaratory-notes/" || slug == "/small-catechism/small-catechism-pdf/")
}

/// The Smalcald Articles are the one exception: that document's root page embeds only a one-paragraph editorial
/// blurb per Part, and the real numbered article text lives on separate per-article pages with a DIFFERENT
/// template -- an `<h2>` title followed directly by numbered paragraphs, with no anchor-and-section wrapper.
/// Those pages are vendored separately and spliced in after their own Part's blurb, which is real text and stays.
struct SmalcaldExtra {
    after_slug: &'static str,
    file_stem: &'static str,
    full_slug: &'static str,
    title_fallback: &'static str,
}

const SMALCALD_EXTRAS: &[SmalcaldExtra] = &[
    SmalcaldExtra { after_slug: "/smalcald-articles/i/", file_stem: "nature-of-god", full_slug: "/smalcald-articles/i/nature-of-god/", title_fallback: "Article I - The Nature of God" },
    SmalcaldExtra { after_slug: "/smalcald-articles/i/", file_stem: "the-father", full_slug: "/smalcald-articles/i/the-father/", title_fallback: "Article II - The Father" },
    SmalcaldExtra { after_slug: "/smalcald-articles/i/", file_stem: "the-son", full_slug: "/smalcald-articles/i/the-son/", title_fallback: "Article III - The Son" },
    SmalcaldExtra { after_slug: "/smalcald-articles/i/", file_stem: "the-work-of-salvation", full_slug: "/smalcald-articles/i/the-work-of-salvation/", title_fallback: "Article IV - The Work of Salvation" },
    SmalcaldExtra { after_slug: "/smalcald-articles/ii/", file_stem: "first-and-chief-article", full_slug: "/smalcald-articles/ii/first-and-chief-article/", title_fallback: "Article I - First and Chief Article" },
    SmalcaldExtra { after_slug: "/smalcald-articles/ii/", file_stem: "of-the-mass", full_slug: "/smalcald-articles/ii/of-the-mass/", title_fallback: "Article II - Of the Mass" },
    SmalcaldExtra { after_slug: "/smalcald-articles/ii/", file_stem: "of-chapters-and-cloisters", full_slug: "/smalcald-articles/ii/of-chapters-and-cloisters/", title_fallback: "Article III - Of Chapters and Cloisters" },
    SmalcaldExtra { after_slug: "/smalcald-articles/ii/", file_stem: "of-the-papacy", full_slug: "/smalcald-articles/ii/of-the-papacy/", title_fallback: "Article IV - Of the Papacy" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-sin", full_slug: "/smalcald-articles/iii/of-sin/", title_fallback: "Article I - Of Sin" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-the-law", full_slug: "/smalcald-articles/iii/of-the-law/", title_fallback: "Article II - Of the Law" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-repentance", full_slug: "/smalcald-articles/iii/of-repentance/", title_fallback: "Article III - Of Repentance" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-the-gospel", full_slug: "/smalcald-articles/iii/of-the-gospel/", title_fallback: "Article IV - Of the Gospel" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-baptism", full_slug: "/smalcald-articles/iii/of-baptism/", title_fallback: "Article V - Of Baptism" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-the-scarament-of-the-altar", full_slug: "/smalcald-articles/iii/of-the-scarament-of-the-altar/", title_fallback: "Article VI - Of the Sacrament of the Altar" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-the-keys", full_slug: "/smalcald-articles/iii/of-the-keys/", title_fallback: "Article VII - Of the Keys" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-confession", full_slug: "/smalcald-articles/iii/of-confession/", title_fallback: "Article VIII - Of Confession" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-excommunication", full_slug: "/smalcald-articles/iii/of-excommunication/", title_fallback: "Article IX - Of Excommunication" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-ordination", full_slug: "/smalcald-articles/iii/of-ordination/", title_fallback: "Article X - Of Ordination" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-the-marriage-of-priests", full_slug: "/smalcald-articles/iii/of-the-marriage-of-priests/", title_fallback: "Article XI - Of the Marriage of Priests" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-the-church", full_slug: "/smalcald-articles/iii/of-the-church/", title_fallback: "Article XII - Of the Church" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-good-works", full_slug: "/smalcald-articles/iii/of-good-works/", title_fallback: "Article XIII - Of Good Works" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-monastic-vows", full_slug: "/smalcald-articles/iii/of-monastic-vows/", title_fallback: "Article XIV - Of Monastic Vows" },
    SmalcaldExtra { after_slug: "/smalcald-articles/iii/", file_stem: "of-human-tradition", full_slug: "/smalcald-articles/iii/of-human-tradition/", title_fallback: "Article XV - Of Human Tradition" },
];

/// Splices each sub-article in right after its own anchor slug, then renumbers every article in the final order.
/// Returns whatever disclosures the sub-pages produced -- the same anomaly class any other article can carry.
fn splice_smalcald_extras(doc: &mut ConcordDocument, sub_dir: &Path) -> Result<Vec<String>> {
    let mut disclosures = Vec::new();
    let mut insert_at: BTreeMap<usize, Vec<ConcordArticle>> = BTreeMap::new();
    for extra in SMALCALD_EXTRAS {
        let idx = doc.articles.iter().position(|a| a.slug == extra.after_slug).with_context(|| format!("smalcald-articles: splice target '{}' not found among parsed articles", extra.after_slug))?;
        let path = sub_dir.join(format!("{}.html", extra.file_stem));
        let html = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let (title, paragraphs, anomalies) = parse_single_article_page(&html, extra.title_fallback);
        for a in &anomalies {
            disclosures.push(format!("smalcald-articles/{}: {}", extra.full_slug, a));
        }
        insert_at.entry(idx).or_default().push(ConcordArticle { article: 0, slug: extra.full_slug.to_string(), title, paragraphs });
    }
    // Inserted in DESCENDING index order, so an earlier insertion never shifts the position of a later one.
    for (idx, group) in insert_at.into_iter().rev() {
        for (offset, article) in group.into_iter().enumerate() {
            doc.articles.insert(idx + 1 + offset, article);
        }
    }
    for (i, a) in doc.articles.iter_mut().enumerate() {
        a.article = (i + 1) as u16;
    }
    Ok(disclosures)
}

/// One per-article page: an `<h2>` title followed directly by numbered paragraphs, ending at the page's own
/// trailing navigation widget rather than at the footer. That widget carries no markers of its own but sits
/// between content and footer, so leaving it in would leak its navigation text into the last paragraph's prose.
fn parse_single_article_page(html: &str, title_fallback: &str) -> (String, Vec<ConcordParagraph>, Vec<String>) {
    let main = main_content_slice(html);
    let (title, body_start) = match main.find("<h2>") {
        Some(h2_open) => match main[h2_open..].find("</h2>") {
            Some(close_rel) => {
                let title_raw = &main[h2_open + 4..h2_open + close_rel];
                (collapse_ws(&decode_entities(&strip_tags(title_raw))), h2_open + close_rel + "</h2>".len())
            }
            None => (title_fallback.to_string(), h2_open),
        },
        None => (title_fallback.to_string(), 0),
    };
    let rest = &main[body_start..];
    let end = rest.find(r#"<div class="next-previous-box"#).unwrap_or(rest.len());
    let body = &rest[..end];
    let (paragraphs, anomalies) = group_and_number_paragraphs(body);
    (title, paragraphs, anomalies)
}

#[derive(Debug, Clone)]
pub struct ConcordParagraph {
    pub paragraph: u16,
    /// The source's OWN visible label, empty for a document that carries no marker at all. Disclosure and
    /// debugging only: `paragraph` above is the assigned graph value.
    pub source_label: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ConcordArticle {
    pub article: u16,
    pub slug: String,
    pub title: String,
    pub paragraphs: Vec<ConcordParagraph>,
}

#[derive(Debug, Clone)]
pub struct ConcordDocument {
    pub part: u8,
    pub key: &'static str,
    pub title: &'static str,
    pub articles: Vec<ConcordArticle>,
}

#[derive(Debug, Clone, Default)]
pub struct ConcordStats {
    pub documents: usize,
    pub articles: usize,
    pub paragraphs: usize,
    pub skipped_articles: usize,
    /// One line per disclosed structural anomaly -- synthetic numbering used, an article skipped, a source-side
    /// label collision remapped -- named by document and article, never silent.
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConcordCorpus {
    pub documents: Vec<ConcordDocument>,
    pub stats: ConcordStats,
}

impl ConcordCorpus {
    /// Every paragraph in canonical corpus order, each with its `(part, article, paragraph)` alongside, so one
    /// pass can build both the text nodes and the containment rows.
    pub fn iter_paragraphs(&self) -> impl Iterator<Item = (u8, u16, &ConcordArticle, &ConcordParagraph)> {
        self.documents.iter().flat_map(|d| {
            let part = d.part;
            d.articles.iter().flat_map(move |a| a.paragraphs.iter().map(move |p| (part, a.article, a, p)))
        })
    }
}

/// The one filesystem-touching entry point; every other function in this module is pure `&str`-in / data-out.
pub fn read_all(root: &Path) -> Result<ConcordCorpus> {
    let mut docs = Vec::with_capacity(DOCUMENTS.len());
    let mut stats = ConcordStats::default();
    for spec in DOCUMENTS {
        let path = root.join(format!("{}.html", spec.key));
        let html = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let (mut doc, mut disclosures, skipped) = parse_document_full(&html, spec)?;
        if spec.key == "smalcald-articles" {
            let extra_disclosures = splice_smalcald_extras(&mut doc, &root.join("smalcald-sub"))?;
            disclosures.extend(extra_disclosures);
        }
        stats.documents += 1;
        stats.articles += doc.articles.len();
        for a in &doc.articles {
            stats.paragraphs += a.paragraphs.len();
        }
        stats.disclosures.extend(disclosures);
        stats.skipped_articles += skipped;
        docs.push(doc);
    }
    Ok(ConcordCorpus { documents: docs, stats })
}

/// The SAME function runs over all ten documents: nothing here branches on which document it is except the skip
/// list and the single-article fallback title, both disclosed structural facts rather than parsing forks.
pub fn parse_document(html: &str, spec: &ConcordDocSpec) -> Result<ConcordDocument> {
    let (doc, _disclosures, _skipped) = parse_document_full(html, spec)?;
    Ok(doc)
}

fn parse_document_full(html: &str, spec: &ConcordDocSpec) -> Result<(ConcordDocument, Vec<String>, usize)> {
    let main = main_content_slice(html);
    let raw_articles = find_articles(main);
    let mut articles = Vec::new();
    let mut disclosures = Vec::new();
    let mut skipped = 0usize;
    let mut article_no: u16 = 0;

    if raw_articles.is_empty() {
        // No heading anywhere: a single-article document, the whole-book preface.
        article_no += 1;
        let (paragraphs, anomalies) = group_and_number_paragraphs(main);
        for a in &anomalies {
            disclosures.push(format!("{}: {}", spec.key, a));
        }
        articles.push(ConcordArticle { article: article_no, slug: format!("/{}/", spec.key), title: spec.title.to_string(), paragraphs });
    } else {
        for raw in &raw_articles {
            if is_skipped_article(spec.key, &raw.href) {
                skipped += 1;
                disclosures.push(format!("{}: skipped non-confessional article '{}' ({}) -- modern site furniture, not Book of Concord text", spec.key, raw.href, raw.title));
                continue;
            }
            article_no += 1;
            let (paragraphs, anomalies) = group_and_number_paragraphs(raw.body);
            for a in &anomalies {
                disclosures.push(format!("{}/{}: {}", spec.key, raw.href, a));
            }
            if paragraphs.is_empty() {
                disclosures.push(format!("{}/{}: zero paragraphs parsed (empty article body)", spec.key, raw.href));
            }
            articles.push(ConcordArticle { article: article_no, slug: raw.href.clone(), title: raw.title.clone(), paragraphs });
        }
    }

    if articles.is_empty() {
        anyhow::bail!("concord::parse_document({}): zero articles found (main-content slice empty or malformed)", spec.key);
    }

    Ok((ConcordDocument { part: spec.part, key: spec.key, title: spec.title, articles }, disclosures, skipped))
}

/// Restricts to the page's own real content, so header, navigation and footer chrome can never contaminate a
/// heading or marker scan.
fn main_content_slice(html: &str) -> &str {
    let start = html.find(r#"id="main-content""#).unwrap_or(0);
    let rest = &html[start..];
    let end = rest.find("<footer").unwrap_or(rest.len());
    &rest[..end]
}

struct RawArticle<'a> {
    href: String,
    title: String,
    body: &'a str,
}

/// Finds every anchor-wrapped heading followed by its section body. Manual scanning, no regex dependency, which
/// also skips the site's own left-nav table of contents for free: a table-of-contents entry carries no heading
/// inside it, so scanning BY heading occurrence never matches one.
fn find_articles(main: &str) -> Vec<RawArticle<'_>> {
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = main[search_from..].find("<h3>") {
        let h3_open = search_from + rel;
        let Some(h3_close_rel) = main[h3_open..].find("</h3>") else { break };
        let title_raw = &main[h3_open + 4..h3_open + h3_close_rel];
        let title = collapse_ws(&decode_entities(&strip_tags(title_raw)));
        let h3_close = h3_open + h3_close_rel + "</h3>".len();

        let href = main[..h3_open]
            .rfind("<a href=\"")
            .and_then(|a_pos| {
                let val_start = a_pos + "<a href=\"".len();
                main[val_start..].find('"').map(|q| main[val_start..val_start + q].to_string())
            })
            .unwrap_or_default();

        let Some(section_open_rel) = main[h3_close..].find("<section>") else {
            search_from = h3_close;
            continue;
        };
        let body_start = h3_close + section_open_rel + "<section>".len();
        let Some(section_close_rel) = main[body_start..].find("</section>") else {
            search_from = body_start;
            continue;
        };
        let body = &main[body_start..body_start + section_close_rel];
        let next = body_start + section_close_rel + "</section>".len();
        out.push(RawArticle { href, title, body });
        search_from = next;
    }
    out
}

struct Marker {
    id: String,
    label: String,
    start: usize,
    end: usize,
}

/// Finds every paragraph-marker span. Its class-name prefix renders empty on a few documents, a template quirk,
/// so matching keys on the constant substrings either way. A marker's `start` is its own opening `<span`, walked
/// back from the attribute match, so a gap between markers always ends at a clean tag boundary: a mid-attribute
/// cut would leave a dangling fragment that the tag stripper would stop on, swallowing the rest of the gap.
fn find_markers(body: &str) -> Vec<Marker> {
    let mut out = Vec::new();
    let mut search_from = 0usize;
    const NEEDLE: &str = "-acontent\" class=\"";
    while let Some(rel) = body[search_from..].find(NEEDLE) {
        let match_start = search_from + rel;
        let Some(id_attr_rel) = body[..match_start].rfind("id=\"") else {
            search_from = match_start + NEEDLE.len();
            continue;
        };
        let id_val_start = id_attr_rel + "id=\"".len();
        let id = body[id_val_start..match_start].to_string();
        let tag_start = body[..id_attr_rel].rfind("<span").unwrap_or(id_attr_rel);

        let after_needle = match_start + NEEDLE.len();
        let Some(class_close_rel) = body[after_needle..].find('"') else { break };
        let after_class = after_needle + class_close_rel + 1;
        let Some(gt_rel) = body[after_class..].find('>') else { break };
        let label_start = after_class + gt_rel + 1;
        let Some(label_end_rel) = body[label_start..].find("</span>") else { break };
        let label = body[label_start..label_start + label_end_rel].to_string();
        let end = label_start + label_end_rel + "</span>".len();

        out.push(Marker { id, label, start: tag_start, end });
        search_from = end;
    }
    out
}

/// The leading digit run of a label, which is the grouping key for a sub-lettered question-and-answer pair.
fn leading_digits(label: &str) -> Option<u16> {
    let digits: String = label.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// True when `id` is the `-ans`-suffixed partner of a preceding non-numeric group. A few of the source's own ids
/// carry a template bug -- a format directive misfiring on a string argument -- which always leaves the literal
/// `-ans` intact right before its trailing `)`, so the check allows that optional `)`.
fn is_ans_suffixed(id: &str) -> bool {
    id.ends_with("-ans") || id.ends_with("-ans)")
}

/// Groups the body's markers into paragraphs and assigns each its paragraph number, returning the paragraphs in
/// order plus any disclosed anomalies.
fn group_and_number_paragraphs(body: &str) -> (Vec<ConcordParagraph>, Vec<String>) {
    let markers = find_markers(body);
    let mut anomalies = Vec::new();

    if markers.is_empty() {
        // No native numbering at all: fall back to splitting on paragraph tags, numbered sequentially.
        let paragraphs = split_on_paragraph_tags(body);
        if !paragraphs.is_empty() {
            anomalies.push(format!("no native paragraph numbering in source -- {} paragraph(s) assigned synthetic sequential positions 1..{}", paragraphs.len(), paragraphs.len()));
        }
        let out = paragraphs
            .into_iter()
            .enumerate()
            .map(|(i, text)| ConcordParagraph { paragraph: (i + 1) as u16, source_label: String::new(), text })
            .collect();
        return (out, anomalies);
    }

    // A fresh digit base OPENS a group; a repeat of the current group's base, or an `-ans`-paired id, CONTINUES
    // it; any other non-numeric marker also opens a new group, so a document whose paragraphs are uniformly
    // marked with a star does not collapse into one paragraph.
    let mut group_of: Vec<usize> = Vec::with_capacity(markers.len());
    let mut group_base: Vec<Option<u16>> = Vec::new();
    let mut cur_gid: Option<usize> = None;
    let mut cur_base: Option<u16> = None;
    for m in &markers {
        let base = leading_digits(&m.label);
        let continues = match cur_gid {
            None => false,
            Some(_) => is_ans_suffixed(&m.id) || (base.is_some() && base == cur_base),
        };
        if !continues {
            cur_gid = Some(cur_gid.map_or(0, |g| g + 1));
            cur_base = base;
            group_base.push(base);
        }
        group_of.push(cur_gid.unwrap());
    }

    // A marker's gap text is the prose between its own end and the next marker's start, so concatenating per
    // group excludes every marker span itself -- number and wrappers -- not merely its tags.
    let mut per_group_raw: BTreeMap<usize, String> = BTreeMap::new();
    for (k, m) in markers.iter().enumerate() {
        let seg_end = markers.get(k + 1).map(|n| n.start).unwrap_or(body.len());
        let gap = &body[m.end..seg_end];
        per_group_raw.entry(group_of[k]).or_default().push_str(gap);
    }

    let mut out = Vec::with_capacity(per_group_raw.len());
    let mut last_assigned: u16 = 0;
    let mut collisions: Vec<(u16, u16, u16)> = Vec::new();
    for (gid, raw) in per_group_raw {
        let base = group_base[gid];
        let text = clean_paragraph_text(&raw);
        let assigned = match base {
            Some(n) if n > last_assigned => n,
            None if gid == 0 => 0,
            _ => {
                let a = last_assigned + 1;
                if let Some(n) = base {
                    collisions.push((n, last_assigned, a));
                }
                a
            }
        };
        last_assigned = assigned;
        let label_repr = markers.iter().zip(group_of.iter()).filter(|(_, g)| **g == gid).map(|(m, _)| m.label.as_str()).collect::<Vec<_>>().join("/");
        out.push(ConcordParagraph { paragraph: assigned, source_label: label_repr, text });
    }
    // One SUMMARY line per article rather than one per collided paragraph: a genuine source-side renumbering
    // restart can span dozens of consecutive paragraphs, and a line each would bury the signal. Every remap
    // stays individually recoverable, since the source label rides on the paragraph itself.
    if let Some(&(first_label, first_prior, first_remap)) = collisions.first() {
        anomalies.push(format!(
            "{} source label(s) collided with an already-used paragraph number within this article (first: label '{first_label}' collided with prior {first_prior}, remapped to {first_remap}) -- all remapped to a contiguous run, never force-fit; a genuine source-side renumbering restart, not a parse failure (each paragraph's own original label survives on `ConcordParagraph.source_label`)",
            collisions.len()
        ));
    }
    (out, anomalies)
}

/// The fallback shape for a document with no markers: one paragraph per paragraph tag, headings excluded the
/// same way they are excluded elsewhere.
fn split_on_paragraph_tags(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(open_rel) = rest.find("<p>") {
        let after_open = &rest[open_rel + 3..];
        let Some(close_rel) = after_open.find("</p>") else { break };
        let inner = &after_open[..close_rel];
        let text = clean_paragraph_text(inner);
        if !text.is_empty() {
            out.push(text);
        }
        rest = &after_open[close_rel + 4..];
    }
    out
}

/// The full cleaning pipeline: excise complete heading blocks and standalone bold sub-labels, strip the
/// remaining tags, decode entities, collapse whitespace.
fn clean_paragraph_text(raw: &str) -> String {
    let no_headings = strip_complete_headings(raw);
    let no_labels = strip_standalone_strong_paragraphs(&no_headings);
    let no_tags = strip_tags(&no_labels);
    let decoded = decode_entities(&no_tags);
    collapse_ws(&decoded)
}

/// Excises a paragraph whose entire content is one bold span: the source is inconsistent, rendering a
/// sub-section label as a heading on some documents and as a bare bold paragraph on others. The adjacency test
/// is narrow on purpose -- bold text midway through real confessional prose has other text in the same
/// paragraph and never matches, so it is left untouched.
fn strip_standalone_strong_paragraphs(s: &str) -> String {
    const OPEN: &str = "<p><strong>";
    const CLOSE: &str = "</strong></p>";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let Some(open_rel) = rest.find(OPEN) else {
            out.push_str(rest);
            break;
        };
        let after_open = open_rel + OPEN.len();
        let Some(close_rel) = rest[after_open..].find(CLOSE) else {
            out.push_str(&rest[..open_rel + "<p>".len()]);
            rest = &rest[open_rel + "<p>".len()..];
            continue;
        };
        out.push_str(&rest[..open_rel]);
        rest = &rest[after_open + close_rel + CLOSE.len()..];
    }
    out
}

/// Excises every COMPLETE heading block, tag and inner text. An opening tag whose close falls beyond the next
/// marker is left here for the generic tag stripper: only a complete pair carries inner text worth excising.
fn strip_complete_headings(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        let Some(rel) = rest.find("<h") else {
            out.push_str(rest);
            break;
        };
        let tag_pos = rel;
        let level = rest[tag_pos + 2..].chars().next();
        if let Some(c) = level {
            if c.is_ascii_digit() {
                let close_tag = format!("</h{c}>");
                if let Some(close_rel) = rest[tag_pos..].find(close_tag.as_str()) {
                    out.push_str(&rest[..tag_pos]);
                    rest = &rest[tag_pos + close_rel + close_tag.len()..];
                    continue;
                }
            }
        }
        out.push_str(&rest[..tag_pos + 2]);
        rest = &rest[tag_pos + 2..];
    }
    out
}

/// Strips every tag, keeping the text between. Non-nested, since no `>` appears inside an attribute value in the
/// real pages, and a lone trailing `<` stops the scan rather than looping.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        match rest.find('<') {
            None => {
                out.push_str(rest);
                break;
            }
            Some(start) => {
                out.push_str(&rest[..start]);
                match rest[start..].find('>') {
                    Some(end) => rest = &rest[start + end + 1..],
                    None => break,
                }
            }
        }
    }
    out
}

/// Decodes the entities actually present in the vendored pages plus generic numeric escapes, to the REAL Unicode
/// character rather than an ASCII flattening. An unrecognized entity is left as literal text, never dropped.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp_rel) = rest.find('&') {
        out.push_str(&rest[..amp_rel]);
        let after = &rest[amp_rel + 1..];
        if let Some(semi_rel) = after.find(';').filter(|&r| r <= 10) {
            let name = &after[..semi_rel];
            if let Some(ch) = decode_one_entity(name) {
                out.push(ch);
                rest = &after[semi_rel + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn decode_one_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "hellip" => '\u{2026}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "middot" => '\u{B7}',
        "nbsp" => ' ',
        other if other.starts_with('#') => {
            let numeric = &other[1..];
            let code = if let Some(hex) = numeric.strip_prefix('x').or_else(|| numeric.strip_prefix('X')) {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                numeric.parse::<u32>().ok()?
            };
            char::from_u32(code)?
        }
        _ => return None,
    })
}

/// Collapses every run of whitespace, meaningless once the tags are stripped, to a single space, trimmed.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One curated, hand-verified alignment row: a catechism item id and the Concord paragraphs carrying that item's
/// own text and explanation. One link row per `(item, paragraph)` pair.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScOverlapRow {
    pub item: String,
    pub article: u16,
    pub paragraphs: Vec<u16>,
}

#[derive(serde::Deserialize)]
struct ScOverlapFile {
    link: Vec<ScOverlapRow>,
}

/// Purely structural: cross-checking against the parsed corpus and the real item set is the graph adapter's job.
pub fn parse_sc_overlap(input: &str) -> Result<Vec<ScOverlapRow>> {
    let f: ScOverlapFile = toml::from_str(input).context("concord-sc-overlap.toml: invalid TOML or does not match the [[link]] schema")?;
    Ok(f.link)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SC_FIRST_COMMANDMENT: &str = r#"<h4 id="the-first-commandment"><strong>The First Commandment.</strong></h4>
<h4 id="hahahugoshortcode-s0-hbhbthou-shalt-have-no-other-gods"><span id="0001" class="bocanchor"> </span><span id="sc-ten-commandments-0001" class="bocanchor"><span id="sc-ten-commandments-0001-acontent" class="bocanchor-content">1</span></span>Thou shalt have no other gods.</h4>
<p><em>What does this mean?</em></p>
<p>&ndash;Answer: <span id="%!d(string=001b)" class="bocanchor"> </span><span id="sc-ten-commandments-%!d(string=001b)" class="bocanchor"><span id="sc-ten-commandments-%!d(string=001b)-acontent" class="bocanchor-content">1b</span></span>We should fear, love, and trust in God above all things.</p>
<h4 id="the-second-commandment"><strong>The Second Commandment.</strong></h4>
<h4 id="hahahugoshortcode-s2-hbhbthou-shalt-not-take-the-name-of-the-lord-thy-god-in-vain"><span id="0002" class="bocanchor"> </span><span id="sc-ten-commandments-0002" class="bocanchor"><span id="sc-ten-commandments-0002-acontent" class="bocanchor-content">2</span></span>Thou shalt not take the name of the Lord, thy God, in vain.</h4>"#;

    #[test]
    fn small_catechism_first_commandment_merges_question_and_answer_into_one_paragraph_and_excludes_the_next_heading() {
        let (paras, anomalies) = group_and_number_paragraphs(SC_FIRST_COMMANDMENT);
        assert!(anomalies.is_empty(), "a clean Q/A merge is not an anomaly: {anomalies:?}");
        assert_eq!(paras.len(), 2, "commandment 1 (merged) + commandment 2 (its own text, article cut short by the fixture)");
        assert_eq!(paras[0].paragraph, 1);
        assert_eq!(
            paras[0].text,
            "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.",
            "verbatim source prose, en dash decoded, 'The Second Commandment.' heading excluded, '1'/'1b' markers excluded"
        );
        assert_eq!(paras[1].paragraph, 2);
    }

    const AC_ARTICLE_IV: &str = r#"<p><span id="0001" class="bocanchor"> </span><span id="ac-iv-0001" class="bocanchor"><span id="ac-iv-0001-acontent" class="bocanchor-content">1</span></span> Also they teach that men cannot be justified before God by their
own strength, merits, or works, but are freely justified for
<span id="0002" class="bocanchor"> </span><span id="ac-iv-0002" class="bocanchor"><span id="ac-iv-0002-acontent" class="bocanchor-content">2</span></span> Christ&rsquo;s sake, through faith, when they believe that
they are received into favor, and that their sins are forgiven
for Christ&rsquo;s sake, who, by His death, has made satisfaction
for our sins. <span id="0003" class="bocanchor"> </span><span id="ac-iv-0003" class="bocanchor"><span id="ac-iv-0003-acontent" class="bocanchor-content">3</span></span> This faith God imputes for righteousness
in His sight. Rom. 3 and 4.</p>"#;

    #[test]
    fn augsburg_confession_article_iv_is_three_plain_sequential_paragraphs() {
        let (paras, anomalies) = group_and_number_paragraphs(AC_ARTICLE_IV);
        assert!(anomalies.is_empty());
        assert_eq!(paras.iter().map(|p| p.paragraph).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(paras[2].text, "This faith God imputes for righteousness in His sight. Rom. 3 and 4.");
    }

    const LORDS_PRAYER_INTRO: &str = r#"<p><strong>Introduction</strong></p>
<h4 id="x"><span id="%!d(string=intro)" class="bocanchor"> </span><span id="sc-lords-prayer-%!d(string=intro)" class="bocanchor"><span id="sc-lords-prayer-%!d(string=intro)-acontent" class="bocanchor-content">*</span></span><span class="forcespan">Our Father who art in heaven.</span>
</h4>
<p><em>What does this mean?</em></p>
<p>&ndash;Answer:  <span id="%!d(string=intro-ans)" class="bocanchor"> </span><span id="sc-lords-prayer-%!d(string=intro-ans)" class="bocanchor"><span id="sc-lords-prayer-%!d(string=intro-ans)-acontent" class="bocanchor-content">*</span></span><span class="forcespan">God would thereby tenderly urge us.</span>
</p>
<p><strong>The First Petition.</strong></p>
<h4 id="y"><span id="0001" class="bocanchor"> </span><span id="sc-lords-prayer-0001" class="bocanchor"><span id="sc-lords-prayer-0001-acontent" class="bocanchor-content">1</span></span><span class="forcespan">Hallowed be Thy name.</span>
</h4>"#;

    #[test]
    fn lords_prayer_introduction_merges_via_explicit_ans_pairing_and_gets_paragraph_zero() {
        let (paras, anomalies) = group_and_number_paragraphs(LORDS_PRAYER_INTRO);
        assert!(anomalies.is_empty());
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].paragraph, 0, "the leading unnumbered unit gets 0 -- never collides with the real 1..7 petitions");
        assert_eq!(paras[0].text, "Our Father who art in heaven. What does this mean? \u{2013}Answer: God would thereby tenderly urge us.");
        assert_eq!(paras[1].paragraph, 1);
    }

    #[test]
    fn preface_style_uniform_star_labels_do_not_collapse_into_one_paragraph() {
        let body = r#"<p><span id="sc-preface-0001" class="bocanchor"> </span><span id="sc-preface-0001-acontent" class="bocanchor-content">*</span>First.</p>
<p><span id="sc-preface-0002" class="bocanchor"> </span><span id="sc-preface-0002-acontent" class="bocanchor-content">*</span>Second.</p>
<p><span id="sc-preface-0003" class="bocanchor"> </span><span id="sc-preface-0003-acontent" class="bocanchor-content">*</span>Third.</p>"#;
        let (paras, _) = group_and_number_paragraphs(body);
        assert_eq!(paras.len(), 3, "three independently-anchored '*' paragraphs, not one merged blob");
        assert_eq!(paras.iter().map(|p| p.paragraph).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn a_duplicate_source_label_is_remapped_and_disclosed_not_silently_duplicated() {
        let body = r#"<p><span id="a" class="bocanchor"> </span><span id="a-acontent" class="bocanchor-content">9</span>Question.<span id="b" class="bocanchor"> </span><span id="b-acontent" class="bocanchor-content">9</span>Answer.</p>"#;
        let (paras, anomalies) = group_and_number_paragraphs(body);
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].paragraph, 9);
        assert_eq!(paras[0].text, "Question. Answer.");
        assert!(anomalies.is_empty(), "an equal-base repeat merges cleanly, it is not a collision");
    }

    #[test]
    fn ecumenical_creeds_style_no_markers_falls_back_to_synthetic_sequential_paragraphs() {
        let body = r#"<p>I believe in God the Father Almighty, Maker of heaven and earth.</p>
<p>And in Jesus Christ, His only Son, our Lord.</p>"#;
        let (paras, anomalies) = group_and_number_paragraphs(body);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].paragraph, 1);
        assert_eq!(paras[1].paragraph, 2);
        assert_eq!(paras[0].source_label, "");
        assert_eq!(anomalies.len(), 1, "synthetic numbering is disclosed exactly once per article");
        assert!(anomalies[0].contains("no native paragraph numbering"));
    }

    #[test]
    fn find_articles_skips_the_toc_sidebar_and_finds_the_real_content_anchor() {
        let main = r#"<span class="toc-item"><a href="/augsburg-confession/of-justification/" class="">Article IV. Of Justification.</a></span>
<a href="/augsburg-confession/of-justification/">
  <h3>
    Article IV. Of Justification.
  </h3>
</a>
<section><p><span id="a" class="bocanchor"> </span><span id="a-acontent" class="bocanchor-content">1</span>Also they teach.</p></section>"#;
        let arts = find_articles(main);
        assert_eq!(arts.len(), 1, "the TOC's own <a> (no <h3> inside it) must never be mistaken for a second article");
        assert_eq!(arts[0].href, "/augsburg-confession/of-justification/");
        assert_eq!(arts[0].title, "Article IV. Of Justification.");
    }

    #[test]
    fn a_document_with_no_h3_at_all_is_one_single_implicit_article() {
        let html = r#"<div class="content" id="main-content"><main>
<p><span id="a" class="bocanchor"> </span><span id="a-acontent" class="bocanchor-content">1</span>To the Readers.</p>
</main></div><footer>...</footer>"#;
        let spec = ConcordDocSpec { part: 1, key: "preface", title: "Preface to the Book of Concord" };
        let doc = parse_document(html, &spec).unwrap();
        assert_eq!(doc.articles.len(), 1);
        assert_eq!(doc.articles[0].article, 1);
        assert_eq!(doc.articles[0].title, "Preface to the Book of Concord");
        assert_eq!(doc.articles[0].paragraphs.len(), 1);
    }

    #[test]
    fn skipped_small_catechism_articles_are_disclosed_and_excluded_never_ingested() {
        let html = r#"<div class="content" id="main-content"><main>
<a href="/small-catechism/prefaratory-notes/"><h3>Prefaratory Notes</h3></a>
<section><p>The 1986 Version is available in PDF format.</p></section>
<a href="/small-catechism/ten-commandments/"><h3>The Ten Commandments</h3></a>
<section><p><span id="a" class="bocanchor"> </span><span id="a-acontent" class="bocanchor-content">1</span>Thou shalt have no other gods.</p></section>
</main></div><footer>...</footer>"#;
        let spec = ConcordDocSpec { part: 7, key: "small-catechism", title: "The Small Catechism" };
        let (doc, disclosures, skipped) = parse_document_full(html, &spec).unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(doc.articles.len(), 1, "only the real Ten Commandments article survives");
        assert_eq!(doc.articles[0].article, 1, "article numbering is not perturbed by a skipped predecessor");
        assert_eq!(doc.articles[0].slug, "/small-catechism/ten-commandments/");
        assert!(disclosures.iter().any(|d| d.contains("prefaratory-notes") && d.contains("skipped")), "disclosures: {disclosures:?}");
    }

    #[test]
    fn entity_decoding_covers_the_real_observed_set_plus_generic_numeric_escapes() {
        assert_eq!(decode_entities("Rock&rsquo;s &amp; a&nbsp;test &ndash; &#39;quoted&#39; &#x2019;"), "Rock\u{2019}s & a test \u{2013} 'quoted' \u{2019}");
        assert_eq!(decode_entities("A &unknown; entity"), "A &unknown; entity", "unrecognized entities are left literal, never dropped");
    }

    #[test]
    fn documents_table_has_ten_distinct_parts_in_the_traditional_order() {
        let parts: Vec<u8> = DOCUMENTS.iter().map(|d| d.part).collect();
        assert_eq!(parts, (1..=10).collect::<Vec<_>>());
        let keys: std::collections::BTreeSet<&str> = DOCUMENTS.iter().map(|d| d.key).collect();
        assert_eq!(keys.len(), 10, "ten distinct document keys, no duplicates");
    }

    #[test]
    fn parse_sc_overlap_reads_valid_toml_and_expands_multi_paragraph_items() {
        let toml = r#"
[[link]]
item = "commandment-1"
article = 2
paragraphs = [1]

[[link]]
item = "baptism-1"
article = 5
paragraphs = [1, 2]
"#;
        let rows = parse_sc_overlap(toml).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].item, "commandment-1");
        assert_eq!(rows[0].article, 2);
        assert_eq!(rows[0].paragraphs, vec![1]);
        assert_eq!(rows[1].paragraphs, vec![1, 2], "a multi-paragraph item keeps every listed position");
    }

    #[test]
    fn parse_sc_overlap_rejects_malformed_toml() {
        assert!(parse_sc_overlap("not valid toml [[[").is_err());
        assert!(parse_sc_overlap("[[link]]\narticle = 2\n").is_err(), "missing required 'item' field must fail loud");
    }
}
