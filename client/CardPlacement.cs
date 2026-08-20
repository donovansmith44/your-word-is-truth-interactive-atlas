namespace BibleAtlas.Client;

/// <summary>
/// HOTFIX batch (batch-hotfix-brief.md requirement 1, user report "if there
/// are locations at the top of the screen and you hover for your hover
/// menu, the hover menu can be cut off by the top of the screen"): pure
/// placement math for PlaceCard's own hover/pinned card. Given the marker's
/// anchor point (the same containerPoint World.razor already feeds
/// PlaceCard as X/Y -- see PlaceCard.razor's own file header), the card's
/// just-measured render box, and its DOM parent's clientWidth (the map
/// container -- .world-page standalone, .split-pane-atlas embedded --
/// PlaceCard is always a direct child of, see MapInterop.MeasureCardPlacement's
/// own comment), this decides two things app.css's own CSS transform can't
/// work out on its own:
///   1. Flipped -- true when the card cannot fit ABOVE the marker (app.css's
///      pre-existing default placement) without its top edge crossing the
///      container's own top edge, i.e. exactly the cut-off bug. The caller
///      swaps which vertical `calc()` the --card-dy custom property carries
///      (see PlaceCard.razor's own markup) -- CSS's native `-100%` transform
///      percentage still does the "subtract my own height" arithmetic, this
///      only supplies the BOOLEAN decision.
///   2. DxPx -- a plain pixel nudge added on top of the native `-50%`
///      horizontal centering transform, clamped so the card never crosses
///      either side of the container (requirement 1's "clamp horizontally").
///      Zero when the naive centered position already clears both edges
///      with EdgeMarginPx to spare (the common case -- most cards need no
///      correction at all).
///
/// Standalone (not a PlaceCard.razor @code member) specifically so it is
/// unit-testable the same direct way YearText/SliderScale already are (see
/// client.Tests/CardPlacementTests.cs) -- unlike PlaceCard's own private
/// Groups/VisibleGroups (Playwright-verified black-box, tightly coupled to
/// Place.Events), this is a plain numeric function with zero Blazor/
/// component-state coupling, so a plain xunit Theory can exercise every
/// edge case (flip boundary, clamp boundary, a container narrower than the
/// card itself) far faster and more precisely than a browser round trip.
/// </summary>
public static class CardPlacement
{
    /// <summary>
    /// Gap kept between the marker's own anchor point and the card's near
    /// edge -- matches app.css's pre-existing 18px (Batch D's own
    /// `calc(-100% - 18px)`), unchanged by this batch; only which SIDE of
    /// the marker that 18px applies to now varies with <see cref="Flipped"/>.
    /// </summary>
    public const double GapPx = 18;

    /// <summary>
    /// Minimum breathing room kept between the card's own edge and either
    /// side of its container once horizontal clamping applies -- "never
    /// leaves the viewport either side" per the brief, with a little real
    /// margin rather than a last-pixel fit.
    /// </summary>
    public const double EdgeMarginPx = 8;

    /// <param name="anchorX">The marker's own containerPoint X (PlaceCard's X parameter).</param>
    /// <param name="anchorY">The marker's own containerPoint Y (PlaceCard's Y parameter).</param>
    /// <param name="cardWidth">The card's own just-measured offsetWidth.</param>
    /// <param name="cardHeight">The card's own just-measured offsetHeight.</param>
    /// <param name="containerWidth">The card's DOM parent's own clientWidth (the map container this instance is currently rendered inside).</param>
    public static (double DxPx, bool Flipped) Compute(double anchorX, double anchorY, double cardWidth, double cardHeight, double containerWidth)
    {
        // Fits above (app.css's own default) exactly when the card's own
        // height plus the marker gap still leaves the container's top edge
        // (local y=0 -- the SAME coordinate origin containerPoint already
        // uses, see this class's own header comment) uncrossed. `< 0`, not
        // `<= 0`: a card that fits EXACTLY flush (zero pixels to spare)
        // still counts as fitting, matching CSS's own `>=`-style boundary
        // behavior for a `top: 0` box.
        var flipped = anchorY - GapPx - cardHeight < 0;

        // The native `-50%` transform already centers the card on anchorX
        // using ITS OWN rendered width -- correct with zero input from
        // here. This only computes the CORRECTIVE nudge on top of that
        // native centering, so a card with room to spare on both sides
        // (the common case) gets exactly 0 back, unchanged from today.
        var naiveLeft = anchorX - cardWidth / 2;
        var minLeft = EdgeMarginPx;
        var maxLeft = containerWidth - cardWidth - EdgeMarginPx;

        double clampedLeft;
        if (maxLeft < minLeft)
        {
            // The container itself is narrower than the card plus both
            // margins (a very small pane, or an oversized card) -- centering
            // within whatever room actually exists beats an inverted clamp
            // (Math.Clamp would throw if min > max) or pinning to one edge.
            clampedLeft = (containerWidth - cardWidth) / 2;
        }
        else
        {
            clampedLeft = Math.Clamp(naiveLeft, minLeft, maxLeft);
        }

        return (clampedLeft - naiveLeft, flipped);
    }
}
