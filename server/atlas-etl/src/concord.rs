//! The Book of Concord parser: one parser over the ten vendored document-root pages, each of which already
//! carries its articles' full text inline. A paragraph boundary is a marker span carrying the source's own
//! visible label; a group with no usable digit takes the previous number plus one, disclosed, never guessed.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use atlas_graph_types::text::{Piece, Rendering, TextPartRole};
use ego_tree::NodeId;

use crate::admission::{Admission, AdmissionStats, Admitted, EditorialReason};
use crate::triglot::TriglotReference;
use scraper::{ElementRef, Html, Node, Selector};

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

/// The Smalcald Articles are the one exception: that document's root page embeds only a one-paragraph editorial blurb
/// per Part, and the real numbered article text lives on separate per-article pages with a DIFFERENT template -- an
/// `<h2>` title followed directly by numbered paragraphs. Those pages are vendored and spliced in after the blurb.
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

fn splice_smalcald_extras(doc: &mut ConcordDocument, sub_dir: &Path, curation: &mut Curation) -> Result<Vec<String>> {
    let mut disclosures = Vec::new();
    let mut insert_at: BTreeMap<usize, Vec<ConcordArticle>> = BTreeMap::new();
    for extra in SMALCALD_EXTRAS {
        let idx = doc.articles.iter().position(|a| a.slug == extra.after_slug).with_context(|| format!("smalcald-articles: splice target '{}' not found among parsed articles", extra.after_slug))?;
        let path = sub_dir.join(format!("{}.html", extra.file_stem));
        let html = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let page = Html::parse_document(&html);
        let main = page_main(&page);
        let title = main.child_elements().find(|e| e.value().name() == "h2").map(|h2| words_of(&h2)).unwrap_or_else(|| extra.title_fallback.to_string());
        let body: Vec<ElementRef> = main.child_elements().filter(is_page_body).collect();
        let (paragraphs, anomalies) = paragraphs_of(&body, &mut curation.article(doc.key, extra.full_slug));
        for a in &anomalies {
            disclosures.push(format!("smalcald-articles/{}: {}", extra.full_slug, a));
        }
        insert_at.entry(idx).or_default().push(ConcordArticle { article: 0, slug: extra.full_slug.to_string(), title, section_title: None, citation: Citation::uncited(), paragraphs });
    }
    for (idx, group) in insert_at.into_iter().rev() {
        for (offset, article) in group.into_iter().enumerate() {
            doc.articles.insert(idx + 1 + offset, article);
        }
    }
    let mut parts: Vec<&str> = SMALCALD_EXTRAS.iter().map(|extra| extra.after_slug).collect();
    parts.dedup();
    for part in parts {
        open_first_article_with_its_part(doc, part)?;
    }
    for (i, a) in doc.articles.iter_mut().enumerate() {
        a.article = (i + 1) as u16;
    }
    Ok(disclosures)
}

