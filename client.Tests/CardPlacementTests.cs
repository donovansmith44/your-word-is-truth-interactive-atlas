namespace BibleAtlas.Client.Tests;

// HOTFIX batch (batch-hotfix-brief.md requirement 1): CardPlacement.Compute
// is a plain numeric function (see its own file header for why it lives
// outside PlaceCard.razor), so every edge case below is a fast, precise
// xunit Theory rather than a browser round trip -- the actual end-to-end
// behavior (real measured card, real CSS transform, real boundingBox) is
// separately proven live by world-hover-text.spec.ts's own top-edge tests.
//
// Fix round 1 (review finding, Important): every call below now passes
// `containerHeight` (the parameter the original cut of this function was
// missing entirely). Tests that predate fix round 1 use a generous 900px
// container height specifically so they keep isolating the SAME thing they
// always tested (the top-edge flip boundary, the horizontal clamp) without
// also tripping the new vertical-clamp branch -- 900px comfortably fits
// every card height used below on whichever side wins. The new vertical
// tests below (from AiMarkerBottomOverflow... on) are what actually exercise
// containerHeight.
public class CardPlacementTests
{
    // A marker comfortably mid-map: plenty of room above (matches app.css's
    // pre-existing default placement), and the naive centered position
    // already clears both container edges with EdgeMarginPx to spare -- the
    // overwhelmingly common case, and it must come back a complete no-op
    // (DxPx exactly 0, Flipped false) so an ordinary card's rendered
    // position is byte-for-byte what it always was before this batch.
    [Fact]
    public void ComfortableMidMapCardIsUntouched()
    {
        var (dx, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 400, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: 1440, containerHeight: 900);
        Assert.Equal(0, dx);
        Assert.Equal(0, dyAdjust);
        Assert.False(flipped);
    }

    // Exactly flush (zero pixels to spare above) still counts as fitting --
    // matches this class's own documented `<=` fits-above boundary.
    [Fact]
    public void ExactlyFlushAboveDoesNotFlip()
    {
        // anchorY - Gap - height == 0
        var (_, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 200, anchorY: 218, cardWidth: 300, cardHeight: 200, containerWidth: 1440, containerHeight: 900);
        Assert.False(flipped);
        Assert.Equal(0, dyAdjust);
    }

    // One pixel short of fitting above -- but a generous 900px container
    // means it comfortably fits BELOW instead, so this still isolates the
    // pre-fix-round-1 "flips when it doesn't fit above" behavior with no
    // vertical clamp involved (dyAdjust stays 0).
    [Fact]
    public void OnePixelShortOfFittingAboveFlipsAndFitsBelowCleanly()
    {
        var (_, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 200, anchorY: 217, cardWidth: 300, cardHeight: 200, containerWidth: 1440, containerHeight: 900);
        Assert.True(flipped);
        Assert.Equal(0, dyAdjust);
    }

    // The controller's own reproduction, reconstructed: a marker near
    // containerPoint y=82 with a real card height (measured live at ~359px
    // for Sidon's own card, see the batch report) has nowhere near enough
    // room above, and a real (900px) viewport has plenty of room below --
    // must flip, no vertical clamp needed.
    [Fact]
    public void TopEdgeMarkerFlips()
    {
        var (_, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 878, anchorY: 82, cardWidth: 336, cardHeight: 359, containerWidth: 1440, containerHeight: 900);
        Assert.True(flipped);
        Assert.Equal(0, dyAdjust);
    }

    // A marker near the LEFT edge: the naive -50% centering would push the
    // card's own left edge past the container's, so DxPx must be positive
    // (nudges the card rightward) and large enough that the corrected left
    // edge lands exactly at EdgeMarginPx.
    [Fact]
    public void NearLeftEdgeClampsRightward()
    {
        var (dx, _, _) = CardPlacement.Compute(anchorX: 20, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: 1440, containerHeight: 900);
        var naiveLeft = 20 - 300.0 / 2; // -130
        var correctedLeft = naiveLeft + dx;
        Assert.Equal(CardPlacement.EdgeMarginPx, correctedLeft, 3);
        Assert.True(dx > 0);
    }

    // Symmetric case at the RIGHT edge: DxPx must be negative (nudges the
    // card leftward), landing its right edge exactly EdgeMarginPx inside
    // the container's own right edge.
    [Fact]
    public void NearRightEdgeClampsLeftward()
    {
        const double containerWidth = 1440;
        var (dx, _, _) = CardPlacement.Compute(anchorX: 1420, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: containerWidth, containerHeight: 900);
        var naiveLeft = 1420 - 300.0 / 2;
        var correctedLeft = naiveLeft + dx;
        Assert.Equal(containerWidth - CardPlacement.EdgeMarginPx - 300, correctedLeft, 3);
        Assert.True(dx < 0);
    }

    // A container narrower than the card plus both margins (a very small
    // pane) must not invert the clamp (min > max) -- centers within
    // whatever room actually exists instead.
    [Fact]
    public void ContainerNarrowerThanCardCentersInstead()
    {
        var (dx, _, _) = CardPlacement.Compute(anchorX: 50, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: 250, containerHeight: 900);
        var naiveLeft = 50 - 300.0 / 2; // -100
        var correctedLeft = naiveLeft + dx;
        Assert.Equal((250 - 300) / 2.0, correctedLeft, 3);
    }

    // Flip and horizontal clamp are independent decisions -- a marker that
    // needs both at once (top-left corner, but still a generous 900px-tall
    // container so it fits below cleanly) gets both corrections together,
    // no vertical clamp needed.
    [Fact]
    public void FlipAndHorizontalClampCanCombine()
    {
        var (dx, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 10, anchorY: 30, cardWidth: 300, cardHeight: 200, containerWidth: 1440, containerHeight: 900);
        Assert.True(flipped);
        Assert.True(dx > 0);
        Assert.Equal(0, dyAdjust);
    }

