# AQC v0.1.0 -- /api/contract advertisement + fail-loud mismatch (spec §2's
# versioning law, the house fail-loud law). This is the ONE new behavioral
# surface this batch adds -- server/atlas-server/src/contract.rs::contract,
# client/AqcContract.cs::Satisfies.
Feature: Versioning -- the server advertises its AQC range; the client fails loud on mismatch

  Scenario: the server advertises the supported AQC version range
    When I query "/api/contract"
    Then the response is a valid "ContractOut"
    And the server advertises AQC version "0.1.0" through "0.1.0"

  Scenario: a client whose version falls inside the advertised range is accepted
    Given the server advertises AQC version "0.1.0" through "0.1.0"
    Then the client accepts the advertised range

  Scenario: a client whose version falls outside the advertised range is rejected, loud
    Given the server advertises AQC version "0.2.0" through "0.5.0"
    Then the client rejects the advertised range

  # Playwright-only (browser-level fail-loud surface -- not exercised by
  # either Gherkin harness, which never render a page): see
  # client/wwwroot's own Playwright suite, "contract mismatch" spec --
  # happy path (real /api/contract, app loads normally) + a mocked-mismatch
  # response (the app shows the contract-mismatch page, never the ordinary
  # shell).
