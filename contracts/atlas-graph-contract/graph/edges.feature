Feature: edge families — the declared relations, as a consumer walks them
  One page of one family: the family's own label, the entries, and the
  cursor to the next page. Each entry carries the target node's
  identity AND the edge id itself — the bijection witness, which is the
  same id the target's own inverse-family page carries back for this
  same connection. A consumer can therefore join both directions
  without guessing, and that ability is a promise, so it is pinned.

  `next` is in the projection because pagination is consumed: a
  consumer that walks a family reads it to decide whether to stop.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: one family's page, as a consumer walks it
    When I GET /api/node/Place:hazor-1/edges?kind=site-of
    Then the consumed projection edge-page equals fixture "edges-hazor-1-site-of"
    And every "kind" in the answer names a term the graph declares
