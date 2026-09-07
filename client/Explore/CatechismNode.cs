using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch F ("the small catechism"): one item of Luther's Small Catechism,
/// reached by pushing this node from a VERSE/PASSAGE popover's own "THE
/// SMALL CATECHISM" section (<see cref="CatechismSeamSection"/>) or from a
/// proof verse's own onward navigation is the REVERSE direction (a
/// <see cref="VerseNode"/>, not this class). <see cref="Title"/> is the
/// item's own display <c>name</c> (e.g. "The First Commandment", "Baptism —
/// Part the Fourth") -- known synchronously at construction, the SAME
/// "never a fetch" discipline every other <see cref="IExplorable"/> node
/// follows (here, handed in directly by the caller, which already has it
/// from a <see cref="CatechismRefDto"/> row -- no need to re-derive it from
/// a fetch this node would otherwise have to make just to render its own
/// popover header).
///
/// Registered content lives ENTIRELY in the section registry (four
/// providers in Explore/PopoverSectionProviders.cs, Kind == "Catechism":
/// text, explanation, where-written, THE SCRIPTURES) -- <see cref="BodyAsync"/>
/// below is the interface's required fallback, never actually reached in
/// practice (every real item always has a non-empty `explanation`, so the
/// explanation provider always resolves a section, exactly the same
/// "BodyAsync survives, unused" shape <c>PlaceNode</c> already established).
/// <see cref="ExploreAsync"/> returns no chips at all -- "No Explore/map
/// section (catechism nodes have no geography -- conditional presence
/// applies to affordances too)," per the batch brief verbatim.
/// </summary>
public sealed class CatechismNode : IExplorable
{
    private readonly string _id;
    private readonly AsyncMemo<CatechismItemDetail> _detail = new();

    public CatechismNode(string id, string name)
    {
        _id = id;
        Title = name;
    }

    public string Id => _id;
    public string Title { get; }
    public string Kind => "Catechism";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = _ => { };
        return Task.FromResult(fragment);
    }

    /// Memoized -- shared by every one of this node's own four section
    /// providers (mirrors <c>VerseNode.DetailAsync</c>'s own reasoning
    /// exactly), so opening one catechism item's popover is ONE fetch
    /// regardless of how many of its sections end up resolving content.
    ///
    /// PERF-3 fix round 1 (review Q-4 named this "Minor... not confirmed
    /// live -- single call site found"; that undercounted it -- CORRECTED
    /// here): FOUR providers call this
    /// (CatechismTextSection/CatechismExplanationSection/
    /// CatechismWhereWrittenSection/CatechismScripturesSection,
    /// PopoverSectionProviders.cs, all Kind == "Catechism"), all under the
    /// same ExplorerPopover.LoadCurrent Task.WhenAll dispatch this batch's
    /// own VerseNode/EventNode/PlaceNode.DetailAsync fixes already
    /// convicted -- this doc comment's own "shared by every one of this
    /// node's own four section providers" line, unchanged above, already
    /// said as much. A LIVE race, the same class as EventNode/PlaceNode,
    /// not merely a preemptive hardening. AsyncMemo-backed now
    /// (Explore/AsyncMemo.cs), same fix.
    public Task<CatechismItemDetail> DetailAsync(AtlasClient api) => _detail.Get(() => api.CatechismItem(_id));
}
