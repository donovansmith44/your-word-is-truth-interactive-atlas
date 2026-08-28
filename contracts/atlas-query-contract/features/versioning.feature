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

  # Fix round 1 (Q-4/§0, controller ruling): a MALFORMED advertisement is
  # the deployment-skew case this gate exists for, not a network failure --
  # App.razor's own catch used to swallow this into "loads normally" (the
  # exact fail-loud violation the house law forbids). Binds on both Gherkin
  # sides (AqcContract.Satisfies / this file's own local `satisfies`
  # mirror, both returning/raising a fail-loud result on a malformed
  # semver string).
  Scenario: a malformed advertised version is a mismatch, loud
    Given the server advertises AQC version "garbage" through "0.1.0"
    Then the malformed advertisement fails loud

  # Fix round 1 (Q-5/§0, controller ruling): Playwright-only (browser-level
  # fail-loud surface -- not exercised by either Gherkin harness, which
  # never render a page; also, "unreachable" and "hangs, then times out"
  # are not phrases either harness's own fixture/live-request model can
  # express honestly): see tests/ux/contract-versioning.spec.ts --
  # (1) happy path (real /api/contract, app loads normally), (2) a
  # mocked-mismatch response (the app shows the contract-mismatch page,
  # never the ordinary shell), (3) an UNREACHABLE /api/contract
  # (route.abort()) -- the app loads normally rather than hanging or
  # failing (App.razor's own 2s CancellationToken timeout, Q-5).
