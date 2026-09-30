using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public abstract record Affordance
{
    public sealed record Arrows : Affordance;

    public sealed record InlineChildren : Affordance;

    public sealed record UpCrumb : Affordance;

    public sealed record SectionList(SectionStyle Style, int InitialClamp, SectionOrder Order) : Affordance;
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
    public static readonly Affordance.SectionList Cites = new(SectionStyle.Quiet, InitialClamp: 3, SectionOrder.VotesRanked);

    public static readonly Affordance.SectionList Mentions = new(SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical);

    public static readonly Affordance.SectionList MentionedIn = new(SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical);

    public static readonly Affordance.SectionList DefaultList = new(SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical);

#pragma warning disable CS8524
    public static Affordance Of(EdgeKind kind) => kind switch
    {
        EdgeKind.FollowsIn or EdgeKind.PrecedesIn => new Affordance.Arrows(),
        EdgeKind.Contains or EdgeKind.Shows => new Affordance.InlineChildren(),
        EdgeKind.MemberOf or EdgeKind.ShownOn => new Affordance.UpCrumb(),
        EdgeKind.Cites => Cites,
        EdgeKind.Mentions => Mentions,
        EdgeKind.MentionedIn => MentionedIn,
        EdgeKind.AttestedIn
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
#pragma warning restore CS8524
}
