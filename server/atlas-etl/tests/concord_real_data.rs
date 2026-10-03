use atlas_etl::admission::{self, Admission, AdmissionStats, Admitted};
use atlas_etl::concord;
use atlas_etl::triglot::{TriglotReference, TriglotWord};
use atlas_graph_types::text::{Piece, Rendering, TextPartRole};

mod common;

use common::{curated_dir, raw_dir};

fn corpus() -> concord::ConcordCorpus {
    concord::read_all(&raw_dir().join("concord"), &curated_dir()).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first")
}

#[test]
fn ten_documents_parse_clean_with_the_expected_per_document_paragraph_counts() {
    let c = corpus();
    assert_eq!(c.documents.len(), 10);

    let counts: Vec<(u8, &str, usize, usize)> = c.documents.iter().map(|d| (d.part, d.key, d.articles.len(), d.articles.iter().map(|a| a.paragraphs.len()).sum())).collect();

    let expected: Vec<(u8, &str, usize, usize)> = vec![
        (1, "preface", 1, 25),
        (2, "ecumenical-creeds", 3, 11),
        (3, "augsburg-confession", 30, 451),
        (4, "defense", 26, 1162),
        (5, "smalcald-articles", 25, 222),
        (6, "power-and-primacy", 3, 114),
        (7, "small-catechism", 9, 70),
        (8, "large-catechism", 7, 748),
        (9, "epitome", 13, 264),
        (10, "solid-declaration", 14, 733),
    ];
    assert_eq!(counts, expected, "per-document (part, key, article_count, paragraph_count)");

    let total_paragraphs: usize = counts.iter().map(|(_, _, _, p)| p).sum();
    assert_eq!(c.stats.paragraphs, total_paragraphs);
    assert_eq!(c.stats.documents, 10);
    assert_eq!(c.stats.skipped_articles, 3, "the three non-Triglot Small Catechism articles on concord-exclusions.toml");
}

#[test]
fn spine_order_is_canonical_part_then_article_then_paragraph_ascending() {
    let c = corpus();
    let seq: Vec<(u8, u16, u16)> = c.iter_paragraphs().map(|(part, article, _a, p)| (part, article, p.paragraph)).collect();
    let mut sorted = seq.clone();
    sorted.sort();
    assert_eq!(seq, sorted, "iter_paragraphs already walks in canonical (part, article, paragraph) order -- no re-sort needed downstream");
    let distinct: std::collections::BTreeSet<_> = seq.iter().collect();
    assert_eq!(distinct.len(), seq.len(), "every (part, article, paragraph) triple is unique");
}

#[test]
fn the_small_catechisms_first_commandment_is_its_headings_then_the_commandment_and_its_meaning() {
    let c = corpus();
    let sc = c.documents.iter().find(|d| d.key == "small-catechism").unwrap();
    let ten_commandments = sc.articles.iter().find(|a| a.slug == "/small-catechism/ten-commandments/").unwrap();
    let first = ten_commandments.paragraphs.iter().find(|p| p.paragraph == 1).unwrap();
    assert_eq!(
        first.rendering.pieces(),
        vec![
            Piece { role: TextPartRole::Heading, text: "As the head of the family should teach them in a simple way to his household. The First Commandment.".to_string() },
            Piece { role: TextPartRole::Text, text: " Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.".to_string() },
        ]
    );
}

#[test]
fn ecumenical_creeds_has_no_native_numbering_disclosed_for_all_three_creeds() {
    let c = corpus();
    let creeds = c.documents.iter().find(|d| d.key == "ecumenical-creeds").unwrap();
    assert_eq!(creeds.articles.len(), 3);
    for a in &creeds.articles {
        for p in &a.paragraphs {
            assert_eq!(p.source_label, "", "no source-native label for any Ecumenical Creeds paragraph");
        }
    }
    let synthetic_disclosures = c.stats.disclosures.iter().filter(|d| d.starts_with("ecumenical-creeds") && d.contains("no native paragraph numbering")).count();
    assert_eq!(synthetic_disclosures, 3, "one disclosure per creed article");
}

#[test]
fn skipped_articles_are_named_in_the_disclosures() {
    let c = corpus();
    assert!(c.stats.disclosures.iter().any(|d| d.contains("prefaratory-notes") && d.contains("skipped")));
    assert!(c.stats.disclosures.iter().any(|d| d.contains("small-catechism-pdf") && d.contains("skipped")));
}

