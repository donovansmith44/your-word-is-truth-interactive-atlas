//! THE CANON-ORDER AUTHORITY (doc added NODE-1 fix round 1, review M-6 --
//! the order became user-visible navigation and needed its disclosure at
//! its source).
//!
//! `BOOKS` is the 66-book PROTESTANT canon in its traditional order --
//! the same books, names, and sequence the King James Version itself
//! prints: Old Testament 39 (Genesis .. Malachi, the Law / History /
//! Wisdom / Prophets arrangement of the Christian OT, not the Tanakh's
//! Torah/Nevi'im/Ketuvim order) followed by New Testament 27
//! (Matthew .. Revelation). No deuterocanon/Apocrypha (excluded
//! throughout this app -- see e.g. the red-letter adapter's own
//! Apocrypha-exclusion guard).
//!
//! AUTHORITY SCOPE -- the ARRAY POSITION here is this codebase's one
//! canonical book index (`atlas_core::refs::BookId(u8)` and every
//! `"bible/{book}.{chapter}.{verse}"` TextUnit id ride it), and, as of
//! Batch NODE-1, the array ORDER is the reader-facing navigation
//! authority: "what comes after Malachi 4" is answered by this array
//! (MAL is index 38, MAT is 39), compiled into the graph's own
//! `CanonSuccession` rows (chapter -> next chapter across book
//! boundaries, book -> next book) by
//! `atlas_graph::bible_container_adapter`. `code` is the canonical
//! 3-character dot-ref code (`GEN.1.1`), `osis` the OSIS abbreviation
//! (cross-reference/OSIS sources), `name` the display name the reader
//! sees (`BookId::name()` / `ChapterOut.book`). Changing the ORDER of
//! this array is therefore a data-visible, navigation-visible act --
//! verse ids, the version root, and every canon-succession row would all
//! move; it is pinned by `atlas-graph`'s own real-data tests.

pub struct BookInfo { pub code: &'static str, pub osis: &'static str, pub name: &'static str }

pub const BOOKS: [BookInfo; 66] = [
    BookInfo{code:"GEN",osis:"Gen",name:"Genesis"}, BookInfo{code:"EXO",osis:"Exod",name:"Exodus"},
    BookInfo{code:"LEV",osis:"Lev",name:"Leviticus"}, BookInfo{code:"NUM",osis:"Num",name:"Numbers"},
    BookInfo{code:"DEU",osis:"Deut",name:"Deuteronomy"}, BookInfo{code:"JOS",osis:"Josh",name:"Joshua"},
    BookInfo{code:"JDG",osis:"Judg",name:"Judges"}, BookInfo{code:"RUT",osis:"Ruth",name:"Ruth"},
    BookInfo{code:"1SA",osis:"1Sam",name:"1 Samuel"}, BookInfo{code:"2SA",osis:"2Sam",name:"2 Samuel"},
    BookInfo{code:"1KI",osis:"1Kgs",name:"1 Kings"}, BookInfo{code:"2KI",osis:"2Kgs",name:"2 Kings"},
    BookInfo{code:"1CH",osis:"1Chr",name:"1 Chronicles"}, BookInfo{code:"2CH",osis:"2Chr",name:"2 Chronicles"},
    BookInfo{code:"EZR",osis:"Ezra",name:"Ezra"}, BookInfo{code:"NEH",osis:"Neh",name:"Nehemiah"},
    BookInfo{code:"EST",osis:"Esth",name:"Esther"}, BookInfo{code:"JOB",osis:"Job",name:"Job"},
    BookInfo{code:"PSA",osis:"Ps",name:"Psalms"}, BookInfo{code:"PRO",osis:"Prov",name:"Proverbs"},
    BookInfo{code:"ECC",osis:"Eccl",name:"Ecclesiastes"}, BookInfo{code:"SNG",osis:"Song",name:"Song of Solomon"},
    BookInfo{code:"ISA",osis:"Isa",name:"Isaiah"}, BookInfo{code:"JER",osis:"Jer",name:"Jeremiah"},
    BookInfo{code:"LAM",osis:"Lam",name:"Lamentations"}, BookInfo{code:"EZK",osis:"Ezek",name:"Ezekiel"},
    BookInfo{code:"DAN",osis:"Dan",name:"Daniel"}, BookInfo{code:"HOS",osis:"Hos",name:"Hosea"},
    BookInfo{code:"JOL",osis:"Joel",name:"Joel"}, BookInfo{code:"AMO",osis:"Amos",name:"Amos"},
    BookInfo{code:"OBA",osis:"Obad",name:"Obadiah"}, BookInfo{code:"JON",osis:"Jonah",name:"Jonah"},
    BookInfo{code:"MIC",osis:"Mic",name:"Micah"}, BookInfo{code:"NAM",osis:"Nah",name:"Nahum"},
    BookInfo{code:"HAB",osis:"Hab",name:"Habakkuk"}, BookInfo{code:"ZEP",osis:"Zeph",name:"Zephaniah"},
    BookInfo{code:"HAG",osis:"Hag",name:"Haggai"}, BookInfo{code:"ZEC",osis:"Zech",name:"Zechariah"},
    BookInfo{code:"MAL",osis:"Mal",name:"Malachi"}, BookInfo{code:"MAT",osis:"Matt",name:"Matthew"},
    BookInfo{code:"MRK",osis:"Mark",name:"Mark"}, BookInfo{code:"LUK",osis:"Luke",name:"Luke"},
    BookInfo{code:"JHN",osis:"John",name:"John"}, BookInfo{code:"ACT",osis:"Acts",name:"Acts"},
    BookInfo{code:"ROM",osis:"Rom",name:"Romans"}, BookInfo{code:"1CO",osis:"1Cor",name:"1 Corinthians"},
    BookInfo{code:"2CO",osis:"2Cor",name:"2 Corinthians"}, BookInfo{code:"GAL",osis:"Gal",name:"Galatians"},
    BookInfo{code:"EPH",osis:"Eph",name:"Ephesians"}, BookInfo{code:"PHP",osis:"Phil",name:"Philippians"},
    BookInfo{code:"COL",osis:"Col",name:"Colossians"}, BookInfo{code:"1TH",osis:"1Thess",name:"1 Thessalonians"},
    BookInfo{code:"2TH",osis:"2Thess",name:"2 Thessalonians"}, BookInfo{code:"1TI",osis:"1Tim",name:"1 Timothy"},
    BookInfo{code:"2TI",osis:"2Tim",name:"2 Timothy"}, BookInfo{code:"TIT",osis:"Titus",name:"Titus"},
    BookInfo{code:"PHM",osis:"Phlm",name:"Philemon"}, BookInfo{code:"HEB",osis:"Heb",name:"Hebrews"},
    BookInfo{code:"JAS",osis:"Jas",name:"James"}, BookInfo{code:"1PE",osis:"1Pet",name:"1 Peter"},
    BookInfo{code:"2PE",osis:"2Pet",name:"2 Peter"}, BookInfo{code:"1JN",osis:"1John",name:"1 John"},
    BookInfo{code:"2JN",osis:"2John",name:"2 John"}, BookInfo{code:"3JN",osis:"3John",name:"3 John"},
    BookInfo{code:"JUD",osis:"Jude",name:"Jude"}, BookInfo{code:"REV",osis:"Rev",name:"Revelation"},
];

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

pub fn resolve_alias(s: &str) -> Option<crate::refs::BookId> {
    let n = norm(s);
    BOOKS.iter().position(|b| norm(b.code) == n || norm(b.osis) == n || norm(b.name) == n)
        .map(|i| crate::refs::BookId(i as u8))
}
