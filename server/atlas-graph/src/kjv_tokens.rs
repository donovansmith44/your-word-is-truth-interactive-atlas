//! The KJV's words, found once so that a mention or a citation can name the words it covers. A word
//! is a maximal run of letters or digits, continued across one joiner that has a letter or digit on
//! both sides; everything else lies between words. Offsets count Unicode scalars, so the tokens and
//! the text between them concatenate back to the verse.

use atlas_graph_types::text::{SpanError, TokenSpan, TranslationId};

use crate::kjv_adapter::KJV_TRANSLATION;

/// Continues a word across itself: "Beer–sheba", "wife’s", "Tubal–cain".
const JOINERS: [char; 4] = ['’', '\'', '–', '-'];

/// One word of a verse: its ordinal and the scalar range of the verse it occupies. Its text is that
/// range of the verse, which is stored once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub ord: u16,
    pub char_start: usize,
    pub char_end: usize,
}

/// The KJV words `first` through `last` of one verse, both ends included.
pub fn span(first: u16, last: u16) -> Result<TokenSpan, SpanError> {
    TokenSpan::new(TranslationId(KJV_TRANSLATION.to_string()), first, last)
}

pub fn tokenize(verse: &str) -> Vec<Token> {
    let chars: Vec<char> = verse.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if !chars[at].is_alphanumeric() {
            at += 1;
            continue;
        }
        let start = at;
        at = word_end(&chars, start);
        let ord = u16::try_from(tokens.len()).expect("a verse has fewer than 65,536 words");
        tokens.push(Token { ord, char_start: start, char_end: at });
    }
    tokens
}

/// Every step lands just past a letter or digit, so a joiner met here always has one before it.
fn word_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() {
        if chars[end].is_alphanumeric() {
            end += 1;
        } else if JOINERS.contains(&chars[end]) && chars.get(end + 1).is_some_and(|c| c.is_alphanumeric()) {
            end += 2;
        } else {
            break;
        }
    }
    end
}
