namespace BibleAtlas.Client.Tests;

// HOTFIX batch (batch-hotfix-brief.md requirement 1): CardPlacement.Compute
// is a plain numeric function (see its own file header for why it lives
// outside PlaceCard.razor), so every edge case below is a fast, precise
// xunit Theory rather than a browser round trip -- the actual end-to-end
// behavior (real measured card, real CSS transform, real boundingBox) is
// separately proven live by world-hover-text.spec.ts's own top-edge tests.
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
        var (dx, flipped) = CardPlacement.Compute(anchorX: 400, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: 1440);
        Assert.Equal(0, dx);
        Assert.False(flipped);
    }

    // Exactly flush (zero pixels to spare above) still counts as fitting --
    // matches this class's own documented `< 0`, not `<= 0`, boundary.
    [Fact]
    public void ExactlyFlushAboveDoesNotFlip()
    {
        // anchorY - Gap - height == 0
        var (_, flipped) = CardPlacement.Compute(anchorX: 200, anchorY: 218, cardWidth: 300, cardHeight: 200, containerWidth: 1440);
        Assert.False(flipped);
    }

    // One pixel short of fitting flips -- the exact top-edge cut-off bug
    // this requirement exists to fix (Sidon at containerPoint y~=77-82,
    // controller-reproduced boundingBox.y ~= -171..-295 before this fix).
    [Fact]
    public void OnePixelShortOfFittingFlips()
    {
        var (_, flipped) = CardPlacement.Compute(anchorX: 200, anchorY: 217, cardWidth: 300, cardHeight: 200, containerWidth: 1440);
        Assert.True(flipped);
    }

    // The controller's own reproduction, reconstructed: a marker near
    // containerPoint y=82 with a real card height (measured live at ~359px
    // for Sidon's own card, see the batch report) has nowhere near enough
    // room above -- must flip.
    [Fact]
    public void TopEdgeMarkerFlips()
    {
        var (_, flipped) = CardPlacement.Compute(anchorX: 878, anchorY: 82, cardWidth: 336, cardHeight: 359, containerWidth: 1440);
        Assert.True(flipped);
    }

    // A marker near the LEFT edge: the naive -50% centering would push the
    // card's own left edge past the container's, so DxPx must be positive
    // (nudges the card rightward) and large enough that the corrected left
    // edge lands exactly at EdgeMarginPx.
    [Fact]
    public void NearLeftEdgeClampsRightward()
    {
        var (dx, _) = CardPlacement.Compute(anchorX: 20, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: 1440);
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
        var (dx, _) = CardPlacement.Compute(anchorX: 1420, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: containerWidth);
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
        var (dx, _) = CardPlacement.Compute(anchorX: 50, anchorY: 400, cardWidth: 300, cardHeight: 200, containerWidth: 250);
        var naiveLeft = 50 - 300.0 / 2; // -100
        var correctedLeft = naiveLeft + dx;
        Assert.Equal((250 - 300) / 2.0, correctedLeft, 3);
    }

    // Flip and horizontal clamp are independent decisions -- a marker that
    // needs both at once (top-left corner) gets both corrections together.
    [Fact]
    public void FlipAndClampCanCombine()
    {
        var (dx, flipped) = CardPlacement.Compute(anchorX: 10, anchorY: 30, cardWidth: 300, cardHeight: 200, containerWidth: 1440);
        Assert.True(flipped);
        Assert.True(dx > 0);
    }
}
