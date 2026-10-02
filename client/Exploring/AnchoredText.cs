using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed record TextPiece(string Text, bool IsWordsOfChrist, Anchor? Anchor);

public static class AnchoredText
{
    public static IReadOnlyList<IReadOnlyList<TextPiece>> Runs(string text, IReadOnlyList<WordsOfChristSpan> wordsOfChrist, IReadOnlyList<Anchor> anchors)
    {
        var red = wordsOfChrist.Select(span => (Start: Utf16IndexOf(text, span.Start), End: Utf16IndexOf(text, span.End))).ToList();
        var anchored = anchors.Select(anchor => (Start: Utf16IndexOf(text, anchor.Start), End: Utf16IndexOf(text, anchor.End), Anchor: anchor)).ToList();
        var cuts = red.SelectMany(r => new[] { r.Start, r.End })
            .Concat(anchored.SelectMany(a => new[] { a.Start, a.End }))
            .Append(0)
            .Append(text.Length)
            .Distinct()
            .Order()
            .ToList();

        var runs = new List<List<TextPiece>>();
        foreach (var (start, end) in cuts.Zip(cuts.Skip(1)))
        {
            var isRed = red.Any(r => r.Start <= start && end <= r.End);
            var anchor = anchored.FirstOrDefault(a => a.Start <= start && end <= a.End).Anchor;
            var piece = new TextPiece(text[start..end], isRed, anchor);
            if (runs.Count > 0 && runs[^1][0].IsWordsOfChrist == isRed)
            {
                runs[^1].Add(piece);
            }
            else
            {
                runs.Add([piece]);
            }
        }
        return runs;
    }

    public static IReadOnlyList<TextPiece> Segments(string text, IReadOnlyList<Anchor> anchors) =>
        Runs(text, [], anchors).SelectMany(run => run).ToList();

    public static int ScalarOffsetOf(string text, int utf16Index) =>
        text[..utf16Index].EnumerateRunes().Count();

    private static int Utf16IndexOf(string text, int scalarOffset) =>
        text.EnumerateRunes().Take(scalarOffset).Sum(rune => rune.Utf16SequenceLength);
}
