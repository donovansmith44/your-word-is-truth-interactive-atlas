# AQC v0.1.0 -- TextWindowQuery(ref, radius) -> verses, including per-verse
# annotation spans. GET /api/text?ref=&n=&dir=&scope=&corpus= --
# server/atlas-server/src/graph_handlers.rs::text_window.
#
# RED-1's alignment law (spec §2, the annotation-spans law this feature
# exists to pin): words_of_christ is the FIRST annotation layer -- the
# general shape every future per-verse annotation layer follows. Every span
# must lie strictly within the length of the SAME verse's own text; a span
# reaching into a neighboring verse, or past its own verse's end, is a
# contract violation, not merely a display bug.
Feature: TextWindowQuery -- a window of verses with annotation spans

  Scenario: a single-verse window carries the real KJV text
    When I run TextWindowQuery for "JHN.3.16" radius 1
    Then the response is a valid "TextWindowOut"
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
