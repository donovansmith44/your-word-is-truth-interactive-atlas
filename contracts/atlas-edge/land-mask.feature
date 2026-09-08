Feature: the land mask — the coastline our partition builds on
  Our parse_land_mask reads the rings, whole.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the whole mask, as we consume it
    When I GET /api/land-mask
    Then the consumed projection land-mask equals fixture "land-mask-consumed"
