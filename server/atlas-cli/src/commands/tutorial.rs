//! `bibex tutorial` -- CONTRACT.md's own "Tutorial contract": seven
//! numbered steps, each running the REAL command-implementation function
//! against the REAL loaded graph (never a canned transcript) -- a change
//! to a command's own output shape changes this tutorial's own output on
//! the next run, automatically, since step 2-6 call `commands::{verse,
//! chapter, node, edges, find}::run` directly, not a copy of their logic.
//!
//! Every ref/id/kind step 2-6 queries is a real, checked-present locus in
//! the committed `data/compiled/graph.bin` (verified by this crate's own
//! `tutorial_smoke_test` in `tests/cli.rs` -- R6).

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;

use crate::error::CliError;

const TOTAL_STEPS: usize = 7;

fn step_header(n: usize, title: &str) -> String {
    format!("Step {n} of {TOTAL_STEPS}: {title}\n")
}

pub fn run(graph: &GraphService, data: &AtlasData, data_dir_shown: &str) -> Result<String, CliError> {
    let mut out = String::new();

    out.push_str(&step_header(1, "how this CLI is organized"));
    out.push_str(&format!(
        "This CLI loads the compiled graph once at startup, from {data_dir_shown} \
(override with --data-dir). Every command below prints its real answer to \
stdout and exits 0 on success. A command that fails prints one line to \
stderr shaped 'atlas: error (<class>): <what> -- <why> -- <what to do>' \
and exits a nonzero code fixed per error class (see CONTRACT.md's own \
error taxonomy) -- scripts can branch on the exit code alone. Add --json \
before or after any real query command (verse/chapter/node/edges/find/ \
kinds) for a single machine-readable JSON value on stdout instead -- a \
failure becomes one JSON object on stderr, same exit codes; --json is not \
available for this tutorial itself, or for bare/help (see CONTRACT.md's \
own --json mode section).\n\n"
    ));

    out.push_str(&step_header(2, "bibex verse <ref> -- one verse, its text, and what's attached to it"));
    out.push_str("$ bibex verse GEN.1.1\n");
    out.push_str(&super::verse::run(graph, data, "GEN.1.1")?);
    out.push_str(
        "\nThe line above is the verse's own reference and text (red-letter \
words, when this verse has any, are wrapped in [brackets]), followed by \
every Place/Person/Event this graph attaches to that exact verse -- \
'(none)' when a section is honestly empty, never blank.\n\n",
    );

    out.push_str(&step_header(3, "bibex chapter <ref> -- a whole chapter, one line per verse"));
    out.push_str("$ bibex chapter GEN.1\n");
    out.push_str(&super::chapter::run(graph, "GEN.1")?);
    out.push_str("\nSame per-verse text and red-letter marking as 'bibex verse', for every verse the chapter has, in order.\n\n");

    out.push_str(&step_header(4, "bibex node <id> -- any node's card + edge summary"));
    out.push_str("$ bibex node Event:ab_ur\n");
    out.push_str(&super::node::run(graph, "Event:ab_ur")?);
    out.push_str(
        "\n'id'/'kind'/'label'/'provenance' identify the node; 'edges' lists \
every edge KIND this node carries and how many entries each has -- the \
counts you pass to 'bibex edges' next.\n\n",
    );

    out.push_str(&step_header(5, "bibex edges <id> --kind K -- walking one frontier"));
    out.push_str("$ bibex edges Event:ab_ur --kind located-at\n");
    out.push_str(&super::edges::run(graph, super::edges::EdgesArgs { id_raw: "Event:ab_ur", kind_raw: Some("located-at"), limit: None, cursor: None })?);
    out.push_str(
        "\nEach row is one connected node reached by that edge kind. A page \
that runs past --limit entries ends with 'more: continue with --cursor N' \
instead of 'end of list' -- pass that N back in to keep walking. A node's \
own edge summary above only lists the kinds THAT node happens to carry --  \
run 'bibex kinds' any time for the full --kind vocabulary this graph \
supports, no node needed first.\n\n",
    );

    out.push_str(&step_header(6, "bibex find <term> -- name lookup across kinds"));
    out.push_str("$ bibex find jericho\n");
    out.push_str(&super::find::run(graph, data, "jericho")?);
    out.push_str("\nA case-insensitive substring match over Place/Event/Narrative/Era/Polity/Person/CatechismItem labels -- useful for finding the exact id 'bibex node'/'bibex edges' need.\n\n");

    out.push_str(&step_header(7, "the full command list"));
    out.push_str(&super::help::text());
    out.push_str("\nThat's the whole surface. Run any of the above with a real ref/id of your own -- every example above was a real, live query, not a canned transcript.\n");

    Ok(out)
}
