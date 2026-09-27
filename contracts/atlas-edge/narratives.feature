Feature: narratives — the journeys we vendor
  Our parse_narratives reads: per row, id, name, color, and ordered legs.

  Vocabulary:
    | projection | any of: books, catechism-item, catechism-list, chapter, contract, edge-page, eras, event, export-format, gazetteer, kretzmann-chapter, land-mask, landmarks, narrative-event, narratives, node-card, place, polities, sources, verse, version-root, vocabulary, xref-list |

  Scenario: the whole narrative book, as we consume it
    When I GET /api/narratives
    Then the consumed projection narratives equals fixture "narratives-consumed"
