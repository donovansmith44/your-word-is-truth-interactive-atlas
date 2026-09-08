Feature: node identity — who a node is, whatever carries it
  A node's consumed projection is its id, its kind, its label, the
  provenance the claim rests on, and the edge families it participates
  in with a count for each. That is the whole of what a consumer needs
  to render a node it has never seen before and to know where to walk
  next.

  `provenance` is in the projection deliberately. PROV-1 put it on the
  wire; a projection is how a promise stops being retractable by
  accident, because removing the field now fails here rather than
  quietly reaching a consumer as a missing key.

  What is NOT in the projection is as deliberate: no `version`. The
  atlas version root is a property of the ANSWER, not of the node — it
  says which graph you read, not who this is — so it lives in its own
  `version-root` projection and in its own law. Keeping it out is what
  lets two transports be asked whether they agree about the NODE
  without the question being confounded by what they say about
  themselves.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: a Place node, as a consumer reads it
    When I GET /api/node/Place:hazor-1
    Then the consumed projection node-card equals fixture "node-place-hazor-1"
    And every "kind" in the answer names a term the graph declares

  Scenario: an Event node, as a consumer reads it
    When I GET /api/node/Event:ab_ur
    Then the consumed projection node-card equals fixture "node-event-ab-ur"
    And every "kind" in the answer names a term the graph declares