    // --- Fix round 1 (review finding, Important): vertical fit/clamp ---

    // The review's own live-reproduced regression, reconstructed exactly
    // from its reported numbers (not approximated): hovering the "Ai"
    // marker in the exodus scene at a 1280x720 viewport produced, under the
    // PRE-fix-round-1 code (which only ever asked "does it fit above"),
    // data-flip="true" with cardBox {y: 426, height: 416} -- i.e. an
    // unclamped flipped top of anchorY + 18 = 426, so anchorY = 408 (exact,
    // back-computed from the review's own reported y). Bottom edge
    // 426 + 416 = 842, exactly the review's own reported "122px past the
    // bottom of the viewport" (842 - 720 = 122). Neither orientation
    // actually fits here (spaceAbove = 408-18 = 390 < 416; spaceBelow =
    // 720-408-18 = 294 < 416) -- this is the exact "neither fits" branch
    // fix round 1 adds. Since spaceAbove (390) > spaceBelow (294), the fix
    // chooses ABOVE (not flipped) -- showing more of the card -- then
    // clamps: naive top = 408-18-416 = -26, clamped to EdgeMarginPx (8),
    // so the corrected box is fully on-screen: top 8, bottom 424, both
    // within [0, 720].
    [Fact]
    public void AiMarkerBottomOverflowScenarioNowClampsFullyOnScreen()
    {
        const double containerHeight = 720;
        const double cardHeight = 416;
        const double anchorY = 408;

        var (_, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 648, anchorY: anchorY, cardWidth: 336, cardHeight: cardHeight, containerWidth: 1280, containerHeight: containerHeight);

        Assert.False(flipped); // chooses ABOVE -- more room (390px) than below (294px)
        var naiveTop = anchorY - CardPlacement.GapPx - cardHeight; // -26
        var correctedTop = naiveTop + dyAdjust;
        Assert.Equal(CardPlacement.EdgeMarginPx, correctedTop, 3);
        Assert.True(correctedTop >= 0, "top edge must not cross the container's own top");
        Assert.True(correctedTop + cardHeight <= containerHeight, "bottom edge must not cross the container's own bottom");
    }

    // Symmetric case: BELOW has strictly more room than above, and even
    // below doesn't fully fit -- the fix must choose below (showing more of
    // the card) and clamp against the container's own BOTTOM edge this time.
    [Fact]
    public void NeitherFitsButBelowHasMoreRoomChoosesBelowAndClampsToBottom()
    {
        const double containerHeight = 700;
        const double cardHeight = 600;
        const double anchorY = 100; // spaceAbove = 82, spaceBelow = 582 -- neither fits 600, below has far more room

        var (_, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 400, anchorY: anchorY, cardWidth: 300, cardHeight: cardHeight, containerWidth: 1440, containerHeight: containerHeight);

        Assert.True(flipped);
        var naiveTop = anchorY + CardPlacement.GapPx; // 118
        var correctedTop = naiveTop + dyAdjust;
        Assert.Equal(containerHeight - CardPlacement.EdgeMarginPx - cardHeight, correctedTop, 3); // clamped to maxTop
        Assert.True(correctedTop >= 0);
        Assert.True(correctedTop + cardHeight <= containerHeight);
    }

    // A container SHORTER than the card itself (even ignoring the marker
    // gap entirely) must not invert the vertical clamp -- centers within
    // whatever room exists instead, the exact vertical mirror of
    // ContainerNarrowerThanCardCentersInstead above.
    [Fact]
    public void ContainerShorterThanCardCentersVerticallyInstead()
    {
        const double containerHeight = 150;
        const double cardHeight = 400;

        var (_, dyAdjust, _) = CardPlacement.Compute(anchorX: 400, anchorY: 75, cardWidth: 300, cardHeight: cardHeight, containerWidth: 1440, containerHeight: containerHeight);

        // naiveTop for the (necessarily) chosen orientation, whichever it is,
        // plus dyAdjust must land exactly at the centered fallback.
        var spaceAbove = 75 - CardPlacement.GapPx;
        var spaceBelow = containerHeight - 75 - CardPlacement.GapPx;
        var flipped = spaceBelow > spaceAbove;
        var naiveTop = flipped ? 75 + CardPlacement.GapPx : 75 - CardPlacement.GapPx - cardHeight;
        var correctedTop = naiveTop + dyAdjust;
        Assert.Equal((containerHeight - cardHeight) / 2.0, correctedTop, 3);
    }

    // Vertical and horizontal clamp are independent decisions -- fix round
    // 1's own vertical clamp and the pre-existing horizontal one must both
    // apply together without interfering, mirroring
    // FlipAndHorizontalClampCanCombine above but for a card that ALSO fails
    // to fit either orientation vertically. anchorX=15 (near the left edge,
    // exercises the horizontal clamp) with anchorY=40 in a 650px-tall
    // container and a 600px-tall card: spaceAbove=22, spaceBelow=592 --
    // neither fits 600, below wins (more room) but still needs its own
    // clamp against the container's bottom (592 < 600).
    [Fact]
    public void VerticalAndHorizontalClampCanCombine()
    {
        var (dx, dyAdjust, flipped) = CardPlacement.Compute(anchorX: 15, anchorY: 40, cardWidth: 300, cardHeight: 600, containerWidth: 1440, containerHeight: 650);

        Assert.True(dx > 0); // horizontal clamp still nudges rightward off the left edge
        Assert.True(flipped); // spaceAbove=22 vs spaceBelow=592 -- below wins
        Assert.NotEqual(0, dyAdjust); // and even below (592 < 600) still needs the vertical clamp
    }
}
