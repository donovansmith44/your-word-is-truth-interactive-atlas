use std::collections::{HashMap, HashSet};

use crate::refs::ScriptureRef;
use crate::xrefs::span_member_verses;

/// Deduplicated by the (id, question) PAIR, never by id alone: one span may cite the
/// same item through two different questions, and both citations are reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatechismRef {
    pub id: String,
    pub name: String,
    pub question: Option<String>,
}

/// First-seen order: member verses in span order, each verse's citations in stored
/// order. An id in `verse_to_items` with no entry in `item_names` is skipped rather
/// than panicked on.
pub fn items_for_span(
    span: &ScriptureRef,
    verse_to_items: &HashMap<String, Vec<(String, Option<String>)>>,
    item_names: &HashMap<String, String>,
) -> Vec<CatechismRef> {
    let mut seen: HashSet<(&str, Option<&str>)> = HashSet::new();
    let mut out: Vec<CatechismRef> = Vec::new();

    for member in span_member_verses(span) {
        let key = format!("{}.{}.{}", member.book.code(), member.chapter, member.verse);
        let Some(hits) = verse_to_items.get(&key) else { continue };
        for (id, question) in hits {
            if !seen.insert((id.as_str(), question.as_deref())) {
                continue;
            }
            if let Some(name) = item_names.get(id) {
                out.push(CatechismRef { id: id.clone(), name: name.clone(), question: question.clone() });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(code: &str) -> crate::refs::BookId {
        crate::canon::resolve_alias(code).unwrap()
    }

    fn verse_span(code: &str, chapter: u16, v: u16) -> ScriptureRef {
        ScriptureRef::Verse(crate::refs::VerseId { book: book(code), chapter, verse: v })
    }

    fn passage_span(code: &str, chapter: u16, from_verse: u16, to_verse: u16) -> ScriptureRef {
        ScriptureRef::Passage { book: book(code), chapter, from_verse, to_verse }
    }

    fn fixture() -> (HashMap<String, Vec<(String, Option<String>)>>, HashMap<String, String>) {
        let mut verse_to_items = HashMap::new();
        verse_to_items.insert(
            "MAT.28.19".to_string(),
            vec![("baptism-1".to_string(), None), ("commandments-close".to_string(), Some("God Visits and Shows Mercy".to_string()))],
        );
        verse_to_items.insert("MAT.28.20".to_string(), vec![("baptism-1".to_string(), None)]);
        let mut item_names = HashMap::new();
        item_names.insert("baptism-1".to_string(), "Baptism — Part the First".to_string());
        item_names.insert("commandments-close".to_string(), "What Does God Say of All These Commandments?".to_string());
        (verse_to_items, item_names)
    }

    #[test]
    fn single_verse_span_returns_its_own_items_in_stored_order() {
        let (verse_to_items, item_names) = fixture();
        let out = items_for_span(&verse_span("MAT", 28, 19), &verse_to_items, &item_names);
        assert_eq!(out, vec![
            CatechismRef { id: "baptism-1".into(), name: "Baptism — Part the First".into(), question: None },
            CatechismRef {
                id: "commandments-close".into(),
                name: "What Does God Say of All These Commandments?".into(),
                question: Some("God Visits and Shows Mercy".into()),
            },
        ]);
    }

    #[test]
    fn passage_span_unions_across_member_verses_without_duplicates() {
        let (verse_to_items, item_names) = fixture();
        let out = items_for_span(&passage_span("MAT", 28, 17, 20), &verse_to_items, &item_names);
        assert_eq!(out.len(), 2, "{out:?}");
        assert_eq!(out[0].id, "baptism-1");
        assert_eq!(out[1].id, "commandments-close");
    }

    #[test]
    fn verse_with_no_citations_returns_empty() {
        let (verse_to_items, item_names) = fixture();
        let out = items_for_span(&verse_span("MAT", 28, 17), &verse_to_items, &item_names);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn unknown_id_missing_from_item_names_is_skipped_not_panicked() {
        let mut verse_to_items = HashMap::new();
        verse_to_items.insert("GEN.1.1".to_string(), vec![("ghost-item".to_string(), None)]);
        let item_names = HashMap::new();
        let out = items_for_span(&verse_span("GEN", 1, 1), &verse_to_items, &item_names);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn book_and_chapter_refs_have_no_member_verses_so_return_empty() {
        let (verse_to_items, item_names) = fixture();
        assert!(items_for_span(&ScriptureRef::Book(book("MAT")), &verse_to_items, &item_names).is_empty());
        assert!(items_for_span(&ScriptureRef::Chapter { book: book("MAT"), chapter: 28 }, &verse_to_items, &item_names).is_empty());
    }

    #[test]
    fn same_item_via_two_different_questions_is_not_collapsed() {
        let mut verse_to_items = HashMap::new();
        verse_to_items.insert("EXO.20.3".to_string(), vec![("commandment-1".to_string(), Some("God Alone as Judge".to_string()))]);
        verse_to_items.insert("EXO.20.5".to_string(), vec![("commandment-1".to_string(), Some("Worship God Alone".to_string()))]);
        let mut item_names = HashMap::new();
        item_names.insert("commandment-1".to_string(), "The First Commandment".to_string());

        let out = items_for_span(&passage_span("EXO", 20, 1, 6), &verse_to_items, &item_names);
        assert_eq!(out.len(), 2, "{out:?}");
        assert_eq!(out[0], CatechismRef { id: "commandment-1".into(), name: "The First Commandment".into(), question: Some("God Alone as Judge".into()) });
        assert_eq!(out[1], CatechismRef { id: "commandment-1".into(), name: "The First Commandment".into(), question: Some("Worship God Alone".into()) });
    }

    #[test]
    fn same_item_same_question_from_two_verses_dedupes_to_one_row() {
        let mut verse_to_items = HashMap::new();
        verse_to_items.insert("GEN.1.1".to_string(), vec![("creed-1".to_string(), Some("Creation".to_string()))]);
        verse_to_items.insert("GEN.1.2".to_string(), vec![("creed-1".to_string(), Some("Creation".to_string()))]);
        let mut item_names = HashMap::new();
        item_names.insert("creed-1".to_string(), "The First Article".to_string());

        let out = items_for_span(&passage_span("GEN", 1, 1, 2), &verse_to_items, &item_names);
        assert_eq!(out.len(), 1, "{out:?}");
    }
}