fn open_first_article_with_its_part(doc: &mut ConcordDocument, part: &str) -> Result<()> {
    let at = doc.articles.iter().position(|article| article.slug == part).with_context(|| format!("smalcald-articles: part page '{part}' not found among parsed articles"))?;
    let lead_in = doc.articles.remove(at);
    let opening = doc.articles.get_mut(at).and_then(|article| article.paragraphs.first_mut()).with_context(|| format!("smalcald-articles: part '{part}' has no first article to open"))?;
    let mut pieces: Vec<Piece> = Vec::new();
    for paragraph in lead_in.paragraphs.iter().chain([&*opening]) {
        spaced_after(&mut pieces, paragraph.rendering.pieces());
    }
    opening.rendering = Rendering::compose(&pieces);
    opening.headings = lead_in.paragraphs.iter().flat_map(|paragraph| paragraph.headings.iter().cloned()).chain(opening.headings.drain(..)).collect();
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ConcordParagraph {
    pub paragraph: u16,
    pub source_label: String,
    pub rendering: Rendering,
    pub headings: Vec<Heading>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub text: String,
    pub wording: HeadingWording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingWording {
    Source,
    Confirmed { page: u16 },
    Ours,
}

#[derive(Debug, Clone)]
pub struct ConcordArticle {
    pub article: u16,
    pub slug: String,
    pub title: String,
    pub section_title: Option<String>,
    pub citation: Citation,
    pub paragraphs: Vec<ConcordParagraph>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub code: String,
    pub numbering: NumberingAgreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberingAgreement {
    Agrees,
    ArticleOnly,
}

impl Citation {
    pub(crate) fn uncited() -> Citation {
        Citation { code: String::new(), numbering: NumberingAgreement::ArticleOnly }
    }

    pub fn label(&self, paragraph: u16) -> String {
        match self.numbering {
            NumberingAgreement::Agrees => format!("{} {paragraph}", self.code),
            NumberingAgreement::ArticleOnly => self.code.clone(),
        }
    }
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
    pub excluded_units: usize,
    pub stripped_fragments: usize,
    pub corrected_roles: usize,
    pub admitted: AdmissionStats,
    /// One line per disclosed structural anomaly -- synthetic numbering used, an article skipped, a source-side
    /// label collision remapped -- named by document and article, never silent.
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConcordCorpus {
    pub title: String,
    pub description: String,
    pub documents: Vec<ConcordDocument>,
    pub stats: ConcordStats,
    pub admissions: Vec<(Admitted, Admission)>,
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

pub fn read_all(root: &Path, curated_dir: &Path) -> Result<ConcordCorpus> {
    let titles = crate::curated::parse_concord_titles(&read_curated(curated_dir, "concord-titles.toml")?)?;
    let exclusions = parse_concord_exclusions(&read_curated(curated_dir, "concord-exclusions.toml")?)?;
    let readings = parse_concord_readings(&read_curated(curated_dir, "concord-roles.toml")?)?;
    let mut docs = Vec::with_capacity(DOCUMENTS.len());
    let mut stats = ConcordStats::default();
    let mut curation = Curation::new(&exclusions, &readings);
    for spec in DOCUMENTS {
        let path = root.join(format!("{}.html", spec.key));
        let html = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let (mut doc, mut disclosures, skipped) = parse_document(&html, spec, &mut curation)?;
        if spec.key == "smalcald-articles" {
            let extra_disclosures = splice_smalcald_extras(&mut doc, &root.join("smalcald-sub"), &mut curation)?;
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
    curation.every_source_word_is_served_or_excluded()?;
    curation.every_entry_matched_once()?;
    stats.excluded_units = curation.unit_hits.iter().sum();
    stats.corrected_roles = readings.correction.iter().map(|row| row.occurrences).sum();
    apply_title_overrides(&mut docs, &titles)?;
    stats.stripped_fragments = apply_strips(&mut docs, &exclusions.strip)?;
    let policy = crate::admission::parse_admission_policy(&read_curated(curated_dir, "concord-admission.toml")?)?;
    let raw = root.parent().context("data/raw/concord sits inside data/raw")?;
    let reference = crate::triglot::TriglotReference::read(raw, policy.shingle_words)?;
    let mut admissions = crate::admission::admit(&docs, &titles, &reference, &policy).map_err(|refusals| {
        anyhow::anyhow!("{} Concord span(s) are not admitted as the 1921 Triglot's:\n{}", refusals.len(), refusals.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))
    })?;
    let corpus_text: CorpusText = toml::from_str(&read_curated(curated_dir, "concord-corpus.toml")?).context("concord-corpus.toml: expected exactly a title and a description")?;
    admissions.push((Admitted::Description, Admission::Editorial { reason: EditorialReason::CorpusDescription }));
    let citations: ConcordCitations = toml::from_str(&read_curated(curated_dir, "concord-citations.toml")?).context("concord-citations.toml: invalid TOML or does not match the [[document]]/[[article]]/[[section]] schema")?;
    cite(&mut docs, &citations, &admissions, &reference, policy.numbering_percent)?;
    stats.admitted = AdmissionStats::of(&admissions);
    let corpus = ConcordCorpus { title: corpus_text.title, description: corpus_text.description, documents: docs, stats, admissions };
    no_excluded_material_is_served(&corpus, &exclusions)?;
    no_served_text_carries_a_non_triglot_marker(&corpus, &exclusions.markers)?;
    Ok(corpus)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusText {
    title: String,
    description: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcordCitations {
    pub document: Vec<DocumentCitation>,
    #[serde(default)]
    pub article: Vec<ArticleDesignation>,
    #[serde(default)]
    pub section: Vec<SectionTitle>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentCitation {
    pub key: String,
    pub abbreviation: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArticleDesignation {
    pub document: String,
    pub article: u16,
    pub designation: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionTitle {
    pub document: String,
    pub article: u16,
    pub title: String,
    pub triglot_page: u16,
}

fn cite(docs: &mut [ConcordDocument], citations: &ConcordCitations, admissions: &[(Admitted, Admission)], reference: &TriglotReference, numbering_percent: u32) -> Result<()> {
    let mut refusals = Vec::new();
    let named = |document: &str, article: u16| docs.iter().any(|d| d.key == document && d.articles.iter().any(|a| a.article == article));
    for row in &citations.article {
        if !named(&row.document, row.article) {
            refusals.push(format!("concord-citations.toml: [[article]] {} {} names no served article", row.document, row.article));
        }
    }
    for row in &citations.section {
        if !named(&row.document, row.article) {
            refusals.push(format!("concord-citations.toml: [[section]] {} {} names no served article", row.document, row.article));
        }
    }
    for doc in docs.iter_mut() {
        let Some(abbreviation) = citations.document.iter().find(|row| row.key == doc.key).map(|row| row.abbreviation.as_str()) else {
            refusals.push(format!("concord-citations.toml: no [[document]] abbreviation for '{}'", doc.key));
            continue;
        };
        for article in doc.articles.iter_mut() {
            let designation = citations.article.iter().find(|row| row.document == doc.key && row.article == article.article).map(|row| row.designation.as_str());
            let code = [Some(abbreviation), designation].into_iter().flatten().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ");
            let numbering = numbering_of(doc.part, article, admissions, reference, numbering_percent);
            article.citation = Citation { code, numbering };
            article.section_title = citations.section.iter().find(|row| row.document == doc.key && row.article == article.article).map(|row| row.title.clone());
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("{}", refusals.join("\n"))
    }
}

fn numbering_of(part: u8, article: &ConcordArticle, admissions: &[(Admitted, Admission)], reference: &TriglotReference, numbering_percent: u32) -> NumberingAgreement {
    let placed: Vec<bool> = article
        .paragraphs
        .iter()
        .filter_map(|p| {
            admissions.iter().find_map(|(admitted, admission)| match (admitted, admission) {
                (Admitted::Paragraph { part: at_part, article: at_article, paragraph }, Admission::Matched { start, .. }) if (*at_part, *at_article, *paragraph) == (part, article.article, p.paragraph) => {
                    Some(reference.numbers_near(*start).contains(&p.paragraph))
                }
                _ => None,
            })
        })
        .collect();
    let found = placed.iter().filter(|found| **found).count();
    if !placed.is_empty() && found * PERCENT >= numbering_percent as usize * placed.len() {
        NumberingAgreement::Agrees
    } else {
        NumberingAgreement::ArticleOnly
    }
}

const PERCENT: usize = 100;

fn read_curated(curated_dir: &Path, name: &str) -> Result<String> {
    let path = curated_dir.join(name);
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

/// Keyed by the served `(document, article)` position, which is final only after the Smalcald splice has
/// renumbered; an override that names no article is a curation error, never a silent no-op.
fn apply_title_overrides(docs: &mut [ConcordDocument], titles: &[ConcordTitleOverride]) -> Result<()> {
    for t in titles {
        let article = docs.iter_mut().find(|d| d.key == t.document).and_then(|d| d.articles.iter_mut().find(|a| a.article == t.article));
        match article {
            Some(a) => a.title = t.title.clone(),
            None => anyhow::bail!("concord-titles.toml: no article {} in document '{}' to retitle as '{}'", t.article, t.document, t.title),
        }
    }
    Ok(())
}

fn parse_document(html: &str, spec: &ConcordDocSpec, curation: &mut Curation) -> Result<(ConcordDocument, Vec<String>, usize)> {
    let page = Html::parse_document(html);
    let main = page_main(&page);
    let raw_articles = articles_of(main);
    let mut articles = Vec::new();
    let mut disclosures = Vec::new();
    let mut skipped = 0usize;
    let mut article_no: u16 = 0;

    if raw_articles.is_empty() {
        article_no += 1;
        let slug = format!("/{}/", spec.key);
        let body: Vec<ElementRef> = main.child_elements().filter(is_page_body).collect();
        let (paragraphs, anomalies) = paragraphs_of(&body, &mut curation.article(spec.key, &slug));
        for a in &anomalies {
            disclosures.push(format!("{}: {}", spec.key, a));
        }
        articles.push(ConcordArticle { article: article_no, slug, title: spec.title.to_string(), section_title: None, citation: Citation::uncited(), paragraphs });
    } else {
        for raw in &raw_articles {
            if let Some(kind) = curation.drops_article(spec.key, &raw.href) {
                skipped += 1;
                disclosures.push(format!("{}: skipped non-Triglot article '{}' ({}) -- {kind:?}, on concord-exclusions.toml", spec.key, raw.href, raw.title));
                continue;
            }
            article_no += 1;
            let (paragraphs, anomalies) = paragraphs_of(&[raw.body], &mut curation.article(spec.key, &raw.href));
            for a in &anomalies {
                disclosures.push(format!("{}/{}: {}", spec.key, raw.href, a));
            }
            if paragraphs.is_empty() {
                disclosures.push(format!("{}/{}: zero paragraphs parsed (empty article body)", spec.key, raw.href));
            }
            articles.push(ConcordArticle { article: article_no, slug: raw.href.clone(), title: raw.title.clone(), section_title: None, citation: Citation::uncited(), paragraphs });
        }
    }

    if articles.is_empty() {
        anyhow::bail!("concord::parse_document({}): zero articles found (main-content slice empty or malformed)", spec.key);
    }

    Ok((ConcordDocument { part: spec.part, key: spec.key, title: spec.title, articles }, disclosures, skipped))
}

fn page_main(page: &Html) -> ElementRef<'_> {
    let main = Selector::parse("#main-content main").expect("a constant selector parses");
    page.select(&main).next().unwrap_or_else(|| page.root_element())
}

fn is_page_body(element: &ElementRef) -> bool {
    let name = element.value().name();
    name != "h2" && !(name == "div" && element.value().classes().any(|class| class == "next-previous-box"))
}

struct RawArticle<'a> {
    href: String,
    title: String,
    body: ElementRef<'a>,
}

fn articles_of(main: ElementRef<'_>) -> Vec<RawArticle<'_>> {
    main.descendent_elements()
        .filter(|anchor| anchor.value().name() == "a")
        .filter_map(|anchor| {
            let heading = anchor.child_elements().find(|child| child.value().name() == "h3")?;
            let body = anchor.next_siblings().find_map(ElementRef::wrap).filter(|next| next.value().name() == "section")?;
            Some(RawArticle { href: anchor.attr("href").unwrap_or_default().to_string(), title: words_of(&heading), body })
        })
        .collect()
}

fn words_of(element: &ElementRef) -> String {
    collapse_ws(&element.text().collect::<String>())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceBlock {
    Heading,
    BoldParagraph,
    Paragraph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceMarker {
    id: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceNode {
    block: SourceBlock,
    marker: Option<SourceMarker>,
    text: String,
    wording: HeadingWording,
}

impl SourceNode {
    fn markup_role(&self) -> TextPartRole {
        match (self.block, &self.marker) {
            (SourceBlock::Heading | SourceBlock::BoldParagraph, None) => TextPartRole::Heading,
            _ => TextPartRole::Text,
        }
    }
}

fn paragraphs_of(body: &[ElementRef], article: &mut ArticleCuration) -> (Vec<ConcordParagraph>, Vec<String>) {
    let text_markers = article.text_markers(body);
    let nodes = source_nodes(body, &text_markers);
    article.agrees("page to source nodes", &source_characters(body, &text_markers), &characters(nodes.iter().map(|node| node.text.as_str())));
    let mut kept_nodes: Vec<(SourceNode, TextPartRole)> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if !article.drops_run(&node) {
            let role = article.role_of(&node);
            let node = if role == TextPartRole::Heading { article.read_heading(node) } else { node };
            kept_nodes.push((node, role));
        }
    }
    let numbered_by_source = kept_nodes.iter().any(|(node, _)| node.marker.is_some());
    let expected = characters(kept_nodes.iter().map(|(node, _)| node.text.as_str()));
    let groups = groups_of(kept_nodes, numbered_by_source);
    article.agrees("source nodes to paragraphs", &expected, &characters(groups.iter().map(|group| group.rendering.text())));
    let kept: Vec<Group> = groups.into_iter().filter(|group| !article.drops_unit(group.rendering.text())).collect();
    number(kept, numbered_by_source)
}

fn characters<'t>(texts: impl Iterator<Item = &'t str>) -> String {
    texts.flat_map(str::chars).filter(|c| !c.is_whitespace()).collect()
}

fn source_nodes(body: &[ElementRef], text_markers: &[NodeId]) -> Vec<SourceNode> {
    let mut walk = NodeWalk { nodes: Vec::new(), open: None, text_markers };
    for element in body {
        walk.block(*element);
    }
    walk.flush();
    walk.nodes
}

struct NodeWalk<'m> {
    nodes: Vec<SourceNode>,
    open: Option<OpenNode>,
    text_markers: &'m [NodeId],
}

struct OpenNode {
    block: SourceBlock,
    marker: Option<SourceMarker>,
    text: String,
    outside_strong: bool,
}

impl NodeWalk<'_> {
    fn block(&mut self, element: ElementRef) {
        self.flush();
        self.children(element, block_of(element.value().name()).unwrap_or(SourceBlock::Paragraph), false);
        self.flush();
    }

    fn children(&mut self, element: ElementRef, block: SourceBlock, strong: bool) {
        for child in element.children() {
            match child.value() {
                Node::Text(text) => self.text(text, block, strong),
                Node::Element(value) => {
                    let child = ElementRef::wrap(child).expect("an element node wraps as an element");
                    if let Some(id) = marker_id(value).filter(|_| !self.text_markers.contains(&child.id())) {
                        self.flush();
                        self.open = Some(OpenNode { block, marker: Some(SourceMarker { id: id.to_string(), label: words_of(&child) }), text: String::new(), outside_strong: false });
                    } else if block_of(value.name()).is_some() {
                        self.block(child);
                    } else if value.name() == "br" {
                        self.text(" ", block, strong);
                    } else {
                        self.children(child, block, strong || matches!(value.name(), "strong" | "b"));
                    }
                }
                _ => {}
            }
        }
    }

    fn text(&mut self, text: &str, block: SourceBlock, strong: bool) {
        let open = self.open.get_or_insert_with(|| OpenNode { block, marker: None, text: String::new(), outside_strong: false });
        open.text.push_str(text);
        if !strong && !text.trim().is_empty() {
            open.outside_strong = true;
        }
    }

    fn flush(&mut self) {
        let Some(open) = self.open.take() else { return };
        let text = collapse_ws(&open.text);
        if open.marker.is_none() && text.is_empty() {
            return;
        }
        let block = match open.block {
            SourceBlock::Paragraph if !open.outside_strong && !text.is_empty() => SourceBlock::BoldParagraph,
            block => block,
        };
        self.nodes.push(SourceNode { block, marker: open.marker, text, wording: HeadingWording::Source });
    }
}

fn block_of(name: &str) -> Option<SourceBlock> {
    match name {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some(SourceBlock::Heading),
        "p" | "div" | "section" | "blockquote" | "ul" | "ol" | "li" | "dl" | "dt" | "dd" | "table" | "thead" | "tbody" | "tr" | "td" | "th" => Some(SourceBlock::Paragraph),
        _ => None,
    }
}

fn marker_id(element: &scraper::node::Element) -> Option<&str> {
    element.id().filter(|_| element.name() == "span").and_then(|id| id.strip_suffix(MARKER_SUFFIX))
}

const MARKER_SUFFIX: &str = "-acontent";

fn source_characters(body: &[ElementRef], text_markers: &[NodeId]) -> String {
    body.iter()
        .flat_map(|element| element.descendants())
        .filter(|node| !node.ancestors().any(|ancestor| ancestor.value().as_element().and_then(marker_id).is_some() && !text_markers.contains(&ancestor.id())))
        .filter_map(|node| node.value().as_text().map(|text| text.to_string()))
        .flat_map(|text| text.chars().filter(|c| !c.is_whitespace()).collect::<Vec<char>>())
        .collect()
}

struct Group {
    markers: Vec<SourceMarker>,
    base: Option<u16>,
    rendering: Rendering,
    headings: Vec<Heading>,
}

fn groups_of(nodes: Vec<(SourceNode, TextPartRole)>, numbered_by_source: bool) -> Vec<Group> {
    let mut groups: Vec<Vec<(SourceNode, TextPartRole)>> = Vec::new();
    let mut pending: Vec<(SourceNode, TextPartRole)> = Vec::new();
    for (node, role) in nodes {
        let opens = match &node.marker {
            Some(marker) => groups.last().is_none_or(|group| !continues(group, marker)),
            None => !numbered_by_source && role == TextPartRole::Text,
        };
        if opens {
            let leading = match groups.last_mut() {
                None => std::mem::take(&mut pending),
                Some(current) => {
                    let first_heading = pending.iter().position(|(_, role)| *role == TextPartRole::Heading).unwrap_or(pending.len());
                    let leading = pending.split_off(first_heading);
                    current.append(&mut pending);
                    leading
                }
            };
            groups.push(leading.into_iter().chain([(node, role)]).collect());
        } else if node.marker.is_some() {
            let current = groups.last_mut().expect("a continuing marker follows the group it continues");
            current.append(&mut pending);
            current.push((node, role));
        } else {
            pending.push((node, role));
        }
    }
    match groups.last_mut() {
        Some(current) => current.append(&mut pending),
        None if !pending.is_empty() => groups.push(pending),
        None => {}
    }
    groups.into_iter().map(group_of).collect()
}

fn continues(group: &[(SourceNode, TextPartRole)], marker: &SourceMarker) -> bool {
    let base = group.iter().find_map(|(node, _)| node.marker.as_ref()).and_then(|opening| leading_digits(&opening.label));
    is_ans_suffixed(&marker.id) || (leading_digits(&marker.label).is_some() && leading_digits(&marker.label) == base)
}

fn group_of(nodes: Vec<(SourceNode, TextPartRole)>) -> Group {
    let markers: Vec<SourceMarker> = nodes.iter().filter_map(|(node, _)| node.marker.clone()).collect();
    let base = markers.first().and_then(|marker| leading_digits(&marker.label));
    let mut pieces = Vec::new();
    for (node, role) in nodes.iter().filter(|(node, _)| !node.text.is_empty()) {
        spaced_after(&mut pieces, pieces_of(*role, &node.text));
    }
    let headings = nodes.iter().filter(|(node, role)| *role == TextPartRole::Heading && !node.text.is_empty()).map(|(node, _)| Heading { text: node.text.clone(), wording: node.wording }).collect();
    Group { markers, base, rendering: Rendering::compose(&pieces), headings }
}

fn spaced_after(pieces: &mut Vec<Piece>, mut next: Vec<Piece>) {
    if !pieces.is_empty() {
        match next.first_mut().filter(|first| first.role != TextPartRole::Bracket) {
            Some(first) => first.text.insert(0, ' '),
            None => pieces.push(Piece { role: TextPartRole::Text, text: " ".to_string() }),
        }
    }
    pieces.append(&mut next);
}

fn pieces_of(role: TextPartRole, text: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut served = 0;
    for (close, _) in text.match_indices(']') {
        if let Some(open) = text[served..close].rfind('[').map(|open| served + open) {
            pieces.push(Piece { role, text: text[served..open].to_string() });
            pieces.push(Piece { role: TextPartRole::Bracket, text: text[open..=close].to_string() });
            served = close + 1;
        }
    }
    pieces.push(Piece { role, text: text[served..].to_string() });
    pieces
}

fn number(groups: Vec<Group>, numbered_by_source: bool) -> (Vec<ConcordParagraph>, Vec<String>) {
    if !numbered_by_source {
        let anomalies = if groups.is_empty() {
            Vec::new()
        } else {
            vec![format!("no native paragraph numbering in source -- {} paragraph(s) assigned synthetic sequential positions 1..{}", groups.len(), groups.len())]
        };
        let paragraphs = groups.into_iter().enumerate().map(|(i, group)| ConcordParagraph { paragraph: (i + 1) as u16, source_label: String::new(), rendering: group.rendering, headings: group.headings }).collect();
        return (paragraphs, anomalies);
    }
    let mut out = Vec::with_capacity(groups.len());
    let mut last_assigned: u16 = 0;
    let mut collisions: Vec<(u16, u16, u16)> = Vec::new();
    for group in groups {
        let assigned = match group.base {
            Some(n) if n > last_assigned => n,
            None if out.is_empty() => 0,
            _ => {
                let a = last_assigned + 1;
                if let Some(n) = group.base {
                    collisions.push((n, last_assigned, a));
                }
                a
            }
        };
        last_assigned = assigned;
        let source_label = group.markers.iter().map(|marker| marker.label.as_str()).collect::<Vec<_>>().join("/");
        out.push(ConcordParagraph { paragraph: assigned, source_label, rendering: group.rendering, headings: group.headings });
    }
    let mut anomalies = Vec::new();
    if let Some(&(first_label, first_prior, first_remap)) = collisions.first() {
        anomalies.push(format!(
            "{} source label(s) collided with an already-used paragraph number within this article (first: label '{first_label}' collided with prior {first_prior}, remapped to {first_remap}) -- all remapped to a contiguous run, never force-fit; a genuine source-side renumbering restart, not a parse failure (each paragraph's own original label survives on `ConcordParagraph.source_label`)",
            collisions.len()
        ));
    }
    (out, anomalies)
}

fn leading_digits(label: &str) -> Option<u16> {
    let digits: String = label.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn is_ans_suffixed(id: &str) -> bool {
    id.ends_with("-ans") || id.ends_with("-ans)")
}

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

/// One curated title served in place of the vendored heading, at that document's served article position.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ConcordTitleOverride {
    pub document: String,
    pub article: u16,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NonTriglotKind {
    CopyrightedTranslation,
    EditorialNote,
    SiteFurniture,
    MarkupResidue,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedArticle {
    pub document: String,
    pub slug: String,
    pub kind: NonTriglotKind,
    pub triglot: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedUnit {
    pub document: String,
    pub slug: String,
    pub text: String,
    pub kind: NonTriglotKind,
    pub triglot: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedRun {
    pub document: String,
    pub slug: String,
    pub text: String,
    pub kind: NonTriglotKind,
    pub triglot: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrippedFragment {
    pub document: String,
    pub slug: String,
    pub text: String,
    #[serde(default)]
    pub with: String,
    pub kind: NonTriglotKind,
    pub triglot: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcordExclusions {
    pub markers: Vec<String>,
    #[serde(default)]
    pub article: Vec<ExcludedArticle>,
    #[serde(default)]
    pub unit: Vec<ExcludedUnit>,
    #[serde(default)]
    pub run: Vec<ExcludedRun>,
    #[serde(default)]
    pub strip: Vec<StrippedFragment>,
}

pub fn parse_concord_exclusions(input: &str) -> Result<ConcordExclusions> {
    toml::from_str(input).context("concord-exclusions.toml: invalid TOML or does not match the markers/[[article]]/[[unit]]/[[strip]] schema")
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleCorrection {
    pub document: String,
    pub slug: String,
    pub text: String,
    #[serde(default = "one_occurrence")]
    pub occurrences: usize,
    pub role: TextPartRole,
    pub triglot_page: u16,
    pub triglot: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextMarker {
    pub document: String,
    pub slug: String,
    pub marker: String,
    pub occurrence: usize,
    pub triglot_page: u16,
    pub triglot: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcordReadings {
    pub correction: Vec<RoleCorrection>,
    #[serde(default)]
    pub text_marker: Vec<TextMarker>,
    #[serde(default)]
    pub heading: Vec<HeadingReading>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadingReading {
    pub document: String,
    pub slug: String,
    pub text: String,
    pub reading: HeadingReadingKind,
    pub triglot: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", tag = "as")]
pub enum HeadingReadingKind {
    TriglotWording { with: String, triglot_page: u16 },
    Confirmed { triglot_page: u16 },
    OurWording { with: String },
}

fn one_occurrence() -> usize {
    1
}

pub fn parse_concord_readings(input: &str) -> Result<ConcordReadings> {
    toml::from_str(input).context("concord-roles.toml: invalid TOML or does not match the [[correction]]/[[text_marker]] schema")
}

struct Curation<'a> {
    exclusions: &'a ConcordExclusions,
    readings: &'a ConcordReadings,
    article_hits: Vec<usize>,
    unit_hits: Vec<usize>,
    run_hits: Vec<usize>,
    role_hits: Vec<usize>,
    text_marker_hits: Vec<usize>,
    heading_hits: Vec<usize>,
    idle_roles: Vec<String>,
    gaps: Vec<String>,
}

impl<'a> Curation<'a> {
    fn new(exclusions: &'a ConcordExclusions, readings: &'a ConcordReadings) -> Self {
        Curation {
            exclusions,
            readings,
            article_hits: vec![0; exclusions.article.len()],
            unit_hits: vec![0; exclusions.unit.len()],
            run_hits: vec![0; exclusions.run.len()],
            role_hits: vec![0; readings.correction.len()],
            text_marker_hits: vec![0; readings.text_marker.len()],
            heading_hits: vec![0; readings.heading.len()],
            idle_roles: Vec::new(),
            gaps: Vec::new(),
        }
    }

    fn article<'c>(&'c mut self, document: &'static str, slug: &'c str) -> ArticleCuration<'c, 'a> {
        ArticleCuration { curation: self, document, slug }
    }

    fn drops_article(&mut self, document: &str, slug: &str) -> Option<NonTriglotKind> {
        let i = self.exclusions.article.iter().position(|x| x.document == document && x.slug == slug)?;
        self.article_hits[i] += 1;
        Some(self.exclusions.article[i].kind)
    }

    fn every_source_word_is_served_or_excluded(&self) -> Result<()> {
        if self.gaps.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("{} article(s) lose source text between the page and its paragraphs:\n{}", self.gaps.len(), self.gaps.join("\n"))
        }
    }

    fn every_entry_matched_once(&self) -> Result<()> {
        let articles = self.exclusions.article.iter().zip(&self.article_hits).filter(|(_, n)| **n != 1).map(|(x, n)| format!("concord-exclusions.toml: [[article]] {}{} matched {n} time(s)", x.document, x.slug));
        let units = self.exclusions.unit.iter().zip(&self.unit_hits).filter(|(_, n)| **n != 1).map(|(x, n)| format!("concord-exclusions.toml: [[unit]] {}{} {:?} matched {n} time(s)", x.document, x.slug, x.text));
        let runs = self.exclusions.run.iter().zip(&self.run_hits).filter(|(_, n)| **n != 1).map(|(x, n)| format!("concord-exclusions.toml: [[run]] {}{} {:?} matched {n} time(s)", x.document, x.slug, x.text));
        let text_markers = self.readings.text_marker.iter().zip(&self.text_marker_hits).filter(|(_, n)| **n != 1).map(|(x, n)| format!("concord-roles.toml: [[text_marker]] {}{} {:?} matched {n} time(s)", x.document, x.slug, x.marker));
        let headings = self.readings.heading.iter().zip(&self.heading_hits).filter(|(_, n)| **n != 1).map(|(x, n)| format!("concord-roles.toml: [[heading]] {}{} {:?} matched {n} heading(s)", x.document, x.slug, x.text));
        let roles = self.readings.correction.iter().zip(&self.role_hits).filter(|(x, n)| **n != x.occurrences).map(|(x, n)| format!("concord-roles.toml: [[correction]] {}{} {:?} matched {n} time(s), not {}", x.document, x.slug, x.text, x.occurrences));
        let unmatched: Vec<String> = articles.chain(units).chain(runs).chain(roles).chain(text_markers).chain(headings).chain(self.idle_roles.iter().cloned()).collect();
        if unmatched.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("every curated entry must name exactly one vendored article, unit or source node, and change it:\n{}", unmatched.join("\n"))
        }
    }
}

struct ArticleCuration<'c, 'a> {
    curation: &'c mut Curation<'a>,
    document: &'static str,
    slug: &'c str,
}

impl ArticleCuration<'_, '_> {
    fn role_of(&mut self, node: &SourceNode) -> TextPartRole {
        let markup = node.markup_role();
        let Some(i) = self.curation.readings.correction.iter().position(|x| x.document == self.document && x.slug == self.slug && x.text == node.text) else {
            return markup;
        };
        self.curation.role_hits[i] += 1;
        let corrected = self.curation.readings.correction[i].role;
        if corrected == markup {
            self.curation.idle_roles.push(format!("concord-roles.toml: [[correction]] {}{} {:?} names the role its markup already gives", self.document, self.slug, node.text));
        }
        corrected
    }

    fn text_markers(&mut self, body: &[ElementRef]) -> Vec<NodeId> {
        let mut listed = Vec::new();
        for (i, row) in self.curation.readings.text_marker.iter().enumerate().filter(|(_, row)| row.document == self.document && row.slug == self.slug) {
            let named: Vec<ElementRef> = body.iter().flat_map(|element| element.descendent_elements()).filter(|element| marker_id(element.value()) == Some(row.marker.as_str())).collect();
            if let Some(marker) = named.get(row.occurrence.saturating_sub(1)) {
                self.curation.text_marker_hits[i] += 1;
                listed.push(marker.id());
            }
        }
        listed
    }

    fn read_heading(&mut self, node: SourceNode) -> SourceNode {
        let Some(i) = self.curation.readings.heading.iter().position(|x| x.document == self.document && x.slug == self.slug && x.text == node.text) else {
            return node;
        };
        self.curation.heading_hits[i] += 1;
        match &self.curation.readings.heading[i].reading {
            HeadingReadingKind::TriglotWording { with, .. } => SourceNode { text: with.clone(), wording: HeadingWording::Source, ..node },
            HeadingReadingKind::Confirmed { triglot_page } => SourceNode { wording: HeadingWording::Confirmed { page: *triglot_page }, ..node },
            HeadingReadingKind::OurWording { with } => SourceNode { text: with.clone(), wording: HeadingWording::Ours, ..node },
        }
    }

    fn drops_run(&mut self, node: &SourceNode) -> bool {
        match self.curation.exclusions.run.iter().position(|x| x.document == self.document && x.slug == self.slug && x.text == node.text) {
            Some(i) => {
                self.curation.run_hits[i] += 1;
                true
            }
            None => false,
        }
    }

    fn drops_unit(&mut self, text: &str) -> bool {
        match self.curation.exclusions.unit.iter().position(|x| x.document == self.document && x.slug == self.slug && x.text == text) {
            Some(i) => {
                self.curation.unit_hits[i] += 1;
                true
            }
            None => false,
        }
    }

    fn agrees(&mut self, stage: &str, expected: &str, found: &str) {
        if expected == found {
            return;
        }
        let at = expected.chars().zip(found.chars()).take_while(|(a, b)| a == b).count();
        let around = |text: &str| text.chars().skip(at.saturating_sub(GAP_CONTEXT)).take(2 * GAP_CONTEXT).collect::<String>();
        self.curation.gaps.push(format!("{}{} ({stage}): expected {:?} but found {:?}", self.document, self.slug, around(expected), around(found)));
    }
}

const GAP_CONTEXT: usize = 40;

fn apply_strips(docs: &mut [ConcordDocument], strips: &[StrippedFragment]) -> Result<usize> {
    for x in strips {
        let at = format!("{}{}", x.document, x.slug);
        let article = docs
            .iter_mut()
            .filter(|d| d.key == x.document)
            .flat_map(|d| d.articles.iter_mut())
            .find(|a| a.slug == x.slug)
            .with_context(|| format!("concord-exclusions.toml: [[strip]] names {at}, which is not served"))?;
        let occurrences = |p: &ConcordParagraph| -> usize { p.rendering.pieces().iter().map(|piece| piece.text.matches(x.text.as_str()).count()).sum() };
        let total: usize = article.paragraphs.iter().map(occurrences).sum();
        if total != 1 {
            anyhow::bail!("concord-exclusions.toml: [[strip]] {:?} occurs {total} time(s) within the pieces of {at}; it must occur exactly once", x.text);
        }
        let unit = article.paragraphs.iter_mut().find(|p| occurrences(p) == 1).expect("the one occurrence lies in a paragraph");
        let mut pieces = unit.rendering.pieces();
        if pieces.iter().any(|piece| piece.role == TextPartRole::Heading && piece.text.contains(x.text.as_str())) {
            anyhow::bail!("concord-exclusions.toml: [[strip]] {:?} lies in a heading of {at}; a heading is read through a [[heading]] row of concord-roles.toml", x.text);
        }
        let piece = pieces.iter_mut().find(|piece| piece.text.contains(x.text.as_str())).expect("the one occurrence lies in a piece");
        piece.text = squeeze_ws(&piece.text.replacen(x.text.as_str(), &x.with, 1));
        unit.rendering = Rendering::compose(&without_doubled_spaces(pieces));
    }
    Ok(strips.len())
}

fn squeeze_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if !c.is_whitespace() {
            out.push(c);
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out
}

fn without_doubled_spaces(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len());
    for mut piece in pieces {
        let follows_space = out.last().is_none_or(|last| last.text.ends_with(' '));
        if follows_space {
            piece.text = piece.text.trim_start().to_string();
        }
        out.push(piece);
    }
    if let Some(last) = out.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    out
}

pub fn no_excluded_material_is_served(corpus: &ConcordCorpus, exclusions: &ConcordExclusions) -> Result<()> {
    let mut offenders = Vec::new();
    for d in &corpus.documents {
        for a in &d.articles {
            if exclusions.article.iter().any(|x| x.document == d.key && x.slug == a.slug) {
                offenders.push(format!("{}{}: an excluded article is served as article {}", d.key, a.slug, a.article));
            }
            for p in &a.paragraphs {
                let text = p.rendering.text();
                if exclusions.unit.iter().any(|x| x.document == d.key && x.slug == a.slug && x.text == text) {
                    offenders.push(format!("{}.{}.{}: an excluded unit is served: {:?}", d.part, a.article, p.paragraph, text));
                }
                for x in exclusions.strip.iter().filter(|x| x.document == d.key && x.slug == a.slug) {
                    if text.contains(x.text.as_str()) {
                        offenders.push(format!("{}.{}.{}: an excluded fragment is served: {:?}", d.part, a.article, p.paragraph, x.text));
                    }
                }
            }
        }
    }
    if offenders.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("{} piece(s) of non-Triglot material on concord-exclusions.toml are served:\n{}", offenders.len(), offenders.join("\n"))
    }
}

pub fn no_served_text_carries_a_non_triglot_marker(corpus: &ConcordCorpus, markers: &[String]) -> Result<()> {
    let mut offenders = Vec::new();
    for d in &corpus.documents {
        for a in &d.articles {
            for m in markers.iter().filter(|m| a.title.contains(m.as_str())) {
                offenders.push(format!("{}.{} title carries {m:?}: {:?}", d.part, a.article, a.title));
            }
            for p in &a.paragraphs {
                for m in markers.iter().filter(|m| p.rendering.text().contains(m.as_str())) {
                    offenders.push(format!("{}.{}.{} carries {m:?}: {:?}", d.part, a.article, p.paragraph, p.rendering.text()));
                }
            }
        }
    }
    if offenders.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("{} served Concord text(s) carry a non-Triglot marker:\n{}", offenders.len(), offenders.join("\n"))
    }
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

    fn document(key: &'static str, titles: &[&str]) -> ConcordDocument {
        let articles = titles.iter().enumerate().map(|(i, t)| ConcordArticle { article: (i + 1) as u16, slug: format!("/{key}/{i}/"), title: t.to_string(), section_title: None, citation: Citation::uncited(), paragraphs: Vec::new() }).collect();
        ConcordDocument { part: 1, key, title: key, articles }
    }

    fn titles_of(docs: &[ConcordDocument]) -> Vec<Vec<&str>> {
        docs.iter().map(|d| d.articles.iter().map(|a| a.title.as_str()).collect()).collect()
    }

    fn served(key: &'static str, slug: &str, paragraphs: &[(u16, &str)]) -> ConcordCorpus {
        let paragraphs = paragraphs.iter().map(|(n, t)| ConcordParagraph { paragraph: *n, source_label: n.to_string(), rendering: Rendering::whole(t.to_string()), headings: Vec::new() }).collect();
        let article = ConcordArticle { article: 1, slug: slug.to_string(), title: "Title".to_string(), section_title: None, citation: Citation::uncited(), paragraphs };
        ConcordCorpus { title: "The Book of Concord".to_string(), description: String::new(), documents: vec![ConcordDocument { part: 2, key, title: key, articles: vec![article] }], stats: ConcordStats::default(), admissions: Vec::new() }
    }

    fn exclusions(input: &str) -> ConcordExclusions {
        parse_concord_exclusions(input).unwrap()
    }

    const ONE_OF_EACH: &str = r#"
markers = ["http", "*"]
[[article]]
document = "ecumenical-creeds"
slug = "/ecumenical-creeds/questions/"
kind = "copyrighted-translation"
triglot = "absent"
[[unit]]
document = "ecumenical-creeds"
slug = "/ecumenical-creeds/apostles-creed/"
text = "A site note."
kind = "site-furniture"
triglot = "absent"
[[strip]]
document = "ecumenical-creeds"
slug = "/ecumenical-creeds/apostles-creed/"
text = "a mark"
kind = "editorial-note"
triglot = "absent"
"#;

    #[test]
    fn the_exclusion_law_refuses_a_served_excluded_article_naming_it() {
        // Arrange
        let corpus = served("ecumenical-creeds", "/ecumenical-creeds/questions/", &[(1, "Text.")]);
        // Act
        let refused = no_excluded_material_is_served(&corpus, &exclusions(ONE_OF_EACH)).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("ecumenical-creeds/ecumenical-creeds/questions/: an excluded article is served"), "{refused}");
    }

    #[test]
    fn the_exclusion_law_refuses_a_served_excluded_unit_naming_its_position() {
        // Arrange
        let corpus = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(1, "I believe."), (4, "A site note.")]);
        // Act
        let refused = no_excluded_material_is_served(&corpus, &exclusions(ONE_OF_EACH)).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("2.1.4: an excluded unit is served"), "{refused}");
    }

    #[test]
    fn the_exclusion_law_refuses_a_served_excluded_fragment_naming_its_position() {
        // Arrange
        let corpus = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(3, "Text with a mark in it.")]);
        // Act
        let refused = no_excluded_material_is_served(&corpus, &exclusions(ONE_OF_EACH)).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("2.1.3: an excluded fragment is served"), "{refused}");
    }

    #[test]
    fn the_exclusion_law_admits_a_corpus_that_serves_none_of_the_excluded_material() {
        // Arrange
        let corpus = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(3, "Text in it."), (4, "Another.")]);
        // Act
        let admitted = no_excluded_material_is_served(&corpus, &exclusions(ONE_OF_EACH));
        // Assert
        assert!(admitted.is_ok(), "{admitted:?}");
    }

    #[test]
    fn the_marker_law_refuses_every_served_text_carrying_a_marker_naming_each() {
        // Arrange
        let corpus = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(1, "(http://bocl.org?AP+IV+1)"), (2, "the holy catholic* Church"), (3, "Clean.")]);
        // Act
        let refused = no_served_text_carries_a_non_triglot_marker(&corpus, &exclusions(ONE_OF_EACH).markers).unwrap_err().to_string();
        // Assert
        assert!(refused.starts_with("2 served Concord text(s)"), "{refused}");
        assert!(refused.contains("2.1.1 carries \"http\""), "{refused}");
        assert!(refused.contains("2.1.2 carries \"*\""), "{refused}");
    }

    #[test]
    fn the_marker_law_refuses_a_served_title_carrying_a_marker() {
        // Arrange
        let mut corpus = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(1, "Clean.")]);
        corpus.documents[0].articles[0].title = "**Heading".to_string();
        // Act
        let refused = no_served_text_carries_a_non_triglot_marker(&corpus, &exclusions(ONE_OF_EACH).markers).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("2.1 title carries \"*\""), "{refused}");
    }

    #[test]
    fn an_exclusion_without_its_triglot_check_is_refused_by_the_parse() {
        // Arrange
        let input = "markers = []\n[[unit]]\ndocument = \"defense\"\nslug = \"/defense/x/\"\ntext = \"t\"\nkind = \"site-furniture\"\n";
        // Act
        let refused = parse_concord_exclusions(input);
        // Assert
        assert!(refused.is_err());
    }

    #[test]
    fn an_exclusion_of_a_kind_outside_the_four_is_refused_by_the_parse() {
        // Arrange
        let input = "markers = []\n[[unit]]\ndocument = \"defense\"\nslug = \"/defense/x/\"\ntext = \"t\"\nkind = \"misc\"\ntriglot = \"absent\"\n";
        // Act
        let refused = parse_concord_exclusions(input);
        // Assert
        assert!(refused.is_err());
    }

    const LINK_RESIDUE_BETWEEN_69_AND_70: &str = r#"<p><span id="a" class="bocanchor"><span id="a-acontent" class="bocanchor-content">69</span></span> Of Justification. <span id="b" class="bocanchor"><span id="b-acontent" class="bocanchor-content">1</span></span>(http://bocl.org?AP+IV+1) <span id="c" class="bocanchor"><span id="c-acontent" class="bocanchor-content">70</span></span> Nor, indeed.</p>"#;

    const RESIDUE_UNIT: &str = r#"
markers = []
[[unit]]
document = "defense"
slug = "/defense/x/"
text = "(http://bocl.org?AP+IV+1)"
kind = "site-furniture"
triglot = "absent"
"#;

    #[test]
    fn an_excluded_unit_is_dropped_before_numbering_so_the_next_paragraph_keeps_its_source_number() {
        // Arrange
        let curated = exclusions(RESIDUE_UNIT);
        // Act
        let (paragraphs, _) = read_article(LINK_RESIDUE_BETWEEN_69_AND_70, &curated, &[]);
        // Assert
        assert_eq!(numbered(&paragraphs), vec![(69, vec![text("Of Justification.")]), (70, vec![text("Nor, indeed.")])]);
    }

    const EDITOR_NOTE_BEFORE_A_PARAGRAPH: &str = r#"<h5><em>Shouldn’t this be V?</em></h5>
<p><span id="a" class="bocanchor"><span id="a-acontent" class="bocanchor-content">1</span></span>Here the adversaries urge against us.</p>"#;

    const EDITOR_NOTE_RUN: &str = r#"
markers = []
[[run]]
document = "defense"
slug = "/defense/x/"
text = "Shouldn’t this be V?"
kind = "editorial-note"
triglot = "absent"
"#;

    #[test]
    fn an_excluded_run_leaves_no_trace_in_the_paragraph_it_preceded() {
        // Arrange
        let curated = exclusions(EDITOR_NOTE_RUN);
        // Act
        let (paragraphs, _) = read_article(EDITOR_NOTE_BEFORE_A_PARAGRAPH, &curated, &[]);
        // Assert
        assert_eq!(numbered(&paragraphs), vec![(1, vec![text("Here the adversaries urge against us.")])]);
    }

    #[test]
    fn an_exclusion_entry_that_matches_nothing_is_refused_naming_it() {
        // Arrange
        let ex = exclusions(ONE_OF_EACH);
        let nothing_read = ConcordReadings::default();
        let mut curation = Curation::new(&ex, &nothing_read);
        curation.drops_article("ecumenical-creeds", "/ecumenical-creeds/questions/");
        // Act
        let refused = curation.every_entry_matched_once().unwrap_err().to_string();
        // Assert
        assert!(refused.contains("[[unit]] ecumenical-creeds/ecumenical-creeds/apostles-creed/ \"A site note.\" matched 0 time(s)"), "{refused}");
        assert!(!refused.contains("[[article]]"), "{refused}");
    }

    #[test]
    fn a_strip_replaces_its_one_occurrence_and_collapses_the_space_it_leaves() {
        // Arrange
        let mut docs = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(3, "Text with a mark in it.")]).documents;
        // Act
        let stripped = apply_strips(&mut docs, &exclusions(ONE_OF_EACH).strip).unwrap();
        // Assert
        assert_eq!(stripped, 1);
        assert_eq!(docs[0].articles[0].paragraphs[0].rendering, Rendering::whole("Text with in it.".to_string()));
    }

    #[test]
    fn a_strip_occurring_twice_in_its_article_is_refused() {
        // Arrange
        let mut docs = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[(3, "a mark and a mark")]).documents;
        // Act
        let refused = apply_strips(&mut docs, &exclusions(ONE_OF_EACH).strip).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("occurs 2 time(s) within the pieces of ecumenical-creeds/ecumenical-creeds/apostles-creed/"), "{refused}");
    }

    #[test]
    fn a_strip_naming_an_unserved_article_is_refused() {
        // Arrange
        let mut docs = served("ecumenical-creeds", "/ecumenical-creeds/nicene-creed/", &[(4, "a mark")]).documents;
        // Act
        let refused = apply_strips(&mut docs, &exclusions(ONE_OF_EACH).strip).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("names ecumenical-creeds/ecumenical-creeds/apostles-creed/, which is not served"), "{refused}");
    }

    #[test]
    fn a_curated_title_replaces_exactly_the_named_article_of_the_named_document() {
        // Arrange
        let mut docs = vec![document("preface", &["Preface", "Second"]), document("small-catechism", &["Preface", "The Ten Commandments"])];
        let titles = vec![ConcordTitleOverride { document: "small-catechism".to_string(), article: 2, title: "I. The Ten Commandments".to_string() }];
        // Act
        apply_title_overrides(&mut docs, &titles).unwrap();
        // Assert
        assert_eq!(titles_of(&docs), vec![vec!["Preface", "Second"], vec!["Preface", "I. The Ten Commandments"]]);
    }

    #[test]
    fn a_curated_title_naming_no_article_is_refused_with_its_own_row_named() {
        // Arrange
        let mut docs = vec![document("small-catechism", &["Preface"])];
        let titles = vec![ConcordTitleOverride { document: "small-catechism".to_string(), article: 2, title: "I. The Ten Commandments".to_string() }];
        // Act
        let refused = apply_title_overrides(&mut docs, &titles).unwrap_err().to_string();
        // Assert
        assert_eq!(refused, "concord-titles.toml: no article 2 in document 'small-catechism' to retitle as 'I. The Ten Commandments'");
    }

    const SC_FIRST_COMMANDMENT: &str = r#"<h4 id="the-first-commandment"><strong>The First Commandment.</strong></h4>
