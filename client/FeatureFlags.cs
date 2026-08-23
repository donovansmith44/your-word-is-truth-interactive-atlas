namespace BibleAtlas.Client;

/// <summary>
/// Compile-time feature gates -- one home, no scattered magic bools.
/// </summary>
public static class FeatureFlags
{
    /// <summary>
    /// Cross-reference superscript markers in the reader (M-D2's
    /// XSCRIPT-1 feature). Was OFF by owner order (2026-08-23, ledgered):
    /// "just disable superscripts until the rework is released" -- the
    /// M-D2 hover behavior opened a popover that had to be X'd out,
    /// ruled a bug. M-D3 (R3, this batch) is that rework: click AND
    /// hover both still open it, but now anchored OVER THE VERSE (never
    /// pane/viewport-centered) and always kept fully on-screen
    /// (ExplorerPopover.razor's own VerseAnchorSelector/VerseAnchorStyle
    /// mechanism), and a HOVER-only open (never a click or keyboard
    /// focus) now auto-dismisses a short grace period after the pointer
    /// leaves both the marker and the popover itself (Reader.razor's own
    /// OnXrefMarker*/CancelHoverClose) -- so a mere hover never again
    /// demands an explicit X. Flipped ON here; the 8 previously-gated
    /// Playwright tests (tests/ux/reader-xref-superscripts.spec.ts) are
    /// un-skipped in this same commit, plus new anchoring/viewport-edge/
    /// hover-dismiss coverage. `static readonly` rather than `const`
    /// deliberately: a `const false` made the gated render branch
    /// provably unreachable and tripped CS0162 on an otherwise
    /// warning-free build; kept `static readonly` now for symmetry with
    /// every other flag this file may grow, not because `true` itself
    /// needs it.
    /// </summary>
    public static readonly bool XrefSuperscripts = true;
}
