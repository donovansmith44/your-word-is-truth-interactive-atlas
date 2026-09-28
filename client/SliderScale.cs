using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

// Maps between a calendar year and a horizontal pixel position on the TimeSlider's era-segmented
// strip, and back.
//
// There is no year zero (1 BC is immediately followed by AD 1), so a plain `year - era.FromYear`
// offset would miscount any era straddling the boundary: era "gospels" (-5..29) spans 34 distinct
// years, not 35. All position math below goes through an era-local, zero-aware year<->index
// conversion (YearToLocalIndex / LocalIndexToYear) instead of raw year subtraction.
//
// Consecutive eras' nominal segments are edge-to-edge. If a year's x position were spread across
// the segment's full nominal width, the last year of one era and the first year of the next would
// land on the exact same pixel (both equal the shared boundary), and YearToX would stop being
// invertible. Each era reserves a fixed, tiny Epsilon off the right end of its own segment purely
// so no two years, in any two eras, ever share a pixel.
public static class SliderScale
{
    // Comfortably above double precision noise at these magnitudes (~1e-13) and comfortably below
    // the width tolerance callers should use (1e-6).
    private const double Epsilon = 1e-7;

    public static double YearToX(int year, IReadOnlyList<Era> eras, double width)
    {
        var widths = EraWidths(eras, width);
        var eraIndex = FindEraForYear(year, eras);
        var era = eras[eraIndex];
        var cumStart = CumulativeStart(widths, eraIndex);

        var count = EraYearCount(era);
        if (count <= 1)
        {
            return cumStart;
        }

        var index = YearToLocalIndex(year, era);
        var step = (widths[eraIndex] - Epsilon) / (count - 1);
        return cumStart + index * step;
    }

    public static int XToYear(double x, IReadOnlyList<Era> eras, double width)
    {
        var widths = EraWidths(eras, width);
        var clampedX = Math.Clamp(x, 0.0, width);
        var eraIndex = FindEraForX(clampedX, widths);
        var era = eras[eraIndex];
        var cumStart = CumulativeStart(widths, eraIndex);

        var count = EraYearCount(era);
        if (count <= 1)
        {
            return era.FromYear;
        }

        var step = (widths[eraIndex] - Epsilon) / (count - 1);
        var rawIndex = (clampedX - cumStart) / step;
        var index = (int)Math.Round(Math.Clamp(rawIndex, 0.0, count - 1), MidpointRounding.AwayFromZero);
        return LocalIndexToYear(index, era);
    }

    // Each era's width is the larger of an equal floor share (width / (eras.Count * 2)) and its
    // proportional share of width by year count. A single max() pass can over-allocate (floor-
    // lifted short eras can sum past width), so eras pinned to the floor are set aside and the
    // remainder re-shares the leftover width, repeating until stable. The returned widths sum to
    // exactly width and never fall below the floor.
    //
    // Public so TimeSlider.razor can render each band at this exact pixel width -- the visual band
    // and the handle math must use the same numbers, not two independent approximations (CSS flex
    // resolution does not land on the same per-era widths this computes).
    public static double[] EraWidths(IReadOnlyList<Era> eras, double width)
    {
        var n = eras.Count;
        var spans = new double[n];
        var totalSpan = 0.0;
        for (var i = 0; i < n; i++)
        {
            spans[i] = EraYearCount(eras[i]);
            totalSpan += spans[i];
        }

        var floor = width / (n * 2.0);
        var widths = new double[n];
        var pinned = new bool[n];
        var remainingWidth = width;
        var remainingSpan = totalSpan;
        var pinnedCount = 0;

        bool pinnedThisPass;
        do
        {
            pinnedThisPass = false;
            for (var i = 0; i < n; i++)
            {
                if (pinned[i])
                {
                    continue;
                }

                var share = remainingSpan > 0 ? remainingWidth * spans[i] / remainingSpan : 0.0;
                if (share < floor)
                {
                    widths[i] = floor;
                    pinned[i] = true;
                    pinnedCount++;
                    remainingWidth -= floor;
                    remainingSpan -= spans[i];
                    pinnedThisPass = true;
                }
            }
        } while (pinnedThisPass && pinnedCount < n);

        if (pinnedCount < n)
        {
            for (var i = 0; i < n; i++)
            {
                if (!pinned[i])
                {
                    widths[i] = remainingWidth * spans[i] / remainingSpan;
                }
            }
        }
        else if (remainingWidth > 0)
        {
            // Not reachable with real era data, but keeps the total honest instead of silently
            // losing pixels if every era's fair share ever undercuts the floor.
            var extra = remainingWidth / n;
            for (var i = 0; i < n; i++)
            {
                widths[i] += extra;
            }
        }

        return widths;
    }

    private static double CumulativeStart(IReadOnlyList<double> widths, int eraIndex)
    {
        var start = 0.0;
        for (var i = 0; i < eraIndex; i++)
        {
            start += widths[i];
        }

        return start;
    }

    private static int FindEraForYear(int year, IReadOnlyList<Era> eras)
    {
        for (var i = 0; i < eras.Count; i++)
        {
            if (year >= eras[i].FromYear && year <= eras[i].ToYear)
            {
                return i;
            }
        }

        return year < eras[0].FromYear ? 0 : eras.Count - 1;
    }

    private static int FindEraForX(double x, IReadOnlyList<double> widths)
    {
        var cumStart = 0.0;
        for (var i = 0; i < widths.Count - 1; i++)
        {
            var eraEnd = cumStart + widths[i];
            if (x < eraEnd)
            {
                return i;
            }

            cumStart = eraEnd;
        }

        return widths.Count - 1;
    }

    // ToYear - FromYear + 1 for an era that doesn't straddle year zero, or one fewer when it does
    // (FromYear < 0 < ToYear), since year 0 is skipped -- era "gospels" (-5..29) is 34 years, not 35.
    private static int EraYearCount(Era era) =>
        era.ToYear - era.FromYear + (era.FromYear < 0 && era.ToYear > 0 ? 0 : 1);

    // For an era that straddles zero, years <= -1 keep the plain offset from FromYear, and years
    // >= 1 continue one slot earlier than the raw offset would put them, since year 0 is skipped.
    private static int YearToLocalIndex(int year, Era era)
    {
        if (era.FromYear < 0 && era.ToYear > 0)
        {
            return year > 0 ? year - era.FromYear - 1 : year - era.FromYear;
        }

        return year - era.FromYear;
    }

    private static int LocalIndexToYear(int index, Era era)
    {
        if (era.FromYear < 0 && era.ToYear > 0)
        {
            var negativeCount = -era.FromYear; // negative years (FromYear..-1) preceding the skip
            return index < negativeCount ? era.FromYear + index : index - negativeCount + 1;
        }

        return era.FromYear + index;
    }
}
