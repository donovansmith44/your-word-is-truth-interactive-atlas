//! # THE CHRONOLOGY AUTHORITY LAW (Batch CHRON-1, owner ruling 2026-08-27,
//! verbatim: "why are we pulling chronology from conflicting sources? we
//! should have one absolute source of truth") -- this module's own
//! governing law from this batch forward. Everything below this block (the
//! HOTFIX-4 root-cause analysis, the sweep, the two curated tables) is now
//! this law's OWN IMPLEMENTATION, not a separate policy standing beside it:
//!
//! 1. **ONE CHRONOLOGICAL SCALE.** Every dated event in the compiled graph
//!    sits on the SAME scale -- the curated, Robertson-anchored, hand-typed
//!    AD-33 Passion-anchor scale HOTFIX-4 already established as this
//!    atlas's own authority (`nt_calibration.rs`'s `THEO_DATE_OVERRIDES` /
//!    `chronology.rs`'s anchor machinery already carry NT-era Theographic
//!    dates onto it). This law does not ask for a new scale -- only that NO
//!    event escape it under a second, uncalibrated id.
//! 2. **ONE PRECEDENCE ORDER.** Wherever a CURATED opinion exists for a
//!    real-world occurrence (a `robertson_section`/`acts_section`/
//!    `atlas_section`/`kjv_superscription`-grounded container -- this
//!    module's own `is_layer0` predicate's negation, LAYER-1), that curated
//!    date is THE truth for it, full stop. Theographic remains, permanently,
//!    a source of event EXISTENCE and WITNESSES -- its no-curated-
//!    counterpart events (the ~295 OT/Acts/Epistle rows the Gospel-harmony
//!    curated set never touches) keep their own coverage, undiminished by
//!    this law -- but its OWN dates are SUBORDINATE: admitted onto the one
//!    scale (today's calibration machinery, unchanged by this law) only
//!    where NO curated opinion exists for that same occurrence, carried
//!    basis-labelled (`PlacementBasis`, unchanged; `graph-types` stays
//!    untouched, standing veto) exactly as every event already is today --
//!    this law invents no new basis value, it forbids a SECOND, unreconciled
//!    opinion from ever reaching a reader.
//! 3. **NO VERSE SURFACES TWO INDEPENDENT DATE OPINIONS FOR ONE EPISODE.**
//!    The leper pair (`rob_leper_healed`, curated, AD 30 vs `theo-286`,
//!    Theographic-calibrated, AD 31, both citing MAT.8.2-4/MAT.8.1-4) is the
//!    charter violation this law forbids by name -- two ids, two dates, one
//!    verse, exactly the root-cause CLASS the HOTFIX-4 doc comment below
//!    already named ("two ids for one event, each with a different opinion
//!    about when it happened") but whose own sweep
//!    (`DUPLICATE_JACCARD_THRESHOLD` at its former 0.8) was empirically
//!    tuned too high to catch (leper-pair verse-jaccard 0.733 -- see
//!    `.superpowers/sdd/2026-08-17-bible-atlas-m1/dup-events-investigation.md`).
//!
//! **ENFORCEMENT (the conformance corollary,
//! `docs/superpowers/specs/2026-08-26-frontend-backend-contract-design.md`
//! §0a):** two independent, fail-loud mechanisms, neither alone sufficient
//! -- mirrors this module's own existing two-detector precedent
//! (`verse_jaccard` + `cross_book_duplicate_candidate` below):
//! (a) **THE SWEEP, lowered.** `DUPLICATE_JACCARD_THRESHOLD` moves from this
//!     module's former 0.8 floor to **0.5**: the investigation's real census
//!     of the whole curated x Theographic corpus found every unaudited
//!     same-pericope pair sitting in the 0.5-0.8 band (~27 pairs, plus the 4
//!     disclosed-but-unswept layer0-layer0 Acts pairs below), with no
//!     confirmed-distinct pair scoring above it once that band is
//!     hand-triaged (ticket 1) -- the SAME "measure, don't guess" discipline
//!     the original 0.8 derivation used, re-run against the wider,
//!     hand-audited sample. `atlas_etl::validate::run_event_merges` is ALSO
//!     widened to compare LAYER-0-against-LAYER-0 pairs (previously
//!     LAYER-0-vs-LAYER-1 only), closing the gap the 4 disclosed Acts pairs
//!     (`p1_pisidian_antioch`/`theo-340` etc., named in the "NOT swept"
//!     section below) sat in.
//! (b) **THE NO-TWO-OPINIONS VALIDATION** (new, `atlas_etl::validate::
//!     run_no_two_opinions`, ticket 2): a corpus-wide, POST-merge check --
//!     unlike the sweep above, which runs on the PRE-merge event set (the
//!     only point a curated/absorbed pair both still exist to compare) --
//!     asserting no two SURVIVING events at verse-jaccard >=
//!     `DUPLICATE_JACCARD_THRESHOLD` (the law's own threshold, reused, not
//!     re-derived) carry independent placements (`when.from_year`,
//!     `when.to_year`, or `order_key` differing). This is the law's DIRECT
//!     enforcement: the sweep is pairwise triage tooling for CURATING the
//!     merge tables; this check fails the build if, after every triage
//!     decision, a placement-level contradiction still reaches a reader.
//!     Design: reuse `event_merge::effective_verses`/`verse_jaccard`
//!     (identical to the union `AtlasData::finish()`'s own `verse_to_events`
//!     index uses, so "who touches this verse" never disagrees with the
//!     check); run once, on the POST-`apply_event_merges` event set, over
//!     every `kind == "event"` pair; skip a pair listed in
//!     `EVENT_DISTINCT_PAIRS` (a genuinely-distinct mega-span/complementary-
//!     beat pair is EXPECTED to keep two placements -- that is what
//!     "distinct" means); a hit fails loud naming both ids, both labels,
//!     both placements, and the jaccard score. Proven both directions in
//!     `validate.rs`'s own test module: PASSES on the real post-triage
//!     corpus, and PROVABLY FAILS on a planted violation (two synthetic
//!     same-verse events given different `from_year`s, neither merged nor
//!     distinct-listed).
//!
//! ---
//!
//! Batch HOTFIX-4 (coordinator amendment, 2026-08-21, owner live report:
//! "the ordering of the narratives is wrong. the temptation of Jesus in the
//! wilderness, for instance, is labeled as being before Jesus' baptism.
//! this is a straight up lie."): duplicate EVENT identity rectification.
//!
//! ROOT CAUSE (verified against the real compiled data before this module
//! was written): the Theographic import produces ~450 "freebie" events (no
//! curated `witnesses`/`robertson_section`/`acts_section`/`atlas_section` --
//! `heading_precedence`'s own LAYER-0) dated on Theographic's OWN
//! approximate scale (e.g. `theo-267` "John Baptizes Jesus," AD 26). Batch
//! T/T2/W1 separately, independently curated the SAME real-world events as
//! richer, Robertson-grounded LAYER-1 containers on the owner's own AD-33
//! Passion-anchor scale (e.g. `jm_jordan` "Jesus is baptized in the Jordan,"
//! AD 29, `robertson_section` set). `heading_precedence` already makes the
//! LAYER-1 container win the READER HEADING at any shared verse -- but
//! HOTFIX-4 requirement 1 makes EVERY dated event a real node in a single
//! global chronological graph, and the freebie was never removed: it is
//! still there, on its own wrong scale, one click away from any verse it
//! shares with its own richer twin. Global (from_year, order_key) ordering
//! then genuinely and reproducibly interleaves the two scales -- e.g.
//! `theo-267` (26) / `theo-268` "Temptations of Jesus" (26) sort BEFORE
//! `jm_jordan` (29) / `rob_temptation` (29), so a user who happens to open
//! the FREEBIE Temptation node sees it precede the REAL Baptism node. Not a
//! resolver bug (the resolver's own ordering is correct GIVEN the dates it's
//! handed) -- a DATA bug: two ids for one event, each with a different
//! opinion about when it happened.
//!
//! FIX: same shape as `crate::merge`'s own same-place pattern (id aliasing,
//! a small hand-curated table, applied once in `AtlasData::finish()`,
//! reversible/auditable) -- but for EVENT identity rather than PLACE
//! identity, and DECISIVE rather than a union: the survivor (always the
//! real, LAYER-1 container -- richer, independently citation-verified, and
//! per the owner's own binding CHRONOLOGY ANCHOR ruling, correctly dated on
//! the AD-33 scale) keeps its OWN fields completely unmodified; the
//! absorbed freebie is removed from the compiled graph entirely, and every
//! reference to its id (narrative legs -- none in today's data, but checked
//! defensively) is repointed to the survivor. This satisfies the owner's
//! own container-algebra law (progress.md "OWNER DIRECTIVE -- passage
//! container algebra": "we don't modify the verses... identity is empty
//! set") by construction: no verse is ever rewritten by this module: a
//! duplicate CONTAINER RECORD disappears, the underlying immutable verses
//! it cited stay exactly where they were (still indexed, via the survivor,
//! by `AtlasData::finish`'s own `verse_to_events` pass). It also satisfies
//! the amendment's own explicit rule B ("the superseded scale is not
//! preserved in shipped data") by construction: nothing here ever reads the
//! absorbed event's own `when`/`order_key` into the survivor.
//!
//! SWEEP METHODOLOGY (full record in batch-hotfix4-report.md): every
//! LAYER-0 ("freebie") event compared against every LAYER-1 ("real") event
//! sharing >=1 book, by verse-set Jaccard overlap (the union of `verses`
//! and every witness's own `translations["kjv"]`, matching
//! `AtlasData::finish`'s own `verse_to_events` union) -- `>= 0.8` is the
//! empirically-verified floor: every real duplicate found sits at
//! 0.875-1.0; the 0.8-0.85 band contains exactly four Theographic MEGA-SPAN
//! freebies that each bundle TWO distinct, separately-curated pericopes
//! (never a clean 1:1 duplicate -- merging either would misattribute the
//! OTHER pericope's own citation), correctly excluded from
//! `EVENT_MERGE_PAIRS` and listed in `EVENT_DISTINCT_PAIRS` instead so the
//! fail-loud validator (`atlas_etl::validate::run_event_merges`) doesn't
//! re-flag them every run. Two further pairs at this same threshold are OT
//! (`theo-128`/`theo-129`) -- the SAME bug class, genuinely real, but
//! outside this batch's own "Gospel-era" scope per the amendment's own
//! "stop at the Gospel-era set... report the remainder for a follow-up"
//! instruction (one of the two, `theo-129`, is ALSO already independently
//! disclosed in batch-w1-report.md as carrying its own separate data
//! anomaly, a stray `GEN.34.1` verse) -- both listed in
//! `EVENT_DISTINCT_PAIRS`, deferred honestly, not silently dropped and not
//! silently merged.
//!
//! WAS not swept, brought into scope by Batch CHRON-1: freebie-vs-freebie
//! near-duplicates (e.g. `theo-145`/`theo-152`, a Judges-era "reign of
//! X"/"death of X" pair; `p1_pisidian_antioch`/`theo-340`, an Acts-era
//! pair) -- neither side is a real (LAYER-1) container, so there was no
//! obvious, citation-verified survivor to merge INTO without first curating
//! one of them, and HOTFIX-4's own validator only ever compared a LAYER-0
//! event against a LAYER-1 one, so these never tripped it. THE CHRONOLOGY
//! AUTHORITY LAW's own enforcement (a) above widens `run_event_merges` to
//! ALSO compare layer0-against-layer0 pairs; where triage finds a genuine
//! duplicate with no pre-existing LAYER-1 side, the better-witnessed/
//! better-dated side survives per that pair's own `EVENT_MERGE_PAIRS`
//! entry, justified there -- see the layer0-layer0 entries below (marked in
//! their own reason strings) for which pairs this batch actually resolved
//! this way, and any still-deferred pair's own honest disclosure.

use std::collections::HashSet;

use crate::data::{Event, Narrative};

/// One curated same-event pair: `absorbed` is removed entirely;
/// `survivor` keeps every one of its own fields completely unmodified --
/// see this module's own doc comment for why (container-algebra
/// compliance, rule B compliance). USUALLY absorbed = the bare
/// Theographic freebie and survivor = the real, Robertson-grounded,
/// AD-33-anchored curated container -- but see the M-D1 entries' own
/// reason strings for two LAYER-0-vs-LAYER-0 exceptions
/// (pr_rome/theo-384 on the AD-60 scale; theo-338/theo-337
/// freebie-over-freebie), where the blanket claim does not hold.
pub struct EventMerge {
    pub survivor: &'static str,
    pub absorbed: &'static str,
    /// Why these two are the SAME event -- for a future reader/curator, not
    /// read by any code (mirrors `crate::merge::PlaceMerge::reason` exactly).
    pub reason: &'static str,
}

