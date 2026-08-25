# Corpus correspondence — cross-corpus linkage design

Date: 2026-08-24
Status: PROPOSAL, awaiting owner review (the review gate decision 6 of
batch-corp2a-brief.md names explicitly) — NO crate changes ship with this
document; implementation is a ruled follow-up once the owner blesses a
direction.
Batch: CORP-2a (the Book of Concord corpus, data half). Written per
controller decision 6, itself downstream of the owner's own framing:
"come up with a corpus correspondence so that we can maintain such things
as cross references across corpora."

## 1. Why this doc exists now

CORP-2a makes the Book of Concord the graph's SECOND corpus. The moment a
second corpus exists, a real question follows immediately: the
Confessions cite Scripture constantly — hundreds of inline citations per
document (`server/atlas-etl/src/concord.rs`'s own module doc comment: the
Apology alone runs to 1,164 numbered paragraphs, most citing at least one
passage). Decision 5 of this batch's own brief is explicit that importing
those as graph edges is OUT of THIS batch's scope — "that is exactly the
CORPUS-CORRESPONDENCE DESIGN's scope... not an ad-hoc import." This
document is that design: the GENERAL shape a cross-corpus linkage
mechanism should take, worked against two concrete cases (Bible↔Concord
citations, ready to implement the moment this doc is approved; Hebrew↔
LXX↔NT alignment, the owner's own next-corpus intent per
`corp2-scouting.md`'s "Owner: crate diffs BLESSED + LXX plan approved"
entry) — not a narrow "how do we link Concord to Bible" ticket.

## 2. What already exists (the types this design builds FROM, not around)

`graph-types/src/text.rs`'s own `TextLocus` is ALREADY corpus-spanning by
construction:

```rust
pub enum TextRef {
    Bible(VerseRef),
    Concord(ConcordRef),
    // a future corpus (Hebrew/LXX/...) adds one more variant here
}
pub struct TextLocus { pub at: TextRef, pub span: Option<TokenSpan> }
```

Every IMPORTED relation already addresses its endpoints through
`TextLocus`, not a typed, single-corpus `Locus<C>` — `Mentions.locus`,
`CrossRef.{from,to}`, `Quotes.quoting`. The corpus-crossing MACHINERY is
not missing; what's missing is a relation whose BOTH ends are free to
name different corpora with a meaning broader than the three relations
that already touch two loci:

- `Quotes { quoting: TextLocus, quoted: BibleLocusRange, .. }` —
  asymmetric BY CONSTRUCTOR, and the `quoted` end is typed to
  `BibleLocusRange` specifically ("Quotation points TOWARD the norming
  norm; the reverse direction has no constructor" — `edge.rs`'s own doc
  comment). This is intentionally narrow: it is Scripture QUOTED,
  verbatim, inside ANY corpus's own text.
- `Confesses { confessing: ConcordLocus, confessed: BibleLocusRange, .. }`
  — Concord-specific, doctrinal ACCORD (not verbatim quotation), also
  asymmetric toward Scripture (`graph.rs`'s own "TWO SENSES OF WITNESS,
  DISAMBIGUATED" doc comment: Confesses is accord, never original
  testimony).
