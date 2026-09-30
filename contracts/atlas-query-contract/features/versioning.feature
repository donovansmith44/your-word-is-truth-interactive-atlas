# GET /api/contract -- server/atlas-contract/src/meta.rs::contract: the schema versions of the data behind every answer.
Feature: Versioning -- the server declares the schema versions of its data

  Scenario: the server declares the schema versions of its data
    When I query "/api/contract"
    Then the response is a valid "Contract"
