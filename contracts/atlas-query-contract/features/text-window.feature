# TextWindowQuery(ref, n, dir, scope, corpus) -> units of text, including the
# per-verse annotation spans. GET /api/text --
# server/atlas-contract/src/graph.rs::text_window. The route's own URL is spelled
# out in the scenario that pins it, not only here: a route named nowhere but a
# comment is a route with no promise, and
# server/atlas-contract/tests/contract_coverage.rs is the law that says so.
#
# The annotation-spans law this feature exists to pin: words_of_christ is the
# FIRST annotation layer -- the general shape every future per-verse annotation
# layer follows. Every span must lie strictly within the length of the SAME
# verse's own text; a span reaching into a neighboring verse, or past its own
# verse's end, is a contract violation, not merely a display bug.
Feature: TextWindowQuery -- a window of verses with annotation spans

  Scenario: a single-verse window carries the real KJV text
    When I run TextWindowQuery for "JHN.3.16" radius 1
    And I query "/api/text?ref=JHN.3.16&n=1"
    Then the response is a valid "TextWindow"
    And the response has exactly 1 unit
    And unit 1's "ref" field equals "JHN.3.16"

  Scenario: a multi-verse window walks onward in ref order
    When I run TextWindowQuery for "JHN.3.16" radius 3
    Then the response has exactly 3 units
    And the units' "ref" fields are "JHN.3.16", "JHN.3.17", "JHN.3.18" in order

  Scenario Outline: every words_of_christ span lies within its own verse's text length
    When I run TextWindowQuery for "<ref>" radius 1
    Then every "words_of_christ" span lies within its own verse's text length

    Examples:
      | ref       |
      | MAT.4.19  |
      | MAT.5.4   |
      | JHN.3.16  |

  Scenario: a chapter-scoped window rejects dir=backward
    When I run a chapter-scoped TextWindowQuery for "JHN.3" with dir "backward"
    Then the request fails with status 400 and code "bad_dir"

  Scenario: an unknown corpus is bad_corpus
    When I run TextWindowQuery for "JHN.3.16" radius 1 with corpus "not-a-real-corpus"
    Then the request fails with status 400 and code "bad_corpus"

  Scenario: an unknown scope is bad_scope
    When I run TextWindowQuery for "JHN.3.16" radius 1 with scope "not-a-real-scope"
    Then the request fails with status 400 and code "bad_scope"

  Scenario: a chapter is a Scripture reading, so a chapter scope over the Concord is bad_scope
    When I query "/api/text?ref=BoC%207.2.1&corpus=concord&scope=chapter"
    Then the request fails with status 400 and code "bad_scope"
