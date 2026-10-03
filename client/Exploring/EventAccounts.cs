using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed record EventAccount(IReadOnlyList<TextSpan> Runs, string? Note)
{
    public string Reference => string.Join(", ", Runs.Select((run, i) => i == 0 ? CanonRef.SpanOf(run) : AfterRun(CanonRef.LastVerseOf(Runs[i - 1]), run)));

    public string FirstVerse => CanonRef.VerseOf(CanonRef.FirstVerseOf(Runs[0]));

    public string FirstRunEnd => CanonRef.VerseOf(CanonRef.LastVerseOf(Runs[0]));

    private static string AfterRun(BibleRef previousEnd, TextSpan run)
    {
        var (first, last) = (CanonRef.FirstVerseOf(run), CanonRef.LastVerseOf(run));
        if (first.Chapter != last.Chapter)
        {
            return $"{first.Chapter}.{first.Verse}-{last.Chapter}.{last.Verse}";
        }

        var verses = first.Verse == last.Verse ? $"{first.Verse}" : $"{first.Verse}-{last.Verse}";
        return first.Chapter == previousEnd.Chapter ? verses : $"{first.Chapter}.{verses}";
    }
}

public static class EventAccounts
{
    public static async Task<IReadOnlyList<EventAccount>> ReadAsync(IExplorableClient graph, string eventId)
    {
        var accounts = new List<EventAccount>();
        foreach (var entry in await Paging.Whole(graph, LegacyNodeIds.Of(NodeKind.Event, eventId), EdgeKind.AttestedIn))
        {
            if (entry.Loci is { Count: > 0 } runs && !accounts.Any(account => account.Runs.SequenceEqual(runs)))
            {
                accounts.Add(new EventAccount(runs, entry.Note));
            }
        }

        return accounts;
    }
}
