namespace BibleAtlas.Client;

/// <summary>
/// Maps between a calendar year and a horizontal pixel position on the
/// TimeSlider's era-segmented strip, and back. The strip is divided into one
/// segment per <see cref="EraDto"/>, laid out left to right in era order.
/// Each era's segment width is proportional to how many years it spans, with
/// a floor of <c>width / (eras.Count * 2)</c> so a short era (Exile, Return,
/// Gospels) never collapses to an unusable sliver -- see <see cref="EraWidths"/>.
///
/// There is no year zero (1 BC is immediately followed by AD 1), so a plain
/// <c>year - era.FromYear</c> offset would miscount any era straddling the
/// boundary: era "gospels" (-5..29) spans 34 distinct years, not 35. All
/// position math below goes through an era-local, zero-aware year&lt;-&gt;index
/// conversion (<see cref="YearToLocalIndex"/> / <see cref="LocalIndexToYear"/>)
/// instead of raw year subtraction, so every year in an era maps to its own
/// 0-based index and vice versa, with index 0 never landing on year 0.
///
/// Consecutive eras' nominal segments are edge-to-edge (segment N's right
/// edge is exactly segment N+1's left edge), so the segments tile the full
/// strip width with no gap. If a year's x position were spread across the
/// segment's *full* nominal width, the last year of one era and the first
/// year of the next would land on the exact same pixel (both equal the
/// shared boundary) -- YearToX would stop being injective and XToYear
/// couldn't invert it. To keep every valid year's position distinct, each
/// era reserves a fixed, tiny <see cref="Epsilon"/> off the right end of its
/// own segment purely for spacing out its years -- far too small to affect
/// the "every era gets a usable width" guarantee, but enough that no two
/// years, in any two eras, ever share a pixel.
/// </summary>
public static class SliderScale
{
    // Reserved off the right edge of every era's segment purely so adjacent
    // eras' year positions never collide (see class doc). Comfortably above
    // double precision noise at these magnitudes (~1e-13) and comfortably
    // below the width tolerance callers should use (1e-6).
    private const double Epsilon = 1e-7;

    public static double YearToX(int year, IReadOnlyList<EraDto> eras, double width)
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

    public static int XToYear(double x, IReadOnlyList<EraDto> eras, double width)
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

    /// <summary>
    /// Each era's width: the larger of an equal floor share
    /// (<c>width / (eras.Count * 2)</c> -- half of an "average" share, so
    /// even the shortest era stays usable) and its proportional share of
    /// <paramref name="width"/> by year count. A straight max() over both
    /// can over-allocate (the floor lifting several short eras above their
    /// natural share adds up to more than <paramref name="width"/>), so eras
    /// pinned to the floor are set aside and the rest re-share the leftover
    /// width, repeating until no remaining era's proportional share falls
    /// under the floor. This mirrors a CSS flex layout with a min-width on
    /// every child (era segments render as flex divs) -- the returned widths
    /// sum to exactly <paramref name="width"/> and never fall below the floor.
    /// </summary>
    private static double[] EraWidths(IReadOnlyList<EraDto> eras, double width)
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
            // Degenerate case (every era's fair share undercuts the floor --
            // not reachable with real era data, but keep the total honest
            // rather than silently losing pixels if it ever happens).
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

    private static int FindEraForYear(int year, IReadOnlyList<EraDto> eras)
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

    /// <summary>
    /// Number of distinct (non-zero) years the era spans: <c>ToYear -
    /// FromYear + 1</c> for an era that doesn't straddle year zero, or one
    /// fewer when it does (FromYear &lt; 0 &lt; ToYear), since year 0 is
    /// skipped -- era "gospels" (-5..29) is 34 years, not 35.
    /// </summary>
    private static int EraYearCount(EraDto era) =>
        era.ToYear - era.FromYear + (era.FromYear < 0 && era.ToYear > 0 ? 0 : 1);

    /// <summary>
    /// 0-based position of <paramref name="year"/> within its era. For an
    /// era that straddles zero, years &lt;= -1 keep the plain offset from
    /// FromYear, and years &gt;= 1 continue the sequence one slot earlier
    /// than the raw offset would put them, since index 0 -- 1 was never
    /// spent on the nonexistent year 0.
    /// </summary>
    private static int YearToLocalIndex(int year, EraDto era)
    {
        if (era.FromYear < 0 && era.ToYear > 0)
        {
            return year > 0 ? year - era.FromYear - 1 : year - era.FromYear;
        }

        return year - era.FromYear;
    }

    /// <summary>Inverse of <see cref="YearToLocalIndex"/>.</summary>
    private static int LocalIndexToYear(int index, EraDto era)
    {
        if (era.FromYear < 0 && era.ToYear > 0)
        {
            var negativeCount = -era.FromYear; // how many negative years (FromYear..-1) precede the skip
            return index < negativeCount ? era.FromYear + index : index - negativeCount + 1;
        }

        return era.FromYear + index;
    }
}
