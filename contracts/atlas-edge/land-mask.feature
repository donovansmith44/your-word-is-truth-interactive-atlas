Feature: the land mask — the coastline our partition builds on
  Our parse_land_mask reads the rings, whole.

  Vocabulary:
    | projection | any of: books, catechism-item, catechism-list, chapter, contract, edge-page, eras, event, export-format, gazetteer, kretzmann-chapter, land-mask, landmarks, narrative-event, narratives, node-card, place, polities, sources, verse, version-root, vocabulary, xref-list |

  Scenario: the whole mask, as we consume it
    When I GET /api/land-mask
    Then the consumed projection land-mask equals fixture "land-mask-consumed"