const SMALCALD_ARTICLES_IN_ORDER: [&str; 25] = [
    "/smalcald-articles/preface/",
    "/smalcald-articles/i/nature-of-god/",
    "/smalcald-articles/i/the-father/",
    "/smalcald-articles/i/the-son/",
    "/smalcald-articles/i/the-work-of-salvation/",
    "/smalcald-articles/ii/first-and-chief-article/",
    "/smalcald-articles/ii/of-the-mass/",
    "/smalcald-articles/ii/of-chapters-and-cloisters/",
    "/smalcald-articles/ii/of-the-papacy/",
    "/smalcald-articles/iii/of-sin/",
    "/smalcald-articles/iii/of-the-law/",
    "/smalcald-articles/iii/of-repentance/",
    "/smalcald-articles/iii/of-the-gospel/",
    "/smalcald-articles/iii/of-baptism/",
    "/smalcald-articles/iii/of-the-scarament-of-the-altar/",
    "/smalcald-articles/iii/of-the-keys/",
    "/smalcald-articles/iii/of-confession/",
    "/smalcald-articles/iii/of-excommunication/",
    "/smalcald-articles/iii/of-ordination/",
    "/smalcald-articles/iii/of-the-marriage-of-priests/",
    "/smalcald-articles/iii/of-the-church/",
    "/smalcald-articles/iii/of-good-works/",
    "/smalcald-articles/iii/of-monastic-vows/",
    "/smalcald-articles/iii/of-human-tradition/",
    "/smalcald-articles/signatories/",
];

#[test]
fn the_smalcald_articles_are_spliced_in_order_and_each_parts_lead_in_opens_its_first_article() {
    // Arrange
    let c = corpus();
    // Act
    let doc = c.documents.iter().find(|d| d.key == "smalcald-articles").unwrap();
    let slugs: Vec<&str> = doc.articles.iter().map(|a| a.slug.as_str()).collect();
    let part_three_opens = doc.articles.iter().find(|a| a.slug == "/smalcald-articles/iii/of-sin/").unwrap().paragraphs[0].rendering.pieces()[..3].to_vec();
    // Assert
    assert_eq!(slugs, SMALCALD_ARTICLES_IN_ORDER);
    assert_eq!(
        part_three_opens,
        vec![
            Piece { role: TextPartRole::Heading, text: "Concerning the following articles we may ".to_string() },
            Piece { role: TextPartRole::Bracket, text: "[will be able to]".to_string() },
            Piece { role: TextPartRole::Heading, text: " treat with learned and reasonable men, or among ourselves. The Pope and his ".to_string() },
        ]
    );
}

#[test]
fn every_source_text_node_lands_in_exactly_one_served_unit_or_one_named_exclusion() {
    // Arrange
    let root = raw_dir().join("concord");
    // Act
    let read = concord::read_all(&root, &curated_dir()).map(|c| (c.stats.paragraphs, c.stats.excluded_units));
    // Assert
    assert_eq!(read.map_err(|refusal| refusal.to_string()), Ok((SERVED_PARAGRAPHS, EXCLUDED_UNITS)));
}

const SERVED_PARAGRAPHS: usize = 3800;
const EXCLUDED_UNITS: usize = 4;

#[test]
fn the_creeds_first_article_keeps_its_title_as_a_heading_and_its_words_as_text() {
    // Arrange
    let c = corpus();
    // Act
    let first_article = paragraph(&c, "/small-catechism/the-creed/", 1).rendering.pieces();
    // Assert
    assert_eq!(
        first_article,
        vec![
            Piece { role: TextPartRole::Heading, text: "As the head of the family should teach it in a simple way to his household. The First Article. Of Creation.".to_string() },
            Piece { role: TextPartRole::Text, text: " I believe in God the Father Almighty, Maker of heaven and earth. What does this mean? \u{2013}Answer: I believe that God has made me and all creatures; that He has given me my body and soul, eyes, ears, and all my limbs, my reason, and all my senses, and still preserves them; in addition thereto, clothing and shoes, meat and drink, house and homestead, wife and children, fields, cattle, and all my goods; that He provides me richly and daily with all that I need to support this body and life, protects me from all danger, and guards me and preserves me from all evil; and all this out of pure, fatherly, divine goodness and mercy, without any merit or worthiness in me; for all which I owe it to Him to thank, praise, serve, and obey Him. This is most certainly true.".to_string() },
        ]
    );
}

#[test]
fn the_confession_form_keeps_both_confessions_and_the_absolution() {
    // Arrange
    let c = corpus();
    // Act
    let form = paragraph(&c, "/small-catechism/how-christians-confess/", 4).rendering.text().to_string();
    // Assert
    let kept: Vec<bool> = CONFESSION_FORM_WORDS.iter().map(|words| form.contains(words)).collect();
    assert_eq!(kept, vec![true; CONFESSION_FORM_WORDS.len()]);
}

