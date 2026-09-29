use atlas_graph_types::raw_manifest::{leaf_line, node_hash, node_line, RawEntry, RawLeaf, RawNode, RAW_DOMAIN_PREFIX};
use atlas_graph_types::sha256::{sha256, sha256_prefixed_128};

fn hex64(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex128(bytes: [u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn leaf_line_is_name_pipe_full_sha256_newline_excluding_bytes() {
    // Arrange
    let sha = hex64(sha256(b"contents of ancient.jsonl"));
    let small = RawLeaf { name: "ancient.jsonl".into(), sha256: sha.clone(), bytes: 9180734 };
    let large = RawLeaf { name: "ancient.jsonl".into(), sha256: sha.clone(), bytes: u64::MAX };

    // Act
    let line_small = leaf_line(&small);
    let line_large = leaf_line(&large);

    // Assert
    assert_eq!(line_small, format!("ancient.jsonl|{sha}\n"));
    assert_eq!(
        line_large, line_small,
        "D3: bytes is excluded from the line, so it cannot move the leaf's identity"
    );
}

#[test]
fn a_node_over_two_leaves_is_the_domain_prefixed_hash_of_their_concatenated_lines() {
    // Arrange
    let hash_a = hex64(sha256(b"a-contents"));
    let hash_b = hex64(sha256(b"b-contents"));
    let a = RawLeaf { name: "a".into(), sha256: hash_a.clone(), bytes: 3 };
    let b = RawLeaf { name: "b".into(), sha256: hash_b.clone(), bytes: 3 };
    let concatenated_lines = format!("a|{hash_a}\nb|{hash_b}\n");
    let expected = hex128(sha256_prefixed_128(RAW_DOMAIN_PREFIX, concatenated_lines.as_bytes()));

    // Act
    let hash = node_hash(&[RawEntry::Leaf(a), RawEntry::Leaf(b)]);

    // Assert
    assert_eq!(hash, expected);
}

#[test]
fn node_hash_orders_children_by_name_not_by_insertion_order() {
    // Arrange
    let a = RawLeaf { name: "a".into(), sha256: hex64(sha256(b"a")), bytes: 1 };
    let b = RawLeaf { name: "b".into(), sha256: hex64(sha256(b"b")), bytes: 1 };
    let inserted_b_then_a = [RawEntry::Leaf(b.clone()), RawEntry::Leaf(a.clone())];
    let inserted_a_then_b = [RawEntry::Leaf(a), RawEntry::Leaf(b)];

    // Act
    let hash_of_b_then_a = node_hash(&inserted_b_then_a);
    let hash_of_a_then_b = node_hash(&inserted_a_then_b);

    // Assert
    assert_eq!(hash_of_b_then_a, hash_of_a_then_b);
}

#[test]
fn a_deep_change_moves_its_ancestors_line_and_hash_but_leaves_its_sibling_untouched() {
    // Arrange
    let x_v1 = RawLeaf { name: "x".into(), sha256: hex64(sha256(b"x-v1")), bytes: 4 };
    let x_v2 = RawLeaf { name: "x".into(), sha256: hex64(sha256(b"x-v2")), bytes: 4 };
    let y = RawLeaf { name: "y".into(), sha256: hex64(sha256(b"y")), bytes: 1 };

    let dir_a_hash_before = node_hash(&[RawEntry::Leaf(x_v1.clone())]);
    let dir_a_before = RawNode { name: "dirA".into(), hash: dir_a_hash_before.clone(), children: vec![RawEntry::Leaf(x_v1)] };
    let dir_a_hash_after = node_hash(&[RawEntry::Leaf(x_v2.clone())]);
    let dir_a_after = RawNode { name: "dirA".into(), hash: dir_a_hash_after.clone(), children: vec![RawEntry::Leaf(x_v2)] };
    let dir_b_hash = node_hash(&[RawEntry::Leaf(y.clone())]);
    let dir_b = RawNode { name: "dirB".into(), hash: dir_b_hash.clone(), children: vec![RawEntry::Leaf(y.clone())] };

    // Act
    let line_before = node_line(&dir_a_before);
    let root_before = node_hash(&[RawEntry::Node(dir_a_before), RawEntry::Node(dir_b.clone())]);
    let root_after = node_hash(&[RawEntry::Node(dir_a_after), RawEntry::Node(dir_b.clone())]);
    let dir_b_hash_recomputed = node_hash(&[RawEntry::Leaf(y)]);

    // Assert
    assert_eq!(line_before, format!("dirA|{dir_a_hash_before}\n"));
    assert_ne!(dir_a_hash_after, dir_a_hash_before, "the leaf's own change must move its directory's hash");
    assert_ne!(root_after, root_before, "the leaf's change must move the root, its ancestor");
    assert_eq!(
        dir_b_hash_recomputed, dir_b_hash,
        "dirB, x's sibling subtree, is untouched by x's change -- its hash depends only on its own children"
    );
}

#[test]
fn the_empty_node_has_a_defined_hash_equal_to_the_domain_prefix_alone() {
    // Arrange
    let expected = hex128(sha256_prefixed_128(RAW_DOMAIN_PREFIX, b""));

    // Act
    let hash = node_hash(&[]);

    // Assert
    assert_eq!(hash, expected, "an empty fetched directory is a real state, not a panic");
}
