using System.Linq;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch R requirement 3 ("the popover becomes the composable content
/// platform"): the ExplorerPopover's own back-channel into ITS stack, handed
/// to every <see cref="IPopoverSectionProvider"/> so a section can act on the
/// shared popover (drill into a new node) without <c>ExplorerPopover.razor</c>
/// growing a bespoke callback per provider -- the same reason
/// <c>ExplorationTarget</c> exists for chips, one level down. Implemented by
/// <c>ExplorerPopover</c> itself.
/// </summary>
public interface IPopoverSectionContext
{
    /// Pushes <paramref name="node"/> onto the popover's own breadcrumb
    /// stack and loads it -- the exact same drill-in behavior an
    /// <see cref="ExplorationTarget.Push"/> chip already gives, just
    /// reachable from inside a section's own body instead of only from the
    /// chips row.
    Task PushAsync(IExplorable node);

    /// <summary>
    /// G2-m2 (batch-g2-report.md's own disclosed v1-scope gap: "popover-
    /// internal explorable rows... are NOT wired for this gesture"): the
    /// SAME Ctrl/Cmd-click toggle-into-the-Selection-Tray gesture
    /// VerseLine.razor/map.js already give their own surfaces, reachable
    /// from every popover-internal explorable row through this ONE
    /// back-channel -- mirrors <see cref="PushAsync"/>'s own "one
    /// interface method, every provider reaches it the same way" shape, so
    /// closing this gap never means teaching each section provider its own
    /// bespoke selection mechanism.
    /// </summary>
    Task ToggleSelectAsync(IExplorable node);

    /// <summary>
    /// Batch F2 requirement 6 (cross-reference truncation, "no more than 3
    /// if xrefs are the ONLY kind of context... no more than 2 if there are
    /// OTHER types of context (small catechism, etc.)"): how many OTHER
    /// resolved sections (excluding the verse-text section itself -- "the
    /// subject, not context" -- and the xrefs section's own count of
    /// itself) are present for the CURRENT node, at the moment this is
    /// read. Read from a <c>RenderFragment</c> at RENDER time (not
    /// captured into a local at <c>ResolveAsync</c> time, when sibling
    /// providers may not have resolved yet -- concurrent resolution means
    /// no provider can know its own siblings' outcomes during its own
    /// <c>ResolveAsync</c> call) -- by the time ANY section's fragment
    /// actually renders, every provider has already finished (<c>LoadCurrent</c>
    /// awaits the whole batch before the render that shows any of them), so
    /// this always reflects the FINAL section list. Generic by construction
    /// -- counts whatever is actually in the resolved section list, never a
    /// hardcoded "is catechism present" check -- so this keeps working
    /// unchanged when Batch P (or any later batch) adds a new provider.
    /// </summary>
    int OtherContextSectionCount { get; }

    /// <summary>
    /// Batch M-D2 (owner's cross-reference superscript directive, entry-
    /// point parameter): <c>true</c> exactly when the CURRENT node is a
    /// <c>VerseNode</c> pushed via its own xref superscript cluster/many-
    /// marker (<see cref="Explore.VerseNode.XrefEntryPoint"/>) rather than
    /// the ordinary verse line/verse-num. Read from a <c>RenderFragment</c>
    /// at RENDER time, same discipline as <see cref="OtherContextSectionCount"/>
    /// immediately above (both are read INSIDE <see cref="CrossRefsSection"/>'s
    /// own returned closure, never captured during the concurrent
    /// <c>ResolveAsync</c> phase) -- house pattern, non-negotiable per this
    /// batch's own brief ("the per-chapter superscript lettering is exactly
    /// the shape that bit Batch N"). A parameter on the ONE shared
    /// abstraction (owner decree: never a parallel interface) -- this
    /// property, plus <see cref="OtherContextSectionCount"/>, are the ONLY
    /// two inputs <c>CrossRefsSection</c>'s own cap decision reads.
    /// </summary>
    bool XrefEntryPoint { get; }

    /// <summary>
    /// Batch P (M-D2 ruling: "the verse popover's PERSONS section... the
    /// FIRST IExplorableClient consumer"): the generic graph client, handed
    /// to every provider through this SAME back-channel <see cref="PushAsync"/>
    /// already uses -- so a section reads the generic contract without
    /// <c>ExplorerPopover.razor</c> growing a bespoke per-provider
    /// dependency, and without widening every OTHER provider's own
    /// <c>ResolveAsync(node, api, ctx)</c> signature (which would touch
    /// ~20 unrelated providers for one new consumer). Implemented by
    /// <c>ExplorerPopover</c> itself, backed by its own <c>@inject
    /// IExplorableClient</c>.
    /// </summary>
    IExplorableClient Graph { get; }

    /// <summary>
    /// EVT-3 Ticket 3 (the map-focus-at-time hatch: kind + {place, window}
    /// params, per the composability rulings -- fix round 1, Q-1: NOT an
    /// instance of this codebase's own formal <see cref="Contracts.IEscapeHatch"/>
    /// machinery, see <see cref="MapFocusHatch"/>'s own doc comment for the
    /// genuine design conflict that rules that out): the SAME NavigateWorld
    /// continuation an <see cref="Exploration"/> chip already gives
    /// (<c>ExplorerPopover</c>'s own Activate/NavigateWorld case, "the
    /// split IS the atlas" hand-off included -- see that file's own
    /// comment), reachable from inside a section's own body instead of
    /// only from the chips row -- so a Place: row's own explorable value
    /// (<see cref="Explore.EventDateAndPlacesSection"/>) can invoke the
    /// SAME hatch a chip would, without <c>ExplorerPopover</c>
    /// growing a bespoke callback per provider (mirrors
    /// <see cref="PushAsync"/>'s own "one interface method, every provider
    /// reaches it the same way" shape). <paramref name="query"/> is the
    /// EXACT same ready-to-append query-string shape
    /// <see cref="ExplorationTarget.NavigateWorld"/> already carries (see
    /// <see cref="MapFocusHatch"/> for the ONE declared site that builds a
    /// {place, window}-carrying query).
    /// </summary>
    Task NavigateWorldAsync(string query);
}

/// <summary>
/// One resolved, ready-to-render section of a popover body. <see cref="Testid"/>
/// becomes the section's own wrapper <c>data-testid="popover-section-{Testid}"</c>
/// -- a stable hook for tests/CSS that exists independent of whatever content
/// a provider happens to render this time.
/// </summary>
public sealed record PopoverSection(string Testid, RenderFragment Body);

/// <summary>
/// Batch R requirement 3: one pluggable chunk of ExplorerPopover content.
/// "A node type maps to an ordered list of section providers; each provider
/// answers (has content? -&gt; render section)" -- <see cref="AppliesTo"/> is
/// the cheap, synchronous first half of that question (does this provider
/// even consider this NODE KIND at all); <see cref="ResolveAsync"/> is the
/// second, possibly-fetching half, returning null for "no content, no
/// section" (conditional presence throughout -- a seam with nothing
/// registered, or a provider whose own data is empty for this node, renders
/// NOTHING, not an empty placeholder).
///
/// Registering (or removing) a provider is the ENTIRE integration surface
/// Batch F (the small catechism) and Batch P (Theographic place descriptions)
/// need -- see <see cref="PopoverSectionRegistry"/>'s own doc comment for
/// exactly where each one's own future provider slots in. Neither
/// <c>ExplorerPopover.razor</c> nor any <c>IExplorable</c> node needs to
/// change for either batch to land its own content.
/// </summary>
public interface IPopoverSectionProvider
{
    bool AppliesTo(IExplorable node);

    Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx);
}