<h4 id="hahahugoshortcode-s0-hbhbthou-shalt-have-no-other-gods"><span id="0001" class="bocanchor"> </span><span id="sc-ten-commandments-0001" class="bocanchor"><span id="sc-ten-commandments-0001-acontent" class="bocanchor-content">1</span></span>Thou shalt have no other gods.</h4>
<p><em>What does this mean?</em></p>
<p>&ndash;Answer: <span id="%!d(string=001b)" class="bocanchor"> </span><span id="sc-ten-commandments-%!d(string=001b)" class="bocanchor"><span id="sc-ten-commandments-%!d(string=001b)-acontent" class="bocanchor-content">1b</span></span>We should fear, love, and trust in God above all things.</p>
<h4 id="the-second-commandment"><strong>The Second Commandment.</strong></h4>
<h4 id="hahahugoshortcode-s2-hbhbthou-shalt-not-take-the-name-of-the-lord-thy-god-in-vain"><span id="0002" class="bocanchor"> </span><span id="sc-ten-commandments-0002" class="bocanchor"><span id="sc-ten-commandments-0002-acontent" class="bocanchor-content">2</span></span>Thou shalt not take the name of the Lord, thy God, in vain.</h4>"#;

    #[test]
    fn a_heading_before_a_marker_opens_the_paragraph_it_titles_and_a_sub_lettered_marker_continues_it() {
        // Arrange
        let none = ConcordExclusions::default();
        // Act
        let (paragraphs, anomalies) = read_article(SC_FIRST_COMMANDMENT, &none, &[]);
        // Assert
        assert_eq!(
            (numbered(&paragraphs), anomalies),
            (
                vec![
                    (1, vec![heading("The First Commandment."), text(" Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.")]),
                    (2, vec![heading("The Second Commandment."), text(" Thou shalt not take the name of the Lord, thy God, in vain.")]),
                ],
                Vec::<String>::new()
            )
        );
    }

    const SA_OF_SIN: &str = r#"<p><span id="a" class="bocanchor"><span id="a-acontent" class="bocanchor-content">1</span></span> Here we must confess that sin originated [and entered the world] from one man [Adam].</p>"#;

    #[test]
    fn every_bracket_of_a_node_is_one_bracket_part() {
        // Arrange
        let none = ConcordExclusions::default();
        // Act
        let (paragraphs, _) = read_article(SA_OF_SIN, &none, &[]);
        // Assert
        assert_eq!(
            numbered(&paragraphs),
            vec![(1, vec![text("Here we must confess that sin originated "), bracket("[and entered the world]"), text(" from one man "), bracket("[Adam]"), text(".")])]
        );
    }

    #[test]
    fn a_nested_or_unmatched_bracket_splits_only_its_innermost_matched_pair() {
        // Arrange
        let source = "[confirmation, who [alone] may] and (a typo] here";
        // Act
        let pieces = Rendering::compose(&pieces_of(TextPartRole::Text, source)).pieces();
        // Assert
        assert_eq!(pieces, vec![text("[confirmation, who "), bracket("[alone]"), text(" may] and (a typo] here")]);
    }

    const DAILY_PRAYER: &str = r#"<p><strong>How the head of the family should teach his household to pray</strong></p>
