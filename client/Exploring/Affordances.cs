using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public abstract record Affordance(int InitialClamp)
{
    public sealed record Arrows(ArrowDirection Direction) : Affordance(Affordances.ArrowsShown);

    public sealed record InlineChildren() : Affordance(Affordances.PageSize);

    public sealed record UpCrumb() : Affordance(Affordances.CrumbsShown);

    public sealed record SectionList(SectionStyle Style, SectionOrder Order) : Affordance(Affordances.PageSize);
}

public enum ArrowDirection
{
    Previous,
    Next,
}

public enum SectionStyle
{
    Standard,
    Quiet,
}

public enum SectionOrder
{
    VotesRanked,
    Canonical,
}

public static class Affordances
{
    public const int ArrowsShown = 1;

    public const int CrumbsShown = 3;

    public const int PageSize = 20;

    public static readonly Affordance.SectionList Cites = new(SectionStyle.Quiet, SectionOrder.VotesRanked);

    public static readonly Affordance.SectionList DefaultList = new(SectionStyle.Standard, SectionOrder.Canonical);

    private static readonly IReadOnlyList<EdgeKind> RuledOrder = [EdgeKind.Attests, EdgeKind.CatechismLink, EdgeKind.Cites];

    private static readonly IReadOnlyDictionary<EdgeKind, int> RuledRank = RuledOrder.Select((kind, rank) => (kind, rank)).ToDictionary(pair => pair.kind, pair => pair.rank);

    public static IEnumerable<FrontierGroup> InRuledOrder(IEnumerable<FrontierGroup> served) =>
        served.OrderBy(group => RuledRank.GetValueOrDefault(group.Kind, RuledOrder.Count));

    public static Affordance Of(EdgeKind kind) => kind switch
    {
        EdgeKind.FollowsIn => new Affordance.Arrows(ArrowDirection.Next),
        EdgeKind.PrecedesIn => new Affordance.Arrows(ArrowDirection.Previous),
        EdgeKind.Contains or EdgeKind.Shows => new Affordance.InlineChildren(),
        EdgeKind.MemberOf or EdgeKind.ShownOn => new Affordance.UpCrumb(),
        EdgeKind.Cites => Cites,
        EdgeKind.Mentions
            or EdgeKind.MentionedIn
            or EdgeKind.AttestedIn
            or EdgeKind.Attests
            or EdgeKind.DatedBy
            or EdgeKind.Dates
            or EdgeKind.LocatedAt
            or EdgeKind.SiteOf
            or EdgeKind.CitedBy
            or EdgeKind.Quotes
            or EdgeKind.QuotedBy
            or EdgeKind.Confesses
            or EdgeKind.ConfessedIn
            or EdgeKind.FulfilledIn
            or EdgeKind.Fulfills
            or EdgeKind.Prefigures
            or EdgeKind.PrefiguredBy
            or EdgeKind.NamedAfter
            or EdgeKind.NamesakeOf
            or EdgeKind.JustifiedBy
            or EdgeKind.Justifies
            or EdgeKind.CommentsOn
            or EdgeKind.CommentedOnBy
            or EdgeKind.SpokenBy
            or EdgeKind.SpeechOf
            or EdgeKind.SpokenAt
            or EdgeKind.SiteOfSpeech
            or EdgeKind.DerivedFrom
            or EdgeKind.Derives
            or EdgeKind.OccursIn
            or EdgeKind.Words
            or EdgeKind.ParentOf
            or EdgeKind.ChildOf
            or EdgeKind.ParticipatesIn
            or EdgeKind.Participants
            or EdgeKind.AuthoredBy
            or EdgeKind.Authored
            or EdgeKind.AnalogousTo
            or EdgeKind.CatechismLink
            or EdgeKind.CorrespondsTo
            or EdgeKind.Parallel
            or EdgeKind.TemporalAdjacency
            or EdgeKind.SpouseOf
            or EdgeKind.BrethrenOf => DefaultList,
    };
}
