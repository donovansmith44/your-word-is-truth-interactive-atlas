# /api/contract advertises the range of contract versions this server answers
# for, and a consumer outside that range must fail loud rather than run on.
# GET /api/contract -- server/atlas-contract/src/meta.rs::contract,
# client/AqcContract.cs::Satisfies.
Feature: Versioning -- the server advertises its AQC range; the client fails loud on mismatch

  Scenario: the server advertises the supported AQC version range
    When I query "/api/contract"
    Then the response is a valid "Contract"
    And the server advertises AQC version "0.8.0" through "0.8.0"

  Scenario: a client whose version falls inside the advertised range is accepted
    Given the server advertises AQC version "0.8.0" through "0.8.0"
    Then the client accepts the advertised range

  Scenario: a client whose version falls outside the advertised range is rejected, loud
    Given the server advertises AQC version "0.9.0" through "1.0.0"
    Then the client rejects the advertised range

  # A MALFORMED advertisement is the deployment-skew case this check exists for,
  # not a network failure, so it must raise rather than read as agreement. Both
  # bindings assert that: AqcContract.Satisfies, and this file's own local mirror
  # of it.
  Scenario: a malformed advertised version is a mismatch, loud
    Given the server advertises AQC version "garbage" through "0.8.0"
    Then the malformed advertisement fails loud

  # Playwright-only: "unreachable" and "hangs, then times out" are browser-level
  # behaviours neither Gherkin harness can express honestly, since neither renders
  # a page. See tests/ux/contract-versioning.spec.ts for the three cases -- the
  # happy path, a mocked mismatch the app must show its mismatch page for, and an
  # unreachable /api/contract, which must load normally rather than hang.
