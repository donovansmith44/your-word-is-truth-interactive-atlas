//! Batch F ("the small catechism"): verse <-> catechism item cross-linking.
//! Pure, no `AtlasData` dependency (mirrors `crate::xrefs`'s own narrow-
//! slice-in, narrow-value-out shape) -- `AtlasData::catechism_items_for_span`
//! is the thin wrapper that hands this module its own derived indexes
//! directly, per `atlas-server::handlers`'s own "no business logic in
//! handlers.rs" rule.
//!
//! Reuses `crate::xrefs::span_member_verses` verbatim for "which verses does
//! this span cover" -- a single verse, or `from_verse..=to_verse` for a
//! same-chapter passage; `Book`/`Chapter` refs have no defined member-verse
//! list, same as for cross-references (`atlas-server::handlers::catechism_for_span`
//! 400s those as `bad_ref` before this function is ever called, mirroring
//! `handlers::xrefs`'s own precedent exactly).

use std::collections::{HashMap, HashSet};

use crate::refs::ScriptureRef;
use crate::xrefs::span_member_verses;

/// One catechism item cited by a span, resolved to its own display name.
/// No "votes" (unlike `xrefs::AggregatedXref`) -- an item is cited or it
/// isn't; a passage citing the same item from two different member verses
/// still lists it once, not twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatechismRef {
    pub id: String,
    pub name: String,
}

/// The union of catechism item ids cited by any member verse of `span`, in
/// first-seen order (iterating member verses in span order, each verse's
/// own item list in its stored order), each resolved to its own (id, name)
/// via `verse_to_items`/`item_names`. An id present in `verse_to_items` but
/// missing from `item_names` (should never happen in practice -- both are
/// built together, in the same pass, by `AtlasData::finish()`) is skipped
/// rather than panicking, same soft-fail-on-an-impossible-inconsistency
/// policy `xrefs::aggregate_span_xrefs`'s own preview lookup already
/// follows.
pub fn items_for_span(
    span: &ScriptureRef,
    verse_to_items: &HashMap<String, Vec<String>>,
    item_names: &HashMap<String, String>,
) -> Vec<CatechismRef> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out: Vec<CatechismRef> = Vec::new();

    for member in span_member_verses(span) {
        let key = format!("{}.{}.{}", member.book.code(), member.chapter, member.verse);
        let Some(ids) = verse_to_items.get(&key) else { continue };
        for id in ids {
            if !seen.insert(id.as_str()) {
                continue;
            }
            if let Some(name) = item_names.get(id) {
                out.push(CatechismRef { id: id.clone(), name: name.clone() });
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

    // A small fixture spanning MAT.28.17..MAT.28.20 (the Great Commission),
    // with MAT.28.19 citing TWO items (in a deliberately reverse-of-id order,
    // to prove the OUTPUT preserves stored order rather than re-sorting) and
    // MAT.28.20 citing one of the SAME two items again (to exercise dedup).
    fn fixture() -> (HashMap<String, Vec<String>>, HashMap<String, String>) {
        let mut verse_to_items = HashMap::new();
        verse_to_items.insert("MAT.28.19".to_string(), vec!["baptism-1".to_string(), "commandments-close".to_string()]);
        verse_to_items.insert("MAT.28.20".to_string(), vec!["baptism-1".to_string()]);
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
            CatechismRef { id: "baptism-1".into(), name: "Baptism — Part the First".into() },
            CatechismRef { id: "commandments-close".into(), name: "What Does God Say of All These Commandments?".into() },
        ]);
    }

    #[test]
    fn passage_span_unions_across_member_verses_without_duplicates() {
        let (verse_to_items, item_names) = fixture();
        // MAT.28.17-20: member verses 17 (no citations), 18 (none), 19
        // (both items), 20 (baptism-1 again). The union must list baptism-1
        // exactly ONCE, first-seen at verse 19, followed by
        // commandments-close (also first-seen at verse 19) -- never twice,
        // never in a different order.
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
        verse_to_items.insert("GEN.1.1".to_string(), vec!["ghost-item".to_string()]);
        let item_names = HashMap::new(); // deliberately empty -- "ghost-item" has no name entry
        let out = items_for_span(&verse_span("GEN", 1, 1), &verse_to_items, &item_names);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn book_and_chapter_refs_have_no_member_verses_so_return_empty() {
        let (verse_to_items, item_names) = fixture();
        assert!(items_for_span(&ScriptureRef::Book(book("MAT")), &verse_to_items, &item_names).is_empty());
        assert!(items_for_span(&ScriptureRef::Chapter { book: book("MAT"), chapter: 28 }, &verse_to_items, &item_names).is_empty());
    }
}
