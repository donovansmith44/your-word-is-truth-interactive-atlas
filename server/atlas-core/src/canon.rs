//! A book's array POSITION is this codebase's book index and the array ORDER is the reader's
//! navigation order, so reordering `BOOKS` moves verse ids and canon-succession rows with it.

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

atlas_graph_types::vocabulary! {
    /// Which half of the canon a book belongs to: the Old Testament, or the New.
    Testament {
        Old => "OT",
        New => "NT",
    }
}

impl Testament {
    /// A book's testament is its POSITION in `BOOKS`, never a second list:
    /// everything from `FIRST_NEW_TESTAMENT_BOOK` onward is the New Testament.
    pub fn of_book_index(index: usize) -> Testament {
        if index < BOOKS_IN_THE_OLD_TESTAMENT {
            Testament::Old
        } else {
            Testament::New
        }
    }
}

/// Naming the book rather than an index keeps the testament boundary derived from
/// `BOOKS`, so reordering the canon moves it without anyone editing it.
const FIRST_NEW_TESTAMENT_BOOK: &str = "MAT";

pub const BOOKS_IN_THE_OLD_TESTAMENT: usize = position_of(FIRST_NEW_TESTAMENT_BOOK);

pub const fn position_of(code: &str) -> usize {
    let mut index = 0;
    while index < BOOKS.len() {
        if code_is(BOOKS[index].code, code) {
            return index;
        }
        index += 1;
    }
    panic!("the code must name a book of BOOKS")
}

const fn code_is(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

pub fn resolve_alias(s: &str) -> Option<crate::refs::BookId> {
    let n = norm(s);
    BOOKS.iter().position(|b| norm(b.code) == n || norm(b.osis) == n || norm(b.name) == n)
        .map(|i| crate::refs::BookId(i as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_testament_round_trips_through_the_two_group_labels_the_reader_sees() {
        // Arrange
        let every_variant = Testament::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        let back: Vec<Testament> = serde_json::from_str(&json).unwrap();
        // Assert
        assert_eq!(json, r#"["OT","NT"]"#);
        assert_eq!(back, every_variant.to_vec());
    }

    #[test]
    #[should_panic(expected = "the code must name a book of BOOKS")]
    fn a_code_naming_no_book_of_the_canon_fails_loud_rather_than_counting_every_book() {
        // Arrange
        let not_a_book = "XXX";
        // Act
        position_of(not_a_book);
    }

    #[test]
    fn a_books_testament_is_read_from_its_own_place_in_the_canon() {
        // Arrange
        let books = ["GEN", "MAL", "MAT", "REV"];
        // Act
        let testaments: Vec<(&str, Testament)> = books
            .iter()
            .map(|code| (*code, Testament::of_book_index(BOOKS.iter().position(|b| b.code == *code).unwrap())))
            .collect();
        // Assert
        assert_eq!(
            testaments,
            vec![("GEN", Testament::Old), ("MAL", Testament::Old), ("MAT", Testament::New), ("REV", Testament::New)]
        );
    }
}
