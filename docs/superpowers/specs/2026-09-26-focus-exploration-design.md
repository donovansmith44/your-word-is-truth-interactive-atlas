# FOCUS-1: Explorable, Frontier, Exploration — design

**Status:** SIGNED OFF 2026-09-27 (rulings R1–R9 in §10). Plans: FOCUS-0
(graph side), then FOCUS-1…9, each its own plan file.
**Governed by:** `docs/PRINCIPLES.md`.
**Depends on:** CONTRACT-1 (generated `NodeKind`, `EdgeKind`, wire records;
`atlas-contract`). **Prerequisite batch:** FOCUS-0 (§7, graph-side). The
"Year → Anchor" wording in earlier drafts is superseded by R2's "Year → the
Event the claim attests" throughout.
**Grounding:** every count and file:line below comes from the 2026-09-26
grounding pass over the live code and API.

## 1. Problem

Two designs coexist. `client/Contracts/Focus.cs` declares `IFocusComponent`,
`IFrontierAbstraction`, `ITraversal`, `IPresentation` — nothing implements
them. The live mechanism is 33 popover providers keyed by node-type
*strings* (`AppliesTo(node.Kind == "Chapter")`), 27 of them on legacy
endpoints, plus hand-authored chips in 14 Node classes (`Push` from one
node, `NavigateWorld` from eight, `NavigateReader` from four). The Reader
never reads the graph's `follows-in` edge: it rebuilds a `ChapterNode` from
the route and computes "next chapter" from the books list
(`Reader.razor:715-751`). Four client kinds have no server node (Author,
TimeAndPlace, Year, PolityDelta); four collapse onto two server kinds
(Verse + ConcordUnit → TextUnit; Chapter + Book + Passage → Container); seven
server kinds have no client Explorable at all (Narrative, Anchor, Era,
Polity, Source, Translation, PeopleGroup, LexiconEntry). The result is the
non-uniformity the owner listed on 2026-09-20: arrows on the Reader but not
the Concord, dead article headers, page numbers, content bleeding across
article boundaries (`Concord.razor:327-361`).

## 2. Decisions (owner, 2026-09-26)

| # | Decision |
|---|---|
| F1 | **You focus Explorables. An Exploration is the trail:** `Exploration = Explorable \| Explorable —EdgeKind→ Exploration`. You *view* an Exploration (the trail list, saved explorations); you never focus one. |
| F2 | **`explore(x)` returns x's one-hop links; Frontier is a set of links.** `Link = (EdgeKind, Explorable)`. Order is not a frontier property — it is how a group is laid out when rendered. Link equality is node identity `(NodeKind, id)`, never reference. |
| F3 | **`Focus = (Explorable, Frontier, Presentation, NavigationRules, EscapeHatches)`.** NavigationRules — in code, `Affordances` (§3.4): how each kind of link is offered — are per-EdgeKind defaults with per-Explorable overrides. |
| F4 | **Frontier is a graph construct and is on the wire** (`edge_summary` + edge pages — CONTRACT-1 D6). Presentation, Affordances and EscapeHatches are UX and never wire. |
| F5 | **A locus is where a link is anchored, not a node.** `Anchor = (Locus, Link)`. Hazor mentioned three times in a chapter is one Link in the frontier, anchored at three loci. |
| F6 | **Uniform presentation is one recursive rule:** `present(Container)` = its title once, then `present(child)` for each `contains` link in order; `present(TextUnit)` = its text with every Anchor inside it clickable. Every click is a Link traversal; the Locus only says where the affordance sits. |
| F7 | **Strangler with a deletion law**, one node kind at a time, graph-native kinds first (owner: "we don't want to change too many things at once"). A law test fails the build if any kind is served by both the old and the new mechanism. |
| F8 | **Back is a traversal too:** going back along a link of kind k appends a link of kind `k.Dual()`. There is no second stack; the breadcrumb is a *view* that collapses a link immediately followed by its dual. (§10 R1 if the owner prefers the current stack+trail pair.) |
| F9 | **The map is a presentation, and what it presents is a node** (owner, 2026-09-27: "the map needs to be a node… a new NodeKind that contains all the explorable elements on the map"). `NodeKind::Map` — a view of the world at a time window — whose frontier `shows` every drawable thing in it. `Presentation.Geography` is how a Map, a Place, a Polity or an Era is presented; the World view is a Focus renderer for it, as the Reader is for `Sequence`. There is no "show on the map" exit: following `located-at` *is* focusing the place, and the place's home surface is the map. `EscapeHatch` keeps only view composition (split, follow). |