<p><span id="a" class="bocanchor"><span id="a-acontent" class="bocanchor-content">1</span></span>In the morning you shall say:</p>
<p><strong>In the name of God the Father, Son, and Holy Ghost. Amen.</strong></p>
<p>Then go to your work with joy.</p>
<p><span id="b" class="bocanchor"><span id="b-acontent" class="bocanchor-content">2</span></span>In the evening.</p>"#;

    fn prayer_correction(role: TextPartRole) -> RoleCorrection {
        RoleCorrection { document: FIXTURE_DOCUMENT.to_string(), slug: FIXTURE_SLUG.to_string(), text: "In the name of God the Father, Son, and Holy Ghost. Amen.".to_string(), occurrences: 1, role, triglot_page: 557, triglot: "running text".to_string() }
    }

    #[test]
    fn a_bold_paragraph_is_a_heading_that_titles_the_next_paragraph_by_default() {
        // Arrange
        let none = ConcordExclusions::default();
        // Act
        let (paragraphs, _) = read_article(DAILY_PRAYER, &none, &[]);
        // Assert
        assert_eq!(
            numbered(&paragraphs),
            vec![
                (1, vec![heading("How the head of the family should teach his household to pray"), text(" In the morning you shall say:")]),
                (2, vec![heading("In the name of God the Father, Son, and Holy Ghost. Amen."), text(" Then go to your work with joy. In the evening.")]),
            ]
        );
    }

    #[test]
    fn a_role_correction_keeps_a_prayer_as_text_in_the_paragraph_that_says_it() {
        // Arrange
        let none = ConcordExclusions::default();
        let corrections = [prayer_correction(TextPartRole::Text)];
        // Act
        let (paragraphs, _) = read_article(DAILY_PRAYER, &none, &corrections);
        // Assert
        assert_eq!(
            numbered(&paragraphs),
            vec![
                (1, vec![heading("How the head of the family should teach his household to pray"), text(" In the morning you shall say: In the name of God the Father, Son, and Holy Ghost. Amen. Then go to your work with joy.")]),
                (2, vec![text("In the evening.")]),
            ]
        );
    }

    #[test]
    fn a_role_correction_that_names_the_role_its_markup_gives_is_refused() {
        // Arrange
        let none = ConcordExclusions::default();
        let corrections = [prayer_correction(TextPartRole::Heading)];
        let corrected = readings(&corrections);
        let mut curation = Curation::new(&none, &corrected);
        let page = Html::parse_fragment(DAILY_PRAYER);
        paragraphs_of(&[page.root_element()], &mut curation.article(FIXTURE_DOCUMENT, FIXTURE_SLUG));
        // Act
        let refused = curation.every_entry_matched_once().unwrap_err().to_string();
        // Assert
        assert!(refused.contains("names the role its markup already gives"), "{refused}");
    }

    #[test]
    fn a_role_correction_that_names_no_node_is_refused() {
        // Arrange
        let none = ConcordExclusions::default();
        let corrections = [prayer_correction(TextPartRole::Text)];
        let corrected = readings(&corrections);
        let curation = Curation::new(&none, &corrected);
        // Act
        let refused = curation.every_entry_matched_once().unwrap_err().to_string();
        // Assert
        assert!(refused.contains("matched 0 time(s), not 1"), "{refused}");
    }

    #[test]
    fn an_article_whose_paragraphs_lose_source_text_is_refused_naming_it() {
        // Arrange
        let none = ConcordExclusions::default();
        let nothing_read = ConcordReadings::default();
        let mut curation = Curation::new(&none, &nothing_read);
        curation.article("defense", "/defense/x/").agrees("source nodes to paragraphs", "Ofthemass.Thefirst", "Thefirst");
        // Act
        let refused = curation.every_source_word_is_served_or_excluded().unwrap_err().to_string();
        // Assert
        assert!(refused.contains("defense/defense/x/ (source nodes to paragraphs)"), "{refused}");
    }

    const AC_ARTICLE_IV: &str = r#"<p><span id="0001" class="bocanchor"> </span><span id="ac-iv-0001" class="bocanchor"><span id="ac-iv-0001-acontent" class="bocanchor-content">1</span></span> Also they teach that men cannot be justified before God by their
