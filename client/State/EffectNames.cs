namespace BibleAtlas.Client.State;

/// <summary>
/// Fix round 1 (Q-4, trivia; enables the S-7 fix): the one vocabulary of
/// <see cref="Contracts.IStateEffect{T}"/> name strings -- mirrors
/// <see cref="Contracts.AtomNames"/>'s own role for atom names exactly.
/// Every effect construction site references a field here (never a bare
/// string literal), so <c>ConformanceTests.cs</c>'s own effect-name
/// conformance test can do REFLECTION over this class (the same technique
/// already proven real for <c>AtomNames</c>) instead of a source-text scan
/// for <c>DelegateEffect&lt;...&gt;("literal"</c>, which the review found
/// vacuous (S-7: invisible to a bespoke <see cref="Contracts.IStateEffect{T}"/>
/// class, and trivially non-empty with exactly one entry).
/// </summary>
public static class EffectNames
{
    /// World.razor's own follow-scene fetch effect (R1) -- see
    /// DelegateEffect's own construction site for the full mechanism.
    public const string FollowScene = "follow-scene";
}
