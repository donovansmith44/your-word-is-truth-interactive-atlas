# FocusQuery(descriptor) -> one node's record.
# GET /api/node/{id} -- server/atlas-contract/src/graph.rs::node_record.
#
# The Examples: table below is GENERATED, not hand-authored -- see
# server/atlas-contract/src/bins/export_aqc_examples.rs. It draws one seed id per
# NODE KIND the real committed graph materializes, verified live against that
# graph at export time, so a stale seed id fails the exporter loud rather than
# silently. Re-running the exporter against an unchanged graph reproduces this
# table byte-identical; nothing in it is wall-clock random.
Feature: FocusQuery -- one node's record, by descriptor

  Scenario Outline: every sampled node kind resolves to a valid Focus record
    Given a node of kind "<kind>" with id "<id>"
    When I run FocusQuery for "<id>"
    Then the response is a valid "NodeRecord"
    And the response "id" field equals "<id>"
    And the response "kind" field equals "<kind>"
    And every adjacency group is a relations! family

    Examples:
      | kind           | id                             |
      | TextUnit       | text-unit:JHN.3.16             |
      | Event          | Event:ab_ur                    |
      | Narrative      | Narrative:abraham-migration    |
      | Anchor         | Anchor:solomon-crowned         |
      | Place          | Place:ur-1                     |
      | Era            | Era:primeval                   |
      | Polity         | Polity:egypt                   |
      | Person         | Person:aaron_1                 |
      | Translation    | Translation:latin_vulgate      |
      | CommentaryItem | CommentaryItem:kretzmann/0.1.0 |
      | CatechismItem  | CatechismItem:commandment-1    |

  Scenario: an id that parses but names no real node is not_found
    Given a node of kind "Person" with id "Person:nonexistent-xyz"
    When I run FocusQuery for "Person:nonexistent-xyz"
    Then the request fails with status 404 and code "not_found"

  Scenario: a malformed id is bad_ref
    When I run FocusQuery for "not-even-a-colon-pair"
    Then the request fails with status 400 and code "bad_ref"
