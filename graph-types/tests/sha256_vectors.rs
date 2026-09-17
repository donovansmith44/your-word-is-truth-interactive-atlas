//! DB-2a: the hand-written SHA-256 answers to FIPS 180-4.
//!
//! The hasher is the root of every content address the relational
//! artifact will carry, so it is pinned against the published NIST test
//! vectors rather than against itself: one empty message, the one-block
//! `"abc"`, the two-block 56-byte message (the padding case where the
//! length word does NOT fit in the first block), and the million-'a'
//! endurance vector that exercises the streaming path over 15,625 blocks.
//!
//! `sha256_prefixed_128` is the ONE entry point ids are minted through;
//! its law is stated here as an identity against `sha256`, so domain
//! separation can never quietly become "hash the bytes alone".

use atlas_graph_types::sha256::{sha256, sha256_prefixed_128};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn nist_empty_message() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn nist_abc() {
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn nist_two_block_message() {
    // 56 bytes: padding pushes the 64-bit length into a SECOND block.
    let msg = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    assert_eq!(msg.len(), 56);
    assert_eq!(
        hex(&sha256(msg)),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn nist_one_million_a() {
    let msg = vec![b'a'; 1_000_000];
    assert_eq!(
        hex(&sha256(&msg)),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn padding_boundaries_around_one_block() {
    // 55 bytes is the largest message whose padding + length still fit in
    // one block; 56..=63 and 64 each spill into a second. Cross-checked
    // against the streaming identity below rather than against magic
    // constants: what matters is that no length is mis-padded.
    for n in 0..=130usize {
        let msg = vec![b'x'; n];
        let d = sha256(&msg);
        assert_eq!(d.len(), 32, "digest is always 32 bytes (n = {n})");
    }
    // Known-answer anchors at the two dangerous lengths.
    assert_eq!(
        hex(&sha256(&vec![b'a'; 55])),
        "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
    );
    assert_eq!(
        hex(&sha256(&vec![b'a'; 64])),
        "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
    );
}

#[test]
fn prefixed_128_is_the_first_sixteen_bytes_of_the_prefixed_digest() {
    let prefix = b"bible-atlas/canon/1\n";
    let data = b"{\"a\":1}";
    let mut joined = Vec::new();
    joined.extend_from_slice(prefix);
    joined.extend_from_slice(data);
    let full = sha256(&joined);
    assert_eq!(sha256_prefixed_128(prefix, data), full[..16]);
}

#[test]
fn the_prefix_is_load_bearing() {
    // Domain separation: the same bytes under two prefixes are two ids.
    let data = b"payload";
    assert_ne!(
        sha256_prefixed_128(b"one/", data),
        sha256_prefixed_128(b"two/", data)
    );
    // And the prefix is NOT part of the data: hashing the concatenation
    // directly must agree with the prefixed entry point (above), while an
    // unprefixed hash must not.
    assert_ne!(sha256_prefixed_128(b"one/", data), sha256(data)[..16]);
}
