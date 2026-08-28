# Atlas Query Contract — CHANGELOG

One line per version, semver-justified (spec §2).

- **0.1.0** (Batch AQC-1) — Initial contract: a snapshot of today's actually-consumed wire shapes for the five query families (FocusQuery, TraversalQuery, TextWindowQuery, SceneQuery, ExplorationRoundTrip) plus the new `/api/contract` version-advertisement endpoint. Zero behavior change to any pre-existing endpoint — this version documents what the server already serves and the client already consumes; it does not redesign anything (spec §9 Q4, recommended scope).
