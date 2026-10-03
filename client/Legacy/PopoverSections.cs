using System.Linq;
using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public interface IPopoverSectionContext
{
    Task PushAsync(PopoverOpening opening, EdgeKind via);

    Task ToggleSelectAsync(NodeRef node);

    // Must be read from a RenderFragment at render time, never captured into a local during
    // ResolveAsync: providers resolve concurrently, so no provider can know its siblings'
    // outcomes during its own ResolveAsync call. By render time every provider has finished.
    int OtherContextSectionCount { get; }

    IExplorableClient Graph { get; }

    Task RenewAsync();

    Task NavigateWorldAsync(string query);
}

public sealed record PopoverSection(string Testid, RenderFragment Body);

public interface IPopoverSectionProvider
{
    bool AppliesTo(IExplorable node);

    Task<PopoverSection?> ResolveAsync(IExplorable node, Explorable current, AtlasClient api, IPopoverSectionContext ctx);
}

public static class PopoverSectionRegistry
{
    private static readonly (IPopoverSectionProvider Provider, int Order)[] Entries =
    {
        (new ChapterCardSection(), 0),
        (new YearFrontierSection(), 5),

        (new PassageTextSection(), 10),
        (new PassageCatechismSection(), 30),
        (new PassageCrossRefsSection(), 50),

        (new CatechismTextSection(), 100),
        (new CatechismExplanationSection(), 110),
        (new CatechismWhereWrittenSection(), 120),
        (new CatechismScripturesSection(), 130),
        (new CatechismInConcordSection(), 131),
        (new EventProvenanceSection(), 134),
        (new EventDateAndPlacesSection(), 135),
        (new EventChronologySection(), 140),
        (new EventWitnessesSection(), 160),
        // SIMILAR ACCOUNTS (161) must stay immediately after PARALLEL ACCOUNTS (160) --
        // PopoverSectionRegistryTests asserts this adjacency directly, not just the order values.
        (new EventAnaloguesSection(), 161),
        (new EventMentionsSection(), 165),
        (new PolityDeltaEventSection(), 170),
        (new PolityDeltaScripturesSection(), 180),
        (new PolityDeltaGroundingSection(), 190),
        (new PersonLifeSection(), 196),
        (new PersonEventsSection(), 197),
        (new PersonFamilySection(), 198),
        (new PersonCardAndMentionsSection(), 200),
        (new CommentaryItemProseSection(), 210),
    };

    // OrderBy is a stable sort: two entries sharing an Order value resolve in this array's
    // own declaration order.
    public static readonly IReadOnlyList<IPopoverSectionProvider> Providers =
        Entries.OrderBy(e => e.Order).Select(e => e.Provider).ToArray();
}
