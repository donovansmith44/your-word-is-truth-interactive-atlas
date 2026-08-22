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
//! NOT swept (genuinely a different problem, disclosed rather than
//! chased): freebie-vs-freebie near-duplicates (e.g. `theo-145`/`theo-152`,
//! a Judges-era "reign of X"/"death of X" pair; `p1_pisidian_antioch`/
//! `theo-340`, an Acts-era pair) -- neither side is a real (LAYER-1)
//! container, so there is no obvious, citation-verified survivor to merge
//! INTO without first curating one of them; this module's own validator
//! only ever compares a LAYER-0 event against a LAYER-1 one, so these never
//! trip it. A real, separate future-work item, not this batch's own to fix.

use std::collections::HashSet;

use crate::data::{Event, Narrative};

/// One curated same-event pair: `absorbed` (always the bare Theographic
/// freebie) is removed entirely; `survivor` (always the real, Robertson-
/// grounded, AD-33-anchored curated container) keeps every one of its own
/// fields completely unmodified -- see this module's own doc comment for
/// why (container-algebra compliance, rule B compliance).
pub struct EventMerge {
    pub survivor: &'static str,
    pub absorbed: &'static str,
    /// Why these two are the SAME event -- for a future reader/curator, not
    /// read by any code (mirrors `crate::merge::PlaceMerge::reason` exactly).
    pub reason: &'static str,
}

