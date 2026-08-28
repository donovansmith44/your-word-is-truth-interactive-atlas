//! Bare `atlas` and `atlas help` -- CONTRACT.md's own "atlas (bare, no
//! arguments)"/"atlas help" sections: identical short-help output, exit 0.

pub fn text() -> String {
    let mut out = String::new();
    out.push_str("atlas -- a really simple CLI to query the Bible Explorer graph\n\n");
    out.push_str("commands:\n");
    out.push_str("  verse <ref>                     text + red-letter marks + attached places/persons/events\n");
    out.push_str("  chapter <ref>                    every verse in a KJV chapter\n");
    out.push_str("  node <id>                        a node's card + edge summary\n");
    out.push_str("  edges <id> --kind K [opts]       one frontier page at a node\n");
    out.push_str("  find <term>                      name lookup across Place/Event/Narrative/Era/Polity\n");
    out.push_str("  tutorial                         a guided, numbered walkthrough (real queries, real output)\n");
    out.push_str("  help                             this text\n\n");
    out.push_str("global flag: --data-dir <path>    where graph.bin lives (default: ../data/compiled)\n\n");
    out.push_str("new here? run 'atlas tutorial' for a guided walkthrough.\n");
    out
}
