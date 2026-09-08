# Atlas → Map API Consumer Contract — CHANGELOG

The atlas's expectations **of map-generator's map API**, published from
this repo for map-generator to run. Bump classes are the same ones
`contracts/atlas-graph-contract/CHANGELOG.md` defines, and the same
`scripts/contract-semver-gate.sh` derives them from the diff.

Cross-repo negotiation runs on this number: map-generator declares the
version of this suite it is written against, and a provider serving an
older MAJOR is a hard break. That is what "cannot push if our contracts
don't agree" means in practice, in this direction.

- **0.1.0** (Batch CDC-1) — Initial suite, deliberately two scenarios.
  The atlas consumes no byte of the map API yet (MAPS-1's coupling
  question Q1 is unruled), so this states only what holds regardless of
  which coupling shape is chosen and what map-generator's own contract set
  already commits to: C6's shared version root, and C3's coordinate
  authority as a referential law over drawn markers. Neither is
  satisfiable by re-blessing a fixture; there are no fixtures.
