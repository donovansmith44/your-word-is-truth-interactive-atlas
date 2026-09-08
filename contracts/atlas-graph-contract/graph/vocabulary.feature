Feature: the declared vocabulary — the root of every other promise
  The graph declares its node kinds in graph-types' `kind_tags!` manifest
  (id.rs) and its edge families in its `relations!` manifest (edge.rs),
  and those two macros are the only place either list exists. Every
  `kind` a consumer ever reads — on a node card, on an edge-page entry,
  in an edge summary, in a CLI dump — is drawn from here.

  This is why it is the FIRST feature and not a footnote: a consumer
  binds to these names. Adding a family is additive and a consumer that
  never asks for it keeps passing; renaming or removing one is a break
  no amount of endpoint stability can soften, because the name IS the
  interface. Pinning the whole manifest means such a change cannot
  happen quietly — it has to arrive as a diff to this fixture, which is
  exactly what the semver gate classifies.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: every node kind and edge family the graph declares
    When I read the graph's declared vocabulary
    Then the consumed projection vocabulary equals fixture "graph-vocabulary"
