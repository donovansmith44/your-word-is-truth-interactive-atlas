namespace BibleAtlas.Client;

public static class CardPlacement
{
    public const double GapPx = 18;

    public const double EdgeMarginPx = 8;

    public static (double DxPx, double DyAdjustPx, bool Flipped) Compute(
        double anchorX, double anchorY, double cardWidth, double cardHeight, double containerWidth, double containerHeight)
    {
        // Deliberately allowed to go negative (e.g. spaceAbove when the marker sits closer to the
        // container's top than GapPx alone accounts for); the comparisons below only compare, they
        // never assume either figure is non-negative.
        var spaceAbove = anchorY - GapPx;
        var spaceBelow = containerHeight - anchorY - GapPx;

        // <=, not <: a card that fits exactly flush (zero pixels to spare) still counts as fitting.
        var fitsAbove = cardHeight <= spaceAbove;
        var fitsBelow = cardHeight <= spaceBelow;

        bool flipped;
        double dyAdjust;

        if (fitsAbove)
        {
            flipped = false;
            dyAdjust = 0;
        }
        else if (fitsBelow)
        {
            flipped = true;
            dyAdjust = 0;
        }
        else
        {
            // Neither orientation fully fits: pick whichever side has strictly more room (ties
            // keep the "prefer above" bias), then clamp the resulting top edge into
            // [EdgeMarginPx, containerHeight - cardHeight - EdgeMarginPx].
            flipped = spaceBelow > spaceAbove;
            var naiveTop = flipped ? anchorY + GapPx : anchorY - GapPx - cardHeight;
            var minTop = EdgeMarginPx;
            var maxTop = containerHeight - cardHeight - EdgeMarginPx;

            double clampedTop;
            if (maxTop < minTop)
            {
                // Card taller than the container has room for at all (even ignoring the marker
                // gap) -- Math.Clamp would throw with min > max, so center in whatever room exists.
                clampedTop = (containerHeight - cardHeight) / 2;
            }
            else
            {
                clampedTop = Math.Clamp(naiveTop, minTop, maxTop);
            }

            dyAdjust = clampedTop - naiveTop;
        }

        var naiveLeft = anchorX - cardWidth / 2;
        var minLeft = EdgeMarginPx;
        var maxLeft = containerWidth - cardWidth - EdgeMarginPx;

        double clampedLeft;
        if (maxLeft < minLeft)
        {
            // Container narrower than the card plus both margins -- Math.Clamp would throw with
            // min > max, so center in whatever room exists instead.
            clampedLeft = (containerWidth - cardWidth) / 2;
        }
        else
        {
            clampedLeft = Math.Clamp(naiveLeft, minLeft, maxLeft);
        }

        return (clampedLeft - naiveLeft, dyAdjust, flipped);
    }
}
