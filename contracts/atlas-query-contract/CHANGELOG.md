# Atlas Query Contract — CHANGELOG

One line per version, semver-justified (spec §2).

- **0.1.0** (Batch AQC-1) — Initial contract: a snapshot of today's actually-consumed wire shapes for the five query families (FocusQuery, TraversalQuery, TextWindowQuery, SceneQuery, ExplorationRoundTrip) plus the new `/api/contract` version-advertisement endpoint. Zero behavior change to any pre-existing endpoint — this version documents what the server already serves and the client already consumes; it does not redesign anything (spec §9 Q4, recommended scope).

- 0.2.0 (DB-4a): re-recorded through the one assembly path (`export_aqc_examples`) — every fixture's "version" is the 32-hex manifest root and every "edge" a canonical edge id (spec §3.6); no scenario text changed (MAJOR class, MINOR bump under the pre-launch 0.x rule). Disclosed: `scene-time.json` also catches up one value that had drifted since the fixture was last exported two roots ago (`galilee-1` quiet place `total_events` 16 -> 17, an event the corpus gained in the meantime) -- not an identity change.

- 0.3.0 (DB-4b): re-recorded through the one assembly path once more -- the version root moved when the logical dump widened to the node projections, the resolved chronology, the heading index, the red-letter spans and the nine folded sidecars (spec 3.4, 5.3-5.6; every section table is now in the root). Only the 19 fixtures' `version` values change; no edge id, no pid, no scenario text (MAJOR class, MINOR bump under the pre-launch 0.x rule).

- 0.4.0 (DB-4c, the LAST root move): `event_date` gained the chronology's `source_meta` columns (`meta_to_year`, `order_key` -- the curated values the Event wire serves; DB-4b had dropped `order_key` as a duplicate of `seq`, which it is not), so core's logical hash and the root moved once more. Only the 19 fixtures' `version` values change; no edge id, no pid, no scenario text (MAJOR class, MINOR bump under the pre-launch 0.x rule). Also (additive, DB-4c's read cutover): `ContractOut` gains optional `manifest_schema` and `section_schema_version` -- the served identity is the manifest's (spec 9).