## 3. Types

All in `namespace BibleAtlas.Client.Explore`. `NodeKind`, `EdgeKind`,
`NodeCard`, `NodeRef`, `EdgePage`, `TextUnit`, `TextWindow` are CONTRACT-1's
generated types; `EdgeKinds.Dual`/`IsSymmetric` are CONTRACT-1's helper.

### 3.1 The graph, as the client holds it

```csharp
public sealed record Explorable(NodeKind Kind, string Id, string Label)
{
    public static Explorable From(NodeRef node);      // (node.Kind, node.Id, node.Label)
    public static Explorable From(NodeCard card);
}
// Equality: positional record over (Kind, Id, Label) — Label is display only; two Explorables with the
// same (Kind, Id) and different Labels cannot arise from one graph version, so record equality suffices.

public sealed record Link(EdgeKind Kind, Explorable Target);

public sealed record FrontierGroup(EdgeKind Kind, int Count);   // one entry of edge_summary; Count > 0 always

public interface IFrontier
{
    Explorable Node { get; }
    IReadOnlyList<FrontierGroup> Groups { get; }                                  // declaration order of edge_summary
    Task<Page<Link>> Links(EdgeKind kind, int? cursor = null, int limit = 20);     // one page of one group, server order
}
public sealed record Page<T>(IReadOnlyList<T> Items, int? Next);

public sealed record Locus(string Ref, int? Start = null, int? End = null);   // wire ref today; CONTRACT-2 structures it
public sealed record Anchor(Locus Locus, Link Link);
```

Laws (tested): a `(Kind, Target)` pair appears at most once across all
pages of a group (`Frontier_is_a_set`); every `Groups[i].Kind` is a member
of `EdgeKind` (`Frontier_kinds_are_declared`); `Links` for a kind not in
`Groups` returns an empty page, never throws.

### 3.2 Exploring

```csharp
public interface IExplorer
{
    Task<Explorable> Resolve(NodeKind kind, string id);   // Card → Explorable; throws NotFound
    Task<IFrontier> Explore(Explorable node);             // Card(node).EdgeSummary + a pager over Edges(node, kind)
    Task<Presentation> Present(Explorable node);          // §3.3, chosen by node.Kind
}
```

One implementation, `GraphExplorer`, over `IExplorableClient`. There is no
per-kind `ExploreAsync`: the frontier of every kind is the same call.

### 3.3 Presentation — the node's own content

```csharp
public abstract record Presentation
{
    // A Container: title once, then the children of its `contains` group, presented in turn.
    public sealed record Sequence(string Title, IReadOnlyList<Link> Children) : Presentation;
    // A TextUnit: its text, with the anchors of every link whose locus falls inside it.
    public sealed record Text(Locus Locus, string Body, IReadOnlyList<Anchor> Anchors) : Presentation;
    // A Map, a Place, a Polity, an Era: the world at a window, drawn. `Shown` is the `shows` group;
    // `Center` is set when the focus is one place on that map.
    public sealed record Geography(TimeRange Window, GeoPoint? Center, IReadOnlyList<Link> Shown) : Presentation;
    // Everything else: the node's own facts, as labelled fields.
    public sealed record Card(string Title, IReadOnlyList<Field> Fields) : Presentation;
    public sealed record Field(string Name, string Value, Link? Link = null);   // a field may itself be a link (a date → its event)
}
public sealed record GeoPoint(double Lat, double Lon);
```

**Home surfaces.** A Presentation is rendered by the view that can draw it:
`Sequence`/`Text` by the reader for its corpus (Reader for the Bible, Concord
for the Book of Concord, Kretzmann for the commentary), `Geography` by the
World view, `Card` by the popover. `HomeSurface(Explorable) -> ViewName` is
one total function over `NodeKind` (and corpus, for text kinds). Following a
link whose target's home surface is not the current one *navigates there*,
carrying the Exploration; the popover is the summary presentation of any
focus on any surface. This replaces `NavigateWorld` and `NavigateReader`
outright: every chip was a link all along.

`Sequence` is where continuous scroll lives (F6): a Container renders its
children in one scroll; a child that is itself a Container renders *its*
title once and *its* children — the Book of Concord, a document, an
article, a paragraph all by the same rule. `Text` is where anchors live:
today the anchors of a verse are its `mentions`, `cites` (cross references,
catechism), `words` links, placed by the client's name scan until
CONTRACT-2 serves spans (§8).

### 3.4 Affordances — how a kind of link is offered