/// 63 pairs, every one individually verified against the real compiled
/// `data/compiled/events.json` before being added here: 62 found by the
/// automated verse-set-Jaccard sweep (>=0.8, book-consistent,
/// label-consistent) plus 1 added by hand (`jm_jordan`/`theo-267`, jaccard
/// 0.5 -- below the sweep's own floor, but the owner's own named proof
/// case, see that entry's own reason) -- the full per-pair table lives in
/// batch-hotfix4-report.md, not duplicated here. Alphabetical by
/// `survivor` for easy scanning/diffing.
pub const EVENT_MERGE_PAIRS: &[EventMerge] = &[
    EventMerge { survivor: "jm_bethsaida", absorbed: "theo-393", reason: "Theographic freebie duplicate (jaccard 0.978 verse-set overlap, same Gospel pericope): \"Feeding of Five Thousand\" (theo-393, Theographic-scale date) is the identical event as \"Jesus feeds the five thousand near Bethsaida\" (jm_bethsaida, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "jm_cana", absorbed: "theo-271", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Water to Wine\" (theo-271, Theographic-scale date) is the identical event as \"Jesus turns water to wine at Cana\" (jm_cana, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "jm_egypt", absorbed: "theo-257", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Joseph and Mary Flee to Egypt\" (theo-257, Theographic-scale date) is the identical event as \"The holy family flees to Egypt\" (jm_egypt, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "jm_jordan", absorbed: "theo-267", reason: "Theographic freebie duplicate (jaccard 0.500 verse-set overlap -- below this module's own 0.8 sweep floor, so NOT found by the automated sweep; added by hand because the owner's own live report named this exact pair as proof the traversal lies: 'the temptation of Jesus in the wilderness... is labeled as being before Jesus' baptism'): \"John Baptizes Jesus\" (theo-267, Theographic-scale AD 26, covering MAT+MRK+LUK) is the identical event as \"Jesus is baptized in the Jordan\" (jm_jordan, the AD-33-anchored, Robertson-grounded curated container, AD 29, MAT-primary). Absorbed; theo-267's own extra Mark/Luke coverage is restored not by touching jm_jordan's own top-level verses but by giving jm_jordan real, individually-verified MAT/MRK/LUK witness rows (data/curated/event-witnesses.toml), the same shape its own very next narrative leg (rob_temptation) already has." },
    EventMerge { survivor: "pw_gethsemane", absorbed: "theo-454", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Prayer and Betrayal in Gethsemane\" (theo-454, Theographic-scale date) is the identical event as \"Jesus prays and is arrested at Gethsemane\" (pw_gethsemane, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "pw_jerusalem_entry", absorbed: "theo-444", reason: "Theographic freebie duplicate (jaccard 0.978 verse-set overlap, same Gospel pericope): \"Triumphal Entry\" (theo-444, Theographic-scale date) is the identical event as \"The triumphal entry into Jerusalem\" (pw_jerusalem_entry, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
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
    EventMerge { survivor: "rob_dedication_feast", absorbed: "theo-431", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Feast of Dedication\" (theo-431, Theographic-scale date) is the identical event as \"At the Feast of Dedication, Jesus will not yet openly claim to be the Messiah\" (rob_dedication_feast, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_defilement_controversy", absorbed: "theo-398", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Commandments and Tradition Discourse\" (theo-398, Theographic-scale date) is the identical event as \"Jesus and the Pharisees dispute over ceremonial hand-washing\" (rob_defilement_controversy, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_demoniac_boy", absorbed: "theo-407", reason: "Theographic freebie duplicate (jaccard 0.935 verse-set overlap, same Gospel pericope): \"Healing a Boy with a Demon\" (theo-407, Theographic-scale date) is the identical event as \"Jesus heals a demoniac boy the disciples could not heal\" (rob_demoniac_boy, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation gains theo-407's own one orphaned boundary verse (MAT.17.21, this pericope's own closing verse, read directly against the compiled KJV text before being added) and date stand -- otherwise -- unmodified." },
    EventMerge { survivor: "rob_eighteen_years", absorbed: "theo-265", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus Growing Up\" (theo-265, Theographic-scale date) is the identical event as \"The hidden years at Nazareth\" (rob_eighteen_years, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_first_disciples", absorbed: "theo-270", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"First Disciples Gathered\" (theo-270, Theographic-scale date) is the identical event as \"Jesus makes his first disciples\" (rob_first_disciples, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_first_galilee_tour", absorbed: "theo-285", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Jesus' 1st Tour of Galilee\" (theo-285, Theographic-scale date) is the identical event as \"The first tour of Galilee\" (rob_first_galilee_tour, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_foretells_death_2", absorbed: "theo-408", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Second Foretelling of Death and Resurrection\" (theo-408, Theographic-scale date) is the identical event as \"Jesus again foretells his death, returning through Galilee\" (rob_foretells_death_2, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_four_fishers", absorbed: "theo-282", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Fishers of Men\" (theo-282, Theographic-scale date) is the identical event as \"Jesus calls four fishermen to follow him\" (rob_four_fishers, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_gadarene_demoniac", absorbed: "theo-387", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Gadarene Demoniacs\" (theo-387, Theographic-scale date) is the identical event as \"Jesus heals the demoniac(s) in the country of the Gadarenes\" (rob_gadarene_demoniac, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_gennesaret_reception", absorbed: "theo-396", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing in Gennesaret\" (theo-396, Theographic-scale date) is the identical event as \"The reception at Gennesaret\" (rob_gennesaret_reception, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_good_samaritan", absorbed: "theo-423", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Good Samaritan Parable Taught\" (theo-423, Theographic-scale date) is the identical event as \"The parable of the Good Samaritan\" (rob_good_samaritan, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_good_shepherd", absorbed: "theo-427", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"The Good Shepherd Teaching\" (theo-427, Theographic-scale date) is the identical event as \"The parable of the Good Shepherd\" (rob_good_shepherd, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_grain_sabbath", absorbed: "theo-292", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Corn on the Sabbath\" (theo-292, Theographic-scale date) is the identical event as \"The disciples pluck grain on the sabbath\" (rob_grain_sabbath, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_jerusalem_feast_healing", absorbed: "theo-291", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Bethesda pool healing\" (theo-291, Theographic-scale date) is the identical event as \"Jesus heals a lame man at a Jerusalem feast on the sabbath\" (rob_jerusalem_feast_healing, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_joseph_annunciation", absorbed: "theo-253", reason: "Theographic freebie duplicate (jaccard 0.875 verse-set overlap, same Gospel pericope): \"An Angel Speaks to Joseph in a Dream\" (theo-253, Theographic-scale date) is the identical event as \"An angel reassures Joseph in a dream\" (rob_joseph_annunciation, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_judas_bargains", absorbed: "theo-451", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Judas Plans Betrayal\" (theo-451, Theographic-scale date) is the identical event as \"Judas bargains with the rulers to betray Jesus\" (rob_judas_bargains, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_judas_suicide", absorbed: "theo-457", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Judas' Suicide\" (theo-457, Theographic-scale date) is the identical event as \"The remorse and suicide of Judas\" (rob_judas_suicide, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_last_nazareth_visit", absorbed: "theo-389", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"2nd Rejection at Nazareth\" (theo-389, Theographic-scale date) is the identical event as \"The last visit to Nazareth\" (rob_last_nazareth_visit, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_lazarus_raised", absorbed: "theo-433", reason: "Theographic freebie duplicate (jaccard 0.957 verse-set overlap, same Gospel pericope): \"Lazarus Raised form the Dead\" (theo-433, Theographic-scale date) is the identical event as \"Jesus raises Lazarus from the dead\" (rob_lazarus_raised, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
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
    EventMerge { survivor: "rob_widow_of_nain", absorbed: "theo-298", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Widow's Son Resurrected\" (theo-298, Theographic-scale date) is the identical event as \"Jesus raises a widow's son at Nain\" (rob_widow_of_nain, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_withered_hand", absorbed: "theo-293", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Healing a Withered Hand\" (theo-293, Theographic-scale date) is the identical event as \"Jesus heals a man's withered hand on the sabbath\" (rob_withered_hand, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_woes_on_cities", absorbed: "theo-300", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"Woe to Chorazin and Bethsaida\" (theo-300, Theographic-scale date) is the identical event as \"Jesus denounces the unrepentant cities of Galilee\" (rob_woes_on_cities, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
    EventMerge { survivor: "rob_zacharias_vision", absorbed: "theo-248", reason: "Theographic freebie duplicate (jaccard 1.000 verse-set overlap, same Gospel pericope): \"John's birth predicted\" (theo-248, Theographic-scale date) is the identical event as \"The angel Gabriel appears to Zacharias in the temple\" (rob_zacharias_vision, the AD-33-anchored, Robertson-grounded curated container). Absorbed; survivor's own citation and date stand unmodified." },
];

/// Pairs the sweep's own threshold (verse-set jaccard >=0.8, book-sharing
/// LAYER-0-vs-LAYER-1) genuinely finds, but which are NOT a 1:1 duplicate --
/// listed here, with a real reason, so `validate::run_event_merges` never
/// re-flags them (the "genuinely-distinct similar-titled events get curated
/// explicitly-distinct entries with ref_notes" half of the amendment).
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

/// The sweep's own empirically-verified floor (this module's own doc
/// comment has the full derivation): every genuine duplicate the automated
/// sweep found sits at 0.875-1.0; the lowest genuinely-distinct mega-span
/// sits at 0.808. `0.8` cleanly separates the two with margin on both
/// sides (`jm_jordan`/`theo-267`, the one hand-added pair below that floor,
/// is the owner's own named exception, not a sweep-threshold case -- see
/// its own table entry). Crate-PUBLIC, same reason as `is_layer0` above.
pub const DUPLICATE_JACCARD_THRESHOLD: f64 = 0.8;

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
    fn merge_table_has_exactly_63_verified_pairs() {
        // 62 found by the automated >=0.8 jaccard sweep + 1 added by hand
        // (jm_jordan/theo-267, jaccard 0.5 -- below the sweep's own floor,
        // the owner's own named proof case, see that entry's own reason).
        assert_eq!(EVENT_MERGE_PAIRS.len(), 63);
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
}
