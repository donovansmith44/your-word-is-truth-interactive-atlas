Feature: HTTP — the transport the Blazor client consumes
  The client is a consumer of the server, and that seam is as real as
  the map one. These are the surfaces it reads that no other suite
  already owns.

  What is NOT here, deliberately, because it is owned elsewhere and a
  second weaker copy is worse than none:

    * the query LANGUAGE and its error taxonomy — SceneQuery's
      `bad_window`/`bad_ref`, TraversalQuery's `bad_kind`, focus
      round-trip identity, pagination never repeating an entry — all
      belong to contracts/atlas-query-contract, which states them as
      algebraic laws and validates against aqc.schema.json.
    * the six cartographic endpoints map-generator consumes — those are
      ITS expectations of US and live, unmodified, in contracts/
      atlas-edge.

  Vocabulary:
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the AQC version range the server advertises
    When I GET /api/contract
    Then the consumed projection contract equals fixture "contract"

  Scenario: the source registry every provenance string resolves against
    When I GET /api/sources
    Then the consumed projection sources equals fixture "sources"

  Scenario: cross-references out of one verse, as the client renders them
    When I GET /api/xrefs/JHN.3.16
    Then the consumed projection xref-list equals fixture "xrefs-jhn-3-16"

  # MAT.28.19, not JHN.3.16. The first draft used JHN.3.16 and blessed a
  # fixture of `[]` — a law satisfiable by its own failure mode, since an
  # endpoint that had stopped resolving catechism items entirely would
  # still have passed it. MAT.28.19 is the most-cited verse in the
  # compiled catechism (13 citations), so the fixture has something to be
  # wrong about.
  Scenario: catechism items citing one verse, as the client renders them
    When I GET /api/catechism/MAT.28.19
    Then the consumed projection catechism-list equals fixture "catechism-mat-28-19"
