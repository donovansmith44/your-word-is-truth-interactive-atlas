use std::fmt;

use anyhow::{Context, Result};

use crate::concord::{ConcordDocument, ConcordTitleOverride};
use crate::triglot::{Coverage, TriglotLeaf, TriglotPage, TriglotReference, TriglotWord};

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionPolicy {
    pub shingle_words: usize,
    pub threshold_percent: u32,
    pub window: Vec<DocumentWindow>,
    #[serde(default)]
    pub confirmed: Vec<Confirmation>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentWindow {
    pub document: String,
    pub first_leaf: u16,
    pub last_leaf: u16,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confirmation {
    pub document: String,
    pub slug: String,
    pub begins: String,
    pub page: u16,
    pub reason: ConfirmationReason,
    pub triglot: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfirmationReason {
    OcrDamage,
    Signature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorialReason {
    OurTitle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Matched { start: TriglotWord, coverage: Coverage },
    Confirmed { page: TriglotPage, reason: ConfirmationReason },
    Editorial { reason: EditorialReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Admitted {
    Title { part: u8, article: u16 },
    Paragraph { part: u8, article: u16, paragraph: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRefusal {
    NoWindow { document: String },
    Unmatched { document: String, slug: String, admitted: Admitted, opening: String, coverage: Coverage },
    StaleConfirmation { document: String, slug: String, begins: String, coverage: Coverage },
    UnplacedConfirmation { document: String, slug: String, begins: String, uses: usize },
}

impl fmt::Display for AdmissionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdmissionRefusal::NoWindow { document } => write!(f, "concord-admission.toml: no [[window]] places document '{document}' in the Triglot"),
            AdmissionRefusal::Unmatched { document, slug, admitted, opening, coverage } => {
                write!(f, "{document}{slug} {admitted:?} is not the Triglot's ({}/{} shingles): {opening:?}", coverage.shared, coverage.total)
            }
            AdmissionRefusal::StaleConfirmation { document, slug, begins, coverage } => {
                write!(f, "concord-admission.toml: [[confirmed]] {document}{slug} {begins:?} already matches the Triglot ({}/{} shingles)", coverage.shared, coverage.total)
            }
            AdmissionRefusal::UnplacedConfirmation { document, slug, begins, uses } => write!(f, "concord-admission.toml: [[confirmed]] {document}{slug} {begins:?} names {uses} served paragraph(s), not one"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdmissionStats {
    pub matched: usize,
    pub confirmed: usize,
    pub editorial: usize,
}

impl AdmissionStats {
    pub fn of(admissions: &[(Admitted, Admission)]) -> AdmissionStats {
        let mut stats = AdmissionStats::default();
        for (_, admission) in admissions {
            match admission {
                Admission::Matched { .. } => stats.matched += 1,
                Admission::Confirmed { .. } => stats.confirmed += 1,
                Admission::Editorial { .. } => stats.editorial += 1,
            }
        }
        stats
    }
}

pub fn parse_admission_policy(input: &str) -> Result<AdmissionPolicy> {
    toml::from_str(input).context("concord-admission.toml: invalid TOML or does not match the policy, [[window]] and [[confirmed]] schema")
}

pub fn admit(documents: &[ConcordDocument], titles: &[ConcordTitleOverride], reference: &TriglotReference, policy: &AdmissionPolicy) -> Result<Vec<(Admitted, Admission)>, Vec<AdmissionRefusal>> {
    let mut admissions = Vec::new();
    let mut refusals = Vec::new();
    let mut confirmed_at = vec![0usize; policy.confirmed.len()];
    for document in documents {
        let Some(window) = policy.window.iter().find(|window| window.document == document.key) else {
            refusals.push(AdmissionRefusal::NoWindow { document: document.key.to_string() });
            continue;
        };
        let window = reference.window(TriglotLeaf(window.first_leaf), TriglotLeaf(window.last_leaf));
        for article in &document.articles {
            let mut after = TriglotWord(window.start);
            let title = Admitted::Title { part: document.part, article: article.article };
            if titles.iter().any(|ours| ours.document == document.key && ours.article == article.article) {
                admissions.push((title, Admission::Editorial { reason: EditorialReason::OurTitle }));
            } else {
                let found = reference.find(&article.title, &window, after);
                match found.start.filter(|_| found.coverage.reaches(policy.threshold_percent)) {
                    Some(start) => admissions.push((title, Admission::Matched { start, coverage: found.coverage })),
                    None => refusals.push(AdmissionRefusal::Unmatched { document: document.key.to_string(), slug: article.slug.clone(), admitted: title, opening: article.title.clone(), coverage: found.coverage }),
                }
            }
            for paragraph in &article.paragraphs {
                let admitted = Admitted::Paragraph { part: document.part, article: article.article, paragraph: paragraph.paragraph };
                let text = paragraph.rendering.text();
                let found = reference.find(text, &window, after);
                let confirmation = policy.confirmed.iter().position(|row| row.document == document.key && row.slug == article.slug && text.starts_with(row.begins.as_str()));
                if let Some(i) = confirmation {
                    confirmed_at[i] += 1;
                }
                match (found.start.filter(|_| found.coverage.reaches(policy.threshold_percent)), confirmation) {
                    (Some(start), None) => {
                        after = start;
                        admissions.push((admitted, Admission::Matched { start, coverage: found.coverage }));
                    }
                    (Some(_), Some(i)) => refusals.push(AdmissionRefusal::StaleConfirmation { document: document.key.to_string(), slug: article.slug.clone(), begins: policy.confirmed[i].begins.clone(), coverage: found.coverage }),
                    (None, Some(i)) => admissions.push((admitted, Admission::Confirmed { page: TriglotPage(policy.confirmed[i].page), reason: policy.confirmed[i].reason })),
                    (None, None) => refusals.push(AdmissionRefusal::Unmatched { document: document.key.to_string(), slug: article.slug.clone(), admitted, opening: text.chars().take(OPENING_CHARACTERS).collect(), coverage: found.coverage }),
                }
            }
        }
    }
    for (row, uses) in policy.confirmed.iter().zip(confirmed_at) {
        if uses != 1 {
            refusals.push(AdmissionRefusal::UnplacedConfirmation { document: row.document.clone(), slug: row.slug.clone(), begins: row.begins.clone(), uses });
        }
    }
    if refusals.is_empty() {
        Ok(admissions)
    } else {
        Err(refusals)
    }
}

const OPENING_CHARACTERS: usize = 80;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concord::{ConcordArticle, ConcordParagraph};
    use atlas_graph_types::text::Rendering;

    const SHINGLE: usize = 3;
    const THRESHOLD: u32 = 40;
    const PAGE: &str = "553";
    const SCAN: &str = "Article XI. Of Confession. Of Confession they teach that Private Absolution ought to be retained in the churches.";

    fn policy(confirmed: Vec<Confirmation>) -> AdmissionPolicy {
        AdmissionPolicy { shingle_words: SHINGLE, threshold_percent: THRESHOLD, window: vec![DocumentWindow { document: "augsburg-confession".to_string(), first_leaf: 0, last_leaf: 0 }], confirmed }
    }

    fn document(title: &str, paragraphs: &[&str]) -> ConcordDocument {
        let paragraphs = paragraphs.iter().zip(1..).map(|(text, paragraph)| ConcordParagraph { paragraph, source_label: paragraph.to_string(), rendering: Rendering::whole(text.to_string()) }).collect();
        ConcordDocument { part: 3, key: "augsburg-confession", title: "The Augsburg Confession", articles: vec![ConcordArticle { article: 1, slug: "/augsburg-confession/of-confession/".to_string(), title: title.to_string(), paragraphs }] }
    }

    fn confirmation(begins: &str) -> Confirmation {
        Confirmation { document: "augsburg-confession".to_string(), slug: "/augsburg-confession/of-confession/".to_string(), begins: begins.to_string(), page: 49, reason: ConfirmationReason::OcrDamage, triglot: "garbled".to_string() }
    }

    #[test]
    fn a_title_and_a_paragraph_the_scan_prints_are_matched_at_their_places() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        // Act
        let admitted = admit(&[document("Article XI. Of Confession.", &["Of Confession they teach that Private Absolution ought to be retained"])], &[], &scan, &policy(Vec::new()));
        // Assert
        assert_eq!(
            admitted,
            Ok(vec![
                (Admitted::Title { part: 3, article: 1 }, Admission::Matched { start: TriglotWord(0), coverage: Coverage { shared: 2, total: 2 } }),
                (Admitted::Paragraph { part: 3, article: 1, paragraph: 1 }, Admission::Matched { start: TriglotWord(4), coverage: Coverage { shared: 9, total: 9 } }),
            ])
        );
    }

    #[test]
    fn a_title_of_our_own_is_editorial() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        let ours = [ConcordTitleOverride { document: "augsburg-confession".to_string(), article: 1, title: "XI. Confession".to_string() }];
        // Act
        let admitted = admit(&[document("XI. Confession", &[])], &ours, &scan, &policy(Vec::new()));
        // Assert
        assert_eq!(admitted, Ok(vec![(Admitted::Title { part: 3, article: 1 }, Admission::Editorial { reason: EditorialReason::OurTitle })]));
    }

    #[test]
    fn a_paragraph_neither_matched_nor_confirmed_is_refused_with_its_coverage() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        // Act
        let refused = admit(&[document("Article XI. Of Confession.", &["We keep private confession in our congregations today."])], &[], &scan, &policy(Vec::new()));
        // Assert
        assert_eq!(
            refused,
            Err(vec![AdmissionRefusal::Unmatched {
                document: "augsburg-confession".to_string(),
                slug: "/augsburg-confession/of-confession/".to_string(),
                admitted: Admitted::Paragraph { part: 3, article: 1, paragraph: 1 },
                opening: "We keep private confession in our congregations today.".to_string(),
                coverage: Coverage { shared: 0, total: 6 },
            }])
        );
    }

    #[test]
    fn a_garbled_paragraph_confirmed_by_hand_is_admitted_with_its_page() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        // Act
        let admitted = admit(&[document("Article XI. Of Confession.", &["Of Confession they teach that Private Absolution ought to be retained", "in the enumeration of all sins is not necessary for confession."])], &[], &scan, &policy(vec![confirmation("in the enumeration")]));
        // Assert
        assert_eq!(admitted.map(|admitted| admitted[2]), Ok((Admitted::Paragraph { part: 3, article: 1, paragraph: 2 }, Admission::Confirmed { page: TriglotPage(49), reason: ConfirmationReason::OcrDamage })));
    }

    #[test]
    fn a_confirmation_for_a_paragraph_the_scan_matches_is_refused_as_stale() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        // Act
        let refused = admit(&[document("Article XI. Of Confession.", &["Of Confession they teach that Private Absolution ought to be retained"])], &[], &scan, &policy(vec![confirmation("Of Confession they teach")]));
        // Assert
        assert_eq!(
            refused,
            Err(vec![AdmissionRefusal::StaleConfirmation {
                document: "augsburg-confession".to_string(),
                slug: "/augsburg-confession/of-confession/".to_string(),
                begins: "Of Confession they teach".to_string(),
                coverage: Coverage { shared: 9, total: 9 },
            }])
        );
    }

    #[test]
    fn a_confirmation_naming_no_served_paragraph_is_refused() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        // Act
        let refused = admit(&[document("Article XI. Of Confession.", &[])], &[], &scan, &policy(vec![confirmation("Nothing served begins so")]));
        // Assert
        assert_eq!(
            refused,
            Err(vec![AdmissionRefusal::UnplacedConfirmation { document: "augsburg-confession".to_string(), slug: "/augsburg-confession/of-confession/".to_string(), begins: "Nothing served begins so".to_string(), uses: 0 }])
        );
    }

    #[test]
    fn a_document_the_policy_places_nowhere_in_the_scan_is_refused() {
        // Arrange
        let scan = TriglotReference::of_one_leaf(SCAN, PAGE, SHINGLE);
        let nowhere = AdmissionPolicy { window: Vec::new(), ..policy(Vec::new()) };
        // Act
        let refused = admit(&[document("Article XI. Of Confession.", &[])], &[], &scan, &nowhere);
        // Assert
        assert_eq!(refused, Err(vec![AdmissionRefusal::NoWindow { document: "augsburg-confession".to_string() }]));
    }
}