own strength, merits, or works, but are freely justified for
<span id="0002" class="bocanchor"> </span><span id="ac-iv-0002" class="bocanchor"><span id="ac-iv-0002-acontent" class="bocanchor-content">2</span></span> Christ&rsquo;s sake, through faith. <span id="0003" class="bocanchor"> </span><span id="ac-iv-0003" class="bocanchor"><span id="ac-iv-0003-acontent" class="bocanchor-content">3</span></span> This faith God imputes for righteousness
in His sight. Rom. 3 and 4.</p>"#;

    #[test]
    fn markers_inside_one_paragraph_tag_cut_it_into_sequential_paragraphs() {
        // Arrange
        let none = ConcordExclusions::default();
        // Act
        let (paragraphs, anomalies) = read_article(AC_ARTICLE_IV, &none, &[]);
        // Assert
        assert_eq!(
            (numbered(&paragraphs), anomalies),
            (
                vec![
                    (1, vec![text("Also they teach that men cannot be justified before God by their own strength, merits, or works, but are freely justified for")]),
                    (2, vec![text("Christ\u{2019}s sake, through faith.")]),
                    (3, vec![text("This faith God imputes for righteousness in His sight. Rom. 3 and 4.")]),
                ],
                Vec::<String>::new()
            )
        );
    }

    const LORDS_PRAYER_INTRO: &str = r#"<p><span id="%!d(string=intro)" class="bocanchor"> </span><span id="sc-lords-prayer-%!d(string=intro)" class="bocanchor"><span id="sc-lords-prayer-%!d(string=intro)-acontent" class="bocanchor-content">*</span></span>Our Father who art in heaven.</p>
