//! A text's words, found once so that a mention or a citation can name the words it covers. A word
//! is a maximal run of letters or digits, continued across one joiner that has a letter or digit on
//! both sides; everything else lies between words. Offsets count Unicode scalars, so the tokens and
//! the text between them concatenate back to the text. The KJV's verses and the Book of Concord's
//! paragraphs are cut into words by this one rule.

use std::ops::Range;

use atlas_graph_types::text::{SpanError, TokenSpan, TranslationId};

/// Continues a word across itself: "Beer–sheba", "wife’s", "Tubal–cain".
const JOINERS: [char; 4] = ['’', '\'', '–', '-'];

/// One word of a text: its ordinal and the scalar range of the text it occupies. Its text is that
/// range of the text, which is stored once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub ord: u16,
    pub char_start: usize,
    pub char_end: usize,
}

/// The words `first` through `last` of one unit in `layer`, both ends included.
pub fn span(layer: &str, first: u16, last: u16) -> Result<TokenSpan, SpanError> {
    TokenSpan::new(TranslationId(layer.to_string()), first, last)
}

/// The words of `layer` that `chars` covers: `None` unless it starts where a word starts and ends
/// where a word ends.
pub fn words_covering(chars: &Range<usize>, tokens: &[Token], layer: &str) -> Option<TokenSpan> {
    let first = tokens.iter().find(|t| t.char_start == chars.start)?;
    let last = tokens.iter().find(|t| t.char_end == chars.end)?;
    span(layer, first.ord, last.ord).ok()
}

pub fn tokenize(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if !chars[at].is_alphanumeric() {
            at += 1;
            continue;
        }
        let start = at;
        at = word_end(&chars, start);
        let ord = u16::try_from(tokens.len()).expect("a unit has fewer than 65,536 words");
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