const CONFESSION_FORM_WORDS: [&str; 4] = [
    "I, a poor sinner, confess myself before God guilty of all sins",
    "In particular I confess before you that I have not faithfully trained my children, domestics, and wife",
    "Dost thou believe that my forgiveness is God\u{2019}s forgiveness?",
    "As thou believest, so be it done unto thee. And by the command of our Lord Jesus Christ I forgive thee thy sins",
];

#[test]
fn every_triglot_bracket_is_one_bracket_part() {
    // Arrange
    let c = corpus();
    // Act
    let mut whole_brackets = 0usize;
    let mut stray_marks = 0usize;
    for (_, _, _, p) in c.iter_paragraphs() {
        for piece in p.rendering.pieces() {
            match piece.role {
                TextPartRole::Bracket => whole_brackets += usize::from(is_one_bracket(&piece.text)),
                _ => stray_marks += usize::from(piece.text.contains('[') || piece.text.contains(']')),
            }
        }
    }
    let brackets = c.iter_paragraphs().flat_map(|(_, _, _, p)| p.rendering.pieces()).filter(|piece| piece.role == TextPartRole::Bracket).count();
    // Assert
    assert_eq!((whole_brackets, brackets, stray_marks), (SOURCE_BRACKETS, SOURCE_BRACKETS, PIECES_WITH_AN_UNMATCHED_SOURCE_BRACKET));
}

const SOURCE_BRACKETS: usize = 1930;
const PIECES_WITH_AN_UNMATCHED_SOURCE_BRACKET: usize = 42;

fn is_one_bracket(text: &str) -> bool {
    text.starts_with('[') && text.ends_with(']') && text.matches('[').count() == 1 && text.matches(']').count() == 1
}

fn paragraph<'c>(c: &'c concord::ConcordCorpus, slug: &str, number: u16) -> &'c concord::ConcordParagraph {
    c.iter_paragraphs().find(|(_, _, a, p)| a.slug == slug && p.paragraph == number).map(|(_, _, _, p)| p).unwrap()
}

const SMALL_CATECHISM_TITLES_AS_SERVED: [&str; 9] = [
    "Luther's Preface to the Small Catechism",
    "I. The Ten Commandments",
    "II. The Creed",
    "III. The Lord's Prayer",
    "IV. The Sacrament of Holy Baptism",
    "V. Confession",
    "VI. The Sacrament of the Altar",
    "Daily Prayers",
    "Table of Duties",
];

#[test]
fn the_small_catechism_articles_are_numbered_by_chief_part_and_the_appendices_are_not() {
    // Arrange
    let c = corpus();
    // Act
    let titles: Vec<&str> = c.documents.iter().find(|d| d.key == "small-catechism").unwrap().articles.iter().map(|a| a.title.as_str()).collect();
    // Assert
    assert_eq!(titles, SMALL_CATECHISM_TITLES_AS_SERVED);
}

#[test]
fn the_served_concord_is_triglot_only_no_excluded_piece_and_no_non_triglot_marker_is_served() {
    // Arrange
    let exclusions = concord::parse_concord_exclusions(&std::fs::read_to_string(curated_dir().join("concord-exclusions.toml")).unwrap()).unwrap();
    // Act
    let c = corpus();
    // Assert
    concord::no_excluded_material_is_served(&c, &exclusions).unwrap();
    concord::no_served_text_carries_a_non_triglot_marker(&c, &exclusions.markers).unwrap();
}

#[test]
fn apology_xviii_serves_the_triglot_paragraphs_67_to_76_each_under_its_own_source_number() {
    // Arrange
    let c = corpus();
    // Act
    let of_free_will: Vec<(u16, String)> = c.documents.iter().find(|d| d.key == "defense").unwrap().articles.iter().find(|a| a.slug == "/defense/of-free-will/").unwrap().paragraphs.iter().map(|p| (p.paragraph, p.source_label.clone())).collect();
    // Assert
    assert_eq!(of_free_will, (67..=76).map(|n| (n, n.to_string())).collect::<Vec<_>>());
}

#[test]
fn the_apostles_creed_serves_its_three_triglot_paragraphs_and_no_note() {
    // Arrange
    let c = corpus();
    // Act
    let creed: Vec<String> = c.documents.iter().find(|d| d.key == "ecumenical-creeds").unwrap().articles[0].paragraphs.iter().map(|p| p.rendering.text().to_string()).collect();
    // Assert
    assert_eq!(creed.len(), 3);
    assert_eq!(creed[2], "I believe in the Holy Ghost; the holy catholic Church, the communion of saints; the forgiveness of sins; the resurrection of the body; and the life everlasting. Amen.");
}

