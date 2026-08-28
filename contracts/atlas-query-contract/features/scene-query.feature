# AQC v0.1.0 -- SceneQuery(window) -> scene. GET /api/scene?from=&to= and
# GET /api/scene/scripture?ref= -- server/atlas-server/src/handlers.rs::
# {scene_time,scene_scripture}. Both variants return the SAME Scene shape.
Feature: SceneQuery -- the map composition query

  Scenario: a time-window scene is a valid Scene
    When I run SceneQuery for the time window "-2100"-"-2000"
    Then the response is a valid "Scene"
    And the response "mode" field equals "time"

  Scenario: a scripture-ref scene is a valid Scene
    When I run SceneQuery for scripture ref "JHN.3.16"
    Then the response is a valid "Scene"
    And the response "mode" field equals "scripture"
    And "quiet_places" is empty

  Scenario: an inverted time window is bad_window
    When I run SceneQuery for the time window "100"-"-100"
    Then the request fails with status 400 and code "bad_window"

  Scenario: a structurally malformed scripture ref is bad_ref
    When I run SceneQuery for scripture ref "not-a-ref-at-all"
    Then the request fails with status 400 and code "bad_ref"
