Feature: events — a leg's when, where, and why
  Our parse_event reads: id; label (from title, falling back to
  label); optional when (from_year, to_year); places (each
  contributing only its id); and verses — not a top-level field, but
  gathered from witnesses[].verse_groups[].verses.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: a known leg event, as we consume it
    When I GET /api/event/ab_haran
    Then the consumed projection event equals fixture "event-ab-haran-consumed"