<p>&ndash;Answer: <span id="%!d(string=intro-ans)" class="bocanchor"> </span><span id="sc-lords-prayer-%!d(string=intro-ans)" class="bocanchor"><span id="sc-lords-prayer-%!d(string=intro-ans)-acontent" class="bocanchor-content">*</span></span>God would thereby tenderly urge us.</p>
<p><span id="0001" class="bocanchor"> </span><span id="sc-lords-prayer-0001" class="bocanchor"><span id="sc-lords-prayer-0001-acontent" class="bocanchor-content">1</span></span>Hallowed be Thy name.</p>"#;

    #[test]
    fn an_answer_marked_as_its_questions_partner_continues_it_and_a_leading_unnumbered_unit_is_paragraph_zero() {
        // Arrange
        let none = ConcordExclusions::default();
        // Act
        let (paragraphs, _) = read_article(LORDS_PRAYER_INTRO, &none, &[]);
        // Assert
        assert_eq!(numbered(&paragraphs), vec![(0, vec![text("Our Father who art in heaven. \u{2013}Answer: God would thereby tenderly urge us.")]), (1, vec![text("Hallowed be Thy name.")])]);
    }

    #[test]
    fn uniform_star_labels_each_open_their_own_paragraph() {
        // Arrange
        let none = ConcordExclusions::default();
        let body = r#"<p><span id="sc-preface-0001-acontent" class="-content">*</span>First.</p>
<p><span id="sc-preface-0002-acontent" class="-content">*</span>Second.</p>
<p><span id="sc-preface-0003-acontent" class="-content">*</span>Third.</p>"#;
        // Act
        let (paragraphs, _) = read_article(body, &none, &[]);
        // Assert
        assert_eq!(numbered(&paragraphs), vec![(0, vec![text("First.")]), (1, vec![text("Second.")]), (2, vec![text("Third.")])]);
    }

    #[test]
    fn a_repeated_source_label_continues_its_paragraph() {
        // Arrange
        let none = ConcordExclusions::default();
        let body = r#"<p><span id="a-acontent" class="bocanchor-content">9</span>Question.<span id="b-acontent" class="bocanchor-content">9</span>Answer.</p>"#;
        // Act
        let (paragraphs, anomalies) = read_article(body, &none, &[]);
        // Assert
        assert_eq!((numbered(&paragraphs), anomalies), (vec![(9, vec![text("Question. Answer.")])], Vec::<String>::new()));
    }

    #[test]
    fn an_article_with_no_markers_is_one_paragraph_per_text_block_numbered_in_order_and_disclosed() {
        // Arrange
        let none = ConcordExclusions::default();
        let body = r#"<p><strong>Written against the Arians.</strong></p>
<p>I believe in God the Father Almighty, Maker of heaven and earth.</p>
<p>And in Jesus Christ, His only Son, our Lord.</p>"#;
        // Act
        let (paragraphs, anomalies) = read_article(body, &none, &[]);
        // Assert
        assert_eq!(
            (numbered(&paragraphs), anomalies),
            (
                vec![(1, vec![heading("Written against the Arians."), text(" I believe in God the Father Almighty, Maker of heaven and earth.")]), (2, vec![text("And in Jesus Christ, His only Son, our Lord.")])],
                vec!["no native paragraph numbering in source -- 2 paragraph(s) assigned synthetic sequential positions 1..2".to_string()]
            )
        );
    }

    #[test]
    fn an_article_is_an_anchored_third_level_heading_and_the_section_after_it_never_a_table_of_contents_link() {
        // Arrange
        let html = r#"<div id="main-content"><main><span class="toc-item"><a href="/augsburg-confession/of-justification/" class="">Article IV. Of Justification.</a></span>
<a href="/augsburg-confession/of-justification/">
  <h3>
    Article IV. Of Justification.
  </h3>
</a>
<section><p><span id="a-acontent" class="bocanchor-content">1</span>Also they teach.</p></section></main></div>"#;
        let page = Html::parse_document(html);
        // Act
        let articles: Vec<(String, String)> = articles_of(page_main(&page)).into_iter().map(|raw| (raw.href, raw.title)).collect();
        // Assert
        assert_eq!(articles, vec![("/augsburg-confession/of-justification/".to_string(), "Article IV. Of Justification.".to_string())]);
    }

    #[test]
    fn a_document_with_no_article_headings_is_one_article_without_its_page_title_or_navigation() {
        // Arrange
        let html = r#"<div class="content" id="main-content"><main>
<div class="next-previous-box"><a href="https://cph.org/concordia">&lt;&lt; Buy a Book of Concord</a></div>
<h2>Preface</h2>
<p><span id="a-acontent" class="-content">1</span>To the Readers.</p>
</main></div><footer>...</footer>"#;
        let spec = ConcordDocSpec { part: 1, key: "preface", title: "Preface to the Book of Concord" };
        let none = ConcordExclusions::default();
        // Act
        let (doc, _, _) = parse_document(html, &spec, &mut Curation::new(&none, &ConcordReadings::default())).unwrap();
        // Assert
        let read: Vec<(u16, String, Vec<(u16, Vec<Piece>)>)> = doc.articles.iter().map(|a| (a.article, a.title.clone(), numbered(&a.paragraphs))).collect();
        assert_eq!(read, vec![(1, "Preface to the Book of Concord".to_string(), vec![(1, vec![text("To the Readers.")])])]);
    }

    #[test]
    fn skipped_small_catechism_articles_are_disclosed_and_excluded_never_ingested() {
        // Arrange
        let html = r#"<div class="content" id="main-content"><main>
<a href="/small-catechism/prefaratory-notes/"><h3>Prefaratory Notes</h3></a>
<section><p>The 1986 Version is available in PDF format.</p></section>
<a href="/small-catechism/ten-commandments/"><h3>The Ten Commandments</h3></a>
<section><p><span id="a-acontent" class="bocanchor-content">1</span>Thou shalt have no other gods.</p></section>
</main></div><footer>...</footer>"#;
        let spec = ConcordDocSpec { part: 7, key: "small-catechism", title: "The Small Catechism" };
        let skips = exclusions("markers = []\n[[article]]\ndocument = \"small-catechism\"\nslug = \"/small-catechism/prefaratory-notes/\"\nkind = \"site-furniture\"\ntriglot = \"absent\"\n");
        // Act
        let (doc, disclosures, skipped) = parse_document(html, &spec, &mut Curation::new(&skips, &ConcordReadings::default())).unwrap();
        // Assert
        let read: Vec<(u16, String)> = doc.articles.iter().map(|a| (a.article, a.slug.clone())).collect();
        assert_eq!((skipped, read), (1, vec![(1, "/small-catechism/ten-commandments/".to_string())]));
        assert!(disclosures.iter().any(|d| d.contains("prefaratory-notes") && d.contains("skipped")), "disclosures: {disclosures:?}");
    }

    #[test]
    fn the_pages_entities_read_as_the_characters_they_name() {
        // Arrange
        let none = ConcordExclusions::default();
        let body = r#"<p><span id="a-acontent" class="bocanchor-content">1</span>Rock&rsquo;s &amp; a&nbsp;test &ndash; &#39;quoted&#39; &#x2019;</p>"#;
        // Act
        let (paragraphs, _) = read_article(body, &none, &[]);
        // Assert
        assert_eq!(numbered(&paragraphs), vec![(1, vec![text("Rock\u{2019}s & a test \u{2013} 'quoted' \u{2019}")])]);
    }

    const SITE_HEADINGS: &str = r#"<p><strong>NEGATIVE THESES: Contrary False Doctrine.</strong></p>
