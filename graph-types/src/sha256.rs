//! SHA-256 (FIPS 180-4), hand-written, std only.
//!
//! DB-2a needs a hash that does NOT move when the toolchain moves.
//! `DefaultHasher` -- what every content address in this project used
//! before the relational artifact -- is explicitly documented by std as
//! an algorithm that "may change between releases", so a `rustup update`
//! could rewrite every id in the artifact with zero data change. A crate
//! would fix that and break the OTHER law (`graph-types` is
//! zero-dependency: see `tests/zero_deps.rs`), so the hasher lives here.
//!
//! Scope is exactly one responsibility: the compression function plus the
//! two entry points ids are minted through. No streaming API, no `Hasher`
//! impl, no incremental state -- the artifact hashes whole byte strings
//! it already holds, and every extra surface is another thing to get
//! wrong. `tests/sha256_vectors.rs` pins it against the published NIST
//! vectors, including the million-'a' case.

/// The 64 round constants: the first 32 bits of the fractional parts of
/// the cube roots of the first 64 primes (FIPS 180-4 §4.2.2).
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The initial hash value: the first 32 bits of the fractional parts of
/// the square roots of the first 8 primes (FIPS 180-4 §5.3.3).
const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// One 64-byte block through the compression function (FIPS 180-4 §6.2.2).
fn compress(h: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for (i, word) in w.iter_mut().take(16).enumerate() {
        let b = i * 4;
        *word = u32::from_be_bytes([block[b], block[b + 1], block[b + 2], block[b + 3]]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);

        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(b);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
    h[5] = h[5].wrapping_add(f);
    h[6] = h[6].wrapping_add(g);
    h[7] = h[7].wrapping_add(hh);
}

/// The digest of the CONCATENATION of `parts`, without ever materialising
/// that concatenation.
///
/// This is the one hashing body in the crate; `sha256` and
/// `sha256_prefixed_128` are both one line over it, so every vector that
/// pins either of them pins this. It is NOT a streaming API (no state a
/// caller can hold between calls, nothing to misuse, nothing to leave
/// unfinished) -- it is the same whole-byte-string hash the module has
/// always offered, with the string allowed to arrive in more than one
/// piece. FINAL REVIEW item 7: `sha256_prefixed_128` used to copy
/// `prefix ‖ data` into a fresh `Vec`, which for the ON version root over
/// the real graph doubles a multi-hundred-megabyte dump.
///
/// Padding is FIPS 180-4 §5.1.1: a `0x80` byte, then zeros, then the
/// TOTAL message length in BITS as a big-endian u64, landing the total on
/// a 64-byte multiple. When the length word does not fit in the block that
/// holds the `0x80` (total lengths 56..=63 mod 64), the padding spills
/// into one more block -- the two-block NIST vector covers exactly that.
///
/// The part boundaries are invisible to the result: `block` carries at
/// most 63 leftover bytes from one part into the next, so a split falling
/// inside a block is the same digest as no split at all
/// (`a_split_inside_a_block_is_the_same_digest` below).
fn sha256_concat(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = H0;
    // The carry: `fill` bytes of a block not yet complete. Always < 64 at
    // every part boundary -- a full block is compressed the moment it
    // fills, never held.
    let mut block = [0u8; 64];
    let mut fill = 0usize;
    let mut total: u64 = 0;

    for part in parts {
        total = total.wrapping_add(part.len() as u64);
        let mut rest: &[u8] = part;
        if fill > 0 {
            let take = std::cmp::min(64 - fill, rest.len());
            block[fill..fill + take].copy_from_slice(&rest[..take]);
            fill += take;
            rest = &rest[take..];
            if fill == 64 {
                compress(&mut h, &block);
                fill = 0;
            }
        }
        if fill == 0 {
            let mut chunks = rest.chunks_exact(64);
            for chunk in &mut chunks {
                block.copy_from_slice(chunk);
                compress(&mut h, &block);
            }
            let rem = chunks.remainder();
            block[..rem.len()].copy_from_slice(rem);
            fill = rem.len();
        }
    }

    // The tail (0..=63 bytes) plus the padding: one block, or two when
    // the 8-byte length word cannot follow the `0x80` in this one. The
    // bytes after `fill` are stale carry from an earlier block, so they
    // are zeroed before the `0x80` goes in.
    block[fill..].iter_mut().for_each(|b| *b = 0);
    block[fill] = 0x80;
    let bit_len = total.wrapping_mul(8);
    if fill < 56 {
        block[56..].copy_from_slice(&bit_len.to_be_bytes());
        compress(&mut h, &block);
    } else {
        compress(&mut h, &block);
        let mut tail = [0u8; 64];
        tail[56..].copy_from_slice(&bit_len.to_be_bytes());
        compress(&mut h, &tail);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The digest of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    sha256_concat(&[data])
}

/// The 128-bit content address: the first 16 bytes of
/// `sha256(prefix ‖ data)`.
///
/// The prefix is DOMAIN SEPARATION and is hashed BEFORE the bytes, never
/// stored in them -- so the canonical bytes on disk stay exactly what
/// `Canon::encode` produced, while a digest taken over a node can never
/// collide with a digest of the same bytes meaning something else.
/// Truncation to 128 bits is the artifact's id width (spec §3.1): ~2^64
/// birthday bound over a corpus of ~10^6 things.
///
/// The two slices are fed to the block loop as they lie: no `Vec`, no
/// copy of `data` (see `sha256_concat`).
pub fn sha256_prefixed_128(prefix: &[u8], data: &[u8]) -> [u8; 16] {
    let full = sha256_concat(&[prefix, data]);
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[..16]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The compression function itself, one block, against the `"abc"`
    /// vector: `compress` over the padded block from H0 IS the digest.
    #[test]
    fn compression_over_one_padded_block_is_the_abc_digest() {
        let mut block = [0u8; 64];
        block[..3].copy_from_slice(b"abc");
        block[3] = 0x80;
        block[56..].copy_from_slice(&24u64.to_be_bytes()); // 3 bytes = 24 bits
        let mut h = H0;
        compress(&mut h, &block);
        let hexed: String = h.iter().map(|w| format!("{w:08x}")).collect();
        assert_eq!(hexed, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(hexed, hex(&sha256(b"abc")), "the entry point agrees with the core");
    }

    /// FINAL REVIEW item 7: the multi-slice entry point must be blind to
    /// WHERE the message is cut. Every cut of `"abc"`, and cuts placed
    /// deliberately inside a block, across a block boundary, exactly ON a
    /// block boundary, and at the padding-spill lengths (55/56/63/64/65)
    /// -- each must equal the one-slice digest of the same bytes.
    ///
    /// What would make it fail: a carry that forgets the leftover bytes
    /// of a part, a length counted per part instead of in total, or a
    /// stale byte left in the block behind the `0x80`.
    #[test]
    fn a_split_inside_a_block_is_the_same_digest() {
        assert_eq!(sha256_concat(&[b"a", b"bc"]), sha256(b"abc"));
        assert_eq!(sha256_concat(&[b"ab", b"c"]), sha256(b"abc"));
        assert_eq!(sha256_concat(&[b"", b"abc"]), sha256(b"abc"));
        assert_eq!(sha256_concat(&[b"abc", b""]), sha256(b"abc"));
        assert_eq!(sha256_concat(&[]), sha256(b""));

        // A 200-byte message (three full blocks + 8) cut at every single
        // offset: mid-block, on the boundary, and in the padding tail.
        let msg: Vec<u8> = (0..200u32).map(|i| (i % 251) as u8).collect();
        let whole = sha256(&msg);
        for cut in 0..=msg.len() {
            let (a, b) = msg.split_at(cut);
            assert_eq!(sha256_concat(&[a, b]), whole, "two-way cut at {cut}");
        }
        // Three pieces, the middle one straddling a block boundary.
        assert_eq!(sha256_concat(&[&msg[..60], &msg[60..70], &msg[70..]]), whole);
        // Many tiny pieces: the carry path runs on every one of them.
        let ones: Vec<&[u8]> = msg.chunks(1).collect();
        assert_eq!(sha256_concat(&ones), whole);

        // The padding-spill lengths, split one byte before the seam.
        for n in [55usize, 56, 63, 64, 65, 119, 120] {
            let m = vec![b'a'; n];
            let (a, b) = m.split_at(n - 1);
            assert_eq!(sha256_concat(&[a, b]), sha256(&m), "{n} × 'a' split at {}", n - 1);
        }
    }

    /// The prefixed entry point is exactly the concatenated digest,
    /// truncated -- proven WITHOUT the `Vec` it used to build, against a
    /// prefix/data pair whose seam lands inside the first block.
    #[test]
    fn prefixed_128_never_needs_the_concatenation() {
        let prefix = b"bible-atlas/canon/1\n";
        let data = vec![b'z'; 100];
        let mut joined = Vec::new();
        joined.extend_from_slice(prefix);
        joined.extend_from_slice(&data);
        assert_eq!(sha256_prefixed_128(prefix, &data), sha256(&joined)[..16]);
    }

    #[test]
    fn the_schedule_and_constants_are_the_published_ones() {
        assert_eq!(K[0], 0x428a2f98);
        assert_eq!(K[63], 0xc67178f2);
        assert_eq!(H0[0], 0x6a09e667);
        assert_eq!(H0[7], 0x5be0cd19);
    }
}
