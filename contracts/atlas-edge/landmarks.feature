Feature: landmarks — named waters and places we label by
  Our parse_landmarks reads four fields per row: name, kind, lat, and
  lon. The position is consumed, not just the naming — a landmark that
  moves moves our label with it.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the whole landmark list, as we consume it
    When I GET /api/landmarks
    Then the consumed projection landmarks equals fixture "landmarks-consumed"