The frontier says *where you can go*; this says *what the button looks
like*. Genesis 1's frontier is `{follows-in: Genesis 2, contains: 31 verses,
member-of: Genesis}`: the reader shows the first as arrows, the second as
the chapter's own text, the third as the "Genesis" crumb. Today each *page*
decides that per node type (which is why the Concord has no arrows). Here
the **kind** decides, once, for every page:

```csharp
public abstract record Affordance
{
    public sealed record Arrows : Affordance;                                                   // prev / next
    public sealed record InlineChildren : Affordance;                                           // the body of a container
    public sealed record UpCrumb : Affordance;                                                  // the parent
    public sealed record SectionList(SectionStyle Style, int InitialClamp, SectionOrder Order) : Affordance;   // a listed group
}

public static class Affordances
{
    public static Affordance Of(EdgeKind kind);   // total over EdgeKind — tested
}
```

The defaults (the table the owner asked for, now code):

| EdgeKind | Affordance |
|---|---|
| `follows-in`, `precedes-in` | `Arrows` — the same arrows on a Bible chapter, a BoC article, an event in a narrative |
| `contains` | `InlineChildren` — the body of a Container (a `Sequence`) |
| `shows` | `InlineChildren` — the body of a Map (a `Geography`) |
| `member-of`, `shown-on` | `UpCrumb` |
| `located-at`, `site-of` | `SectionList(Standard, 20, Canonical)` — ordinary links; the target's home surface is the map |
| `cites` | `SectionList(Quiet, 3, VotesRanked)` — today's `EdgeSectionRegistry.Cites` |
| `mentions` | `SectionList(Standard, 50, Canonical)` |
| `mentioned-in` | `SectionList(Standard, 12, Canonical)` |
| `commented-on-by` | `SectionList(Standard, 20, Canonical)` |
| every other kind | `SectionList(Standard, 20, Canonical)` |

`EdgeSectionRegistry`'s four entries are the `SectionList` rows above; it is
deleted. `Affordances_are_total` asserts `Of` is defined for every member of
`EdgeKind`; adding a relation in Rust fails the client build until the table
says what the new kind means. Per-Explorable overrides (F3) are one
`Func<EdgeKind, Affordance>` on the `Focus` (§3.5), defaulting to
`Affordances.Of`; no override is defined in FOCUS-1 — the first real case
adds it, not before.

### 3.5 Focus and the trail

```csharp
public sealed record Focus(Explorable Node, IFrontier Frontier, Presentation Presentation,
                           Func<EdgeKind, Affordance> Affordances, IReadOnlyList<IEscapeHatch> Hatches);