/// 92 pairs (68 pre-CHRON-1 + 24 added by Batch CHRON-1's own triage of
/// every pair the lowered `DUPLICATE_JACCARD_THRESHOLD = 0.5` sweep (widened
/// to layer0-layer0 pairs too) newly flagged -- see each `"Batch CHRON-1"`
/// entry's own reason for its individual derivation; the complete 68-pair
/// triage table (every flagged pair, disposition, reason) lives in
/// batch-chron1-report.md), every one individually verified against the
/// real compiled event set before being added here. The pre-CHRON-1 68:
/// 62 found by the
/// automated verse-set-Jaccard sweep (>=0.8, book-consistent,
/// label-consistent) plus 1 added by hand (`jm_jordan`/`theo-267`, jaccard
/// 0.5 -- below the sweep's own floor, but the owner's own named proof
/// case, see that entry's own reason) -- the full per-pair table lives in
/// batch-hotfix4-report.md, not duplicated here -- plus 1 added by Batch W4
/// (`oba_vision`/`theo-244`, jaccard 1.0, Obadiah's own single-container-book
/// exception, see that entry's own reason) -- plus 1 added by Batch W4 fix
/// round 1 (`jer_jeremiah_stays_with_gedaliah`/`exl_mizpah`, jaccard 0.000,
/// a CROSS-BOOK duplicate the verse-jaccard metric cannot see at all, found
/// only by the new `cross_book_duplicate_candidate` detector below -- see
/// that entry's own reason) -- plus 3 added by Batch M-D1 (the HOTFIX-5
/// "remaining duplicates" tail, parked across HOTFIX-4/W4/HOTFIX-6 and
/// finally rectified here, per the owner's own report #6 "rectify those
/// kinds of problems" generalized): `pr_rome`/`theo-384` (CROSS-BOOK,
/// title jaccard 1.000, both LAYER-0 -- the original layer0-vs-layer1
/// sweep never compared them), `theo-338`/`theo-337` (SAME-book, a
/// verse-set PREFIX the plain jaccard metric scores at 0.063 despite full
/// containment), `ezr_altar_and_foundation`/`ret_jerusalem_altar` (below
/// BOTH existing detectors' own floors -- found only by HOTFIX-6's own
/// chronology audit; see each entry's own reason below).
/// Alphabetical by `survivor` for easy scanning/diffing.
pub const EVENT_MERGE_PAIRS: &[EventMerge] = &[
    EventMerge { survivor: "1ki_jehoshaphat_summary", absorbed: "theo-181", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-181 \"Reign of Jehoshaphat\" (jaccard 0.529, 1Kgs.22.41-44+46-50) is the SAME regnal summary as 1ki_jehoshaphat_summary (curated, atlas_section-provenanced, LAYER-1, 1KI.22.41-50 -- the fuller range, including v.45 which theo-181's own citation skips -- PLUS a second witness book, 2CH.20.31-37, theo-181 does not touch at all). 1ki_jehoshaphat_summary survives (richer, LAYER-1, two-book witnessed); theo-181 absorbed." },
    EventMerge { survivor: "ab_egypt", absorbed: "theo-73", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-73 \"Abraham goes to Egypt\" (jaccard 0.727, Gen.12.10-20) and ab_egypt \"Famine drives Abram to Egypt\" (curated, GEN.12.10+GEN.12.14-20) narrate the identical occasion; ab_egypt's own citation deliberately skips GEN.12.11-13 (Abram's instruction to Sarai to pose as his sister), which no separate curated event claims either -- an honest, disclosed gap in curated coverage, not fabricated here. ab_egypt survives; theo-73 absorbed." },
    EventMerge { survivor: "ezr_altar_and_foundation", absorbed: "ret_jerusalem_altar", reason: "SAME real-world occasion under the decisive-container pattern (the jm_bethany/pw_bethany class, but for IDENTITY, not just heading precedence), Batch M-D1 remaining-duplicates sweep (queued at HOTFIX-6 fix round 1, 2026-08-22, controller ruling: \"ret_jerusalem_altar/ezr_altar_and_foundation added to HOTFIX-5 remaining-duplicates list\"): ret_jerusalem_altar (a bare `return`-narrative leg, LAYER-0, EZR.2.1 + EZR.3.1-2, dated -537) and ezr_altar_and_foundation (Batch W2's own real curated container, LAYER-1 via `atlas_section`, EZR.3.1-13, dated -536) both narrate the altar's own rebuilding in Ezra 3. Verse-jaccard is only 0.143 (ret_jerusalem_altar's own EZR.2.1 falls outside chapter 3 entirely -- already independently covered by the pre-existing `ezr_list_of_returnees`, EZR.2.1-70, so absorbing loses no coverage) and title-jaccard only 0.286, both well under either existing automated sweep's own floor -- this pair was invisible to both, found only by HOTFIX-6's own live chronology audit. ezr_altar_and_foundation survives (the richer, atlas_section-provenanced container, on its own established -536 dating, \"the second year of their coming,\" Ezra 3:8); ret_jerusalem_altar absorbed, its own `return`-narrative leg repointed to the survivor automatically by this module's own ALIASING mechanism below -- no hand edit to `data/curated/narratives/return.toml` needed or made." },
    EventMerge { survivor: "je_egypt_ruler", absorbed: "theo-111", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-111 \"Joseph Promoted in Egypt\" (jaccard 0.600, Gen.41.37-46) and je_egypt_ruler \"Joseph rises to rule over Egypt\" (curated, GEN.41.41-46) narrate the identical occasion -- Pharaoh's elevation of Joseph -- theo-111's own citation simply opens 4 verses earlier (41:37-40, Pharaoh's own deliberation before the appointment). No separate curated container claims Gen.41.37-40. je_egypt_ruler survives; theo-111 absorbed." },
    EventMerge { survivor: "jer_jeremiah_stays_with_gedaliah", absorbed: "exl_mizpah", reason: "CROSS-BOOK duplicate (jaccard 0.000 by this module's own verse-ID metric -- completely invisible to it, since the two sides cite entirely disjoint books, 2KI vs JER, for the identical occurrence; caught instead by Batch W4 fix round 1's own new cross-book title-similarity detector, `cross_book_duplicate_candidate` below, added specifically because this exact shape slipped past the original sweep -- see batch-w4-review.md Critical-1): \"Gedaliah governs the remnant at Mizpah\" (exl_mizpah, a pre-existing, bare, zero-witness curated event, 2KI.25.22-25) coarsely compresses TWO real, separately-narrated occasions -- Gedaliah's own governorship (2KI.25.22-24) AND, distinctly, his own assassination (2KI.25.25) -- that Jeremiah 40-41 narrates at much finer granularity and which this batch's own fresh authoring already dates as two separate containers. Per the controller's own binding ruling (\"where granularity genuinely differs... finer events dated, coarse twin absorbed\"), exl_mizpah is absorbed into the governorship half (jer_jeremiah_stays_with_gedaliah, this pair's own survivor); its own 2KI.25.22-24 verses are preserved as that survivor's own new witness row, and its own 2KI.25.25 verse becomes a witness on jer_the_assassination_of_gedaliah instead (data/curated/event-witnesses.toml) -- no coverage lost, only reorganized onto the honest, finer structure. The 'exile' narrative's own leg list is updated accordingly (data/curated/narratives/exile.toml), gaining a genuine extra leg rather than losing one." },
    EventMerge { survivor: "jm_bethsaida", absorbed: "theo-393", reason: "Theographic freebie duplicate (jaccard 0.978 verse-set overlap, same Gospel pericope): \"Feeding of Five Thousand\" (theo-393, Theographic-scale date) is the identical event as \"Jesus feeds the five thousand near Bethsaida\" (jm_bethsaida, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "jm_caesarea_philippi", absorbed: "theo-404", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-404 \"Peter's Confession \\\"Upon this Rock\\\"\" (jaccard 0.500, Matt.16.13-20+Mark.8.27-30+Luke.9.18-21) is the same episode as jm_caesarea_philippi \"Peter's confession at Caesarea Philippi\" (curated, Robertson Section 82, MAT.16.13-20) -- identical on their shared book (MAT.16.13-20 matches exactly). jm_caesarea_philippi did not yet carry MRK/LUK witness rows; per the standing coverage-completion law, theo-404's own MRK.8.27-30/LUK.9.18-21 coverage is restored as real witness rows (data/curated/event-witnesses.toml, the jm_jordan/theo-267 precedent), not dropped. jm_caesarea_philippi survives; theo-404 absorbed." },
    EventMerge { survivor: "jm_cana", absorbed: "theo-271", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Water to Wine\" (theo-271, Theographic-scale date) is the identical event as \"Jesus turns water to wine at Cana\" (jm_cana, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "jm_egypt", absorbed: "theo-257", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Joseph and Mary Flee to Egypt\" (theo-257, Theographic-scale date) is the identical event as \"The holy family flees to Egypt\" (jm_egypt, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "jm_jordan", absorbed: "theo-267", reason: "Theographic freebie duplicate (jaccard 0.500 verse-set overlap -- below this module's own 0.8 sweep floor, so NOT found by the automated sweep; added by hand because the owner's own live report named this exact pair as proof the traversal lies: 'the temptation of Jesus in the wilderness... is labeled as being before Jesus' baptism'): \"John Baptizes Jesus\" (theo-267, Theographic-scale AD 26, covering MAT+MRK+LUK) is the identical event as \"Jesus is baptized in the Jordan\" (jm_jordan, the AD-33-anchored, Robertson-grounded curated container, AD 29, MAT-primary). Absorbed; theo-267's own extra Mark/Luke coverage is restored not by touching jm_jordan's own top-level verses but by giving jm_jordan real, individually-verified MAT/MRK/LUK witness rows (data/curated/event-witnesses.toml), the same shape its own very next narrative leg (rob_temptation) already has." },
    EventMerge { survivor: "jm_nazareth", absorbed: "theo-260", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-260 \"Joseph and Mary return from Egypt\" (jaccard 0.667, Matt.2.19-22) is a clean subset of jm_nazareth \"The family settles in Nazareth\" (curated, Robertson Section 16, MAT.2.19-23 top-level + LUK.2.39 witness). jm_nazareth survives; theo-260 absorbed." },
    EventMerge { survivor: "jm_nazareth", absorbed: "theo-262", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-262 \"Joseph and Mary return to Nazareth\" (jaccard 0.500, Matt.2.22-23+Luke.2.39) is Theographic's OWN second freebie for the identical return-to-Nazareth episode jm_nazareth already curates as one event (see theo-260's own sibling entry immediately above) -- every one of theo-262's own verses (MAT.2.22, MAT.2.23, LUK.2.39) is already inside jm_nazareth's own effective verse set (MAT.2.19-23 top-level + LUK.2.39 witness). jm_nazareth survives; theo-262 absorbed." },
    EventMerge { survivor: "jm_sychar", absorbed: "theo-278", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-278 \"The Samaritan Woman\" (jaccard 0.564, John.4.5-42) and jm_sychar \"Jesus speaks with the woman at the well in Sychar\" (curated, Robertson Section 35) are the same episode. theo-278's own trailing verses (JHN.4.27-42, the disciples' return and the Samaritans' own belief) had no separate curated container of their own; per the standing coverage-completion law, jm_sychar's own top-level citation is widened from JHN.4.4-26 to the full JHN.4.4-42 (matching Robertson's own §35 exactly, data/curated/events-extra.toml) to preserve that coverage rather than drop it -- the jm_jordan/theo-267 precedent, applied to a same-book top-level widening instead of a cross-book witness row. jm_sychar survives; theo-278 absorbed." },
    EventMerge { survivor: "oba_vision", absorbed: "theo-244", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap -- Obadiah is a single 21-verse book, the brief's own explicit lawful single-container exception): \"Prophecies of Obadiah\" (theo-244, Theographic-scale date -585) is the identical event as \"The vision of Obadiah.\" (oba_vision, Batch W4, kjv_superscription-titled whole-book container, GENERAL-kind/undated per that file's own kind policy). Absorbed; survivor's own citation stands unmodified." },
    EventMerge { survivor: "p1_iconium", absorbed: "theo-341", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0 (one of the 4 disclosed Acts pairs): theo-341 \"Mission to Iconium\" (jaccard 0.714, ACT.14.1-5) is a clean subset of p1_iconium \"The city divided at Iconium\" (curated, paul-first-journey chain, ACT.14.1-7 -- the fuller citation, including the flight to Lystra/Derbe that closes the Iconium scene). p1_iconium survives (part of the deliberately-authored journey chain); theo-341 absorbed." },
    EventMerge { survivor: "p1_lystra", absorbed: "theo-342", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0 (one of the 4 disclosed Acts pairs): theo-342 \"Mission to Lystra and Derbe\" (jaccard 0.867, ACT.14.6-20) and p1_lystra \"Stoned at Lystra\" (curated, paul-first-journey chain, ACT.14.8-20) narrate the same episode; theo-342's own two extra lead-in verses, ACT.14.6-7 (the flight from Iconium), are already claimed by the neighboring p1_iconium waypoint's own wider citation (ACT.14.1-7) -- a boundary-allocation difference between the two curated traditions, not a distinct scene. p1_lystra survives (part of the deliberately-authored journey chain); theo-342 absorbed." },
    EventMerge { survivor: "p1_pisidian_antioch", absorbed: "theo-340", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0 (both bare freebies -- one of the 4 disclosed-but-formerly-unswept Acts pairs named in this module's own former \"NOT swept\" section): theo-340 \"Mission to Antioch in Pisidia\" (jaccard 0.975, ACT.13.13-52) and p1_pisidian_antioch \"Sermon in Pisidian Antioch\" (curated, part of the paul-first-journey narrative chain, ACT.13.14-52) are the same sermon episode; theo-340's own one extra verse, ACT.13.13 (\"John Mark departs at Perga\"), already belongs to the separate, already-curated p1_perga waypoint event -- no coverage lost. Better-witnessed/better-dated side survives per THE CHRONOLOGY AUTHORITY LAW's layer0-layer0 convention: p1_pisidian_antioch is part of a deliberately-authored, narrative-integrated journey chain (predecessor/successor waypoints all individually curated) vs theo-340's own bare, isolated freebie. p1_pisidian_antioch survives; theo-340 absorbed." },
    EventMerge { survivor: "pr_rome", absorbed: "theo-384", reason: "CROSS-BOOK duplicate, Batch M-D1 remaining-duplicates sweep (queued at HOTFIX-4 fix round 1's own re-review, batch-hotfix4-rereview.md's own Minor finding, 2026-08-22): theo-384 \"Paul arrives at Rome\" (Theographic freebie, LAYER-0, ACT.28.11-16) is the identical event as pr_rome \"Paul arrives at Rome\" (this atlas's own curated `paul-rome-voyage` narrative's own final leg, LAYER-0 too, ACT.28.16 -- a genuine verse-set subset of theo-384's own wider range) -- IDENTICAL labels (title jaccard 1.000), same year AD 60, common place 'rome'. Both sides being LAYER-0 is exactly why the original layer0-vs-layer1 verse-jaccard sweep never even compared them; caught only by the cross-book title-similarity detector. pr_rome survives: it is this atlas's own deliberately-authored, narrative-integrated identity (nine-leg `paul-rome-voyage` chain) on the atlas's own established AD-60 scale -- the exact id `server/atlas-graph/tests/narrative_real_data.rs`'s own `global_timeline_true_extremes_of_the_real_atlas` test and `tests/ux/event-timeline.spec.ts` already assert resolves and sorts strictly before theo-385's own imprisonment. theo-384 absorbed; its own wider ACT.28.11-15 coverage is not preserved as a witness (unlike jm_jordan/theo-267) -- pr_rome's own single-verse identity, ACT.28.16, is this atlas's own deliberate anchor for the arrival MOMENT, not the wider stay, and no other event narrows to exactly that gap." },
    EventMerge { survivor: "pw_gethsemane", absorbed: "theo-454", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Prayer and Betrayal in Gethsemane\" (theo-454, Theographic-scale date) is the identical event as \"Jesus prays and is arrested at Gethsemane\" (pw_gethsemane, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "pw_jerusalem_entry", absorbed: "theo-444", reason: "Theographic freebie duplicate (jaccard 0.978 verse-set overlap, same Gospel pericope): \"Triumphal Entry\" (theo-444, Theographic-scale date) is the identical event as \"The triumphal entry into Jerusalem\" (pw_jerusalem_entry, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "pw_temple_cleansing", absorbed: "theo-445", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-445 \"Temple Cleansed\" (jaccard 0.667, Matt.21.12-17+Mark.11.15-19+Luke.19.45-48) and pw_temple_cleansing \"Jesus cleanses the temple a second time\" (curated, Robertson Section 129, MAT.21.12-13 top-level + MRK.11.15-18/LUK.19.45-48 witnesses) narrate the same cleansing; theo-445's own small extra tail (MAT.21.14-17 healing-in-the-temple + children's Hosanna, MRK.11.19) has no separate curated container of its own -- an honest, disclosed gap, not preserved onto the survivor (container-algebra: survivor's own fields stand unmodified). pw_temple_cleansing survives; theo-445 absorbed." },
    EventMerge { survivor: "rob_adulterous_woman", absorbed: "theo-419", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Woman Caught in Adultery\" (theo-419, Theographic-scale date) is the identical event as \"A woman taken in adultery is brought to Jesus for judgment\" (rob_adulterous_woman, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_annunciation_mary", absorbed: "theo-250", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"The Annuciation\" (theo-250, Theographic-scale date) is the identical event as \"The angel Gabriel announces Jesus's birth to Mary\" (rob_annunciation_mary, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_baptist_birth", absorbed: "theo-252", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Birth of John the Baptist\" (theo-252, Theographic-scale date) is the identical event as \"The birth of John the Baptist\" (rob_baptist_birth, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_blind_bartimaeus", absorbed: "theo-439", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing Blind Bartimaeus\" (theo-439, Theographic-scale date) is the identical event as \"Blind Bartimaeus healed at Jericho\" (rob_blind_bartimaeus, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_boy_in_temple", absorbed: "theo-264", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus as a child in the Temple\" (theo-264, Theographic-scale date) is the identical event as \"The boy Jesus among the teachers in the temple\" (rob_boy_in_temple, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_bread_of_life", absorbed: "theo-397", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Bread of Life Sermon\" (theo-397, Theographic-scale date) is the identical event as \"The Bread of Life discourse in the Capernaum synagogue\" (rob_bread_of_life, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_call_of_matthew", absorbed: "theo-288", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus Calls Matthew\" (theo-288, Theographic-scale date) is the identical event as \"The call of Matthew (Levi) the tax collector\" (rob_call_of_matthew, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_capernaum_demoniac", absorbed: "theo-283", reason: "Theographic freebie duplicate (jaccard 0.933 verse-set overlap, same Gospel pericope): \"The Demoniac\" (theo-283, Theographic-scale date) is the identical event as \"Jesus heals a demoniac in the Capernaum synagogue\" (rob_capernaum_demoniac, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_capernaum_sojourn", absorbed: "theo-272", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Visiting Capernaum\" (theo-272, Theographic-scale date) is the identical event as \"A brief sojourn at Capernaum\" (rob_capernaum_sojourn, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_centurion_servant", absorbed: "theo-297", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing Centurion's Servant\" (theo-297, Theographic-scale date) is the identical event as \"Jesus heals a centurion's servant at Capernaum\" (rob_centurion_servant, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_childhood_nazareth", absorbed: "theo-263", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Childhood of Jesus\" (theo-263, Theographic-scale date) is the identical event as \"Jesus grows up at Nazareth\" (rob_childhood_nazareth, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_circumcision", absorbed: "theo-259", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus Circumsized\" (theo-259, Theographic-scale date) is the identical event as \"The circumcision of Jesus\" (rob_circumcision, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_crippled_woman_sabbath", absorbed: "theo-429", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-429 \"Healing Woman With 18-year Infirmity\" (jaccard 0.667, Luke.13.10-17) is a clean subset of rob_crippled_woman_sabbath \"Jesus heals a crippled woman on the sabbath\" (curated, Robertson Section 110, LUK.13.10-21 -- Robertson's own section title itself notes it also covers \"Repetition of the Parables of the Mustard Seed and of the Leaven,\" 13:18-21, deliberately bundled as one section, same pattern as rob_fasting_parables above). rob_crippled_woman_sabbath survives; theo-429 absorbed." },
    EventMerge { survivor: "rob_deaf_mute_feeding_4000", absorbed: "theo-401", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-401 \"Feeding of Four Thousand\" (jaccard 0.593, Matt.15.32-39+Mark.8.1-9) and rob_deaf_mute_feeding_4000 \"Jesus heals a deaf-mute in Decapolis and feeds four thousand\" (curated, Robertson Section 79, MAT.15.29-38 top-level + MRK.7.31-37/8.1-9 witnesses) are the same combined episode -- Robertson's own section already bundles the deaf-mute healing AND the feeding as one unit, matching curated's own title; the boundary differences (rob's citation starts 3 verses earlier at the healing, theo's runs 1 verse later at the dismissal) are the same pericope viewed with slightly different edges, not a distinct scene. rob_deaf_mute_feeding_4000 survives; theo-401 absorbed." },
    EventMerge { survivor: "rob_dedication_feast", absorbed: "theo-431", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Feast of Dedication\" (theo-431, Theographic-scale date) is the identical event as \"At the Feast of Dedication, Jesus will not yet openly claim to be the Messiah\" (rob_dedication_feast, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_defilement_controversy", absorbed: "theo-398", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Commandments and Tradition Discourse\" (theo-398, Theographic-scale date) is the identical event as \"Jesus and the Pharisees dispute over ceremonial hand-washing\" (rob_defilement_controversy, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_demoniac_boy", absorbed: "theo-407", reason: "Theographic freebie duplicate (jaccard 0.935 verse-set overlap, same Gospel pericope): \"Healing a Boy with a Demon\" (theo-407, Theographic-scale date) is the identical event as \"Jesus heals a demoniac boy the disciples could not heal\" (rob_demoniac_boy, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation gains theo-407's own one orphaned boundary verse (MAT.17.21, this pericope's own closing verse, read directly against the compiled KJV text before being added) and date stand -- otherwise -- unmodified." },
    EventMerge { survivor: "rob_eighteen_years", absorbed: "theo-265", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus Growing Up\" (theo-265, Theographic-scale date) is the identical event as \"The hidden years at Nazareth\" (rob_eighteen_years, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_fasting_parables", absorbed: "theo-289", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-289 \"Fasting Question\" (jaccard 0.500, Matt.9.14-15+Mark.2.18-20+Luke.5.33-35) is the FIRST HALF of rob_fasting_parables' own combined pericope (curated, Robertson Section 48, MAT.9.14-17 top-level + MRK.2.18-22/LUK.5.33-39 witnesses) -- curated deliberately keeps the fasting question AND the wineskins parable that follows (Theographic's own separate theo-290, see next entry) as ONE undivided section, matching Robertson's own table. theo-289's own effective verse set is a clean subset of rob_fasting_parables' own wider effective set (8/16, jaccard 0.5 exactly). rob_fasting_parables survives; theo-289 absorbed." },
    EventMerge { survivor: "rob_fasting_parables", absorbed: "theo-290", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-290 \"Parable of the Winseskins\" (jaccard 0.500, Matt.9.16-17+Mark.2.21-22+Luke.5.36-39) is the SECOND HALF of the same combined pericope rob_fasting_parables already curates as one section (see theo-289's own sibling entry immediately above for the full derivation) -- Theographic splits the fasting question and the wineskins parable into two freebies; curated does not. rob_fasting_parables survives; theo-290 absorbed." },
    EventMerge { survivor: "rob_first_disciples", absorbed: "theo-270", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"First Disciples Gathered\" (theo-270, Theographic-scale date) is the identical event as \"Jesus makes his first disciples\" (rob_first_disciples, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_first_galilee_tour", absorbed: "theo-285", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus' 1st Tour of Galilee\" (theo-285, Theographic-scale date) is the identical event as \"The first tour of Galilee\" (rob_first_galilee_tour, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_foretells_death_2", absorbed: "theo-408", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Second Foretelling of Death and Resurrection\" (theo-408, Theographic-scale date) is the identical event as \"Jesus again foretells his death, returning through Galilee\" (rob_foretells_death_2, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_four_fishers", absorbed: "theo-282", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Fishers of Men\" (theo-282, Theographic-scale date) is the identical event as \"Jesus calls four fishermen to follow him\" (rob_four_fishers, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_gadarene_demoniac", absorbed: "theo-387", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Gadarene Demoniacs\" (theo-387, Theographic-scale date) is the identical event as \"Jesus heals the demoniac(s) in the country of the Gadarenes\" (rob_gadarene_demoniac, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_gennesaret_reception", absorbed: "theo-396", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing in Gennesaret\" (theo-396, Theographic-scale date) is the identical event as \"The reception at Gennesaret\" (rob_gennesaret_reception, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_good_samaritan", absorbed: "theo-423", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Good Samaritan Parable Taught\" (theo-423, Theographic-scale date) is the identical event as \"The parable of the Good Samaritan\" (rob_good_samaritan, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_good_shepherd", absorbed: "theo-427", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"The Good Shepherd Teaching\" (theo-427, Theographic-scale date) is the identical event as \"The parable of the Good Shepherd\" (rob_good_shepherd, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_grain_sabbath", absorbed: "theo-292", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Corn on the Sabbath\" (theo-292, Theographic-scale date) is the identical event as \"The disciples pluck grain on the sabbath\" (rob_grain_sabbath, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_herod_fears", absorbed: "theo-391", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-391 \"John Beheaded\" (jaccard 0.613, Matt.14.6-12+Mark.6.21-29+Luke.9.7-9) is a clean subset of rob_herod_fears \"Herod Antipas's guilty fears about Jesus\" (curated, Robertson Section 71, MAT.14.1-12 top-level + MRK.6.14-29/LUK.9.7-9 witnesses -- the fuller citation, including Herod's own backstory/fear verses 14:1-5/6:14-20 that theo-391 omits). No separate curated sibling claims the extra verses. rob_herod_fears survives; theo-391 absorbed." },
    EventMerge { survivor: "rob_jerusalem_feast_healing", absorbed: "theo-291", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Bethesda pool healing\" (theo-291, Theographic-scale date) is the identical event as \"Jesus heals a lame man at a Jerusalem feast on the sabbath\" (rob_jerusalem_feast_healing, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_joseph_annunciation", absorbed: "theo-253", reason: "Theographic freebie duplicate (jaccard 0.875 verse-set overlap, same Gospel pericope): \"An Angel Speaks to Joseph in a Dream\" (theo-253, Theographic-scale date) is the identical event as \"An angel reassures Joseph in a dream\" (rob_joseph_annunciation, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_judas_bargains", absorbed: "theo-451", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Judas Plans Betrayal\" (theo-451, Theographic-scale date) is the identical event as \"Judas bargains with the rulers to betray Jesus\" (rob_judas_bargains, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_judas_suicide", absorbed: "theo-457", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Judas' Suicide\" (theo-457, Theographic-scale date) is the identical event as \"The remorse and suicide of Judas\" (rob_judas_suicide, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_last_nazareth_visit", absorbed: "theo-389", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"2nd Rejection at Nazareth\" (theo-389, Theographic-scale date) is the identical event as \"The last visit to Nazareth\" (rob_last_nazareth_visit, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_lazarus_effect", absorbed: "theo-434", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-434 \"Chief Priests Conspire Against Jesus\" (jaccard 0.700, John.11.47-53) is a clean subset of rob_lazarus_effect \"The Sanhedrin resolves to kill Jesus after Lazarus's raising\" (curated, Robertson Section 119, JHN.11.45-54 -- the wider citation, \"the effect of the raising of Lazarus on the people, on the Sanhedrin, on the movements of Jesus\"). rob_lazarus_effect survives; theo-434 absorbed." },
    EventMerge { survivor: "rob_lazarus_raised", absorbed: "theo-433", reason: "Theographic freebie duplicate (jaccard 0.957 verse-set overlap, same Gospel pericope): \"Lazarus Raised form the Dead\" (theo-433, Theographic-scale date) is the identical event as \"Jesus raises Lazarus from the dead\" (rob_lazarus_raised, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_leaving_judea", absorbed: "theo-277", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-277 \"Jesus Leaves Judea toward Galilee\" (jaccard 0.571, Matt.4.12+Mark.1.14+Luke.4.14+John.4.1-4) is the same transitional episode as rob_leaving_judea \"Why Jesus left Judea for Galilee\" (curated, `kind=\"general\"`, Robertson Section 34, JHN.4.1-4 top-level) -- rob_leaving_judea's own ref_note already explicitly discloses MAT.4.12/MRK.1.14/LUK.4.14 as this SAME event's own fragmented lead-in verses (\"the other three fragments disclosed here rather than separately witnessed\"), so theo-277's full verse set is already accounted for by rob_leaving_judea's own existing citation, not merely overlapping with it. rob_leaving_judea survives on its own established `general` kind (deliberately undated, per Batch T2's own no-fabricated-date policy -- curated's own considered opinion, THE precedence THE CHRONOLOGY AUTHORITY LAW gives it, even when that opinion is \"no specific date\"); theo-277 absorbed." },
    EventMerge { survivor: "rob_leper_healed", absorbed: "theo-286", reason: "THE CHARTER CASE (Batch CHRON-1, owner ruling: \"why are we pulling chronology from conflicting sources? we should have one absolute source of truth\"): theo-286 \"Healing the Leper\" (jaccard 0.733, Matt.8.1-4+Mark.1.40-44+Luke.5.12-14, Theographic-calibrated AD 31) and rob_leper_healed \"A leper healed; great popular excitement\" (curated, Robertson (1922) Section 45, AD 30, MAT.8.2-4 top-level + MRK.1.40-45/LUK.5.12-16 witness rows) are the identical Gospel pericope -- the exact defect the whole batch exists to fix, invisible to the former 0.8 sweep floor (leper-pair jaccard 0.733) and to the cross-book title detector (title jaccard only ~0.167). rob_leper_healed survives (the richer, Robertson-grounded, witness-complete curated container, per THE CHRONOLOGY AUTHORITY LAW's own precedence order); theo-286 absorbed. MAT.8.3 now surfaces exactly one event." },
    EventMerge { survivor: "rob_magi", absorbed: "theo-255", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-255 \"Wise Men Visit Herod\" (jaccard 0.667, Matt.2.1-8) is a clean subset of rob_magi \"The Magi visit the young child with gifts\" (curated, Robertson Section 14, MAT.2.1-12 -- the fuller citation, running through the magi's own worship of the child and departure, 2:9-12, which theo-255's own narrower \"visit Herod\" framing stops short of). No separate curated sibling claims 2:9-12. rob_magi survives; theo-255 absorbed." },
    EventMerge { survivor: "rob_man_born_blind", absorbed: "theo-426", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Blind Man Healed\" (theo-426, Theographic-scale date) is the identical event as \"Jesus heals a man born blind\" (rob_man_born_blind, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_martha_and_mary", absorbed: "theo-424", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus meets Martha and Mary\" (theo-424, Theographic-scale date) is the identical event as \"Jesus the guest of Martha and Mary\" (rob_martha_and_mary, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_message_from_baptist", absorbed: "theo-299", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus' Message on John the Baptist\" (theo-299, Theographic-scale date) is the identical event as \"John's disciples ask Jesus 'Art thou he that should come?'\" (rob_message_from_baptist, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_mission_of_seventy", absorbed: "theo-422", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"70 Sent Out\" (theo-422, Theographic-scale date) is the identical event as \"The mission of the Seventy\" (rob_mission_of_seventy, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_nazareth_rejection", absorbed: "theo-280", reason: "Theographic freebie duplicate (jaccard 0.938 verse-set overlap, same Gospel pericope): \"Jesus Rejected at Nazareth\" (theo-280, Theographic-scale date) is the identical event as \"Jesus is first rejected at Nazareth\" (rob_nazareth_rejection, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_nicodemus", absorbed: "theo-274", reason: "Theographic freebie duplicate (jaccard 0.875 verse-set overlap, same Gospel pericope): \"Jesus and Nicodemus\" (theo-274, Theographic-scale date) is the identical event as \"Nicodemus comes to Jesus by night\" (rob_nicodemus, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_olivet_discourse", absorbed: "theo-449", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Olivet Discourse\" (theo-449, Theographic-scale date) is the identical event as \"The great eschatological discourse on the Mount of Olives\" (rob_olivet_discourse, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_parallel_ministry", absorbed: "theo-275", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"John the Baptist Witnesses\" (theo-275, Theographic-scale date) is the identical event as \"Jesus and John baptize side by side in Judea\" (rob_parallel_ministry, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_paralytic_roof", absorbed: "theo-287", reason: "Theographic freebie duplicate (jaccard 0.909 verse-set overlap, same Gospel pericope): \"Healing the Man with Palsy\" (theo-287, Theographic-scale date) is the identical event as \"A paralytic lowered through the roof at Capernaum\" (rob_paralytic_roof, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_peters_mother_in_law", absorbed: "theo-284", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Peter's Mother-in-law and Others Healed\" (theo-284, Theographic-scale date) is the identical event as \"Jesus heals Peter's mother-in-law and many others\" (rob_peters_mother_in_law, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_prayer_model_again", absorbed: "theo-425", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Lord's Prayer Taught\" (theo-425, Theographic-scale date) is the identical event as \"Jesus again gives a model of prayer\" (rob_prayer_model_again, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_predicts_crucifixion_timing", absorbed: "theo-450", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Sanhedrin Conspiracy\" (theo-450, Theographic-scale date) is the identical event as \"Jesus predicts his crucifixion two days hence\" (rob_predicts_crucifixion_timing, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_presentation", absorbed: "theo-261", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Temple Presentation of Jesus\" (theo-261, Theographic-scale date) is the identical event as \"The presentation in the temple; Simeon and Anna\" (rob_presentation, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_second_galilee_tour", absorbed: "theo-302", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus' 2nd Tour of Galilee\" (theo-302, Theographic-scale date) is the identical event as \"The second tour of Galilee\" (rob_second_galilee_tour, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_sermon_on_the_mount", absorbed: "theo-296", reason: "Theographic freebie duplicate (jaccard 0.979 verse-set overlap, same Gospel pericope; owner's own live report named this pair specifically): \"Sermon on the Mount\" (theo-296, Theographic-scale date) is the identical event as \"The Sermon on the Mount\" (rob_sermon_on_the_mount, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_sinful_woman", absorbed: "theo-301", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Anointed by a Sinner Woman\" (theo-301, Theographic-scale date) is the identical event as \"A sinful woman anoints Jesus's feet in a Pharisee's house\" (rob_sinful_woman, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_slaughter_innocents", absorbed: "theo-258", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Herod slays male children\" (theo-258, Theographic-scale date) is the identical event as \"Herod slaughters the children of Bethlehem\" (rob_slaughter_innocents, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_syrophoenician_woman", absorbed: "theo-399", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing Canaanite Daughter\" (theo-399, Theographic-scale date) is the identical event as \"Jesus heals the Syrophoenician woman's daughter\" (rob_syrophoenician_woman, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_temple_tax", absorbed: "theo-409", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Tax Coin in a Fish's Mouth\" (theo-409, Theographic-scale date) is the identical event as \"Jesus pays the temple tax with a coin from a fish\" (rob_temple_tax, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_temptation", absorbed: "theo-268", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope; owner's own live report's own proven case): \"Temptations of Jesus\" (theo-268, Theographic-scale AD 26) is the identical event as \"The temptation of Jesus in the wilderness\" (rob_temptation, the AD-33-anchored, Robertson-grounded curated container, AD 29). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_third_tour_sending_twelve", absorbed: "theo-390", reason: "Theographic freebie duplicate (jaccard 0.984 verse-set overlap, same Gospel pericope): \"3rd Tour of Galilee\" (theo-390, Theographic-scale date) is the identical event as \"The third tour of Galilee; the Twelve sent out by twos\" (rob_third_tour_sending_twelve, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_through_samaria", absorbed: "theo-421", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Samaritans Reject Jesus\" (theo-421, Theographic-scale date) is the identical event as \"Jesus sets his face toward Jerusalem, through Samaria\" (rob_through_samaria, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_way_to_golgotha", absorbed: "theo-458", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-458 \"Bearing Cross to Golgotha\" (jaccard 0.778, Matt.27.32-34+Mark.15.21-23+Luke.23.26-32+John.19.17) is a clean subset of rob_way_to_golgotha \"Jesus is led away on the road to Golgotha\" (curated, Robertson Section 163, MAT.27.31-34 top-level + MRK.15.20-23/LUK.23.26-33/JHN.19.16-17 witnesses -- every one of theo-458's own verses falls fully inside rob_way_to_golgotha's own wider effective set). rob_way_to_golgotha survives; theo-458 absorbed." },
    EventMerge { survivor: "rob_widow_of_nain", absorbed: "theo-298", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Widow's Son Resurrected\" (theo-298, Theographic-scale date) is the identical event as \"Jesus raises a widow's son at Nain\" (rob_widow_of_nain, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_withered_hand", absorbed: "theo-293", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing a Withered Hand\" (theo-293, Theographic-scale date) is the identical event as \"Jesus heals a man's withered hand on the sabbath\" (rob_withered_hand, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_woes_on_cities", absorbed: "theo-300", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Woe to Chorazin and Bethsaida\" (theo-300, Theographic-scale date) is the identical event as \"Jesus denounces the unrepentant cities of Galilee\" (rob_woes_on_cities, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_zacharias_vision", absorbed: "theo-248", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"John's birth predicted\" (theo-248, Theographic-scale date) is the identical event as \"The angel Gabriel appears to Zacharias in the temple\" (rob_zacharias_vision, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "theo-145", absorbed: "theo-152", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, no curated counterpart for either side: theo-152 \"Death of Eli, Ark Captured\" (jaccard 1.000, 1Sam.4.15-18) cites EXACTLY the same four verses as theo-145 \"Judgeship of Eli\" (1Sam.4.15-18) -- a full verse-set identity, not merely an overlap, so unlike the genuine \"Lifetime of X\"/\"Birth-or-Death of X\" mega-span pattern elsewhere in this table (see the Genesis genealogy DISTINCT entries below), there is no disjoint material on either side to justify keeping two ids: this is Theographic's own citation quirk (its \"Judgeship of Eli\" event cites only the closing Ark-capture scene rather than a proper 1Sa.1-4 span), not two accounts of two different moments. Per THE CHRONOLOGY AUTHORITY LAW's layer0-layer0 convention, the more informative title (identifies the judge, continuing the reign-chain naming this atlas's own compiled OT backbone already uses) survives: theo-145; theo-152 absorbed." },
    EventMerge { survivor: "theo-209", absorbed: "theo-210", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, no curated counterpart for either side: theo-209 \"Reign of Shallum\" and theo-210 \"Death of Zachariah\" (jaccard 1.000) cite the identical single verse, 2Kgs.15.10 (\"And Shallum the son of Jabesh conspired against him [Zachariah], and smote him... and reigned in his stead\") -- one verse narrating one atomic occurrence (the regicide/accession), not two distinct moments, so (as with theo-145/theo-152 immediately above) this is not the genuine mega-span pattern. Both sides already agree on the year (-771); no placement conflict, only a redundant second id. theo-209 survives (the reign-chain-continuing title, matching this atlas's own OT regnal-succession naming convention); theo-210 absorbed." },
    EventMerge { survivor: "theo-338", absorbed: "theo-337", reason: "SAME-book duplicate by verse-set CONTAINMENT, Batch M-D1 remaining-duplicates sweep (queued at Batch W4 fix round 1, 2026-08-22, this module's own former `EVENT_DISTINCT_PAIRS` entry has the full jaccard derivation): theo-337 \"First missionary journey begins\" (ACT.12.24-13.3, 5 verses) is verse-for-verse a PREFIX of theo-338 \"First Missionary Journey\" (ACT.12.24-14.28, 79 verses) -- a short lead-in freebie and the mega-span covering that same lead-in plus the whole rest of the journey, both LAYER-0, same year AD 48, common place antioch_68 (verse jaccard only 0.063, title jaccard 0.750, caught only by the cross-book title-similarity detector, invisible to the plain verse-jaccard sweep despite full containment). theo-338 survives: the broader, verse-complete identity -- absorbing theo-337 loses ZERO coverage, since theo-337's own entire 5-verse set is already a proper subset of theo-338's own 79. theo-337 absorbed." },
    EventMerge { survivor: "theo-354", absorbed: "theo-355", reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, no curated (LAYER-1) counterpart exists for either side (this atlas's own curated Acts coverage stops at the first missionary journey's own p1_* chain; the second journey's Philippi imprisonment is not yet separately curated): theo-355 \"Philippian jailer converted\" (jaccard 0.625, ACT.16.25-34) is a clean subset of theo-354 \"Paul and Silas imprisoned\" (ACT.16.19-34, the fuller citation spanning the whole imprisonment-to-conversion arc). Per THE CHRONOLOGY AUTHORITY LAW's layer0-layer0 convention (better-witnessed/better-dated side survives), theo-354 survives (fuller coverage, fully contains theo-355's own material, no distinct dramatic content lost); theo-355 absorbed." },
];

/// Pairs a sweep's own threshold genuinely finds, but which are NOT a 1:1
/// duplicate -- listed here, with a real reason, so the sweep never
/// re-flags them (the "genuinely-distinct similar-titled events get curated
/// explicitly-distinct entries with ref_notes" half of the amendment).
/// Batch W4 fix round 1: this list is now consulted by BOTH
/// `validate::run_event_merges`'s own verse-jaccard sweep (>=0.8,
/// book-sharing LAYER-0-vs-LAYER-1) AND the new cross-book title-similarity
/// detector below (`cross_book_duplicate_candidate`) -- one shared
/// exemption registry for "same real-world event or genuinely distinct,
/// documented either way," regardless of which sweep raised the question.
pub struct EventDistinct {
    pub a: &'static str,
    pub b: &'static str,
    pub reason: &'static str,
}

pub const EVENT_DISTINCT_PAIRS: &[EventDistinct] = &[
    EventDistinct {
        a: "theo-294",
        b: "rob_multitudes_by_sea",
        reason: "Theographic mega-span (jaccard 0.813): theo-294 \"Healing Multitudes\" bundles BOTH rob_multitudes_by_sea's own MAT.12/MRK.3 healing scene AND (its own extra 3 verses, LUK.6.17-19) the lead-in to rob_sermon_on_the_mount's own Luke witness -- merging into either curated sibling would misattribute the other's own citation. Left as its own, separate, un-enriched freebie -- the same 'multi-scene mega-span, not worth a single misleading heading' disposition batch-w1-report.md already established.",
    },
    EventDistinct {
        a: "theo-394",
        b: "rob_walks_on_water",
        reason: "Theographic mega-span (jaccard 0.815): theo-394 \"Jesus Walks on Water\" bundles rob_walks_on_water's own scene PLUS its own extra 5 boundary verses (JHN.6.15, MAT.14.22-23, MRK.6.45-46), all independently claimed by the immediately-prior rob_prevents_kingship pericope. Left as its own, separate, un-enriched freebie.",
    },
    EventDistinct {
        a: "theo-412",
        b: "rob_tabernacles_feast",
        reason: "Theographic mega-span (jaccard 0.808): theo-412 \"Feast of Tabernacles\" (all of John 7) bundles rob_tabernacles_feast's own JHN.7.11-52 scene AND rob_brothers_counsel_him's own JHN.7.2-9 lead-in, plus two connective verses (JHN.7.1, JHN.7.10) neither curated sibling claims. Left as its own, separate, un-enriched freebie -- its own full-chapter coverage is not lost (still a real, standalone container), only not collapsed into either finer sibling.",
    },
    EventDistinct {
        a: "theo-420",
        b: "rob_pharisees_stone_attempt",
        reason: "Theographic mega-span (jaccard 0.813): theo-420 \"Light of the World/I am discourse\" (JHN.8.12-59) bundles BOTH rob_light_of_the_world's own JHN.8.12-20 scene (Robertson section 98) AND rob_pharisees_stone_attempt's own JHN.8.21-59 scene (Robertson section 99) -- two Robertson-numbered, separately-curated pericopes. Left as its own, separate, un-enriched freebie.",
    },
    EventDistinct {
        a: "theo-128",
        b: "exo_tabernacle_instructions",
        reason: "Real OT duplicate (jaccard 0.807), same bug class as the Gospel-era sweep, but OUTSIDE this batch's own Gospel-era scope (Amendment A: 'stop at the Gospel-era set... report the remainder honestly for a follow-up' once the sweep exceeded ~25 pairs). theo-128 'Tabernacle Built' and exo_tabernacle_instructions (Exodus 25-31, W1) are almost certainly the same real-world instructions/construction span. Deferred, not silently merged, not silently dropped -- a real follow-up item.",
    },
    EventDistinct {
        a: "theo-129",
        b: "deu_death_of_moses",
        reason: "Real OT duplicate (jaccard 0.923), same bug class, OUTSIDE Gospel-era scope -- ALSO already independently disclosed in batch-w1-report.md section 5: theo-129's own compiled `verses` field carries a genuine pre-existing anomaly (a stray GEN.34.1 entry colliding with gen_dinah_shechem), which is why W1 authored deu_death_of_moses fresh rather than enriching theo-129 directly. Merging theo-129 into deu_death_of_moses today would also import that anomaly's own within-layer collision risk -- deferred to a follow-up that fixes the GEN.34.1 anomaly first, not silently merged around it.",
    },
    // --- Batch W4 fix round 1: entries raised by the NEW cross-book
    // title-similarity detector (`cross_book_duplicate_candidate` below).
    // Two siblings that once stood here -- theo-384/pr_rome and
    // theo-337/theo-338 -- were the HOTFIX-5 "remaining duplicates" this
    // module's own doc comment names; Batch M-D1 rectified both (see
    // `EVENT_MERGE_PAIRS`'s own `pr_rome`/`theo-338` entries, each with the
    // full jaccard derivation this comment used to carry). ret_susa/
    // neh_nehemiah_hears_report is the one genuinely CONFIRMED-DISTINCT
    // survivor of that original three-pair group -- it stays here, not in
    // EVENT_MERGE_PAIRS, because the two events are two complementary
    // narrative beats of ONE moment, never two accounts of one occurrence
    // (see its own reason below).
    EventDistinct {
        a: "ret_susa",
        b: "neh_nehemiah_hears_report",
        reason: "CONFIRMED NOT a duplicate, disclosed rather than silently excluded by a higher threshold: title jaccard 0.714 (\"Nehemiah hears of Jerusalem's ruin in Susa\" / \"...and prays\", same year -445, common place 'susa') clears this module's own cross-book gate, but the two events' own verse sets are ZERO-OVERLAP and immediately adjacent within the same chapter -- ret_susa is NEH.1.1 alone (the book's own scene-setting superscription-like opening verse), neh_nehemiah_hears_report is NEH.1.2-11 (Hanani's own report and Nehemiah's own prayer that follows it) -- two genuinely sequential, complementary narrative beats of ONE continuous moment, not two independent accounts of the identical occurrence. Exactly the 'legitimate same-place-same-year neighbor' this module's own threshold is tuned not to flood on; listed here explicitly, with the verse-level evidence, rather than silently tuning the threshold just high enough to dodge it.",
    },
    // --- Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW): every pair the
    // lowered DUPLICATE_JACCARD_THRESHOLD=0.5 sweep (widened to
    // layer0-layer0 pairs) flagged that is NOT a genuine 1:1 duplicate --
    // 44 entries, two recurring shapes: (1) a Theographic freebie MEGA-SPAN
    // bundling two or more separately-curated pericopes (the theo-294/
    // theo-394/theo-412/theo-420 pattern above, generalized); (2) a
    // Genesis/regnal "Lifetime of X"/"Reign of X" SPAN container vs its own
    // "Birth of Y"/"Death of X" INSTANT sub-event -- different granularity
    // of the SAME genealogical/regnal record, not two accounts of one
    // occurrence. The complete 68-pair triage table (every flagged pair,
    // disposition, reason) lives in batch-chron1-report.md; each entry
    // below also carries its own full derivation.
    EventDistinct {
        a: "ex_rameses",
        b: "ex_succoth",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, both curated: two SEQUENTIAL Exodus waypoint events (EXO.12.37 departs Rameses; EXO.12.37+EXO.13.20 first camp at Succoth) sharing only the single transition verse (EXO.12.37, jaccard 0.5) that narrates the departure from one and the arrival at the other -- complementary sequential journey beats of the SAME wilderness-route narration, not two accounts of one occurrence (same shape as the pre-existing ret_susa/neh_nehemiah_hears_report entry above).",
    },
    EventDistinct {
        a: "pr_rhegium",
        b: "pr_puteoli",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, both curated: two sequential legs of Paul's own Rome voyage (ACT.28.13 stop at Rhegium; ACT.28.13-14 landing at Puteoli) sharing the single transition verse (ACT.28.13, jaccard 0.5) -- complementary sequential waypoints of the paul-rome-voyage narrative chain, same shape as ex_rameses/ex_succoth above, not a duplicate account of one occurrence.",
    },
    EventDistinct {
        a: "theo-102",
        b: "theo-103",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern (see this table's own header note on the pattern, repeated across the Genesis 5/11/21/25/46/50 genealogical genealogies): theo-102 \"Birth of Joseph\" (a 1-day instant, Gen.30.23-24) is the birth boundary-verse subset of theo-103 \"Lifetime of Joseph\" (a 110-year span container, Gen.30.23-24+Gen.50.22, jaccard 0.667) -- a whole-life span and a specific birth instant are different granularities of the SAME genealogical record, not two accounts of one occurrence; the span necessarily touches its own birth verse. Left as two ids, same disposition as every other Genesis genealogy pair in this table.",
    },
    EventDistinct {
        a: "theo-108",
        b: "theo-119",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-119 \"Death of Job\" (1-day instant, Job.42.16-17) is the death boundary-verse subset of theo-108 \"Lifetime of Job\" (140-year span, Gen.46.13+Job.42.16-17, jaccard 0.667) -- span vs instant, same pattern as theo-102/theo-103 above.",
    },
    EventDistinct {
        a: "theo-175",
        b: "theo-177",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, reign-span mega-span pattern (the regnal-succession sibling of the Genesis genealogy pattern): theo-177 \"Death of Tibni\" (1-day instant, 1Kgs.16.22) is the death boundary-verse subset of theo-175 \"Reign of Tibni\" (4-year span, 1Kgs.16.21-22, jaccard 0.5) -- a reign span and its own terminal death instant, same granularity distinction as the Genesis lifetime pairs.",
    },
    EventDistinct {
        a: "theo-22",
        b: "theo-26",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-26 \"Birth of Noah\" (1-day instant, Gen.5.28-29) is the birth boundary-verse subset of theo-22 \"Lifetime of Lamech\" (777-year span, Gen.5.28-31, jaccard 0.5) -- Noah's birth verse falls inside his own father Lamech's lifetime span, per Genesis 5's own genealogical table; span vs instant.",
    },
    EventDistinct {
        a: "theo-251",
        b: "rob_magnificat",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-251 \"Mary Visits Elizabeth\" (jaccard 0.611, Luke.1.39-56) is a Theographic mega-span bundling TWO separately-curated Gospel pericopes as one freebie: rob_elizabeth_greets_mary \"Elizabeth greets Mary; the babe leaps for joy\" (curated, Robertson Section 6, LUK.1.39-45, not itself flagged -- jaccard against theo-251 is only ~0.389, below the sweep floor) AND rob_magnificat \"Mary's Magnificat\" (curated, Robertson Section 7, LUK.1.46-56, the pair actually flagged here). Curated deliberately keeps these two distinct dramatic beats (Elizabeth's greeting vs. Mary's own song) as separate containers, each with its own Robertson section number; theo-251 is left un-enriched, same disposition as the pre-existing theo-294/theo-394/theo-412/theo-420 mega-span entries above.",
    },
    EventDistinct {
        a: "theo-254",
        b: "rob_shepherds",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-254 \"Birth of Jesus\" (jaccard 0.650, Luke.2.1-20) is a Theographic mega-span bundling jm_bethlehem \"Jesus is born in Bethlehem\" (curated, Robertson Section 10, LUK.2.4-7, not itself flagged -- jaccard against theo-254 is only 0.2) AND rob_shepherds \"The angels and the shepherds at Bethlehem\" (curated, Robertson Section 11, LUK.2.8-20, the pair actually flagged here). Curated keeps the birth itself and the shepherds' own visitation as two separate, Robertson-numbered containers; theo-254 is left un-enriched, same mega-span disposition as the table's other entries.",
    },
    EventDistinct {
        a: "theo-269",
        b: "rob_sanhedrin_testimony",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-269 \"John's Witness\" (jaccard 0.5, John.1.15-34) is a Theographic mega-span straddling THREE separately-curated John-1 pericopes: the tail of the prologue (JHN.1.15-18, its own curated container), rob_sanhedrin_testimony \"John's testimony before the priests and Levites\" (curated, Robertson Section 26, JHN.1.19-28, the pair actually flagged here), and the \"Behold the Lamb of God\" pericope that follows (JHN.1.29-34, its own curated container). Curated's own finer three-way split is deliberate; theo-269 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-273",
        b: "jm_temple_cleansing",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-273 \"First Passover and Temple Cleansing\" (jaccard 0.769, John.2.13-25) is a Theographic mega-span bundling jm_temple_cleansing \"Jesus cleanses the temple for the first time\" (curated, Robertson Section 31, JHN.2.13-22, the pair actually flagged here) AND the lead-in verses of the separately-curated Nicodemus pericope (JHN.2.23-25, itself part of a curated event whose own range continues into JHN.3.1-21). Curated keeps the temple cleansing and the Nicodemus material as two separate containers; theo-273 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-279",
        b: "rob_cana_nobleman",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-279 \"Jesus visits Galilee\" (jaccard 0.529, Matt.4.12+Mark.1.14-15+Luke.4.14-15+John.4.43-54) is a Theographic mega-span bundling THREE separately-curated fragments/events: rob_leaving_judea's own MAT.4.12 fragment, rob_general_summary_of_teaching's own MRK.1.14-15 summary event, rob_galilee_arrival's own JHN.4.43-45 (curated, its own container), and rob_cana_nobleman \"Jesus heals a nobleman's son at Cana\" (curated, Robertson Section 38, JHN.4.46-54, the pair actually flagged here). Curated's own finer split is deliberate; theo-279 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-295",
        b: "rob_twelve_apostles",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-295 \"Apostles Chosen\" (jaccard 0.733, Matt.10.2-4+Mark.3.13-18+Luke.6.12-16) is a Theographic mega-span conflating TWO chronologically distinct moments Robertson's own harmony keeps separate: the CHOOSING of the twelve at prayer (rob_twelve_apostles, curated, Robertson Section 53, MRK.3.13-19 top-level + LUK.6.12-16 witness, the pair actually flagged here) and the NAMING of the twelve at the later sending (Matthew's own MAT.10.2-4 apostle list, which this atlas's own curated data places inside the much-later rob_third_tour_sending_twelve event, MAT.9.35-38+MAT.10.1-42+MAT.11.1) -- Matthew narrates the apostle-list at the point of the mission charge, not the original Mark/Luke calling scene, a well-known harmony subtlety. theo-295 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-337",
        b: "p1_antioch",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0: theo-337 \"First missionary journey begins\" (jaccard 0.6, ACT.12.24-13.3) already has its own established disposition in this table's own EVENT_MERGE_PAIRS entries -- it is absorbed into theo-338 \"First Missionary Journey\" (the mega-span covering the whole journey, ACT.12.24-14.28, a pre-existing Batch M-D1 entry, verse-set CONTAINMENT) -- a prior batch's own settled call, outside THIS batch's own charter (curated<->theographic DATE conflicts) to revisit. Since `run_event_merges` runs on the PRE-merge event set, theo-337 still independently trips the newly-widened layer0-layer0 loop against p1_antioch \"Barnabas and Saul sent from Antioch\" (curated) purely because it is theo-337's own short 5-verse prefix, not because p1_antioch and theo-337 are a genuinely separate identity question: `merge_table_has_no_duplicate_absorbed_ids` forbids listing theo-337 as absorbed a second time (into p1_antioch) alongside its own existing theo-338 entry. Once theo-337 is absorbed (its established disposition), only theo-338 (the mega-span, jaccard against p1_antioch well under the sweep floor at ~0.038, never itself flagged) and p1_antioch remain in the compiled graph, each with its own placement -- no live duplicate reaches a reader.",
    },
    EventDistinct {
        a: "theo-339",
        b: "p1_paphos",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0 (one of the 4 disclosed Acts pairs, this specific pairing newly found by the widened loop): theo-339 \"Barnabas and Saul go to Cyprus\" (jaccard 0.778, ACT.13.4-12) is a Theographic mega-span bundling THREE separate curated paul-first-journey waypoints: p1_seleucia \"Down to the port of Seleucia\" (ACT.13.4), p1_salamis \"Preaching in the synagogues of Salamis\" (ACT.13.5), and p1_paphos \"Elymas blinded; the proconsul believes\" (ACT.13.6-12, the pair actually flagged here). Curated's own finer waypoint-by-waypoint split (already established, deliberately authored) is not collapsed; theo-339 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-34",
        b: "theo-45",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-45 \"Birth of Arphaxad\" (1-day instant, Gen.11.10) is the birth boundary-verse subset of theo-34 \"Lifetime of Shem\" (600-year span, Gen.11.10-11, jaccard 0.5) -- span vs instant, same pattern as theo-22/theo-26 above.",
    },
    EventDistinct {
        a: "theo-34",
        b: "theo-84",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-84 \"Death of Shem\" (1-day instant, Gen.11.11) is the death boundary-verse subset of theo-34 \"Lifetime of Shem\" (600-year span, Gen.11.10-11, jaccard 0.5) -- the SAME span container's own two boundary instants (birth in the entry above, death here) both sit inside it, exactly as expected of a lifetime span.",
    },
    EventDistinct {
        a: "theo-386",
        b: "rob_first_parable_group",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-386 \"Seaside Parables and Miracle\" (jaccard 0.791, Matt.13.1-53+Mark.4.1-41+Luke.8.4-25+Luke.9.57-62+Matt.8.18-27) is a Theographic mega-span bundling THREE separately-curated pericopes: rob_first_parable_group \"The first great group of parables\" (curated, Robertson Section 64, MAT.13.1-53 top-level + MRK.4.1-34/LUK.8.4-18 witnesses, the pair actually flagged here), the storm-stilling scene (curated, MAT.8.18+8.23-27, its own container per that event's own ref_note: \"Matthew's own reference is itself non-contiguous... written as two ranges, matching Robertson's own citation\"), and the \"would-be followers\" sayings (LUK.9.57-62, narrated much later in Robertson's own harmony). theo-386 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-388",
        b: "rob_jairus_daughter",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-388 \"Capernaum Miracles\" (jaccard 0.719, Matt.9.18-34+Mark.5.21-43+Luke.8.40-48) is a Theographic mega-span bundling rob_jairus_daughter \"Jesus raises Jairus's daughter and heals the woman who touched his garment\" (curated, Robertson Section 67, MAT.9.18-26 top-level + MRK.5.21-43/LUK.8.40-56 witnesses, the pair actually flagged here) AND the separately-curated \"Jesus heals two blind men and a mute demoniac\" event (curated, Robertson Section 68, MAT.9.27-34) that immediately follows it. theo-388 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-405",
        b: "rob_foretells_death_1",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-405 \"Foretelling the Death and Resurrection\" (jaccard 0.739, Matt.16.21-28+Mark.8.31-9.1+Luke.9.22-27) is a Theographic mega-span bundling rob_foretells_death_1 \"Jesus foretells his death and resurrection at Caesarea Philippi\" (curated, Robertson Section 83, MAT.16.21-26 top-level + MRK.8.31-37/LUK.9.22-25 witnesses, the pair actually flagged here) AND the separately-curated rob_coming_of_son_of_man \"The coming of the Son of Man in that generation\" event (curated, Robertson Section 84, MAT.16.27-28+MRK.8.38-9.1+LUK.9.26-27) that immediately follows it. theo-405 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-406",
        b: "rob_transfiguration",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-406 \"The Transfiguation\" (jaccard 0.706, Matt.17.1-13+Mark.9.2-13+Luke.9.28-36) is a Theographic mega-span bundling rob_transfiguration \"The Transfiguration\" (curated, Robertson Section 85, MAT.17.1-8 top-level + MRK.9.2-8/LUK.9.28-36 witnesses, the pair actually flagged here) AND the separately-curated descent-from-the-mountain/Elijah discussion event (curated, MAT.17.9-13) that immediately follows it. theo-406 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-428",
        b: "rob_hypocrisy_covetousness",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-428 \"Woes and Parables with Pharisees\" (jaccard 0.686, Luke.11.37-54+Luke.12.1-59+Luke.13.1-9) is a Theographic mega-span bundling THREE separately-curated Luke pericopes: the \"woes upon the Pharisees\" dinner scene (curated, LUK.11.37-54, its own container), rob_hypocrisy_covetousness \"Jesus warns against hypocrisy, covetousness, and worldly anxiety\" (curated, Robertson Section 108, LUK.12.1-59, the pair actually flagged here), and \"Repent or perish; the parable of the barren fig tree\" (curated, LUK.13.1-9, its own container). theo-428 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-440",
        b: "rob_parable_of_pounds",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-440 \"Zaccheus Converted and Parable of the Pounds\" (jaccard 0.643, Luke.19.1-28) is a Theographic mega-span bundling \"Jesus calls Zacchaeus at Jericho\" (curated, LUK.19.1-10) AND rob_parable_of_pounds \"The parable of the pounds, on the way out of Jericho\" (curated, Robertson Section 127, LUK.19.11-28, the pair actually flagged here) -- rob_parable_of_pounds's own ref_note already documents this exact split as deliberate and OWNER-CONFIRMED: \"Robertson's own Section 127 covers both the Zacchaeus encounter and the parable that follows it as ONE undivided section... the two-container split is this app's own DISPLAY choice... Owner's own live-review sweep (2026-08-21) confirmed this disposition.\" theo-440 is left un-enriched, consistent with that standing ruling.",
    },
    EventDistinct {
        a: "theo-456",
        b: "rob_before_pilate_2",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-456 \"Roman Trials\" (jaccard 0.547, Matt.27.11-31+Mark.15.1-20+Luke.23.1-25+John.18.28-19.16) is a Theographic mega-span bundling FOUR separately-curated Passion Week trial scenes: \"Jesus before Pilate the first time\" (curated, Robertson Section 159), rob_before_herod \"Jesus before Herod Antipas the tetrarch\" (curated, Robertson Section 160), rob_before_pilate_2 \"Jesus the second time before Pilate; Barabbas released\" (curated, Robertson Section 161, MAT.27.15-26 top-level + witnesses, the pair actually flagged here), and the scourging/mocking scene (curated, MAT.27.27-30). theo-456 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-459",
        b: "pw_golgotha",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-459 \"Crucifixion and Burial\" (jaccard 0.514, John.19.18-42+Matt.27.35-66+Mark.15.24-47+Luke.23.33-56) is a Theographic mega-span bundling pw_golgotha \"The crucifixion at Golgotha\" (curated, Robertson Section 164, JHN.19.16-30 top-level + witnesses, the pair actually flagged here -- that event's own module-adjacent doc already discloses it covers only \"the first three hours... does NOT reach Section 166\") AND THREE further separately-curated events: the phenomena accompanying Christ's death (MAT.27.51-56), the burial (MAT.27.57-60), and the guard at the tomb (MAT.27.61-66). theo-459 is left un-enriched.",
    },
    EventDistinct {
        a: "theo-46",
        b: "theo-47",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-47 \"Birth of Salah\" (1-day instant, Gen.11.12) is the birth boundary-verse subset of theo-46 \"Lifetime of Arphaxad\" (438-year span, Gen.11.12-13, jaccard 0.5) -- span vs instant, same pattern repeated down the Genesis 11 Shem-to-Abraham genealogy.",
    },
    EventDistinct {
        a: "theo-46",
        b: "theo-78",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-78 \"Death of Arphaxad\" (1-day instant, Gen.11.13) is the death boundary-verse subset of theo-46 \"Lifetime of Arphaxad\" (438-year span, Gen.11.12-13, jaccard 0.5) -- the same lifetime span's own death boundary, sibling to theo-46/theo-47 above.",
    },
    EventDistinct {
        a: "theo-48",
        b: "theo-49",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-49 \"Birth of Eber\" (1-day instant, Gen.11.14) is the birth boundary-verse subset of theo-48 \"Lifetime of Salah\" (433-year span, Gen.11.14-15, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-48",
        b: "theo-82",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-82 \"Death of Salah\" (1-day instant, Gen.11.15) is the death boundary-verse subset of theo-48 \"Lifetime of Salah\" (433-year span, Gen.11.14-15, jaccard 0.5), sibling to theo-48/theo-49 above.",
    },
    EventDistinct {
        a: "theo-50",
        b: "theo-51",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-51 \"Birth of Peleg\" (1-day instant, Gen.11.16) is the birth boundary-verse subset of theo-50 \"Lifetime of Eber\" (464-year span, Gen.11.16-17, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-50",
        b: "theo-89",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-89 \"Death of Eber\" (1-day instant, Gen.11.17) is the death boundary-verse subset of theo-50 \"Lifetime of Eber\" (464-year span, Gen.11.16-17, jaccard 0.5), sibling to theo-50/theo-51 above.",
    },
    EventDistinct {
        a: "theo-52",
        b: "theo-54",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-54 \"Birth of Reu\" (1-day instant, Gen.11.18) is the birth boundary-verse subset of theo-52 \"Lifetime of Peleg\" (239-year span, Gen.11.18-19, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-52",
        b: "theo-62",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-62 \"Death of Peleg\" (1-day instant, Gen.11.19) is the death boundary-verse subset of theo-52 \"Lifetime of Peleg\" (239-year span, Gen.11.18-19, jaccard 0.5), sibling to theo-52/theo-54 above.",
    },
    EventDistinct {
        a: "theo-55",
        b: "theo-56",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-56 \"Birth of Serug\" (1-day instant, Gen.11.20) is the birth boundary-verse subset of theo-55 \"Lifetime of Reu\" (239-year span, Gen.11.20-21, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-55",
        b: "theo-68",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-68 \"Death of Reu\" (1-day instant, Gen.11.21) is the death boundary-verse subset of theo-55 \"Lifetime of Reu\" (239-year span, Gen.11.20-21, jaccard 0.5), sibling to theo-55/theo-56 above.",
    },
    EventDistinct {
        a: "theo-57",
        b: "theo-58",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-58 \"Birth of Nahor\" (1-day instant, Gen.11.22) is the birth boundary-verse subset of theo-57 \"Lifetime of Serug\" (230-year span, Gen.11.22-23, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-57",
        b: "theo-69",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-69 \"Death of Serug\" (1-day instant, Gen.11.23) is the death boundary-verse subset of theo-57 \"Lifetime of Serug\" (230-year span, Gen.11.22-23, jaccard 0.5), sibling to theo-57/theo-58 above.",
    },
    EventDistinct {
        a: "theo-59",
        b: "theo-60",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-60 \"Birth of Terah\" (1-day instant, Gen.11.24) is the birth boundary-verse subset of theo-59 \"Lifetime of Nahor\" (148-year span, Gen.11.24-25, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-59",
        b: "theo-63",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-63 \"Death of Nahor\" (1-day instant, Gen.11.25) is the death boundary-verse subset of theo-59 \"Lifetime of Nahor\" (148-year span, Gen.11.24-25, jaccard 0.5), sibling to theo-59/theo-60 above.",
    },
    EventDistinct {
        a: "theo-61",
        b: "theo-65",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-65 \"Birth of Abraham\" (1-day instant, Gen.11.26-27) is the birth boundary-verse subset of theo-61 \"Lifetime of Terah\" (205-year span, Gen.11.26-28, jaccard 0.667) -- the final rung of the Genesis 11 Shem-to-Abraham genealogy chain this table's own entries walk down.",
    },
    EventDistinct {
        a: "theo-66",
        b: "theo-88",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, mega-span pattern (Abraham's own lifetime, Genesis 21/25 rather than the chapter-11 genealogy table): theo-88 \"Death of Abraham\" (1-day instant, Gen.25.7) is the death boundary-verse subset of theo-66 \"Lifetime of Abraham\" (175-year span, Gen.21.5+Gen.25.7, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-77",
        b: "theo-90",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, mega-span pattern: theo-90 \"Death of Ishmael\" (1-day instant, Gen.25.17) is the death boundary-verse subset of theo-77 \"Lifetime of Ishmael\" (137-year span, Gen.16.16+Gen.25.17, jaccard 0.5).",
    },
    EventDistinct {
        a: "theo-79",
        b: "gen_sodom_destroyed",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5): theo-79 \"Sodom Destroyed\" (jaccard 0.674, Gen.18.20-19.29) is a Theographic mega-span bundling \"Abraham pleads for Sodom\" (curated, atlas_section, GEN.18.20-33, Abraham's own intercession dialogue) AND gen_sodom_destroyed \"The destruction of Sodom and Gomorrah; Lot's escape\" (curated, atlas_section, GEN.19.1-29, the pair actually flagged here) -- two distinct narrative beats (a bargaining dialogue, then the destruction itself). gen_sodom_destroyed's own pre-existing ref_note already records the correspondence explicitly: \"Traditional (Ussher) year, 1897 BC -- matches Theographic's own theo-79 exactly\" -- the two ids' own dates were already deliberately aligned; this batch keeps them as two ids (the mega-span pattern) rather than merging, consistent with gen_abraham_pleads being its own separately-curated container.",
    },
    EventDistinct {
        a: "theo-80",
        b: "theo-81",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-80 \"Birth of Isaac\" (1-day instant, Gen.17.19+21.3) is the birth boundary-verse subset of theo-81 \"Lifetime of Isaac\" (180-year span, Gen.17.19+21.3+35.28, jaccard 0.667).",
    },
    EventDistinct {
        a: "theo-85",
        b: "theo-86",
        reason: "Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW, DUPLICATE_JACCARD_THRESHOLD lowered to 0.5), layer0-layer0, Genesis genealogy mega-span pattern: theo-85 \"Birth of Jacob and Esau\" (1-day instant, Gen.25.24-26) is the birth boundary-verse subset of theo-86 \"Lifetime of Jacob\" (180-year span, Gen.25.24-26+49.33, jaccard 0.75).",
    },
    // --- Batch CHRON-1, second finding: NOT surfaced by the pairwise sweep
    // above at all (both sides carry `atlas_section`, LAYER-1 vs LAYER-1 --
    // `run_event_merges`'s own loops only ever compare LAYER-0 against
    // LAYER-0 or LAYER-1, never LAYER-1 against LAYER-1) -- found only by
    // the NEW `run_no_two_opinions` validation (ticket 2), proving its own
    // stated purpose (a direct, placement-based check "beyond the pairwise
    // sweep") against real data on its very first real-corpus run.
    EventDistinct {
        a: "theo-113",
        b: "theo-114",
        reason: "Batch CHRON-1, found by run_no_two_opinions (not the pairwise sweep -- both sides carry atlas_section, LAYER-1 vs LAYER-1, invisible to run_event_merges' own LAYER-0-gated loops): theo-113 \"7 years of famine\" (jaccard 0.500, Gen.41.54-57+42.1-38+43.1-34, atlas_section: \"Genesis 41:54-43:34, the seven years of famine, through the brothers' own second visit to Egypt with Benjamin\") is a mega-span, its own atlas_section text explicitly describing it as bundling BOTH Joseph's brothers' first journey (all of Gen.42, theo-114 \"Joseph's brothers go to Egypt,\" the pair actually flagged here, atlas_section: \"Genesis 42, Joseph's own brothers' first journey to Egypt for grain\") AND the bulk of their second journey with Benjamin (Gen.43, shared with the separate, unenriched theo-116 \"Joseph's brothers return to Egypt\"). Two curators' own deliberate, self-documented span-vs-instant authoring; left as two ids, same disposition as every other mega-span pair in this table.",
    },
];

/// Effective verse set for jaccard/duplicate-detection purposes: `verses`
/// UNION every witness's own `translations["kjv"]` -- the SAME union
/// `AtlasData::finish()`'s own `verse_to_events` index already uses (see
/// that function's own Batch T requirement-3 comment), so "does this event
/// touch this verse" never disagrees between the two computations.
///
/// `pub(crate)` (HOTFIX-4 fix round 1, C-1): `nt_calibration` reuses this
/// EXACT union rather than re-deriving its own "which verses does this
/// event touch" logic -- "who is a Theographic NT-clock event" (that
/// module's own predicate) and "who is a duplicate" (this module's own
/// sweep) must never disagree about what counts as this event's own
/// effective verses.
pub(crate) fn effective_verses(e: &Event) -> HashSet<&str> {
    let mut set: HashSet<&str> = e.verses.iter().map(String::as_str).collect();
    for w in &e.witnesses {
        if let Some(vs) = w.translations.get(crate::translation::DEFAULT_TRANSLATION) {
            set.extend(vs.iter().map(String::as_str));
        }
    }
    set
}

/// A LAYER-0 ("freebie") event per `heading_precedence`'s own layer bit --
/// heading-worthy (if at all) only by riding a pre-existing `Event::label`,
/// never a curated container of its own. Crate-PUBLIC (not `pub(crate)`) --
/// `atlas_etl::validate::run_event_merges` (a genuinely different crate)
/// needs the identical predicate to run the same sweep against the
/// PRE-merge event set (before `AtlasData::finish()` ever runs).
pub fn is_layer0(e: &Event) -> bool {
    e.witnesses.is_empty()
        && e.robertson_section.is_none()
        && e.acts_section.is_none()
        && e.atlas_section.is_none()
        && e.kjv_superscription.is_none() // Batch W3: the KJV's own literal-citation sibling of the other three
}

/// Verse-set Jaccard overlap (intersection / union) between two events'
/// own `effective_verses` -- the SAME metric this module's own sweep
/// methodology (doc comment above) was verified against. `0.0` when either
/// side has no verses at all (never a division by zero). Crate-PUBLIC, same
/// reason as `is_layer0` above.
pub fn verse_jaccard(a: &Event, b: &Event) -> f64 {
    let (sa, sb) = (effective_verses(a), effective_verses(b));
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.len() + sb.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// THE CHRONOLOGY AUTHORITY LAW's own threshold (this module's own top doc
/// comment, part (a)): Batch CHRON-1 lowers this from the former 0.8 floor
/// to **0.5**. The former derivation (every genuine duplicate the original
/// automated sweep found sat at 0.875-1.0; the lowest genuinely-distinct
/// mega-span sat at 0.808) is superseded, not erased: it correctly
/// separated 1:1 Gospel-pericope duplicates from mega-spans, but the leper
/// pair (`rob_leper_healed`/`theo-286`, jaccard 0.733) and the wider
/// ~27-pair 0.5-0.8 band the CHRON-1 investigation found (`.superpowers/sdd/
/// 2026-08-17-bible-atlas-m1/dup-events-investigation.md`) prove 0.8 was
/// tuned to the WRONG floor -- high enough to admit real duplicates as
/// "distinct by omission." Every pair the lowered threshold newly flags is
/// hand-triaged (ticket 1) into `EVENT_MERGE_PAIRS` or
/// `EVENT_DISTINCT_PAIRS` with a written reason; `jm_jordan`/`theo-267`
/// (jaccard 0.5) is no longer a below-floor hand-added exception -- it now
/// sits exactly on the new floor, consistent with every other entry.
/// Crate-PUBLIC, same reason as `is_layer0` above.
pub const DUPLICATE_JACCARD_THRESHOLD: f64 = 0.5;

// -------------------------------------------------------------------------
// Batch W4 fix round 1 (batch-w4-review.md Critical-1's own SYSTEMIC GUARD):
// a SECOND duplicate-identity detector, orthogonal to `verse_jaccard` above.
//
// `verse_jaccard` is structurally blind to a real class of duplicate: two
// dated events narrating the SAME real-world historical occurrence via
// DISJOINT verse sets, because they cite DIFFERENT books (e.g. one witnesses
// 2 Kings, the other Jeremiah, and neither yet witnesses the other) or
// because one is a small subset embedded in the other's own much larger
// verse range (near-zero jaccard despite full containment). This exact
// shape shipped live in fresh Batch W4 data (`exl_jerusalem` vs. the
// original `jer_the_fall_of_jerusalem_retold`; `exl_mizpah` vs. the original
// `jer_jeremiah_stays_with_gedaliah`/`jer_the_assassination_of_gedaliah`,
// all three fixed this same fix round) and, independently, in older,
// pre-existing data this fix round's own sweep discovered while tuning the
// new detector against the real compiled dataset (`theo-384`/`pr_rome`,
// `theo-337`/`theo-338`) -- "the blind spot... proven three times," per the
// controller's own words. A cross-book/cross-verse-set twin is invisible to
// ANY verse-ID-based metric by construction; the only signal left is the
// event's own metadata: does it happen in the same place, in an overlapping
// year, described in near-identical words?
//
// A pair is a CANDIDATE duplicate when ALL THREE hold:
// 1. Both `kind == "event"` (dated; a general-kind passage has no `when` to
//    compare and is out of scope for a TIMELINE-node duplicate by definition).
// 2. `when.intersects` (`TimeRange::intersects`, already used elsewhere for
//    exactly this "do these two years overlap" question).
// 3. `places` share >= 1 common id (`Event::places[0]` is already this
//    app's own narrative-arrow anchor; two events set in different places
//    are not narrating the same occurrence, almost by definition here).
// AND title similarity (word-set Jaccard over normalized, stopword-stripped
// `label` tokens) is >= `TITLE_JACCARD_THRESHOLD`.
//
// THRESHOLD DERIVATION (empirical, against the real compiled `events.json`,
// the same "measure, don't guess" discipline `DUPLICATE_JACCARD_THRESHOLD`
// above was derived with): every (year-overlapping, place-sharing) dated
// pair in the whole compiled dataset was scored. The four real, confirmed
// duplicate-shaped pairs score 1.000, 1.000 (`theo-384`/`pr_rome`, an exact
// title match), 0.750 (`theo-337`/`theo-338`), and (after this fix round's
// own repair) no longer exist in duplicate form at all. The highest score
// among every pair CONFIRMED legitimately distinct is 0.714
// (`ret_susa`/`neh_nehemiah_hears_report` -- title-similar, but their own
// verse sets are zero-overlap and simply adjacent, two complementary beats
// of one scene, not two accounts of one occurrence) with a clean gap down
// to 0.667 for the next-highest legitimate neighbor. `0.70` sits inside
// that gap, catching all three real candidates (all three now disclosed in
// `EVENT_DISTINCT_PAIRS`, none newly fixed by this fix round beyond the
// three genuinely fresh W4-authored cases already reconciled directly) and
// nothing else in today's real data -- tuned to avoid flooding on
// legitimate same-place-same-year neighbors, per the controller's own
// explicit instruction, not merely set low enough to catch every known case.
// -------------------------------------------------------------------------

/// Stopwords stripped before comparing two event titles -- common English
/// function words this app's own titles are saturated with (articles,
/// prepositions, the house style's own frequent "his own"/"her own"
/// possessive filler) that would otherwise inflate the similarity score of
/// almost any two titles regardless of real content, the same reasoning
/// `TITLE_STOPWORDS`'s own absence would make `DUPLICATE_JACCARD_THRESHOLD`
/// pointless if verse ids worked the same way (they don't need this
/// treatment -- a canonical verse id has no "stopword" version).
const TITLE_STOPWORDS: &[&str] = &[
    "the", "a", "an", "of", "to", "in", "and", "at", "his", "her", "own", "is", "for", "with",
    "by", "from", "upon", "that", "this", "when", "into", "unto", "on", "as", "he", "she", "it",
    "was", "are", "be", "were", "their", "them", "who", "which", "or", "but", "not", "all",
    "out", "up",
];

/// Normalizes one event `label` into a lowercased, punctuation-stripped,
/// stopword-filtered word set -- the unit `title_jaccard` compares. Crate-
/// PUBLIC, same reason as `is_layer0`/`verse_jaccard` above (this module's
/// own unit tests, and any future ETL-side consumer, need the identical
/// normalization the fail-loud sweep itself uses, never a second
/// reimplementation that could silently drift from it).
pub fn title_words(label: &str) -> HashSet<String> {
    label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|w| !TITLE_STOPWORDS.contains(w))
        .map(|w| w.to_string())
        .collect()
}

/// Word-set Jaccard similarity between two event titles, using
/// `title_words`'s own normalization. `0.0` when either side has no
/// content words left after stopword-stripping (never a division by zero;
/// mirrors `verse_jaccard`'s own empty-set handling exactly).
pub fn title_jaccard(a: &str, b: &str) -> f64 {
    let (wa, wb) = (title_words(a), title_words(b));
    if wa.is_empty() || wb.is_empty() {
        return 0.0;
    }
    let inter = wa.intersection(&wb).count();
    let union = wa.union(&wb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// The threshold's own empirically-verified value -- this module's own doc
/// comment (above `TITLE_STOPWORDS`) has the full derivation against the
/// real compiled dataset. Crate-PUBLIC, same reason as
/// `DUPLICATE_JACCARD_THRESHOLD` above.
pub const TITLE_JACCARD_THRESHOLD: f64 = 0.70;

/// True when `a`/`b` are a CANDIDATE cross-book duplicate per this module's
/// own doc comment (dated, year-overlapping, place-sharing, title-similar
/// above `TITLE_JACCARD_THRESHOLD`) -- a candidate, not a verdict: the real
/// fail-loud decision (candidate AND not listed in `EVENT_MERGE_PAIRS` or
/// `EVENT_DISTINCT_PAIRS`) lives in `atlas_etl::validate::run_cross_book_duplicates`,
/// mirroring `verse_jaccard`'s/`DUPLICATE_JACCARD_THRESHOLD`'s own split
/// between "compute the metric" (here, pure, unit-testable) and "decide
/// what counts as unlisted" (the validator, which needs the curated
/// exemption tables this module intentionally does not import). Crate-
/// PUBLIC, same reason as `verse_jaccard` above.
pub fn cross_book_duplicate_candidate(a: &Event, b: &Event) -> bool {
    if a.kind != "event" || b.kind != "event" {
        return false;
    }
    if !a.when.intersects(&b.when) {
        return false;
    }
    let pa: HashSet<&str> = a.places.iter().map(String::as_str).collect();
    let pb: HashSet<&str> = b.places.iter().map(String::as_str).collect();
    if pa.is_disjoint(&pb) {
        return false;
    }
    title_jaccard(&a.label, &b.label) >= TITLE_JACCARD_THRESHOLD
}

/// Applies `EVENT_MERGE_PAIRS` to `events`/`narratives` in place -- called
/// once by `AtlasData::finish()`, BEFORE `events.sort_by_key` (mirrors
/// `crate::merge::apply_place_merges`'s own call position/rationale
/// exactly: every derived index built afterward -- `event_index`,
/// `verse_to_events`, `verse_heading`, the global timeline index -- must
/// see only canonical, post-merge ids). Idempotent: a pair whose `absorbed`
/// id is no longer present (e.g. `finish()` running a second time) is
/// silently skipped, matching `apply_place_merges`'s own re-run contract.
///
/// Identity-only (see this module's own doc comment for why): `survivor`'s
/// own fields are NEVER read from or written by this function. The only
/// effects are (1) `absorbed` is removed from `events` entirely, (2) any
/// `Narrative.legs` entry naming `absorbed` is repointed to `survivor`
/// (defensive -- no pair in `EVENT_MERGE_PAIRS` is a narrative leg in
/// today's curated data, verified, but a future merge pair might be).
pub fn apply_event_merges(events: &mut Vec<Event>, narratives: &mut [Narrative]) {
    for pair in EVENT_MERGE_PAIRS {
        let Some(absorbed_idx) = events.iter().position(|e| e.id == pair.absorbed) else {
            continue; // already merged (finish() re-run), or curation drifted -- no-op, not a panic
        };
        if !events.iter().any(|e| e.id == pair.survivor) {
            continue; // defensive: a curated survivor id that stopped existing -- never worth a runtime panic
        }

        events.remove(absorbed_idx);

        for n in narratives.iter_mut() {
            for leg in n.legs.iter_mut() {
                if leg == pair.absorbed {
                    *leg = pair.survivor.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::EventWitness;
    use crate::time::TimeRange;
    use std::collections::HashMap;

    fn theo_freebie(id: &str, label: &str, year: i32, verses: &[&str]) -> Event {
        Event {
            id: id.into(),
            label: label.into(),
            when: TimeRange::new(year, year).unwrap(),
            verses: verses.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    fn real_container(id: &str, label: &str, year: i32, verses: &[&str]) -> Event {
        Event {
            id: id.into(),
            label: label.into(),
            when: TimeRange::new(year, year).unwrap(),
            verses: verses.iter().map(|s| s.to_string()).collect(),
            robertson_section: Some(format!("Robertson (1922) §{id}")),
            ..Default::default()
        }
    }

    // --- EVENT_MERGE_PAIRS/EVENT_DISTINCT_PAIRS table integrity ------------

    #[test]
    fn merge_table_has_no_duplicate_absorbed_ids() {
        let mut seen = HashSet::new();
        for pair in EVENT_MERGE_PAIRS {
            assert!(seen.insert(pair.absorbed), "{} listed as absorbed more than once", pair.absorbed);
        }
    }

    #[test]
    fn merge_table_never_absorbs_a_survivor_of_another_pair() {
        let survivors: HashSet<&str> = EVENT_MERGE_PAIRS.iter().map(|p| p.survivor).collect();
        for pair in EVENT_MERGE_PAIRS {
            assert!(!survivors.contains(pair.absorbed), "{} is both a survivor and an absorbed id", pair.absorbed);
        }
    }

    #[test]
    fn the_proven_baptism_and_temptation_pairs_are_in_the_table() {
        assert!(EVENT_MERGE_PAIRS.iter().any(|p| p.survivor == "jm_jordan" && p.absorbed == "theo-267"));
        assert!(EVENT_MERGE_PAIRS.iter().any(|p| p.survivor == "rob_temptation" && p.absorbed == "theo-268"));
    }

    #[test]
    fn the_sermon_on_the_mount_pair_is_in_the_table() {
        assert!(EVENT_MERGE_PAIRS.iter().any(|p| p.survivor == "rob_sermon_on_the_mount" && p.absorbed == "theo-296"));
    }

    #[test]
    fn the_chron1_charter_leper_pair_is_in_the_table() {
        // THE CHARTER CASE (owner ruling: "why are we pulling chronology
        // from conflicting sources? we should have one absolute source of
        // truth"). MAT.8.3 must surface exactly one event once this pair
        // is applied.
        assert!(EVENT_MERGE_PAIRS.iter().any(|p| p.survivor == "rob_leper_healed" && p.absorbed == "theo-286"));
    }

    #[test]
    fn merge_table_has_exactly_92_verified_pairs() {
        // 68 pre-CHRON-1 (62 found by the automated >=0.8 jaccard sweep
        // (HOTFIX-4) + 1 added by hand (jm_jordan/theo-267, jaccard 0.5 --
        // below the sweep's own former floor, the owner's own named proof
        // case, see that entry's own reason) + 1 added by Batch W4
        // (oba_vision/theo-244, jaccard 1.0, Obadiah's own brief-sanctioned
        // single-container-book exception) + 1 added by Batch W4 fix round 1
        // (jer_jeremiah_stays_with_gedaliah/exl_mizpah, jaccard 0.000 by
        // verse-ID -- found only by the new cross-book title-similarity
        // detector, `cross_book_duplicate_candidate`, tested below) + 3
        // added by Batch M-D1 (pr_rome/theo-384, theo-338/theo-337,
        // ezr_altar_and_foundation/ret_jerusalem_altar -- the HOTFIX-5
        // "remaining duplicates" tail, see each entry's own reason)) + 24
        // added by Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW's own
        // pairwise sweep, `DUPLICATE_JACCARD_THRESHOLD` lowered 0.8 -> 0.5,
        // widened to layer0-layer0 pairs -- every one of the 68 pairs the
        // widened sweep flagged was individually triaged; 24 were genuine
        // duplicates and merged here, the other 44 are genuinely distinct
        // and documented in `EVENT_DISTINCT_PAIRS` below; the leper pair,
        // rob_leper_healed/theo-286, is the charter case). 68 + 24 = 92.
        assert_eq!(EVENT_MERGE_PAIRS.len(), 92);
    }

    #[test]
    fn the_gedaliah_mizpah_cross_book_pair_is_in_the_table() {
        assert!(EVENT_MERGE_PAIRS
            .iter()
            .any(|p| p.survivor == "jer_jeremiah_stays_with_gedaliah" && p.absorbed == "exl_mizpah"));
    }

    #[test]
    fn distinct_pairs_table_has_no_exact_duplicate_entries() {
        let mut seen = HashSet::new();
        for pair in EVENT_DISTINCT_PAIRS {
            let key = if pair.a < pair.b { (pair.a, pair.b) } else { (pair.b, pair.a) };
            assert!(seen.insert(key), "{key:?} listed more than once in EVENT_DISTINCT_PAIRS");
        }
    }

    #[test]
    fn the_new_cross_book_detectors_findings_are_documented_in_distinct_pairs() {
        // Batch W4 fix round 1: every pair the new `cross_book_duplicate_candidate`
        // sweep actually flags in today's real compiled data (per this
        // module's own threshold-derivation doc comment) must be either
        // merged (EVENT_MERGE_PAIRS, checked by the previous test) or
        // explicitly exempted here -- never silently unlisted, which is
        // exactly what `validate::run_cross_book_duplicates` fails loud on.
        // Batch M-D1: theo-384/pr_rome and theo-337/theo-338, this
        // detector's own original two findings alongside ret_susa/
        // neh_nehemiah_hears_report, are RECTIFIED now (see
        // `the_paul_arrives_at_rome_pair_is_in_the_merge_table`/
        // `the_first_missionary_journey_prefix_pair_is_in_the_merge_table`
        // below) -- removed from this expectation, since they no longer
        // exist as two separate ids for the sweep to compare at all.
        let expected = [("ret_susa", "neh_nehemiah_hears_report")];
        for (a, b) in expected {
            assert!(
                EVENT_DISTINCT_PAIRS
                    .iter()
                    .any(|p| (p.a == a && p.b == b) || (p.a == b && p.b == a)),
                "{a}/{b} must be documented in EVENT_DISTINCT_PAIRS"
            );
        }
    }

    // --- Batch M-D1: the three remaining HOTFIX-5 duplicate pairs ----------

    #[test]
    fn the_paul_arrives_at_rome_pair_is_in_the_merge_table() {
        assert!(EVENT_MERGE_PAIRS.iter().any(|p| p.survivor == "pr_rome" && p.absorbed == "theo-384"));
    }

    #[test]
    fn the_first_missionary_journey_prefix_pair_is_in_the_merge_table() {
        assert!(EVENT_MERGE_PAIRS.iter().any(|p| p.survivor == "theo-338" && p.absorbed == "theo-337"));
    }

    #[test]
    fn the_jerusalem_altar_rebuilt_pair_is_in_the_merge_table() {
        assert!(EVENT_MERGE_PAIRS
            .iter()
            .any(|p| p.survivor == "ezr_altar_and_foundation" && p.absorbed == "ret_jerusalem_altar"));
    }

    /// Batch M-D1 red-then-green (mirrors `red_then_green_baptism_pair_
    /// collapses_to_one_event_on_the_ad33_scale` above): the narrower
    /// `ret_jerusalem_altar` freebie -- deliberately built here NOT a
    /// clean subset (it carries an extra EZR.2.1, exercising the
    /// container-algebra "no coverage lost, the other verse is already
    /// covered elsewhere" reasoning this pair's own table reason names) --
    /// collapses into `ezr_altar_and_foundation`, and the `return`
    /// narrative's own leg naming the absorbed id repoints automatically.
    #[test]
    fn red_then_green_jerusalem_altar_pair_collapses_and_repoints_the_return_narrative_leg() {
        let mut events = vec![
            theo_freebie("ret_jerusalem_altar", "The altar rebuilt in Jerusalem", -537, &["EZR.2.1", "EZR.3.1", "EZR.3.2"]),
            real_container("ezr_altar_and_foundation", "The altar rebuilt; the temple's foundation laid", -536, &["EZR.3.1", "EZR.3.2", "EZR.3.3"]),
        ];
        assert_eq!(events.len(), 2, "RED: both ids exist pre-merge");
        let mut narratives = vec![Narrative {
            id: "return".into(),
            name: "The Return from Exile".into(),
            color: "#000".into(),
            legs: vec!["ret_babylon".into(), "ret_jerusalem_altar".into(), "ret_jerusalem_temple".into()],
        }];

        apply_event_merges(&mut events, &mut narratives);

        assert_eq!(events.len(), 1, "GREEN: ret_jerusalem_altar removed, ezr_altar_and_foundation alone remains");
        assert_eq!(events[0].id, "ezr_altar_and_foundation");
        assert_eq!(events[0].when.from_year, -536, "survivor's own date is untouched, never the absorbed freebie's -537");
        assert_eq!(
            narratives[0].legs,
            vec!["ret_babylon".to_string(), "ezr_altar_and_foundation".to_string(), "ret_jerusalem_temple".to_string()],
            "the return narrative's own leg list must repoint to the survivor, in place, order unchanged"
        );
    }

    // --- apply_event_merges -------------------------------------------------

    #[test]
    fn red_then_green_baptism_pair_collapses_to_one_event_on_the_ad33_scale() {
        // RED (pre-merge): both theo-267 (freebie, AD 26) and jm_jordan
        // (real, AD 29) exist as independent events -- the exact shape that
        // makes the global timeline lie (a Temptation-scale freebie sorting
        // before a Baptism-scale real container, or vice versa).
        let mut events = vec![
            theo_freebie("theo-267", "John Baptizes Jesus", 26, &["MAT.3.13", "MRK.1.9"]),
            real_container("jm_jordan", "Jesus is baptized in the Jordan", 29, &["MAT.3.13"]),
        ];
        assert_eq!(events.len(), 2, "RED: both ids exist pre-merge");
        let mut narratives: Vec<Narrative> = vec![];

        apply_event_merges(&mut events, &mut narratives);

        // GREEN: exactly one event survives, on the survivor's OWN
        // unmodified date -- never averaged, never overwritten with the
        // absorbed freebie's scale (Amendment rule B).
        assert_eq!(events.len(), 1, "GREEN: theo-267 removed, jm_jordan alone remains");
        assert_eq!(events[0].id, "jm_jordan");
        assert_eq!(events[0].when.from_year, 29, "survivor's own date is untouched, never the absorbed freebie's 26");
        assert_eq!(events[0].verses, vec!["MAT.3.13".to_string()], "survivor's own verses are untouched (identity-only merge)");
    }

    #[test]
    fn repoints_a_narrative_leg_naming_the_absorbed_id() {
        let mut events = vec![
            theo_freebie("theo-267", "John Baptizes Jesus", 26, &["MAT.3.13"]),
            real_container("jm_jordan", "Jesus is baptized in the Jordan", 29, &["MAT.3.13"]),
        ];
        let mut narratives = vec![Narrative {
            id: "n".into(),
            name: "N".into(),
            color: "#000".into(),
            legs: vec!["theo-267".into()],
        }];

        apply_event_merges(&mut events, &mut narratives);

        assert_eq!(narratives[0].legs, vec!["jm_jordan".to_string()]);
    }

    #[test]
    fn is_idempotent_when_run_twice() {
        let mut events = vec![
            theo_freebie("theo-267", "John Baptizes Jesus", 26, &["MAT.3.13"]),
            real_container("jm_jordan", "Jesus is baptized in the Jordan", 29, &["MAT.3.13"]),
        ];
        let mut narratives: Vec<Narrative> = vec![];

        apply_event_merges(&mut events, &mut narratives);
        let after_first = events.clone();
        apply_event_merges(&mut events, &mut narratives); // must no-op, not panic or double-apply

        assert_eq!(events, after_first);
    }

    #[test]
    fn a_pair_whose_survivor_is_missing_is_skipped_defensively() {
        let mut events = vec![theo_freebie("theo-267", "John Baptizes Jesus", 26, &["MAT.3.13"])];
        let mut narratives: Vec<Narrative> = vec![];

        apply_event_merges(&mut events, &mut narratives); // jm_jordan absent -- must not panic

        assert_eq!(events.len(), 1, "theo-267 stays -- no survivor to merge into");
    }

    // --- verse_jaccard / is_layer0 (the sweep's own primitives) -------------

    #[test]
    fn verse_jaccard_is_one_for_identical_sets() {
        let a = theo_freebie("a", "A", 1, &["MAT.1.1", "MAT.1.2"]);
        let b = real_container("b", "B", 1, &["MAT.1.1", "MAT.1.2"]);
        assert_eq!(verse_jaccard(&a, &b), 1.0);
    }

    #[test]
    fn verse_jaccard_is_zero_for_disjoint_sets() {
        let a = theo_freebie("a", "A", 1, &["MAT.1.1"]);
        let b = real_container("b", "B", 1, &["MRK.1.1"]);
        assert_eq!(verse_jaccard(&a, &b), 0.0);
    }

    #[test]
    fn verse_jaccard_counts_witness_verses_too_matching_verse_to_events() {
        let a = theo_freebie("a", "A", 1, &["MAT.1.1", "MAT.1.2"]);
        let mut translations = HashMap::new();
        translations.insert("kjv".to_string(), vec!["MAT.1.1".to_string(), "MAT.1.2".to_string()]);
        let b = Event {
            id: "b".into(),
            label: "B".into(),
            when: TimeRange::new(1, 1).unwrap(),
            verses: vec![],
            witnesses: vec![EventWitness { book: "MAT".into(), translations, ref_note: None, robertson_section: None }],
            robertson_section: Some("x".into()),
            ..Default::default()
        };
        assert_eq!(verse_jaccard(&a, &b), 1.0, "witness-only verses must count, same union verse_to_events uses");
    }

    #[test]
    fn is_layer0_true_only_for_a_bare_freebie() {
        assert!(is_layer0(&theo_freebie("a", "A", 1, &["MAT.1.1"])));
        assert!(!is_layer0(&real_container("b", "B", 1, &["MAT.1.1"])));
    }

    // --- title_words / title_jaccard (the new detector's own primitives,
    // Batch W4 fix round 1) -------------------------------------------------

    #[test]
    fn title_words_strips_punctuation_and_stopwords() {
        let words = title_words("The Fall of Jerusalem, Retold.");
        let expected: HashSet<String> = ["fall", "jerusalem", "retold"].iter().map(|s| s.to_string()).collect();
        assert_eq!(words, expected);
    }

    #[test]
    fn title_words_is_empty_for_an_all_stopword_title() {
        assert!(title_words("The a of to in and").is_empty());
    }

    #[test]
    fn title_jaccard_is_one_for_identical_titles() {
        assert_eq!(title_jaccard("Paul arrives at Rome", "Paul arrives at Rome"), 1.0);
    }

    #[test]
    fn title_jaccard_is_zero_for_disjoint_titles() {
        assert_eq!(title_jaccard("Water to Wine", "Feeding of Five Thousand"), 0.0);
    }

    #[test]
    fn title_jaccard_is_zero_when_only_stopwords_overlap() {
        // "The Death of the King" / "A Reign of the Queen" share only
        // stopwords ("the"/"of") after stripping -- without stopword
        // removal these would score deceptively high, the exact reason
        // `TITLE_STOPWORDS` exists (see this module's own doc comment).
        assert_eq!(title_jaccard("The Death of the King", "A Reign of the Queen"), 0.0);
    }

    #[test]
    fn title_jaccard_partial_overlap_matches_hand_computed_value() {
        // {"gedaliah","governs","remnant","mizpah"} vs
        // {"gedaliah","assassinated","mizpah"} -- intersection 2
        // (gedaliah, mizpah), union 5 -- 0.4, a hand-computed check of the
        // module's own arithmetic on a realistic partial-overlap title pair.
        let j = title_jaccard("Gedaliah governs the remnant at Mizpah", "Gedaliah is assassinated at Mizpah");
        assert!((j - 0.4).abs() < 1e-9, "expected 0.4, got {j}");
    }

    // --- cross_book_duplicate_candidate (the new detector itself, Batch W4
    // fix round 1) ------------------------------------------------------------

    fn dated_event(id: &str, label: &str, year: i32, places: &[&str]) -> Event {
        Event {
            id: id.into(),
            label: label.into(),
            when: TimeRange::new(year, year).unwrap(),
            places: places.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn red_then_green_cross_book_twin_invisible_to_verse_jaccard_but_caught_here() {
        // RED: this is exactly the shape batch-w4-review.md Critical-1 found
        // live in real curated data -- two events narrating the SAME
        // occasion via DIFFERENT books' own disjoint verse sets (2 Kings vs
        // Jeremiah, here). verse_jaccard (the OLD sweep) scores a pair like
        // this 0.0 -- and, worse, `run_event_merges`'s own layer0-vs-layer1
        // nested loop would never even compare two LAYER-1 events against
        // each other in the first place, since both sides here carry their
        // own real provenance.
        let a = Event {
            verses: vec!["2KI.25.22".into()],
            robertson_section: None,
            ..dated_event("exl_mizpah", "Gedaliah governs the remnant at Mizpah", -586, &["mizpah"])
        };
        let b = Event {
            verses: vec!["JER.40.7".into()],
            robertson_section: None,
            ..dated_event(
                "jer_jeremiah_stays_with_gedaliah",
                "Gedaliah governs the remnant at Mizpah",
                -586,
                &["mizpah"],
            )
        };
        assert_eq!(verse_jaccard(&a, &b), 0.0, "RED: the old verse-ID metric is structurally blind to a cross-book pair like this");

        // GREEN: the new detector catches it anyway, from title+year+place
        // alone -- exactly the blind spot this fix round's own SYSTEMIC
        // GUARD was written to close.
        assert!(
            cross_book_duplicate_candidate(&a, &b),
            "GREEN: caught by title/year/place even though verse_jaccard could never see it"
        );
    }

    #[test]
    fn cross_book_duplicate_candidate_false_when_years_do_not_overlap() {
        let a = dated_event("a", "Gedaliah governs the remnant at Mizpah", -586, &["mizpah"]);
        let b = dated_event("b", "Gedaliah governs the remnant at Mizpah", -400, &["mizpah"]);
        assert!(!cross_book_duplicate_candidate(&a, &b));
    }

    #[test]
    fn cross_book_duplicate_candidate_false_when_places_are_disjoint() {
        let a = dated_event("a", "Gedaliah governs the remnant at Mizpah", -586, &["mizpah"]);
        let b = dated_event("b", "Gedaliah governs the remnant at Mizpah", -586, &["rome"]);
        assert!(!cross_book_duplicate_candidate(&a, &b));
    }

    #[test]
    fn cross_book_duplicate_candidate_false_below_title_threshold() {
        // Same year, same place -- title alone must be the deciding factor,
        // and this pair (jaccard ~0.143) sits well below the threshold: a
        // "legitimate same-place-same-year neighbor" the detector must not
        // flood on, per the controller's own explicit instruction.
        let a = dated_event("a", "Gedaliah governs the remnant at Mizpah", -586, &["mizpah"]);
        let b = dated_event("b", "The people gather at Mizpah to mourn", -586, &["mizpah"]);
        assert!(!cross_book_duplicate_candidate(&a, &b));
    }

    #[test]
    fn cross_book_duplicate_candidate_false_for_general_kind_events() {
        let mut a = dated_event("a", "Paul arrives at Rome", 60, &["rome"]);
        let mut b = dated_event("b", "Paul arrives at Rome", 60, &["rome"]);
        a.kind = "general".into();
        b.kind = "general".into();
        assert!(
            !cross_book_duplicate_candidate(&a, &b),
            "a general-kind (undated in spirit) passage is out of scope for a TIMELINE-node duplicate check"
        );
    }

    #[test]
    fn cross_book_duplicate_candidate_true_for_the_confirmed_theo_384_pr_rome_shape() {
        // Not synthetic: this is the real, controller-confirmed pair this
        // module's own EVENT_DISTINCT_PAIRS entry documents (identical
        // labels, same year, common place) -- pinning that the detector's
        // own logic actually reproduces the finding the threshold was tuned
        // against, not just a made-up example.
        let a = dated_event("theo-384", "Paul arrives at Rome", 60, &["rome"]);
        let b = dated_event("pr_rome", "Paul arrives at Rome", 60, &["rome"]);
        assert!(cross_book_duplicate_candidate(&a, &b));
    }
}
