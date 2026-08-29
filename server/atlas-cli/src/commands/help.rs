//! Bare `atlas` and `bibex help` -- CONTRACT.md's own "bibex (bare, no
//! arguments)"/"bibex help" sections: identical short-help output, exit 0.

pub fn text() -> String {
    let mut out = String::new();
    out.push_str("bibex -- a really simple CLI to query the Bible Explorer graph\n\n");
    out.push_str("commands:\n");
    out.push_str("  verse <ref>                     text + red-letter marks + attached places/persons/events/passages\n");
    out.push_str("  chapter <ref>                    every verse in a KJV chapter\n");
    out.push_str("  node <id>                        a node's card + edge summary\n");
    out.push_str("  edges <id> --kind K [opts]       one frontier page at a node\n");
    out.push_str("  find <term>                      name lookup across Place/Event/Narrative/Era/Polity/Person/CatechismItem\n");
    out.push_str("  kinds                            the full edge-kind vocabulary (--kind tokens) for 'edges'\n");
    out.push_str("  tutorial                         a guided, numbered walkthrough (real queries, real output)\n");
    out.push_str("  help                             this text\n\n");
    out.push_str("global flags:\n");
    out.push_str("  --data-dir <path>                where graph.bin lives (default: ../data/compiled)\n");
    out.push_str("  --json                           machine-readable JSON on stdout instead of prose (see CONTRACT.md's\n");
    out.push_str("                                    own --json mode section); a failure is a JSON object on stderr,\n");
    out.push_str("                                    same exit codes; not available for tutorial/help/a bare invocation\n\n");
    out.push_str("new here? run 'bibex tutorial' for a guided walkthrough.\n");
    out
}
