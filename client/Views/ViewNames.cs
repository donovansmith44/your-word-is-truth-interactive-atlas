namespace BibleAtlas.Client.Views;

/// <summary>
/// Batch VC-1 (R1): the generalization seam's own vocabulary of stable view
/// NAMES -- "reader" | "world" | "sources" -- extend-only, the same
/// discipline <see cref="Contracts.AtomNames"/> already established for atom
/// names. Every <see cref="ViewArrangement"/> member, every
/// <see cref="ViewRegistry"/> registration key, and every
/// <see cref="EnterSplitHatch"/>'s Owner/Partner reference one of these
/// constants -- never a bare string literal (conformance-tested, see
/// client.Tests/State/ViewRegistryConformanceTests.cs).
/// </summary>
public static class ViewNames
{
    public const string Reader = "reader";
    public const string World = "world";
    public const string Sources = "sources";
}

/// <summary>
/// R1: "declared CAPABILITIES as data ... capability flags are contract
/// data, the §4c pattern" -- the same "declare it as data, never a name
/// check" discipline <c>client/Contracts/Interactions.cs</c>'s own
/// <c>IInteractionContract</c>/<c>TimingDiscipline</c> already establish for
/// gestures. <see cref="BearsLocus"/> (reader: "I have a chapter/verse
/// position the follow link can read") and <see cref="BearsWindow"/> (world:
/// "I have a time/scripture window the follow link can write") are the two
/// R5 needs -- FollowTextLink.Active queries THESE flags via the registry,
/// never a "== ViewNames.Reader"/"== ViewNames.World" check.
/// </summary>
[Flags]
public enum ViewCapabilities
{
    None = 0,
    BearsLocus = 1 << 0,
    BearsWindow = 1 << 1,
}

/// <summary>R4: the one Kind every enter-split hatch declares -- contract
/// data (<see cref="Contracts.IEscapeHatch.Kind"/>), never a per-hatch bespoke
/// string.</summary>
public static class HatchKinds
{
    public const string EnterSplit = "enter-split";
}