#[test]
fn every_served_concord_span_is_admitted_against_the_triglot() {
    // Arrange
    let root = raw_dir().join("concord");
    // Act
    let admitted = concord::read_all(&root, &curated_dir()).map(|c| c.stats.admitted);
    // Assert
    assert_eq!(admitted.map_err(|refusal| refusal.to_string()), Ok(AdmissionStats { matched: MATCHED_SPANS, confirmed: CONFIRMED_BY_HAND, editorial: OUR_TITLES }));
}

const MATCHED_SPANS: usize = 3864;
const CONFIRMED_BY_HAND: usize = 6;
const OUR_TITLES: usize = 61;

#[test]
fn a_paragraph_the_triglot_does_not_print_is_refused() {
    // Arrange
    let c = corpus();
    let mut articles = c.documents.into_iter().find(|d| d.key == "augsburg-confession").unwrap();
    articles.articles.truncate(1);
    articles.articles[0].paragraphs = MODERN_PARAPHRASES.iter().zip(1..).map(|(text, paragraph)| concord::ConcordParagraph { paragraph, source_label: paragraph.to_string(), rendering: Rendering::whole(text.to_string()) }).collect();
    let policy = admission::parse_admission_policy(&std::fs::read_to_string(curated_dir().join("concord-admission.toml")).unwrap()).unwrap();
    let reference = TriglotReference::read(&raw_dir(), policy.shingle_words).unwrap();
    // Act
    let refused = admission::admit(&[articles], &[], &reference, &policy).unwrap_err();
    // Assert
    let refused_paragraphs: Vec<u16> = refused.iter().filter_map(|refusal| match refusal {
        admission::AdmissionRefusal::Unmatched { admitted: Admitted::Paragraph { paragraph, .. }, .. } => Some(*paragraph),
        _ => None,
    }).collect();
    assert_eq!(refused_paragraphs, (1..=MODERN_PARAPHRASES.len() as u16).collect::<Vec<_>>());
}

const MODERN_PARAPHRASES: [&str; 16] = [
    "Our churches teach with one accord that people cannot be made right with God by their own efforts, merits, or deeds.",
    "Rather, people are declared righteous freely because of Christ, through faith, when they trust that they are received into grace.",
    "God counts this trust as righteousness before Him, as Paul explains in the third and fourth chapters of Romans.",
    "We also teach that this faith must produce good works, because God has commanded them, not so that we may earn grace by them.",
    "The church is the gathering of all believers in which the gospel is preached purely and the sacraments are given rightly.",
    "For the true unity of the church it is enough to agree about the teaching of the gospel and the giving of the sacraments.",
    "It is not necessary that human traditions, rites, or ceremonies set up by people be the same everywhere.",
    "Concerning baptism our churches teach that it is necessary for salvation and that God offers His grace through it.",
    "Children should be baptized, because through baptism they are brought to God and received into His favor.",
    "Concerning the Lord's Supper we teach that the true body and blood of Christ are really present and are given to those who eat.",
    "Concerning confession we teach that private absolution should be kept in the churches, though listing every sin is not needed.",
    "Repentance has two parts: sorrow over sin that terrifies the conscience, and faith that trusts the promise of forgiveness.",
    "Nobody should teach publicly in the church or administer the sacraments unless properly called to that office.",
    "Church customs that can be kept without sin and that serve peace and good order in the church should be observed.",
    "Christians may lawfully hold public office, serve as judges, impose just punishments, and take an oath when required.",
    "At the end of the world Christ will return to judge, raise all the dead, and give eternal life to the faithful.",
];

#[test]
fn an_articles_admitted_spans_follow_the_triglots_order() {
    // Arrange
    let c = corpus();
    // Act
    let mut out_of_order: Vec<(Admitted, Admitted)> = Vec::new();
    let mut previous: Option<(Admitted, u8, u16, TriglotWord)> = None;
    for (admitted, admission) in &c.admissions {
        let (Admitted::Paragraph { part, article, .. }, Admission::Matched { start, .. }) = (admitted, admission) else { continue };
        if let Some((before, before_part, before_article, before_start)) = previous {
            if (before_part, before_article) == (*part, *article) && *start < before_start {
                out_of_order.push((before, *admitted));
            }
        }
        previous = Some((*admitted, *part, *article, *start));
    }
    // Assert
    assert_eq!(out_of_order, Vec::new());
}

#[test]
fn no_paragraph_text_is_admitted_as_our_own_words() {
    // Arrange
    let c = corpus();
    // Act
    let ours: Vec<&Admitted> = c.admissions.iter().filter(|(admitted, admission)| matches!(admitted, Admitted::Paragraph { .. }) && matches!(admission, Admission::Editorial { .. })).map(|(admitted, _)| admitted).collect();
    // Assert
    assert_eq!(ours, Vec::<&Admitted>::new());
}
