Feature: the gazetteer — the atlas is the coordinate authority
  Contract C3, in the map system's own words: "atlas Place nodes are the
  coordinate authority; a place moving in the atlas moves every border
  built through it (one fact, one home)." map-generator vendors this
  exact artifact and builds geometry through it, and its law 12(c)
  requires every survey waypoint to resolve to a live atlas place. So
  this is not an internal file. It is a published book with a live
  consumer.

  Pinned row by row — every place, every coordinate — for the same
  reason map-generator pins our whole polity book rather than a sample:
  a silently moved place fails here before it can move a pixel of
  anyone's. A count, or a spot check on a handful of famous places,
  would be satisfiable by its own failure mode.

  It is a large fixture. That is the correct size for the promise, and
  it is the size map-generator already chose for the same class of
  promise (its polities-consumed.json is 132 KB of exactly this).

  This is also where an id DELETION becomes visible. PLACE-1a absorbed
  15 place ids into their survivors; to a consumer still pinned to the
  older book, that is 15 dangling references. Pinned here, the next such
  change arrives as a reviewable diff in this repo rather than as
  someone else's broken build.

  Vocabulary:
    | export | any of: gazetteer, chronology, kretzmann-chronology |
    | projection | any of: catechism-list, contract, edge-page, eras, event, export-format, gazetteer, land-mask, landmarks, narratives, node-card, polities, sources, version-root, vocabulary, xref-list |

  Scenario: the coordinate authority, row by row
    When I read the gazetteer export
    Then the consumed projection gazetteer equals fixture "gazetteer-consumed"
