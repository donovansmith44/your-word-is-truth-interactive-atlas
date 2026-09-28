# PlacePeriodQuery(place, from, to) -> the place, with the name and the history
# it bore in that span of years. GET /api/place/{id}?from=&to= --
# server/atlas-contract/src/places.rs::place. The period is asked for with both
# years or with neither, and a pair this atlas cannot read is the same refusal
# every other pair of years is refused with.
Feature: PlacePeriodQuery -- one place read through a span of years

  Scenario: a year that is not a year is bad_window
    When I query "/api/place/hazor-1?from=notayear"
    Then the request fails with status 400 and code "bad_window"
