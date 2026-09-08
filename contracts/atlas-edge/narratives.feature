Feature: narratives — the journeys we vendor
  Our parse_narratives reads: per row, id, name, color, and ordered legs.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the whole narrative book, as we consume it
    When I GET /api/narratives
    Then the consumed projection narratives equals fixture "narratives-consumed"