- `Corresponds<C: Corpus> { a: Locus<C>, b: Locus<C>, .. }` — SYMMETRIC,
  but typed to ONE corpus `C` on BOTH ends ("span-level alignment between
  layers of ONE corpus family — same-type-ness enforced by the
  parameter" — `edge.rs`'s own doc comment). This is exactly right for
  what it was built for (a KJV-vs-ASV versification difference WITHIN the
  Bible corpus, e.g.) and exactly wrong for a Hebrew locus corresponding
  to a DIFFERENT corpus's Greek locus — the type parameter forbids it by
  construction.

None of the three can express "Isaiah's own Hebrew locus and the LXX's
own Greek rendering of it are the SAME underlying material, related by
translation, not quotation or doctrinal accord" — the gap this design
closes.

## 3. The proposal: widen `Corresponds`, corpus-erased, class-tagged

```rust
/// A claim that two loci -- possibly in DIFFERENT corpora -- are the
/// SAME underlying material under some named relationship. Symmetric
/// (like the existing Corresponds<C>): "a corresponds-to b" reads the
/// same either direction, matching the existing manifest label.
pub struct Corresponds {
    pub a: TextLocus,
    pub b: TextLocus,
    pub class: CorrespondenceClass,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// WHAT KIND of correspondence -- decision 6's own "provenance classes
/// for editorial-vs-textual correspondence." Not a confidence score (the
/// justification/provenance fields already carry who-asserts and why);
/// this is what SHAPE of claim it is, load-bearing for how a consumer
/// renders/trusts the row.
pub enum CorrespondenceClass {
    /// One corpus's own text AT THIS LOCUS is a translation of the
    /// other's, asserted by the text itself (the LXX genuinely IS a
    /// Greek translation of the Hebrew at Isa 7:14 -- a fact about the
    /// TEXTS, not an editor's opinion about them).
    Translation,
    /// A modern editorial/scholarly alignment between two corpora's own
    /// position SCHEMES (e.g. a Hebrew-versification-to-LXX-versification
    /// map where chapter/verse numbering diverges) -- a claim ABOUT the
    /// texts' addressing, not a claim the texts make about each other.
    /// This is where TVTMS-class, CC-BY-attributed alignment metadata
    /// lands (corp2-scouting.md's own "CC BY 4.0 is NOT public domain...
    /// attribution recorded in LICENSES.md when imported" note) --
    /// texts stay PD; an Editorial-class row's own `provenance` carries
    /// the attribution-required source.
    Editorial,
    /// One corpus's text QUOTES another's at this locus, but the
    /// quotation is not directed at Scripture specifically (the one case
    /// the EXISTING `Quotes` relation already covers, narrowly and well)
    /// -- e.g. the NT quoting the LXX'S OWN WORDING rather than
    /// retranslating from the Hebrew independently. Generalizes `Quotes`'
    /// own asymmetric shape to a same-relation-different-target case;
    /// `Quotes` itself is UNCHANGED (still the right, narrower relation
    /// for "Scripture quoted, verbatim, inside any corpus's text" --
    /// see §5).
    Quotation,
}
```

Manifest row (mirrors the existing symmetric `Corresponds<C>` entry,
`edge.rs`'s own `relations!` macro):

```
symmetric { Corresponds => "corresponds-to", .. }
```

The type-parameterized `Corresponds<C: Corpus>` this codebase already
ships is NOT deleted by this proposal — see §6's own migration note for
why both can coexist, and which one a future implementer should reach
for by default.

## 4. Worked example: the LXX almah/parthenos north star

The owner's own example (`corp2-scouting.md`'s "LXX: owner wants a
DISCUSSION" entry; "Controller recommendation on table: LXX as OWN CORPUS
... NT-quotes-LXX via existing quotes relation is the prize; OT->LXX->NT
bridge (ISA.7.14 almah/parthenos -> MAT.1.23 as the north-star example)"):

1. **Hebrew (Masoretic Text) ISA.7.14** reads `almah` ("young woman," no
   inherent virginity claim).
2. **LXX ISA.7.14** (Swete 1909 Greek OT, per `corp2-scouting.md`'s own
   source recon) renders this `parthenos` ("virgin") — a genuine
   TRANSLATION choice made by the Greek translators, centuries before
   Matthew. This is a `Corresponds { a: Hebrew-locus, b: LXX-locus, class:
   Translation }` row: two DIFFERENT corpora's own loci, the same
   underlying verse, related by translation — exactly what today's
   `Corresponds<C>` cannot express (Hebrew and LXX are different corpora
   under this codebase's own scheme) and exactly what §3's widened form
   can.
3. **Matthew 1:23** quotes the Greek `parthenos` wording directly — this
   is Scripture QUOTING a non-Bible corpus's own wording, the
   `CorrespondenceClass::Quotation` case (§3), OR arguably the existing
   `Quotes` relation depending on how "quoted toward the norming norm"
   gets read once Scripture itself is doing the quoting FROM another
   corpus rather than being quoted — a genuine open design question,
   flagged for owner review in §7, not resolved unilaterally here.

The PAYOFF (P1: "everything a query"): a reader at Isaiah 7:14 walks
`corresponds-to` once to reach the LXX's own Greek rendering, then either
`corresponds-to`/`quoted-by` again to reach Matthew's own citation of it
— "the two-hop path is the honest shape... and it is ordinary
exploration" (the SAME sentence `graph.rs`'s own Confesses doc comment
already uses for the structurally identical Concord↔Scripture case).
Nothing about almah/parthenos is asserted as a TRANSLATION CRITICISM or a
textual dispute (the KJV inerrancy directive's own "no textual criticism
anywhere" — this is a fact about TRANSLATION HISTORY across corpora, the
same register this project already uses for e.g. brain-fuel/bible's
six parallel PD editions, never a claim the KJV itself is wrong or
uncertain).

## 5. Bible↔Concord citations, under this design

The Confessions' own inline Scripture citations (module doc comment,
`concord.rs`: "Inline citation TEXT stays verbatim in the paragraphs, of
course" — this batch imports NO edges for them, decision 5) become, once
this design is implemented, a straightforward EXTRACTION pass:

- The overwhelming majority are `Quotes` rows (`quoting: TextLocus`
  already accepts a Concord locus today, no widening needed at all —
  `Quotes.quoted: BibleLocusRange` already fits "Scripture quoted inside
  the Confessions" exactly). Example: the Small Catechism's own
  institution-of-the-altar paragraph quoting 1 Corinthians 11 verbatim.
- A minority are `Confesses` rows ALREADY (the existing, Concord-specific
  relation) — doctrinal accord without verbatim quotation. Both `Quotes`
  and `Confesses` are UNCHANGED by this proposal; `Corresponds` is not a
  replacement for either, it is the relation for the cases neither one
  fits (a Concord locus that neither quotes nor doctrinally confesses a
  Bible passage, but references/aligns with it some other way — e.g. a
  citation to another CONCORD document's own earlier article, which is
  same-corpus and arguably belongs on the existing `Corresponds<ConcordTag>`
  form instead; flagged in §7).

This means Bible↔Concord citation extraction, when it lands, is MOSTLY
"populate `Quotes` and `Confesses` for real" (tables that exist today but
are always empty, `artifact.rs`'s own "SCOPE, disclosed" note) rather
than a new relation shipping alongside this design — `Corresponds`'s own
FIRST real rows are more likely to be the Hebrew↔LXX↔NT case (§4) than
Bible↔Concord, once LXX lands as corpus three.

## 6. Migration: does `Corresponds<C>` retire, or do both forms coexist?

Two honest options, NEITHER decided here (owner gate):

**Option A — retire `Corresponds<C>`, keep only the corpus-erased form.**
Simpler (one relation, one manifest row); loses the STATIC "both ends
really are the same corpus" guarantee for the common same-corpus case
(e.g. a KJV/ASV versification difference) — that guarantee moves to a
runtime law (`class == Editorial` rows whose `a`/`b` happen to share a
corpus still parse and store fine; a same-corpus assumption elsewhere
would need an explicit check instead of a compiler-enforced one).

**Option B — keep both.** `Corresponds<C>` stays for same-corpus
alignment (its own existing, narrower, type-safe job); a NEW,
differently-named symmetric relation (working name: `CrossCorpus`, or
simply widen the manifest to carry TWO `Corresponds`-family rows) handles
the cross-corpus case. Costs one more manifest row and one more BiIndex,
buys back the compile-time same-corpus guarantee where it's still wanted.

This document's own author's LEAN (disclosed, not a ruling): Option B —
the existing `Corresponds<C>` has zero real callers today (like
`contains_bible`, `quotes`, `confesses`; this batch's own artifact.rs
scope note), so retiring it costs nothing in migration pain, but keeping
the type-safe same-corpus form available for the (likely still common)
same-corpus case seems worth one extra manifest row. The owner's own call
either way is a ruled follow-up, not assumed here.

## 7. Open questions for the owner review gate

1. Does Matthew quoting the LXX's own wording belong on `Quotes`
   (Scripture is ALWAYS the `quoted` end today, never the `quoting` end —
   widening that asymmetry is itself a real design choice) or on the new
   `Corresponds{class: Quotation}` (§3/§4 point 3)?
2. Option A vs. Option B (§6) for `Corresponds<C>`'s own fate.
3. A same-corpus Concord-cites-Concord row (e.g. the Solid Declaration
   citing the Augsburg Confession by article number, a real, observed
   pattern in the vendored text) — `Corresponds<ConcordTag>` (existing,
   type-safe) or the new corpus-erased form? This document's own lean:
   the existing typed form, since same-corpus-ness is genuinely known and
   worth keeping compiler-enforced there.
4. Confidence/dispute handling: `CorrespondenceClass::Translation` rows
   are sometimes genuinely disputed among scholars (LXX-vs-MT divergences
   are the classic case) — does that dispute ride in `Justification.text`
   (prose) alone, or does this design need a fourth field the existing
   `Justification`/`Ground` shape doesn't carry? No proposal made here;
   flagged as a real question the almah/parthenos case will force an
   answer to the moment LXX ingestion is briefed.

## 8. Non-goals (this batch, this document)

- NO crate changes ship with this document (decision 6, verbatim).
- NO Bible↔Concord citation edges ship in CORP-2a (decision 5) — the
  Concord paragraphs' own inline citation TEXT stays verbatim prose;
  extraction is this design's own implementation, a ruled follow-up.
- NO LXX ingestion is proposed or scoped here — corpus three is a
  separate, future-briefed batch; this document exists so THAT batch
  does not have to invent its own cross-corpus shape from scratch, per
  the owner's own "come up with a corpus correspondence" framing.
