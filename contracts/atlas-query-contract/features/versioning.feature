# /api/contract advertises the range of contract versions this server answers
# for. Agreement between the client and the server is proven in CI, where both
# ship together, so no consumer checks the range at startup.
# GET /api/contract -- server/atlas-contract/src/meta.rs::contract.
Feature: Versioning -- the server advertises its AQC range

  Scenario: the server advertises the supported AQC version range
    When I query "/api/contract"
    Then the response is a valid "Contract"
    And the server advertises AQC version "0.9.0" through "0.9.0"
