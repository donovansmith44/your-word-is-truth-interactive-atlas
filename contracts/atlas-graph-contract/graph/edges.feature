Feature: edge families — the declared relations, as a consumer walks them
  One page of one family: the family's own label, the entries, and the
  cursor to the next page. Each entry carries its neighbour — the far
  end's `position`, then a node's identity or an edge's id — AND the
  edge id itself — the bijection witness, which is the same id the
  target's own inverse-family page carries back for this same
  connection. A consumer can therefore join both directions without
  guessing, and that ability is a promise, so it is pinned.

  `next` is in the projection because pagination is consumed: a
  consumer that walks a family reads it to decide whether to stop.

  Vocabulary:
    | projection | any of: books, catechism-item, catechism-list, chapter, contract, edge-page, eras, event, export-format, gazetteer, kretzmann-chapter, land-mask, landmarks, narrative-event, narratives, node-card, place, polities, sources, verse, version-root, vocabulary, xref-list |

  Scenario: one family's page, as a consumer walks it
    When I GET /api/node/Place:hazor-1/edges?kind=site-of
    Then the consumed projection edge-page equals fixture "edges-hazor-1-site-of"
    And every "kind" of a node, an edge page, an edge summary or an anchor in the answer names a term the graph declares
    And every field the schema publishes as an enum carries one of its values
