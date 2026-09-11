@featurelevel
Feature: the oracle's own corpus

  Scenario: plain, no tags
    Given nothing

  @target
  Scenario: a plain target
    Given nothing

  @wip @target
  Scenario: the multi-tag form that defeated the regex
    Given nothing

  @a @b @c   @target
  Scenario: target last on a crowded line
    Given nothing

  @wip@target
  Scenario: the control -- one word, not two tags
    Given nothing
