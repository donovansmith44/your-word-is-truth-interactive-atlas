//! What `bibex` exposes as a library: the parts its integration tests reach without spawning
//! the binary. A binary crate's modules are private to it, so the raw-tree walker lives here.

pub mod raw;
