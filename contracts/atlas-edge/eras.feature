Feature: eras — the named periods that resolve standings
  Our parse_eras reads four fields per era: id, name, from_year, and
  to_year. Era ids are how vendored data declares WHO STANDS WHEN
  without hardcoded years.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the whole era table, as we consume it
    When I GET /api/eras
    Then the consumed projection eras equals fixture "eras-consumed"
