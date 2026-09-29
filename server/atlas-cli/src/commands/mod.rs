pub mod chapter;
pub mod edges;
pub mod find;
pub mod help;
pub mod kinds;
pub mod node;
pub mod raw;
pub mod tutorial;
pub mod verify;
pub mod verse;

pub(crate) fn commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
