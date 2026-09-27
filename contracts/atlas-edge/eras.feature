Feature: eras — the named periods that resolve standings
  Our parse_eras reads four fields per era: id, name, from_year, and
  to_year. Era ids are how vendored data declares WHO STANDS WHEN
  without hardcoded years.

  Vocabulary:
    | projection | any of: books, catechism-item, catechism-list, chapter, contract, edge-page, eras, event, export-format, gazetteer, kretzmann-chapter, land-mask, landmarks, narrative-event, narratives, node-card, place, polities, sources, verse, version-root, vocabulary, xref-list |

  Scenario: the whole era table, as we consume it
    When I GET /api/eras
    Then the consumed projection eras equals fixture "eras-consumed"
