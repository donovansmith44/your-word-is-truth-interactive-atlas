# AQC v0.1.0 -- TraversalQuery(descriptor, frontierGroup, page?) -> [Focus refs].
# GET /api/node/{id}/edges?kind=&cursor=&limit= --
# server/atlas-server/src/graph_handlers.rs::node_edges.
Feature: TraversalQuery -- expand one frontier abstraction into traversable targets

  Scenario: a real edge kind expands to a page of live targets
    Given a node of kind "TextUnit" with id "text-unit:JHN.3.16"
    When I run TraversalQuery for "text-unit:JHN.3.16" frontier "cites"
    Then the response is a valid "EdgePageOut"
    And the response "kind" field equals "cites"
    And every traversal target resolves to a live node

  Scenario: the bijection witness travels on the wire
    Given a node of kind "Event" with id "Event:ab_ur"
    When I run TraversalQuery for "Event:ab_ur" frontier "located-at"
    Then the response is a valid "EdgePageOut"
    And every entry's "edge" id is present on the matching inverse-kind page of its own target node

  Scenario: pagination pages are windows over the total
    Given a node of kind "TextUnit" with id "text-unit:JHN.3.16"
    When I run TraversalQuery for "text-unit:JHN.3.16" frontier "cites" with limit 1
    Then the response "entries" array has at most 1 entry
    And a further page reached by following "next" never repeats an entry already seen

  Scenario: an unrecognized frontier kind is bad_kind
    Given a node of kind "TextUnit" with id "text-unit:JHN.3.16"
    When I run TraversalQuery for "text-unit:JHN.3.16" frontier "not-a-real-kind"
    Then the request fails with status 400 and code "bad_kind"
