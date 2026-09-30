mod common;

use atlas_graph::tokens::{tokenize, Token};

const KJV_VERSES: usize = 31_102;
const KJV_TOKENS: usize = 790_892;
const GEN_1_1: &str = "In the beginning God created the heaven and the earth.";
const MAT_6_32_OPENING: &str = "(For after all these things do the Gentiles seek:) for";
const EDGE_TEXTS: [&str; 8] = ["", "...", " a ", "a", "(a)", "a–", "–a", "wife’s–"];

fn token(ord: u16, form: &str, char_start: usize, char_end: usize) -> (String, Token) {
    (form.to_string(), Token { ord, char_start, char_end })
}

fn words(verse: &str) -> Vec<(String, Token)> {
    let chars: Vec<char> = verse.chars().collect();
    tokenize(verse).into_iter().map(|t| (chars[t.char_start..t.char_end].iter().collect(), t)).collect()
}

#[test]
fn tokens_are_words_with_character_offsets() {
    // Act
    let tokens = words(GEN_1_1);

    // Assert
    assert_eq!(
        tokens,
        vec![
            token(0, "In", 0, 2),
            token(1, "the", 3, 6),
            token(2, "beginning", 7, 16),
            token(3, "God", 17, 20),
            token(4, "created", 21, 28),
            token(5, "the", 29, 32),
            token(6, "heaven", 33, 39),
            token(7, "and", 40, 43),
            token(8, "the", 44, 47),
            token(9, "earth", 48, 53),
        ]
    );
}

#[test]
fn an_opening_bracket_and_closing_punctuation_lie_between_the_words() {
    // Act
    let tokens = words(MAT_6_32_OPENING);

    // Assert
    assert_eq!(
        tokens,
        vec![
            token(0, "For", 1, 4),
            token(1, "after", 5, 10),
            token(2, "all", 11, 14),
            token(3, "these", 15, 20),
            token(4, "things", 21, 27),
            token(5, "do", 28, 30),
            token(6, "the", 31, 34),
            token(7, "Gentiles", 35, 43),
            token(8, "seek", 44, 48),
            token(9, "for", 51, 54),
        ]
    );
}

#[test]
fn joined_words_stay_one_word_and_offsets_count_characters_not_bytes() {
    // Act
    let tokens = ["of Beer–sheba.", "his wife’s name"].map(words);

    // Assert
    assert_eq!(
        tokens,
        [
            vec![token(0, "of", 0, 2), token(1, "Beer–sheba", 3, 13)],
            vec![token(0, "his", 0, 3), token(1, "wife’s", 4, 10), token(2, "name", 11, 15)],
        ]
    );
}

#[test]
fn every_edge_text_is_its_tokens_and_the_text_between_them() {
    // Act
    let broken: Vec<&str> = EDGE_TEXTS.into_iter().filter(|text| !is_lossless(text, &tokenize(text))).collect();

    // Assert
    assert_eq!(broken, Vec::<&str>::new());
}

#[test]
fn every_kjv_verse_is_its_tokens_and_the_text_between_them() {
    // Arrange
    let (_, verses) = atlas_etl::kjv::parse(&common::kjv_json()).expect("kjv.json must parse");
    let restored = atlas_etl::brainfuel::restore_kjv_case(&common::brainfuel_corpus(), &verses).0;

    // Act
    let tokens: Vec<(&String, Vec<Token>)> = restored.iter().map(|(dot_ref, text)| (dot_ref, tokenize(text))).collect();

    // Assert
    let mut broken: Vec<&String> = tokens.iter().filter(|(dot_ref, t)| !is_lossless(&restored[*dot_ref], t)).map(|(dot_ref, _)| *dot_ref).collect();
    broken.sort();
    assert_eq!((tokens.len(), tokens.iter().map(|(_, t)| t.len()).sum::<usize>(), broken), (KJV_VERSES, KJV_TOKENS, Vec::<&String>::new()));
}

fn is_lossless(verse: &str, tokens: &[Token]) -> bool {
    let chars: Vec<char> = verse.chars().collect();
    let mut at = 0;
    for (i, t) in tokens.iter().enumerate() {
        let (Some(gap), Some(form)) = (chars.get(at..t.char_start), chars.get(t.char_start..t.char_end)) else { return false };
        let is_word = form.first().zip(form.last()).is_some_and(|(first, last)| first.is_alphanumeric() && last.is_alphanumeric());
        if gap.iter().any(|c| c.is_alphanumeric()) || !is_word || usize::from(t.ord) != i {
            return false;
        }
        at = t.char_end;
    }
    !chars[at..].iter().any(|c| c.is_alphanumeric())
}