// F1 written flat: the base case is Start alone; every further cell is a Link. (A "step" is a Link.)
public sealed record Exploration(Explorable Start, IReadOnlyList<Link> Links)
{
    public Explorable Current { get; }                 // Links.Count == 0 ? Start : Links[^1].Target
    public Exploration Follow(Link link);              // one hop: append the link
    public Exploration Back();                         // append new Link(Links[^1].Kind.Dual(), the node before it); no links → unchanged
    public IReadOnlyList<Link> Breadcrumb { get; }     // Links with each (k, k.Dual()) pair collapsed — the popover's trail
}
```

State: `StateAtom<T>` requires a non-null `T`, so the atom holds
```csharp
public abstract record ExplorationState
{
    public sealed record Closed : ExplorationState;
    public sealed record Open(Exploration Exploration) : ExplorationState;
}
```
(`AtomNames.Exploration = "exploration"`), replacing `StateAtom<FocusStack>`;
intents `Open(Explorable)`, `Follow(Link)`, `Back()`, `Reset()`,
`Reseed(Exploration)` replace `FocusStack`'s. The ownership/claim mechanics
in `ExplorerPopover` are kept as they are (they are about which popover owns
the atom, not about the type). The popover does not know its host view
today; it gains a `HostView` parameter so `Focus.Hatches` can be
`ViewRegistry.Get(HostView).EscapeHatches`.

**Selection** uses the same identity: `StateAtom<IReadOnlyList<Explorable>>`
(`selection-v2`, translated from `selection-v1` by the same rule as saves).

**Strangler bridge (FOCUS-1 only, dismantled by FOCUS-2…9):** every existing
`IExplorable` gains `Explorable Identity { get; }`; `LegacyNodes.For(Explorable)`
is today's `Reconstruct` switch keyed by `(NodeKind, Id)`; a legacy section
provider's `PushAsync(node)` becomes `PushAsync(node, via: EdgeKind)` so that
even the old path produces true Links. Identities of the client-only kinds
follow R2: `AuthorNode` → its Book (until the `authored-by` link replaces
the node), `TimeAndPlaceNode` → its Event, `YearNode` → the Event its claim
attests (FOCUS-0 §7 item 4), `PolityDeltaNode` → its Polity (until MAPS).

### 3.6 Persistence

```csharp
public sealed record SavedExploration(Guid Id, string Name, DateTimeOffset CreatedUtc, SavedNode Start, IReadOnlyList<SavedLink> Links);
public sealed record SavedNode(NodeKind Kind, string Id, string Label);
public sealed record SavedLink(EdgeKind Kind, SavedNode Target);
```

Stored under localStorage key `explorations-v2`. `explorations-v1`
(`SavedExploration{Nodes:[{Kind,Key,Title,IsGeneralKind}]}`) is read once
and translated (§10 R4): Verse → `TextUnit` `text-unit:{Key}`; ConcordUnit →
`TextUnit` `text-unit:{Key}`; Chapter → `Container` `Container:bible-chapter-{BOOK}-{n}`;
Book → `Container:bible-book-{BOOK}`; Place, Person, Event, CatechismItem,
CommentaryItem → same id; Passage → the passage container's id (looked up by
sref); Author, TimeAndPlace, Year, PolityDelta → dropped, counted, and
reported once ("3 steps from an older version could not be restored").
Reopen = `Reseed(Exploration)`; `ExplorationDescriptor.Reconstruct`'s
14-way switch is gone — a step is resolved by `IExplorer.Resolve(Kind, Id)`.

### 3.7 Escape hatches

Unchanged in shape (`IEscapeHatch { Kind; Invoke() }`, `ViewRegistry`); the
seven registrations stay and mean only view composition (enter a split,
toggle follow). `NavigateWorld` and `NavigateReader` stop existing: a
"show on the map" chip was a `located-at` link whose target's home surface
is the World view; "read in context" was a `member-of` link whose target's
home surface is the reader (§3.3, home surfaces). `MapFocusHatch` (a
`/world` query-string builder) is deleted with them.

## 4. Where the Explorable comes from — one factory

```csharp
public static class Explorables
{
    public static Explorable From(NodeRef node);                      // generated wire type → Explorable
    public static Task<Presentation> Present(Explorable node, IExplorableClient graph);   // switch on node.Kind, total
}
```

`Present` is the only place that switches on `NodeKind`, and it is total
(`Present_is_total_over_NodeKind`). Per kind:

| NodeKind | Presentation | Source today → source after |
|---|---|---|
| Container | `Sequence(title, contains links)` | route + `/api/chapter` → `Card` + `Reading` |
| TextUnit | `Text(locus, body, anchors)` | `/api/verse`, `/api/chapter` → `Reading` window (`TextUnit` carries `edge_summary` per unit) |
| Person | `Card(life fields)` | `Card.Person` (already graph) |
| Event, Place, CatechismItem, CommentaryItem | `Card(fields)` | legacy detail endpoints → `Card` + frontier; fields that only the legacy endpoint carries are listed in §8 for CONTRACT-2 |
| Narrative, Anchor, Era, Polity, Source, Translation, PeopleGroup, LexiconEntry | `Card(label, provenance)` + frontier | nothing today → explorable by construction (§10 R3); `Anchor` is what a place's dates become (R2) |

The four client-only kinds resolve to existing nodes (§10 R2): Author is a
`Person` reached by `authored-by`, TimeAndPlace is the `Event`, Year is an
`Anchor` reached by `dated-by`, PolityDelta becomes a node with MAPS.

## 5. The migration order (FOCUS-2 … FOCUS-9), each a shippable batch

Each batch: implement the kind on the new path, delete its providers, its
Node class, its chips and the `AtlasClient` methods only it used; retire the
legacy route in `atlas-contract` (a visible `openapi.yaml` diff); Playwright
green with the preserved test ids (§6); the deletion law
(`No_kind_is_served_by_both_mechanisms`: for every migrated kind, no
`AppliesTo` string and no `*Node` class remains) green.

| Batch | Kind(s) | Replaces | Deletes |
|---|---|---|---|
| FOCUS-2 | TextUnit (Verse, ConcordUnit) | `VerseTextSectionProvider`, `CrossRefsSection`, `CatechismSeamSection`, `VerseParallelsSection`, `VerseEventMembershipSection`, `VersePassageMembershipSection`, `VersePersonsSection`, `ConcordUnitTextSection`, `ConcordSmallCatechismSection` | `VerseNode`, `ConcordUnitNode`, `/api/verse`, `/api/xrefs`, `/api/catechism/{sref}` |
| FOCUS-3 | Container (Chapter, Book, Passage, BoC document/article) | `ChapterCardSection`; Reader/Concord page bodies become `Sequence` | `ChapterNode`, `BookNode`, `PassageNode`, `ComputeAdjacent`, `_toc`, Concord `LoadWindowAsync` paging, `/api/chapter`, `/api/books` (nav use) |
| FOCUS-4 | Person | `PersonLifeSection`, `PersonEventsSection`, `PersonFamilySection`, `PersonCardAndMentionsSection` | `PersonNode` |
| FOCUS-5 | Event | six `Event*Section`s | `EventNode`, `/api/event`, `/api/narrative/event/{id}` (`ArrowNav` reads event `follows-in` instead) |
| FOCUS-6 | Place, Map, Polity, Era — the World view becomes the `Geography` renderer | four `Place*Section`s; `World.razor`'s own place/polity card plumbing | `PlaceNode`, `/api/place`; `/api/scene` once a Map's `shows` frontier feeds the renderer (MAPS may retire it instead) |
| FOCUS-7 | CatechismItem, CommentaryItem | five `Catechism*Section`s, `CatechismInConcordSection`, `CommentaryItemProseSection` | `CatechismNode`, `CommentaryItemNode`, `/api/catechism/item/{id}` |
| FOCUS-8 | the seven new kinds | — (FOCUS-1 already presents every kind without legacy providers as the generic `Card` + frontier, so these seven are explorable from FOCUS-1 on) | FOCUS-8 verifies each with a Playwright spec and deletes the legacy fallback path in the popover |
| FOCUS-9 | Author → `authored-by` link to a Person; TimeAndPlace → the Event; Year → `dated-by` link to an Anchor (R2) | `YearFrontierSection`, `PlaceDatesSection` (→ `dated-by` links) | `AuthorNode`, `TimeAndPlaceNode`, `YearNode`; the v1 save reader; `PopoverSectionRegistry` once only `PolityDelta*` remain (those three sections and `PolityDeltaNode` leave with MAPS) |

The 2026-09-20 notes land as FOCUS-3's acceptance criteria: arrows on the
Concord (`follows-in` via FOCUS-0), one cover title per container, no page
numbers, continuous scroll, no article bleed (the paging code is deleted,
and a Playwright spec asserts an article's `Sequence` contains exactly its
own paragraphs).

## 6. Test ids that survive (Presentation constants)

`reader-next`/`reader-prev` (66 refs), `popover-section-*` (189),
`popover-chip-map`/`-book`/`-context` (25) — rendered by the `MapHatch`,
`UpCrumb` and reader affordances respectively — `event-*` (509),
`place-*` (256), `catechism*` (158), `xrefs*` (131), `concord-*` (117),
`person-*` (58), `verse-text` (52), `contents-*` (22), `chapter-card` (13),
`exploration*` (63). The popover root stays `popover`. A batch is not done
while any of its ids' specs are red.

## 7. FOCUS-0 — the graph side (prerequisite, `atlas-etl` + `atlas-graph`)

Confirmed live: the Book of Concord has **no** `follows-in`/`precedes-in`
edges (no `canon_succession` table in the concord section; `edge_index`
holds only `contains`), and its ten document roots have no `member-of`.
Uniform arrows and a single cover title need:

1. Corpus root containers for **both** corpora — `Container:concord` ("The
   Book of Concord") ⊃ the ten documents, and `Container:bible` ("The Holy
   Bible") ⊃ the 66 books (neither corpus has a root today; `bible-book-GEN`
   has no `member-of`). `/api/contents/{corpus}` discovers roots by id
   prefix today (`contents.rs:139-146`); it changes to "the corpus root's
   `contains` children", so its output is unchanged.
2. `CanonSuccession` rows for Concord: document → next document (in
   `DOCUMENTS` order), article → next article within a document — the same
   pairwise shape as the KJV's. `CanonSuccession` belongs only to the KJV
   section today (`sections.rs:266-272`); the family is added to the
   Concord section with an id-prefix split (`bible-` → Kjv, `concord-` →
   Concord) in the style of `section_of_contains_bible`, the Concord
   section gets the `canon_succession` DDL, and `SECTION_SCHEMA_VERSION`
   goes 14 → 15. The forest law (`law_check::container_containment_is_a_forest`)
   already covers both corpora.
3. AGC pins for the new edges and AQC scenarios for the Concord frontier;
   the pact re-blessed (`ATLAS_BLESS_PACT=1`), AGC `VERSION` minor.
4. (R2) `AuthoredBy => "authored-by" / "authored"` appended **last** among
   the directed relations (`edge_index.rel` is positional), with a row
   family `Authored { book: ContainerNodeId, person: PersonId, provenance, justification }`
   in the Core section, following the D5 precedent for `ParentOf`. Source:
   `data/curated/books.toml` gains `author_ids` — the 14 authors that
   resolve to exactly one `Person` label today (Moses, Paul, Luke, Matthew,
   Isaiah, Ezekiel, Habakkuk, Haggai, Hosea, Jonah, Jude, Malachi, Nahum,
   Solomon) are filled in this batch; the 11 ambiguous and 14 unresolved
   ones (James, John, Mark, Peter, "David and others", "(traditional)"…)
   stay free text until curated, and no edge is emitted for them. Place date
   claims in `place-history.toml` gain `event_id` per R2 (existing events;
   `nineveh`/`samaria_1022` established are already noted as theo-87 /
   theo-176).
5. (R7) Article titles are corrected through a curated override
   (`data/curated/concord-titles.toml`, keyed by document and article),
   applied in the ETL — never by editing the vendored raw HTML. Scheme: the
   six chief parts of the Small Catechism numbered I–VI (Commandments,
   Creed, Lord's Prayer, Baptism, Confession, Sacrament of the Altar); the
   preface and the appendices (Daily Prayers, Table of Duties, Christian
   Questions) unnumbered. The client renders what is served.
6. (F9/R10) `NodeKind::Map` in `kind_tags!` (ordinal appended in
   `partition.rs`), `Shows` appended last in `relations!`, the `Shown` row
   family in the Core section, one `Map:era-{id}` node per Era with `shows`
   rows for every place, event, polity and narrative the scene computation
   returns for the era's window, and `Map —follows-in→ Map` in era order
   (`CanonSuccession` between maps, so the slider's "next era" is a link).

Paragraph order stays `contains` order (the `Sequence`), not succession:
succession is between siblings a reader steps *between*, containment is
what a reader scrolls *through*. FOCUS-0 bumps AGC minor and, where a new
node kind's rows change `section_schema_version`, the section schema; it
runs the standing block like any graph batch.

## 8. Wire gaps this exposes (input to CONTRACT-2)

Recorded here so CONTRACT-2's spec starts from FOCUS's demand, not a guess:

- Mention spans: `mentions.locus_start/end` are NULL; the client scans names
  (`MentionScan`, `PlaceMentions.FindAll`). CONTRACT-2 computes spans in ETL
  and serves them on the text window's per-unit anchors.
- Structured `Locus` on the wire (`corpus, book, chapter, verse, layer?, start?, end?`)
  replacing `CanonRef` parsing (five providers).
- Display years as labels; versification/boundary answers from
  `Reading`/`Contents` (`EventDateAndPlacesSection` uses `Versification`).
- Fields served only by legacy detail endpoints that the `Card` must carry
  once those routes retire (enumerated per kind in each FOCUS batch's plan
  from the legacy `*Detail` struct minus what `NodeCard` already has).
- Event chronology adjacency (`/api/narrative/event/{id}`) is server-computed
  today; it is `follows-in`/`precedes-in` on events and should be served as
  such.

## 9. What dies

| Dies | Batch | Proof |
|---|---|---|
| `Contracts/Focus.cs`: `IFocusComponent`, `IFrontierAbstraction`, `ITraversal`, `IPresentation`, the old `Focus(Descriptor, Node)` | FOCUS-1 | compiler; nothing referenced them |
| `Exploration(Label, ChipTestId, Target)`, `ExplorationTarget` | renamed `Chip` / `ChipTarget` in FOCUS-1 (the name `Exploration` is the trail's); deleted with the last chip author in FOCUS-9 | compiler |
| `FocusStack` and its intents | FOCUS-1 | compiler; `ExplorerPopover` retargeted; `ExplorationTests` replace `FocusStackTests` |
| `ExplorationDescriptor.Capture` | FOCUS-1 — `IExplorable.Identity` | compiler |
| `ExplorationDescriptor.Reconstruct` | renamed `LegacyNodes.For(Explorable)` in FOCUS-1; shrinks per batch; deleted in FOCUS-9 | deletion law |
| the v1 save/selection translation | FOCUS-9 | `SavedExplorationsTests` |
| `EdgeSectionRegistry` | FOCUS-1 | absorbed into `Affordances.Of` (§3.4) |
| `IExplorable.ExploreAsync`, `BodyAsync` (three of them already dead fallbacks) | FOCUS-2…9, per kind | deletion law |
| 33 providers, `PopoverSectionRegistry`, `IPopoverSectionProvider` | FOCUS-2…9 | deletion law; empty registry deleted in FOCUS-9 |
| 14 `*Node` classes | FOCUS-2…9 | deletion law |
| `MapFocusHatch`, `ChipTarget.NavigateWorld`, `ChipTarget.NavigateReader` | FOCUS-1 (home surfaces) | compiler; the `popover-chip-map` / `-context` ids now sit on links |
| `Reader.ComputeAdjacent`, `_toc`, `Concord.LoadWindowAsync` paging and trim | FOCUS-3 | `Concord` article spec; `reader-next` specs |
| legacy routes: `/api/verse`, `/api/xrefs`, `/api/catechism/{sref}`, `/api/chapter`, `/api/event`, `/api/narrative/event/{id}`, `/api/place`, `/api/catechism/item/{id}` and their handlers (≈1,400 lines) and `AtlasClient` methods | FOCUS-2…7 | `openapi.yaml` diff per batch; `contract_coverage.rs` |
| `client.Tests/ConformanceTests` entries naming deleted files | each batch | the scans |

## 10. Rulings (owner, 2026-09-27)

- **R1 — Back is a dual traversal (F8).** Ruled: yes.
- **R2 — the four client-only kinds are graph nodes.** Ruled: "they should
  be graph nodes." Applied:
  - **Author** → the `Person` node, reached from the Book by a new relation
    `AuthoredBy => "authored-by" / "authored"` in `relations!` (one row;
    `DECLARED_DIRECTED_RELATIONS` becomes 21 and `DECLARED_EDGE_KINDS` 48
    when FOCUS-0 lands — the reason those are named constants). Source data:
    the curated book metadata the legacy `BookMeta.author` already carries.
  - **TimeAndPlace** → the `Event` node it always was; an event at a place
    is the event, reached from the place by `site-of`. The synthesized kind
    is deleted; no graph change.
  - **Year** → the **Event** the claim attests. The grounding found only
    seven curated date claims over four places, `dated-by` rows are
    event-keyed by construction (`DatedBy { event, placement, … }`), and a
    founding or a destruction *is* an event (the curated file already notes
    "aligned to theo-87", "matches theo-176"). Each claim gains an
    `event_id` — the existing event attested by the claim's verses, when
    there is one — and is reached from the place by `site-of`; a claim with
    no event stays a dated field on the place's card, not an explorable.
    No new node kind, no new row family.
  - **PolityDelta** → a graph node when the map's `ChangeEvent` arrives in
    this graph (MAPS owns that data); `PolityDeltaNode` is retired then, in
    the MAPS batch, not by FOCUS.
  The first three are FOCUS-0 work (§7).
- **R3 — the seven server kinds** get the generic `Card` and their frontier
  in FOCUS-8. Ruled: yes.
- **R4 — old saves:** translate what maps, drop what doesn't, report the
  count once; the v1 reader is deleted in FOCUS-9. Ruled: yes.
- **R5 — FOCUS-0 scope** as §7. Ruled: yes.
- **R6 — batch order** as §5. Ruled: yes.
- **R7 — the inconsistent BoC titles** are fixed in the curated
  source/ETL in FOCUS-0; the client renders titles as served. Ruled: "yes
  fix."
- **R8 — remembered reading position** (from the 2026-09-20 notes): one
  piece of Presentation state, `ReadingPosition(Explorable Container, Locus Locus)`,
  per container in local storage, written on scroll and read when the
  container is presented; never wire. Lands in FOCUS-3. Ruled: yes.
- **R9 — where focus shows** (amended by F9, 2026-09-27): every kind has a
  *home surface* — text kinds the reader for their corpus, geographic kinds
  (Map, Place, Polity, Era) the World view — and the popover is the summary
  of any focus on any surface. Following a link to another surface
  navigates there with the exploration carried. Ruled: yes (as the map
  correction).
- **R10 — the Map kind** (owner: "a new NodeKind that contains all the
  explorable elements on the map"). Applied with these specifics, each
  overridable:
  - name `NodeKind::Map` (not `Scene`/`Snapshot`, which map-generator uses
    for its own things; a Map node *will be* one of its snapshots later);
  - one Map per Era, id `Map:era-{eraId}`, label the era's name, payload
    `{ window }`; membership = the scene computation at the era's window;
  - relation `Shows => "shows" / "shown-on"`, appended last after
    `AuthoredBy`; a `Shown` row family `{ map: MapId, node: AnyNodeId, provenance }`
    (places, events, polities, narratives) in the Core section;
  - `Presentation.Geography` for Map, Place, Polity, Era; the World view
    presents it; the time slider scrubs *within* the focused Map's window
    and crossing into another era follows `follows-in` to the next Map
    (Eras already succeed one another).
  All graph-side parts are FOCUS-0 §7 item 6.

Nothing open remains; the spec awaits the plans.

## 11. Rulings (owner, 2026-09-29, ahead of the FOCUS-3 and FOCUS-6 plans)

- **R11 — Book of Concord paragraph numbers (PARA-NUM-1):** readers see the
  standard public-domain citation convention (Concordia Triglotta style:
  `Ap IV 48`, `SA III 3`, `LC I 12`), small and muted in the margin like
  verse numbers in the Bible reader. Internal codes (`BoC 7.2.1`) are never
  shown. FOCUS-3.
- **R12 — continuous scroll scope (with R8's remembered position):** one
  continuous scroll runs a whole Bible book (chapter headings inline; the
  arrows jump chapter to chapter); crossing into the next book is an
  explicit click. FOCUS-3.
- **R13 — one navigation principle for every corpus (NAV-UNIFORM-1):**
  owner: "think about how you're doing the bible. you can click through
  chapters or scroll through chapters, and you can scroll through a whole
  book. you have to click to get to the next book. same principle in BoC.
  article/topic you can click or scroll through, but you have to click to
  get to the next part, if you've scrolled to the bottom." So in the BoC the
  article/topic plays the chapter's role (arrows or scroll), and the part
  (or the document where it has no parts) plays the book's (an explicit
  click at the bottom). Also: "we will also need to come up with a nice
  table of contents component abstraction that we can reuse wherever we
  want; im thinking we'll just use for the boc though for now" — FOCUS-3
  builds ONE reusable table-of-contents component (over the served
  `contents` tree, stopping at article/topic per the 2026-09-15 queue) and
  uses it for the BoC only for now.
- **R14 — polity focus (FOCUS-6):** focusing a Polity keeps the World view
  in the current era, highlights its territory at the slider's current
  year, and shows its reign window on the slider so the reader can scrub
  its rise and fall. No jump, no multi-era overlay.

## 12. Rulings (owner, 2026-09-30, amending §3.1–§3.3 and §3.5)

- **R15 — an Explorable is explorable because it has a frontier.** "Explorable
  essentially means that we can interactively get to another node by graph
  traversal (i.e., get context): this is the explore monad." §3.1's
  `Explorable(Kind, Id, Label)` plus a separate `IFrontier` is replaced by:
  ```csharp
  public sealed class Explorable                    // a node WITH its frontier; sealed, no presentation methods
  {
      public NodeKind Kind { get; }  public string Id { get; }  public string Label { get; }
      public IReadOnlyList<FrontierGroup> Groups { get; }                   // the card's edge_summary
      public Task<Page<Link>> Links(EdgeKind kind, int? cursor = null);     // pages this node's edges, server order
      // equality and hash: (Kind, Id)
  }
  public sealed record Link(EdgeKind Kind, NodeRef Target);   // a reference, not yet explorable
  public interface IExplorer
  {
      Task<Explorable> Resolve(NodeRef target);   // unit: one card fetch → the node with its frontier
      Task<Explorable> Follow(Link link);          // bind: through one edge to the next node with its frontier
      Task<Presentation> Present(Explorable node, Surface surface);
  }
  ```
  `IFrontier` is folded in; `Explorable.From(NodeRef)` does not exist (a
  reference is resolved, never promoted); `GraphExplorer` is the only
  constructor of an `Explorable`. `FrontierGroup`, `Page<T>` and the three
  frontier laws stand.
- **R16 — presentation is a functor over the explorable, keyed by kind and
  surface.** The same `Explorable` is presented differently on each surface
  (a place is a region on the World view and a card in the popover) and
  offers the same frontier on every surface. `Presentation.Of(NodeKind,
  Surface)` with `Surface = World | Reader | Popover`, and
  `Affordances.Of(EdgeKind)`, are the two exhaustive tables (a missing arm is
  a build error, `client/BibleAtlas.Client.csproj`). There is no per-node
  presentation method and no interface a node implements to present itself:
  that is the legacy `IExplorable` design §9 retires. R9's home surfaces
  become rows of the table.
- **R17 — Back is the dual of the last un-returned hop** (`Breadcrumb[^1]`),
  so Back at the bottom is a no-op and Back-after-Back never goes forward
  (amends §3.5's literal `Links[^1]`).