/// <summary>
/// The registry itself: one FLAT, ORDERED list of providers. ExplorerPopover
/// filters this list to whichever providers' own <see cref="IPopoverSectionProvider.AppliesTo"/>
/// says yes for the CURRENT node (preserving THIS list's own order), resolves
/// each, and renders every non-null result in that same order -- "registering/
/// removing a provider is the whole of adding/removing a kind of content,"
/// per the batch brief, verbatim.
///
/// Requirement 3's own explicit ordering, realized as registration order:
/// VERSE/PASSAGE get (a) the verse/passage itself with its own expand
/// affordance (requirement 4), (b) cross-references inline, (c) Batch F's
/// own "THE SMALL CATECHISM" seam (<see cref="CatechismSeamSection"/>,
/// filled -- see that class's own doc comment); PLACE gets (a) description
/// (Batch P fills this -- register its own provider REPLACING
/// PlaceDescriptionSection), (b) established/destroyed, (c) period blurb,
/// (d) events. CATECHISM (Batch F's own new node kind, reached by pushing a
/// citing item from the seam above) gets, in this order: (a) the item's own
/// text (conditional -- absent for Baptism/Confession/Sacrament-of-the-Altar
/// items, see <see cref="CatechismTextSection"/>), (b) the explanation under
/// Luther's own verbatim heading, (c) "Where is this written?" (conditional),
/// (d) "THE SCRIPTURES" -- the item's own proof verses, each explorable
/// (conditional). "Explore" (the map affordance) is deliberately NOT one of
/// these providers -- it stays the pre-existing <c>popover-chip-map</c>
/// chip, rendered by ExplorerPopover's own already-working chips row below
/// every section, unchanged by this batch (see ExplorerPopover.razor's own
/// comment on why) -- moot for CATECHISM nodes regardless, since
/// <c>CatechismNode.ExploreAsync</c> offers no chips at all (no geography).
///
/// A node kind no provider here claims at all falls back to that node's
/// own <c>BodyAsync</c> -- ExplorerPopover's pre-Batch-R rendering path,
/// untouched. As of Batch M-D3/EVT-3, Chapter and Year both moved OFF this
/// fallback (ChapterCardSection/YearFrontierSection below) -- Book/Author/
/// TimeAndPlace are the remaining kinds still reached this way today.
///
/// Batch N ("narratives as first-class graph structure") originally added
/// two providers to VERSE's own list plus a NarrativeEvent traversal target
/// -- Batch T ("events as the narrative nodes") RETIRES both from Verse
/// (the owner, verbatim: "rather than putting the next/previous event on
/// every verse, add titles of events... traversal lives on event nodes")
/// and replaces NarrativeEventNode with a richer EVENT node kind. VERSE
/// gains ONE new provider instead -- "EVENT" membership
/// (<see cref="VerseEventMembershipSection"/>, conditional on the verse
/// touching >=1 titled EVENT-kind passage), each row explorable, opening a
/// fresh <see cref="EventNode"/>.
///
/// UX-1 (FRONTIER-ORDER-1), owner verbatim order (SUPERSEDES the M-D3/U6
/// order this paragraph used to describe): "Verse, Event, Catechism,
/// Parallels, then cross references." VERSE (and PASSAGE, where the same
/// provider also applies) sections now render in EXACTLY this order: the
/// verse's own text (<see cref="VerseTextSectionProvider"/>, "focus"),
/// "EVENT" membership (<see cref="VerseEventMembershipSection"/>,
/// Verse-only -- unchanged scope note below), "THE SMALL CATECHISM"
/// (<see cref="CatechismSeamSection"/>, capped to 2 shown + U2's shared
/// reveal mechanic -- MOVED above Parallels this batch), "PARALLELS"
/// (<see cref="VerseParallelsSection"/> -- other witnesses of an event the
/// verse belongs to, a quick peek without a click into the EVENT node
/// first; see that class's own doc comment -- MOVED below Catechism this
/// batch), and cross-references LAST (<see cref="CrossRefsSection"/>,
/// unchanged). Realized as an explicit ORDER KEY on each registration (see
/// the registry's own doc comment immediately below) -- REGISTRY-1's own
/// "registration order is render order" rule now reads as "Order value is
/// render order," array position no longer significant. ("PERSONS" --
/// <see cref="VersePersonsSection"/> --
/// used to sit here too, between Parallels and Catechism, the controller's
/// own "Persons then Places" reconciliation; O4, 2026-08-23, owner
/// live-preview correction, unregistered it -- "remove persons from hover
/// menus for now," machinery retained, not deleted -- see the registry's
/// own comment below for the fuller story.) EVENT node sections, in order
/// (CHRONO-MERGE-1, 2026-08-24, "put chronology up top" -- RESPEC'D from
/// the M-D3/U1 order this paragraph used to describe, see the registry
/// list's own comment below for the reordering itself): Chronology FIRST
/// (<see cref="EventChronologySection"/> -- the SAME global arrow row
/// TRAV-1/HOTFIX-4 built, now the atlas's ENTIRE traversal surface for an
/// EVENT node, occupying the top position the narrative nav used to hold
/// before this batch retired it whole), THEN date + place(s)
/// (<see cref="EventDateAndPlacesSection"/>, narrative-nav-free now --
/// that class's own doc comment has the retirement story), THEN PARALLEL
/// ACCOUNTS (<see cref="EventWitnessesSection"/>, conditional presence: no
/// "PARALLEL ACCOUNTS" framing at all when the event has exactly one
/// witness) -- recursion falls out of an EventNode's own traversal arrow
/// pushing ANOTHER EventNode, the SAME `AppliesTo` clause matching it too,
/// not a second mechanism.
/// </summary>
public static class PopoverSectionRegistry
{
    // Batch UX-1 (FRONTIER-ORDER-1, owner order verbatim: "when clicking on
    // a verse, things should be ordered as such (visually): Verse, Event,
    // Catechism, Parallels, then cross references"): ordering is now an
    // explicit ORDER KEY on each registration, not bare array position --
    // "a section's position changes by editing one registration line," THE
    // COMPOSABILITY BAR's own standing bar for this knob. `Entries.OrderBy`
    // is a STABLE sort (LINQ's own documented guarantee), so two entries
    // sharing an Order value keep resolving in REGISTRATION order --
    // REGISTRY-1's own pre-existing rule, unchanged for every provider this
    // batch does not touch. See CONTRACT.md's own FRONTIER-ORDER-1 row for
    // the full ruled VERSE/PASSAGE sequence (Order 10/20/21/30/40/50) and
    // the disclosed "Order left unclustered above 50 for future
    // Place/Catechism-detail/Event/PolityDelta/Person providers" spacing.
    private static readonly (IPopoverSectionProvider Provider, int Order)[] Entries =
    {
        // M-D3 (U4/B3): CHAPTER's own metadata-and-context card -- a
        // brand-new node kind (Kind == "Chapter"), so its own Order has no
        // effect relative to any VERSE/PASSAGE/PLACE/EVENT/CATECHISM/
        // PolityDelta/Person provider below; listed first only because it
        // is the FIRST node kind alphabetically among this file's own
        // section headers, not because order matters here.
        (new ChapterCardSection(), 0),
        // EVT-3 Ticket 3: YearNode's own registry-facing body (see
        // YearFrontierSection's own doc comment) -- a brand-new provider
        // for a PRE-EXISTING node kind (Kind == "Year", previously
        // BodyAsync-fallback only), so its own Order has no effect
        // relative to any other provider below (no other AppliesTo ever
        // matches Kind == "Year").
        (new YearFrontierSection(), 5),

        // FRONTIER-ORDER-1's own ruled VERSE/PASSAGE sequence -- Verse
        // (focus, unconditional whenever the graph has text) / Event
        // (dated-event membership, then the sibling general-kind Passage
        // membership row PERI-1's own "dated first, general second" rule
        // nests under the SAME visual "Event" category) / Catechism
        // (MOVED above Parallels this batch) / Parallels (MOVED below
        // Catechism this batch) / cross references LAST. Every OTHER
        // provider below is Order>=60, so none of them can ever interleave
        // with this six regardless of where its own registration LINE sits
        // in this file.
        (new VerseTextSectionProvider(), 10),
        // Batch T ("events as the narrative nodes"): VERSE-only (a
        // shift-click passage span's own per-verse event membership is
        // genuinely ambiguous in a way a single verse never is -- REGISTRY-1's
        // own standing scope note, unchanged).
        (new VerseEventMembershipSection(), 20),
        // Batch PERI-1 (PRESENTATION CATEGORY LAW): the sibling "PASSAGE"
        // section for this verse's own general-kind (dateless) passages --
        // Order 21 (immediately after EVENT membership's own 20) so a verse
        // touching both kinds renders dated events first, general-kind
        // passages second, unchanged from the pre-UX-1 registration-order
        // behavior.
        (new VersePassageMembershipSection(), 21),
        // UX-1 (FRONTIER-ORDER-1): "Catechism" now precedes "Parallels" --
        // the owner's own ruled sequence. Was registered AFTER
        // VerseParallelsSection pre-this-batch; only the Order value moved,
        // not the class or its own resolution logic.
        (new CatechismSeamSection(), 30),
        // M-D3/U6 (Batch M-D3): "PARALLELS" -- other witnesses of an event
        // the current verse belongs to, a quick peek without a click into
        // the EVENT node first. Verse OR Passage (first-verse-anchored for
        // a Passage, the SAME convention VersePersonsSection used to
        // establish here) -- see VerseParallelsSection's own doc comment
        // for the full conditional-presence rule. UX-1: now AFTER
        // Catechism (Order 40 > 30), the owner's own ruled sequence.
        (new VerseParallelsSection(), 40),
        // Cross-references LAST among Verse/Passage sections, per the
        // owner's own explicit ordering (unchanged position, still the
        // highest Order among the six).
        (new CrossRefsSection(), 50),

        // Batch P: Persons -- O4 (owner live-preview correction,
        // 2026-08-23: "remove persons from hover menus for now")
        // UNREGISTERS VersePersonsSection; the class itself is NOT deleted
        // (a deliberate, disclosed dead-code-law exception) -- see
        // Explore/PopoverSectionProviders.cs's own VersePersonsSection for
        // the still-intact class. In-text person mentions (Reader.razor's
        // own verse-mention-person-* spans, PersonNode/
        // PersonCardAndMentionsSection below) are a DIFFERENT affordance,
        // not named by FRONTIER-ORDER-1, and stay fully live.

        (new PlaceDescriptionSection(), 60),
        (new PlaceDatesSection(), 70),
        (new PlaceBlurbSection(), 80),
        (new PlaceEventsSection(), 90),
        (new CatechismTextSection(), 100),
        (new CatechismExplanationSection(), 110),
        (new CatechismWhereWrittenSection(), 120),
        (new CatechismScripturesSection(), 130),
        // EVT-META-TOP-1 (fix round 2, owner verbatim: "time + place block
        // should be moved to the top, right below the event header"):
        // SUPERSEDES CHRONO-MERGE-1's own "put chronology up top" ordering
        // -- EVENT node sections now run: date+place(s) FIRST
        // (EventDateAndPlacesSection, the bare Time:/Place: values,
        // EVT-META-TOP-1's own "right below the header"); THEN Chronology
        // (traversal: the SAME global arrow row TRAV-1/HOTFIX-4 built, now
        // ALSO carrying the divergence-only story-thread line that
        // survives of the retired narrative nav -- EventChronologySection's
        // own doc comment has the full story); THEN PARALLEL ACCOUNTS.
        (new EventDateAndPlacesSection(), 135),
        (new EventChronologySection(), 140),
        (new EventWitnessesSection(), 160),
        // Batch ATTEST-1 (the owner's two orders, plus his own placement
        // AMENDMENT, verbatim: "let's have a 'Similar Accounts' or
        // something similar added to the frontier part of the UI where it
        // was getting pulled in as a parallel account. Have that section be
        // right below the 'Parallel ..' section.").
        //
        // THE PLACEMENT IS LAW, not taste. A row that was WRONGLY rendering
        // as a parallel account (Matthew's leper under
        // `rob_leper_healed`'s PARALLEL ACCOUNTS) must move DOWN EXACTLY
        // ONE SECTION, into a heading that tells the truth -- not vanish
        // from where the reader last saw it. So SIMILAR ACCOUNTS takes 161,
        // directly after PARALLEL ACCOUNTS at 160, with nothing permitted
        // between them; `EventSectionsInReviewedOrder` (PopoverSectionRegistry
        // tests) asserts the ADJACENCY itself, not merely the number, so a
        // future provider registered at any value in between fails loud.
        //
        // MENTIONED IN follows at 165. It is scriptural basis (for a
        // mention-only event like the Espousal of Mary it is the ONLY
        // scriptural section, PARALLEL ACCOUNTS being correctly absent),
        // but it is not what the owner asked to sit under the parallels --
        // the analogue is.
        //
        // Both are conditionally present through the ordinary "return null
        // when empty" path -- the smart-frontier law, no new mechanism, and
        // never an empty section.
        (new EventAnaloguesSection(), 161),
        (new EventMentionsSection(), 165),
        // Batch M ("the DAG grows a node type"): PolityDelta's own three
        // sections, in order -- event text, THE SCRIPTURES, grounding note.
        // None of the three's own AppliesTo overlaps any earlier provider's
        // (Kind == "PolityDelta" is brand new), so Order here is ALSO
        // render order for this node kind.
        (new PolityDeltaEventSection(), 170),
        (new PolityDeltaScripturesSection(), 180),
        (new PolityDeltaGroundingSection(), 190),
        // Batch P (the extensibility proof): the PERSON node's own card +
        // mentioned-in frontier. VersePersonsSection itself (Batch P's
        // OTHER half -- the verse's own entry point INTO persons) no
        // longer lives here -- M-D3/U6 moved it up to Order 30-40's
        // neighborhood historically; O4 above unregistered it entirely.
        (new PersonCardAndMentionsSection(), 200),
        // Batch CORP-1b (owner authorization, resolving CORP-1's own
        // disclosed NEEDS_CONTEXT gap): the CommentaryItem node's own real
        // prose -- highest Order, same "later batches append below, never
        // disturb" convention every prior batch's own new provider already
        // followed, now expressed as "the largest Order value" instead of
        // "the last array element."
        (new CommentaryItemProseSection(), 210),
    };

    // OrderBy is a STABLE sort (documented LINQ guarantee) -- two entries
    // sharing an Order value resolve in the ARRAY's own declaration order,
    // preserving REGISTRY-1's pre-existing tie-break rule exactly.
    public static readonly IReadOnlyList<IPopoverSectionProvider> Providers =
        Entries.OrderBy(e => e.Order).Select(e => e.Provider).ToArray();
}
