// Each test binary compiles this module on its own and reads only the counts it pins, so the
// lint, which sees one binary at a time, would call the others unused.
#![allow(dead_code)]
pub const DECLARED_NODE_KINDS: usize = 16;
pub const DECLARED_DIRECTED_RELATIONS: usize = 22;
pub const DECLARED_SYMMETRIC_RELATIONS: usize = 6;
pub const DECLARED_EDGE_KINDS: usize = 2 * DECLARED_DIRECTED_RELATIONS + DECLARED_SYMMETRIC_RELATIONS;
