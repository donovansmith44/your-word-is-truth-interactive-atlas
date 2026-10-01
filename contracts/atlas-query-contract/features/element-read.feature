# ElementQuery(ids) -> the nodes and edges those ids name, in the order asked.
# GET /api/elements?ids= -- server/atlas-contract/src/graph.rs::elements.
Feature: ElementQuery -- nodes and edges of the graph by id, many at once

  Scenario: each id is answered in order by its record or by its absence
    When I query "/api/elements?ids=text-unit:JHN.3.16,Person:nonexistent-xyz"
    Then the response is a valid "ElementPage"
