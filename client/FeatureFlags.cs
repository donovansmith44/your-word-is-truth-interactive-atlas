namespace BibleAtlas.Client;

/// <summary>
/// Compile-time feature gates -- one home, no scattered magic bools.
/// </summary>
public static class FeatureFlags
{
    /// <summary>
    /// Cross-reference superscript markers in the reader (M-D2's
    /// XSCRIPT-1 feature). OFF by owner order (2026-08-23, ledgered):
    /// "just disable superscripts until the rework is released" -- the
    /// M-D2 hover behavior opened a popover that had to be X'd out,
    /// ruled a bug. M-D3's rework (click AND hover entry, anchored over
    /// the verse, always visible, no auto-modal) flips this ON and
    /// un-skips the 8 gated Playwright tests
    /// (tests/ux/reader-xref-superscripts.spec.ts). `static readonly`
    /// rather than `const` deliberately: a `const false` makes the
    /// gated render branch provably unreachable and trips CS0162 on an
    /// otherwise warning-free build.
    /// </summary>
    public static readonly bool XrefSuperscripts = false;
}
