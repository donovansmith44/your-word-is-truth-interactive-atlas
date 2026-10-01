use std::ops::Range;

use atlas_graph_types::text::{SpanError, TokenSpan, TranslationId};

const JOINERS: [char; 4] = ['’', '\'', '–', '-'];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub ord: u16,
    pub char_start: usize,
    pub char_end: usize,
}

pub fn span(layer: &str, first: u16, last: u16) -> Result<TokenSpan, SpanError> {
    TokenSpan::new(TranslationId(layer.to_string()), first, last)
}

pub fn words_covering(chars: &Range<usize>, tokens: &[Token], layer: &str) -> Option<TokenSpan> {
    let first = tokens.iter().find(|t| t.char_start == chars.start)?;
    let last = tokens.iter().find(|t| t.char_end == chars.end)?;
    span(layer, first.ord, last.ord).ok()
}

pub fn chars_of(words: &TokenSpan, tokens: &[Token]) -> Option<Range<usize>> {
    let first = tokens.get(usize::from(words.start))?;
    let last = tokens.get(usize::from(words.end))?;
    Some(first.char_start..last.char_end)
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
