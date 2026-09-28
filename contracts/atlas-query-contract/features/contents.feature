# ContentsQuery(corpus) -> the containment forest, two levels deep.
# GET /api/contents/{corpus} -- server/atlas-contract/src/contents.rs::contents.
#
# The tree IS the graph's own containment forest (Container nodes and contains
# edges), read through the port -- nothing here is a hand-maintained list -- and it
# stops at the chapter of a book and the article of a Concord document, which is
# the level a reader navigates by.
Feature: ContentsQuery -- the containment forest as a table of contents

  Scenario: the Bible's contents are books then chapters
    When I query "/api/contents/bible"
    Then the response is a valid "Contents"
    And the response "corpus" field equals "bible"

  Scenario: the Concord's contents are documents then articles
    When I query "/api/contents/concord"
    Then the response is a valid "Contents"
    And the response "corpus" field equals "concord"

  Scenario: an unknown corpus is not found
    When I query "/api/contents/nope"
    Then the request fails with status 404 and code "not_found"
