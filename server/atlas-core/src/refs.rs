use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Index into `canon::BOOKS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookId(pub u8);

impl BookId {
    pub fn code(&self) -> &'static str {
        crate::canon::BOOKS[self.0 as usize].code
    }
    pub fn name(&self) -> &'static str {
        crate::canon::BOOKS[self.0 as usize].name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerseId {
    pub book: BookId,
    pub chapter: u16,
    pub verse: u16,
}

impl VerseId {
    /// Parse a strict canonical single-verse ref, e.g. `"GEN.1.1"`.
    pub fn parse_canonical(s: &str) -> Result<Self, crate::CoreError> {
        match ScriptureRef::parse(s)? {
            ScriptureRef::Verse(v) => Ok(v),
            _ => Err(crate::CoreError::BadRef(s.to_string())),
        }
    }
}

impl Serialize for VerseId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}.{}.{}", self.book.code(), self.chapter, self.verse))
    }
}

impl<'de> Deserialize<'de> for VerseId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        VerseId::parse_canonical(&s).map_err(serde::de::Error::custom)
    }
}

/// A scripture reference at book, chapter, verse, or verse-range granularity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptureRef {
    Book(BookId),
    Chapter { book: BookId, chapter: u16 },
    Passage { book: BookId, chapter: u16, from_verse: u16, to_verse: u16 },
    Verse(VerseId),
}

fn parse_positive(s: &str, whole: &str) -> Result<u16, crate::CoreError> {
    let n: u16 = s.parse().map_err(|_| crate::CoreError::BadRef(whole.to_string()))?;
    if n == 0 {
        return Err(crate::CoreError::BadRef(whole.to_string()));
    }
    Ok(n)
}

impl ScriptureRef {
    /// Strict canonical parse: `BOOK` | `BOOK.CH` | `BOOK.CH.V` | `BOOK.CH.V1-V2`.
    /// Book codes match case-insensitively; the canonical `Display` form is always uppercase.
    pub fn parse(s: &str) -> Result<Self, crate::CoreError> {
        let parts: Vec<&str> = s.split('.').collect();
        let book_code = parts[0];
        let book = crate::canon::BOOKS
            .iter()
            .position(|b| b.code.eq_ignore_ascii_case(book_code))
            .map(|i| BookId(i as u8))
            .ok_or_else(|| crate::CoreError::BadRef(s.to_string()))?;

        match parts.as_slice() {
            [_] => Ok(ScriptureRef::Book(book)),
            [_, ch] => {
                let chapter = parse_positive(ch, s)?;
                Ok(ScriptureRef::Chapter { book, chapter })
            }
            [_, ch, v] => {
                let chapter = parse_positive(ch, s)?;
                match v.split_once('-') {
                    Some((from_s, to_s)) => {
                        let from_verse = parse_positive(from_s, s)?;
                        let to_verse = parse_positive(to_s, s)?;
                        if from_verse >= to_verse {
                            return Err(crate::CoreError::BadRef(s.to_string()));
                        }
                        Ok(ScriptureRef::Passage { book, chapter, from_verse, to_verse })
                    }
                    None => {
                        let verse = parse_positive(v, s)?;
                        Ok(ScriptureRef::Verse(VerseId { book, chapter, verse }))
                    }
                }
            }
            _ => Err(crate::CoreError::BadRef(s.to_string())),
        }
    }
}

impl fmt::Display for ScriptureRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptureRef::Book(book) => write!(f, "{}", book.code()),
            ScriptureRef::Chapter { book, chapter } => write!(f, "{}.{}", book.code(), chapter),
            ScriptureRef::Passage { book, chapter, from_verse, to_verse } => {
                write!(f, "{}.{}.{}-{}", book.code(), chapter, from_verse, to_verse)
            }
            ScriptureRef::Verse(v) => write!(f, "{}.{}.{}", v.book.code(), v.chapter, v.verse),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn parse_forms() {
        assert_eq!(ScriptureRef::parse("GEN").unwrap().to_string(), "GEN");
        assert_eq!(ScriptureRef::parse("EXO.14").unwrap().to_string(), "EXO.14");
        assert_eq!(ScriptureRef::parse("EXO.14.21").unwrap().to_string(), "EXO.14.21");
        assert_eq!(ScriptureRef::parse("EXO.14.21-31").unwrap().to_string(), "EXO.14.21-31");
        assert!(ScriptureRef::parse("NOPE.1").is_err());
        assert!(ScriptureRef::parse("GEN.0.1").is_err());
        assert!(ScriptureRef::parse("EXO.14.31-21").is_err()); // inverted range
    }
    #[test]
    fn empty_segment_rejected() {
        assert!(ScriptureRef::parse("gen..1").is_err());
    }
    #[test]
    fn parse_case_insensitive_book_code_displays_uppercase() {
        assert_eq!(ScriptureRef::parse("gen.1.1").unwrap().to_string(), "GEN.1.1");
        assert_eq!(ScriptureRef::parse("Exo.14").unwrap().to_string(), "EXO.14");
    }
    #[test]
    fn aliases_resolve() {
        for (alias, code) in [("Genesis","GEN"),("Gen","GEN"),("1Sam","1SA"),("1Kgs","1KI"),
                              ("Song","SNG"),("Ps","PSA"),("Phlm","PHM"),("Rev","REV")] {
            assert_eq!(crate::canon::resolve_alias(alias).unwrap().code(), code);
        }
    }
    proptest! {
        #[test]
        fn display_parse_roundtrip(book in 0u8..66, ch in 1u16..150, v in 1u16..176, v2 in 0u16..176) {
            let b = BookId(book);
            let refs = vec![
                ScriptureRef::Book(b),
                ScriptureRef::Chapter { book: b, chapter: ch },
                ScriptureRef::Verse(VerseId { book: b, chapter: ch, verse: v }),
            ];
            for r in refs {
                prop_assert_eq!(ScriptureRef::parse(&r.to_string()).unwrap(), r);
            }
            if v2 > v {
                let p = ScriptureRef::Passage { book: b, chapter: ch, from_verse: v, to_verse: v2 };
                prop_assert_eq!(ScriptureRef::parse(&p.to_string()).unwrap(), p);
            }
        }
    }
}
