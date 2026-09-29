//! The raw input tree: per-file SHA-256 leaves rolled up into name-ordered directory nodes,
//! with one root per dataset. Pure -- no `std::fs`, no I/O -- so it is testable without a
//! filesystem and reusable by both the walker (`atlas-cli`, which builds these values from
//! `data/raw`) and the verifier and writer that read and produce them.

use std::fmt::Write;

use crate::sha256::sha256_prefixed_128;

/// Domain separation for the raw tree, distinct from `canon::DOMAIN_PREFIX` (the compiled
/// sections' prefix) so a raw node hash can never collide with a section hash even over
/// identical bytes.
pub const RAW_DOMAIN_PREFIX: &[u8] = b"bible-atlas/raw/1\n";

/// A file's full SHA-256 digest. The tree carries bytes; only the manifest's TOML boundary
/// spells it as 64 lowercase hex digits, via [`Sha256::hex`] and [`Sha256::parse`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sha256(pub [u8; 32]);

/// A directory node's 128-bit domain-prefixed hash, the width D2 fixes for every build; spelled
/// as 32 lowercase hex digits at the TOML boundary only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawHash(pub [u8; 16]);

/// Why a hex spelling is not a hash: the manifest has one spelling (lowercase, exact width), so
/// anything else is refused with its position rather than read leniently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HexError {
    Length { expected: usize, found: usize },
    Digit { at: usize, found: char },
}

impl Sha256 {
    pub fn hex(&self) -> String {
        hex_of(&self.0)
    }

    pub fn parse(text: &str) -> Result<Self, HexError> {
        bytes_of(text).map(Sha256)
    }
}

impl RawHash {
    pub fn hex(&self) -> String {
        hex_of(&self.0)
    }

    pub fn parse(text: &str) -> Result<Self, HexError> {
        bytes_of(text).map(RawHash)
    }
}

/// A fetched file. `sha256` is the full digest of its bytes -- a raw file's bytes are
/// its whole identity (D2), unlike the compiled sections' logical/transport split, so it
/// earns full width rather than the sections' 128-bit truncation. `bytes` is carried for a
/// human reading `MANIFEST.toml` and for the fetch script to reject a truncated download
/// cheaply before hashing, but it is excluded from every preimage (D3): it is redundant with
/// the hash and must never be able to move a leaf's identity on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawLeaf {
    pub name: String,
    pub sha256: Sha256,
    pub bytes: u64,
}

/// A directory. `hash` is the roll-up over `children`'s lines, name-ordered, produced by
/// [`node_hash`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawNode {
    pub name: String,
    pub hash: RawHash,
    pub children: Vec<RawEntry>,
}

/// One entry of a directory: a fetched file or a nested directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawEntry {
    Leaf(RawLeaf),
    Node(RawNode),
}

impl RawEntry {
    pub fn name(&self) -> &str {
        match self {
            RawEntry::Leaf(leaf) => &leaf.name,
            RawEntry::Node(node) => &node.name,
        }
    }

    fn line(&self) -> String {
        match self {
            RawEntry::Leaf(leaf) => leaf_line(leaf),
            RawEntry::Node(node) => node_line(node),
        }
    }
}

/// `<name>|<64-hex sha256>\n`. Never `bytes` (D3).
pub fn leaf_line(leaf: &RawLeaf) -> String {
    format!("{}|{}\n", leaf.name, leaf.sha256.hex())
}

/// `<name>|<32-hex node hash>\n`, so a change deep in a subtree moves exactly its ancestors'
/// lines and nothing else.
pub fn node_line(node: &RawNode) -> String {
    format!("{}|{}\n", node.name, node.hash.hex())
}

/// The roll-up over `entries`: each entry's line (a leaf's or a nested node's), name-ordered
/// so insertion order never affects the result, concatenated and hashed under
/// [`RAW_DOMAIN_PREFIX`]. This mirrors `sections::root_of_lines`'s construction -- a domain
/// prefix over name-ordered lines -- so there is one node construction in the repo, not two,
/// but it calls [`sha256_prefixed_128`] directly rather than `root_of_lines` itself: that
/// function returns the sections' `ContentHash`, whose width depends on the `canon-ids`
/// feature (64 bits off, 128 bits on), while D2 fixes the raw tree's node hash at 128 bits in
/// every build -- the raw tree is a domain of its own and must not inherit the graph's
/// feature-gated identity width. An empty `entries` is a defined state (the hash of the
/// prefix alone), not a panic: an empty fetched directory is real.
pub fn node_hash(entries: &[RawEntry]) -> RawHash {
    let mut ordered: Vec<&RawEntry> = entries.iter().collect();
    ordered.sort_by(|a, b| a.name().cmp(b.name()));
    let mut lines = Vec::new();
    for entry in ordered {
        lines.extend_from_slice(entry.line().as_bytes());
    }
    RawHash(sha256_prefixed_128(RAW_DOMAIN_PREFIX, &lines))
}

fn hex_of(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(text, "{b:02x}");
    }
    text
}

fn bytes_of<const WIDTH: usize>(text: &str) -> Result<[u8; WIDTH], HexError> {
    let expected = WIDTH * 2;
    let found = text.chars().count();
    if found != expected {
        return Err(HexError::Length { expected, found });
    }
    let mut out = [0u8; WIDTH];
    for (at, digit) in text.chars().enumerate() {
        let value = match digit {
            '0'..='9' => digit as u8 - b'0',
            'a'..='f' => digit as u8 - b'a' + 10,
            _ => return Err(HexError::Digit { at, found: digit }),
        };
        out[at / 2] = (out[at / 2] << 4) | value;
    }
    Ok(out)
}
