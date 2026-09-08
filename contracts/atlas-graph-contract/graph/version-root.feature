Feature: one graph, one root — contract C6 as an executable law
  Every artifact the atlas publishes records the version root of the
  graph it was compiled against. map-generator's C6 stale-pin compares
  exactly this value, and its `MapError::StaleAgainstAtlas { pinned,
  current }` is that comparison as a type.

  The law is stated between TWO artifacts rather than against a literal
  hash on purpose. A pinned hash would have to be re-blessed on every
  recompile, and a check that is re-blessed as a matter of routine stops
  being read — this project has the scar. Stated as an agreement, it
  keeps holding across every compile and only ever fires when two
  artifacts genuinely disagree, which is the only case anyone cares
  about.

  Vocabulary:
    | export | any of: gazetteer, chronology, kretzmann-chronology |
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the three published exports were compiled against one graph
    When I read the gazetteer export as gazetteer
    And I read the chronology export as chronology
    And I read the kretzmann-chronology export as kretzmann
    Then gazetteer and chronology declare the same atlas version root
    And gazetteer and kretzmann declare the same atlas version root

  Scenario: the gazetteer declares which format it is in
    When I read the gazetteer export
    Then the consumed projection export-format equals fixture "gazetteer-format"

  Scenario: the chronology declares which format it is in
    When I read the chronology export
    Then the consumed projection export-format equals fixture "chronology-format"