<p><span id="a-acontent" class="bocanchor-content">7</span>Accordingly, we reject.</p>
<p><strong>The Issues</strong></p>
<p><span id="b-acontent" class="bocanchor-content">8</span>It is a remarkable favor.</p>"#;

    #[test]
    fn a_heading_is_read_in_the_triglots_words_or_served_as_ours() {
        // Arrange
        let none = ConcordExclusions::default();
        let reading = |text: &str, reading: HeadingReadingKind| HeadingReading { document: FIXTURE_DOCUMENT.to_string(), slug: FIXTURE_SLUG.to_string(), text: text.to_string(), reading, triglot: "checked".to_string() };
        let readings = ConcordReadings {
            correction: Vec::new(),
            text_marker: Vec::new(),
            heading: vec![
                reading("NEGATIVE THESES: Contrary False Doctrine.", HeadingReadingKind::TriglotWording { with: "NEGATIVA. Contrary False Doctrine.".to_string(), triglot_page: 787 }),
                reading("The Issues", HeadingReadingKind::OurWording { with: "The Issues".to_string() }),
            ],
        };
        let mut curation = Curation::new(&none, &readings);
        let page = Html::parse_fragment(SITE_HEADINGS);
        // Act
        let (paragraphs, _) = paragraphs_of(&[page.root_element()], &mut curation.article(FIXTURE_DOCUMENT, FIXTURE_SLUG));
        // Assert
        let read: Vec<(u16, Vec<Heading>)> = paragraphs.iter().map(|p| (p.paragraph, p.headings.clone())).collect();
        assert_eq!(
            (read, curation.every_entry_matched_once().is_ok()),
            (
                vec![
                    (7, vec![Heading { text: "NEGATIVA. Contrary False Doctrine.".to_string(), wording: HeadingWording::Source }]),
                    (8, vec![Heading { text: "The Issues".to_string(), wording: HeadingWording::Ours }]),
                ],
                true
            )
        );
    }

    #[test]
    fn a_strip_that_would_rewrite_a_heading_is_refused() {
        // Arrange
        let mut docs = served("ecumenical-creeds", "/ecumenical-creeds/apostles-creed/", &[]).documents;
        docs[0].articles[0].paragraphs.push(ConcordParagraph { paragraph: 3, source_label: "3".to_string(), rendering: Rendering::compose(&[heading("Of a mark")]), headings: Vec::new() });
        // Act
        let refused = apply_strips(&mut docs, &exclusions(ONE_OF_EACH).strip).unwrap_err().to_string();
        // Assert
        assert!(refused.contains("lies in a heading"), "{refused}");
    }

    const FIXTURE_DOCUMENT: &str = "defense";
    const FIXTURE_SLUG: &str = "/defense/x/";

    fn read_article(fixture: &str, curated: &ConcordExclusions, roles: &[RoleCorrection]) -> (Vec<ConcordParagraph>, Vec<String>) {
        let corrected = readings(roles);
        let mut curation = Curation::new(curated, &corrected);
        let page = Html::parse_fragment(fixture);
        let (paragraphs, anomalies) = paragraphs_of(&[page.root_element()], &mut curation.article(FIXTURE_DOCUMENT, FIXTURE_SLUG));
        curation.every_source_word_is_served_or_excluded().unwrap();
        (paragraphs, anomalies)
    }

    fn readings(corrections: &[RoleCorrection]) -> ConcordReadings {
        ConcordReadings { correction: corrections.to_vec(), text_marker: Vec::new(), heading: Vec::new() }
    }

    fn numbered(paragraphs: &[ConcordParagraph]) -> Vec<(u16, Vec<Piece>)> {
        paragraphs.iter().map(|p| (p.paragraph, p.rendering.pieces())).collect()
    }

    fn heading(words: &str) -> Piece {
        Piece { role: TextPartRole::Heading, text: words.to_string() }
    }

    fn text(words: &str) -> Piece {
        Piece { role: TextPartRole::Text, text: words.to_string() }
    }

    fn bracket(words: &str) -> Piece {
        Piece { role: TextPartRole::Bracket, text: words.to_string() }
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
