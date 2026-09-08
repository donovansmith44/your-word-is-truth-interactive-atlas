Feature: what the atlas expects of the map API
  Owner order 1, the second half: "thye consume our rgaph, we consume
  their maps." This file is the atlas speaking as a CONSUMER of
  map-generator's map API, in the atlas's own repo, for map-generator to
  run against its own server.

  It is deliberately short. We do not yet consume a single byte of the
  map API — MAPS-1's coupling question (Q1) is unruled and no integration
  code exists on either side. Writing a broad suite now would be
  inventing expectations of an integration nobody has designed, which is
  worse than writing none: it would look like a commitment and would be
  re-blessed into meaninglessness the first time it collided with a real
  decision.

  So this states the two things that are true regardless of which
  coupling shape the owner picks, and that map-generator's own contract
  set already commits to. Both are cross-repo referential laws — neither
  is satisfiable by re-blessing a fixture, and both are laws we would
  otherwise only be able to state as prose.

  HOW TO RUN IT (from map-generator, whose server this is):

    contract-runner run --base-url http://127.0.0.1:PORT \
      --exports ../bible-atlas/data/exports \
      contracts/map-api-consumer

  It is NOT run by the atlas's own gate — we are not this API's provider
  and its server is not ours to start. The atlas holds it to totality,
  vocabulary and semver so that what we hand over is at least total,
  self-describing and versioned.

  Vocabulary:
    | export | any of: gazetteer, chronology, kretzmann-chronology |

  Scenario: the map server compiled against the atlas we are actually running
    When I GET /api/contract as map
    And I read the gazetteer export as gazetteer
    Then map and gazetteer declare the same atlas version root

  Scenario: every place the map draws is a place the gazetteer carries
    When I GET /api/scene as scene
    And I read the gazetteer export as gazetteer
    Then every place scene draws is a place gazetteer carries
