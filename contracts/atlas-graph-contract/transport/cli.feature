Feature: bibex — a second transport over the same graph
  This is the feature that makes the addendum's ruling enforceable
  rather than aspirational. bibex and the HTTP API are two carriers of
  one graph. If the graph is the primary juncture, then the two must
  project to the SAME value — not to two similar values that a human
  compares by eye, and not to two fixtures that can be re-blessed apart
  one at a time.

  So these scenarios pin nothing at all. They compare the two
  transports to each other through the graph's own projection, which
  means there is no fixture to re-bless and no way to make them pass
  except by making the two carriers actually agree. A third transport
  added tomorrow inherits the same law by adding one `When`.

  bibex's contract surface is `--json` (its own Cargo.toml: "BIBEX-1
  (--json flag, contract-first)"). Its human-readable stdout is a
  contract too and is covered where prose belongs — the transcript
  assertions in server/atlas-cli/tests/cli.rs — not here.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the CLI and the wire agree about a node's identity
    When I GET /api/node/Place:hazor-1 as wire
    And I run bibex node Place:hazor-1 as cli
    Then wire and cli agree on the consumed projection node-card

  Scenario: the CLI and the wire agree about one edge family's page
    When I GET /api/node/Place:hazor-1/edges?kind=site-of as wire
    And I run bibex edges Place:hazor-1 --kind site-of as cli
    Then wire and cli agree on the consumed projection edge-page

  # RED ON PURPOSE, and disclosed rather than silenced (CDC-1 finding).
  # `GET /api/node/{id}` carries `version` -- the atlas version root the
  # answer was computed at (graph_handlers.rs::NodeCardOut.version) --
  # and `bibex --json node` carries no such field. A consumer reading the
  # graph over the CLI therefore cannot tell WHICH graph it read, so it
  # cannot implement the C6 stale-pin that the HTTP consumer can. The fix
  # is one field on the CLI's JSON node payload, in atlas-cli, which this
  # batch does not own. @target keeps it printed red in every single run
  # of the gate instead of hiding it behind a passing suite.
  @target
  Scenario: the CLI declares which graph it read
    When I GET /api/node/Place:hazor-1 as wire
    And I run bibex node Place:hazor-1 as cli
    Then wire and cli declare the same atlas version root
