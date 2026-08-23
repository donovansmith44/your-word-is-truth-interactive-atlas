using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch P (the extensibility proof): a Theographic Person, reached by
/// clicking a PERSONS-section entry on a verse/passage popover (see
/// <c>VersePersonsSection</c>, Explore/PopoverSectionProviders.cs). Its own
/// "mentioned-in" frontier is verses, so a Person never pushes another
/// Person -- there is no recursive Person->Person traversal in this batch.
///
/// CORRECTED, fix round 1 (R-P2): the source itself is NOT silent on
/// person-to-person relationships -- Theographic's own `people.json`
/// carries `father`/`mother`/`children`/`siblings`/`partners` (each an
/// array of foreign-key record ids resolving to OTHER Person records in
/// the SAME file -- the identical "array of foreign-key ids" shape
/// `verses` already uses for mentions) plus `birthPlace`/`deathPlace`
/// (resolving to place-like records) and `memberOf`/`timeline`. This data
/// is real, already vendored, and this adapter already knows how to turn
/// that exact shape into typed edges -- it is simply NOT DONE this batch:
/// a genealogy/family edge or a Person-to-Place `birthplace`/`deathplace`
/// edge (analogous to `located-at`) is a NEW RELATION KIND, which design
/// doc §3's own relation-manifest law makes a types-first decision
/// (`graph-types`, owner approval), not an adapter-only one -- correctly
/// out of this batch's own card+mentions scope (batch-p-brief.md's own
/// requirement 2 names exactly "person cards + frontiers (mentioned-in)",
/// nothing more), and ledgered for the owner rather than silently built.
///
/// <see cref="PersonId"/> is the graph's own WIRE-encoded node id (e.g.
/// <c>"Person:aaron_1"</c>) -- <see cref="IExplorableClient"/>'s own
/// string-everywhere contract (see that interface's own doc comment); this
/// class never parses or re-derives it, only threads it through
/// <c>Card()</c>/<c>Edges()</c> calls untouched, exactly the way
/// <c>VerseNode.Title</c> already IS the vref with no wrapper type.
///
/// Card + frontier BOTH ride the generic contract (<see cref="IExplorableClient"/>),
/// the batch's own thesis test -- see <c>PersonCardAndMentionsSection</c>
/// (PopoverSectionProviders.cs) for the one section provider that does the
/// actual work; that provider's own <c>AppliesTo(node) => node.Kind ==
/// "Person"</c> means <c>PopoverSectionRegistry</c> claims this kind
/// outright, so <c>ExplorerPopover.LoadCurrent</c> never calls
/// <see cref="BodyAsync"/> for it (the SAME "registry supersedes BodyAsync
/// entirely" shape every other registry-claimed kind already follows --
/// see <c>PlaceNode</c>'s own doc comment). <see cref="BodyAsync"/> exists
/// only as the defensive <see cref="IExplorable"/> fallback.
///
/// Doctrine (batch-p-brief.md: "persons are Scripture's persons -- labels/
/// notes in the positive register; no speculative identifications beyond
/// the source"): <see cref="ExploreAsync"/> offers no invented chips (no
/// map affordance -- Person carries no lat/lon; no speculative "about this
/// person" prose beyond what the generic mentions frontier already shows)
/// -- the SAME "no chips at all" shape <c>CatechismNode</c> already
/// established for a node kind with nothing extra to offer.
/// </summary>
public sealed class PersonNode : IExplorable
{
    private readonly string _personId;
    private readonly string _label;

    public PersonNode(string personId, string label)
    {
        _personId = personId;
        _label = label;
    }

    /// The graph's own wire node id (e.g. "Person:aaron_1") -- what
    /// PersonCardAndMentionsSection threads into Card()/Edges() calls.
    public string PersonId => _personId;

    public string Title => _label;
    public string Kind => "Person";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    // Defensive fallback only -- see this class's own doc comment.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });
}
