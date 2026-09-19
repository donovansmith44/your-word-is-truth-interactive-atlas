# AQC v0.6.0 -- ContentsQuery(corpus) -> the containment forest, two levels
# deep. GET /api/contents/{corpus} -- server/atlas-server/src/contents.rs.
#
# D4 (owner, 2026-09-15, verbatim: "Table of contents = a tree ... Stop at the
# level of ARTICLE (BoC) or TOPIC (Small Catechism). Pages are not a
# meaningful way of thinking about things."): the tree IS the graph's own
# containment forest (Container nodes + contains edges), read through the
# port; nothing here is a hand-maintained list.
Feature: ContentsQuery -- the containment forest as a table of contents

  Scenario: the Bible's contents are books then chapters
    When I query "/api/contents/bible"
    Then the response is a valid "ContentsOut"
    And the response "corpus" field equals "bible"

  Scenario: the Concord's contents are documents then articles
    When I query "/api/contents/concord"
    Then the response is a valid "ContentsOut"
    And the response "corpus" field equals "concord"

  Scenario: an unknown corpus is not found
    When I query "/api/contents/nope"
    Then the request fails with status 404 and code "not_found"
