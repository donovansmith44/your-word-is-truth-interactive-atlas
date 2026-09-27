# AQC v0.1.0 -- SceneQuery(window) -> scene. GET /api/scene?from=&to= and
# GET /api/scene/scripture?ref= -- server/atlas-server/src/handlers.rs::
# {scene_time,scene_scripture}. Both variants return the SAME Scene shape.
#
# Each variant's own URL is spelled out in the scenario that pins it, not only
# here: a route named nowhere but a comment is a route with no promise, and
# atlas-contract/tests/contract_coverage.rs is the law that says so.
Feature: SceneQuery -- the map composition query

  Scenario: a time-window scene is a valid Scene
    When I run SceneQuery for the time window "-2100"-"-2000"
    And I query "/api/scene?from=-2100&to=-2000"
    Then the response is a valid "Scene"
    And the response "mode" field equals "time"

  Scenario: a scripture-ref scene is a valid Scene
    When I run SceneQuery for scripture ref "JHN.3.16"
    And I query "/api/scene/scripture?ref=JHN.3.16"
    Then the response is a valid "Scene"
    And the response "mode" field equals "scripture"
    And "quiet_places" is empty

  Scenario: an inverted time window is bad_window
    When I run SceneQuery for the time window "100"-"-100"
    Then the request fails with status 400 and code "bad_window"

  Scenario: a structurally malformed scripture ref is bad_ref
    When I run SceneQuery for scripture ref "not-a-ref-at-all"
    Then the request fails with status 400 and code "bad_ref"
