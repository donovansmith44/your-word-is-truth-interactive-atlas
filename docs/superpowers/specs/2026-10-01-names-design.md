# NAMES: one registry of names and entities, for text and maps (DRAFT: types for sign-off)

**Status:** DRAFT skeleton, types only, 2026-09-30. Owner sign-off pending (QUEUE `A-NAMES`, O-NAMES). No plan until signed.
**Binds:** `docs/PRINCIPLES.md` (rule 9: no comments in app code; 12: every type shown). Rulings: QUEUE `A-NAMES` (owner, 2026-09-29). Sources and licences: `.superpowers/analysis/name-sources.md`.

## 1. The four things kept apart

| Thing | Is | Lives in |
|---|---|---|
| **Entity** | what the graph is about: Person, PeopleGroup, Polity, Place, Event | existing node kinds |
| **Name** | a form of words in one language (`Israel`, `Beth-el`, `Kirjath-arba`) | new node kind `Name` |
| **Title** | a form of words that refers without naming (`the king of Assyria`, `the Preacher`, `the Son of man`) | new node kind `Title` |
| **Occurrence** | one span of words in one text where a name or title stands | new row family `Occurrence` |

An original-language word (a Hebrew or Greek token with its own reference and a Strong's number) is the anchor every translation meets at. It already exists as a row of the lexicon section's `token` table.

## 2. Identity (graph-types)

```rust
NodeKind { …existing…, Name, Title }

kind_tags! { …existing…, NameTag => Name, TitleTag => Title }
pub type NameId = NodeId<NameTag>;
pub type TitleId = NodeId<TitleTag>;

pub enum Language { English, Hebrew, Aramaic, Greek }

pub struct NamePayload {
    pub form: String,
    pub language: Language,
    pub words: NonZeroU8,
}

pub struct TitlePayload {
    pub form: String,
    pub language: Language,
    pub words: NonZeroU8,
}

pub enum Designator {
    Name(NameId),
    Title(TitleId),
}
```

`NameId` is content-addressed from `(language, form)`. One `Name` node per form per language; the same form in English and Hebrew are two names.

## 3. What a name can mean

```rust
pub enum Denotable {
    Person(PersonId),
    PeopleGroup(PeopleGroupId),
    Polity(PolityId),
    Place(PlaceId),
    Event(EventId),
}

pub struct Denotes {
    pub designator: Designator,
    pub entity: Denotable,
    pub window: Option<YearSpan>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}
```

- relation `Denotes => "denotes" / "denoted-by"` appended to `relations!`.
- `window` carries NAME-1's dated names (Luz before Bethel): a name denotes the entity only inside it.
- Entities that share a name link through the existing `NamedAfter { namesake, eponym, … }` (18 rows today). Its `Namesake` gains nothing; the owner's "Israel" chain (Jacob → the Israelites → the kingdom → the land) is `NamedAfter` rows, curated.

## 4. Original words, their numbering, and alignment

```rust
pub enum Versification { Kjv, Masoretic, TextusReceptus }

pub struct OriginalRef {
    pub text: OriginalText,
    pub versification: Versification,
    pub verse: VerseRef,
    pub ord: u16,
}

pub enum OriginalText { Masoretic, TextusReceptus }

pub struct VerseCorrespondence {
    pub from: (Versification, VerseRef),
    pub to: (Versification, VerseRef),
    pub kind: Correspondence,
    pub provenance: ProvenanceId,
}

pub enum Correspondence { Same, Renumbered, SplitFrom, MergedInto }

pub struct Alignment {
    pub translation: TranslationId,
    pub words: TextLocus,
    pub originals: NonEmpty<OriginalRef>,
    pub provenance: ProvenanceId,
}
```

- Each text keeps its own numbering; `VerseCorrespondence` rows (from STEPBible TVTMS) are the only way between numberings.
- `Alignment` rows (for the KJV, from BibleForgeDB joined to our words) are the only way from a translation's words to original words. Never positional.
- The lexicon's existing `token` rows are keyed in KJV numbering because STEPBible renumbered them upstream. `OriginalRef` records both the KJV key and the original's own reference; the renumbering becomes provenance.

## 5. Occurrences and their resolution

```rust
pub struct Occurrence {
    pub words: TextLocus,
    pub designator: Designator,
    pub resolution: Resolution,
    pub provenance: ProvenanceId,
}

pub enum Resolution {
    ThroughOriginal { original: OriginalRef },
    Direct { entity: Denotable, by: ResolvedBy },
    Unresolved(Unresolvable),
}

pub struct OriginalDenotation {
    pub original: OriginalRef,
    pub entity: Denotable,
    pub by: ResolvedBy,
}

pub enum ResolvedBy {
    SourceTag(ProvenanceId),
    Rule(ResolutionRule),
    Curated(ProvenanceId),
}

pub enum ResolutionRule { SoleDenotation, DatedName, TitleGovernsPolity }

pub enum Unresolvable {
    NoAlignment,
    SuppliedWord,
    Ambiguous(NonEmpty<Denotable>),
    NoCandidate,
    ReviewPending,
}
```

- The meaning is stored once, on the original word (`OriginalDenotation`, from TIPNR's per-token tags where they exist). An English occurrence that aligns to that word inherits it (`ThroughOriginal`); any other aligned translation does the same.
- `Direct` is only for what has no original word: italic, translator-supplied words, and titles resolved per verse (BibleData).
- `Unresolved` is listed, counted, and becomes the curation worklist. Nothing is guessed.
- Titles whose referent is theologically loaded resolve to `Unresolved(ReviewPending)` until the owner rules.

## 6. Laws

1. An `Occurrence` resolving to an entity names an entity its `designator` `Denotes` (inside the denotation's `window`, if any).
2. Every `Alignment` has provenance; no alignment is computed from word position.
3. `ThroughOriginal` resolves only if exactly one `OriginalDenotation` exists for that original word.
4. A `VerseCorrespondence` map declares whether it's total over its source numbering; lookups outside a partial map are refused by name.
5. Every `Unresolved` occurrence appears exactly once in the served worklist, counted by `Unresolvable` kind.
6. CONTRACT-2's `mentions` rows are replaced: a mention is an `Occurrence` whose designator is a `Name` and whose resolution is an entity. Its counts are carried as a regression census.

## 7. Wire (sketch; the contract section comes with the plan)

- `NodeCard` for `Name` and `Title`: the form, the language, the entities it denotes (each a `NodeRef`, with its `window`), and the occurrence count per text.
- Text windows: each `Anchor` over an occurrence gains `denotes: NodeRef` (or `unresolved: Unresolvable`).
- One cross-translation read: given a text locus, the original words it aligns to and, for each, the same original word's locus in another aligned text.

## 8. What it replaces

CONTRACT-2 Task 7a's search-name scan (`mention_spans::scan`), `event_world::kjv_aliases_of`, NAME-1's per-place dated names (they become `Denotes.window`), the 6,187 unplaced mentions (they become `Unresolved` rows with a reason).

## 9. Open for the owner (answer before the plan)

1. **Language of a name:** is the KJV's English "Israel" one `Name`, with the Hebrew and Greek forms reached only through alignment (as drafted), or is there one cross-language name node with forms per language?
2. **Events as denotations:** can a name denote an event ("Pentecost", "the Passover")? Drafted as yes.
3. **Map entities:** map-generator's registry (`map_canon::Registry`) keeps "one identity per real thing, unified by written reason". Adopt its identities as this spec's `Denotable` ids for places and polities (A-MAPS-SPEC then maps nothing), or keep atlas ids and write a correspondence table?
4. **Dated names:** is Luz/Bethel a `window` on one name's denotation (as drafted), or two names with a succession between them?
5. **Titles for review:** which titles need your ruling before they're served? A seed list (Son of man, Angel of the LORD, the Word, the Branch, Wonderful Counsellor, the Holy One of Israel) comes from BibleData.

## 10. Owner answers (2026-09-30)

1. **One Name per language.** English "Israel" and Hebrew יִשְׂרָאֵל are separate `Name` nodes; they meet only through aligned original words. §2 stands.
2. **Events are denotable** ("Pentecost", "the Passover", "the Exodus"). §3 stands.
3. **Map entities: map-generator's registry, tentatively.** Owner: "I'm gonna say map but there may be copyright permissions that make that a more difficult answer. So we may have to see." Adopt the registry's identities for places and polities IF the licensing review (O-GPL; the owner's permissive rule) clears the sources those identities rest on; otherwise atlas ids plus a correspondence table. Decided at A-MAPS-SPEC.
4. **Dated names are two names, succeeding** — and, generally, "we basically need a type that covers when the same thing has many names." So §3's `Denotes.window` is replaced by an entity's naming history as a first-class value:
   ```rust
   pub struct Naming {
       pub entity: Denotable,
       pub name: Designator,
       pub window: Option<YearSpan>,
       pub kind: NamingKind,
       pub provenance: ProvenanceId,
       pub justification: Justification,
   }
   pub enum NamingKind { Original, Renamed { from: NameId }, Also, Epithet }
   ```
   One entity has many `Naming`s: concurrent (`Also`: Jacob/Israel after Gen 32; Simon/Peter/Cephas; Esau/Edom), successive (`Renamed`: Luz → Bethel, Gen 28:19; Laish → Dan; Jebus → Jerusalem), or descriptive (`Epithet`). A rename is a fact with its verse and is explorable (a `Renamed` naming links the old name to the new). `Denotes` becomes the projection "this designator names this entity", derived from the namings, never stored twice.
5. **Titles for review: open.** The owner hasn't chosen yet ("not sure yet"). Until they do, EVERY title from BibleData is held as `Unresolved(ReviewPending)`; none is served from the source's mapping.
