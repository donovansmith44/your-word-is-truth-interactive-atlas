# EventPage(id) -> one event, with each place it names as a node of the graph.
# GET /api/event/{id} -- server/atlas-contract/src/events.rs::event.
Feature: EventPage -- one event and the places it names

  Scenario: every place an event names carries its node
    When I query "/api/event/ab_ur"
    Then the response is a valid "EventPage"
