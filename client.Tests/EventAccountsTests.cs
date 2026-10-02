using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class EventAccountsTests
{
    private const string MatthewsNote = "Matt 5:1-7:29";

    private static readonly TextSpan MatthewsSermon = Span(BookId.MAT, (5, 1), (7, 29));
    private static readonly TextSpan LukesSermon = Span(BookId.LUK, (6, 17), (6, 49));

    [Fact]
    public async Task An_events_accounts_are_the_distinct_runs_its_attestations_are_served_with_read_across_every_page_and_an_attestation_served_without_runs_is_none()
    {
        // Arrange
        var atlas = new StubbedAtlas(new Dictionary<string, string>
        {
            [$"/api/node/Event%3Arob_sermon_on_the_mount/edges?kind=attested-in&limit={int.MaxValue}"] = Page(next: 2, Attestation("MAT.5.1", MatthewsSermon, MatthewsNote), Attestation("MAT.5.2", MatthewsSermon, MatthewsNote)),
            [$"/api/node/Event%3Arob_sermon_on_the_mount/edges?kind=attested-in&limit={int.MaxValue}&cursor=2"] = Page(next: null, Attestation("LUK.6.17", LukesSermon, note: null), Unplaced("LUK.6.18", "null"), Unplaced("LUK.6.19", "[]")),
        });

        // Act
        var accounts = await EventAccounts.ReadAsync(atlas.Graph(), "rob_sermon_on_the_mount");

        // Assert
        Assert.Equal(
            WholeValue.Of(new[] { new EventAccount([MatthewsSermon], MatthewsNote), new EventAccount([LukesSermon], null) }),
            WholeValue.Of(accounts));
    }

    [Fact]
    public void An_accounts_reference_names_its_first_run_whole_and_each_later_run_by_what_changes()
    {
        // Arrange
        var accounts = new EventAccount[]
        {
            new([Span(BookId._2KI, (1, 17), (1, 17)), Span(BookId._2KI, (8, 16), (8, 24))], null),
            new([Span(BookId.PSA, (96, 1), (96, 13)), Span(BookId.PSA, (105, 1), (105, 15)), Span(BookId.PSA, (106, 1), (106, 1)), Span(BookId.PSA, (106, 47), (106, 48))], null),
            new([Span(BookId.MAT, (4, 23), (4, 25)), Span(BookId.MAT, (5, 3), (7, 29))], null),
        };

        // Act
        var references = accounts.Select(account => (account.Reference, account.FirstVerse, account.FirstRunEnd)).ToList();

        // Assert
        Assert.Equal(
            [("2KI.1.17, 8.16-24", "2KI.1.17", "2KI.1.17"), ("PSA.96.1-13, 105.1-15, 106.1, 47-48", "PSA.96.1", "PSA.96.13"), ("MAT.4.23-25, 5.3-7.29", "MAT.4.23", "MAT.4.25")],
            references);
    }

    [Fact]
    public void An_account_reads_a_verse_inside_one_of_its_runs_and_no_other()
    {
        // Arrange
        var peterDenies = new EventAccount([Span(BookId.MRK, (14, 54), (14, 54)), Span(BookId.MRK, (14, 66), (14, 72))], null);
        var verses = new[] { new BibleRef(BookId.MRK, 14, 54), new BibleRef(BookId.MRK, 14, 60), new BibleRef(BookId.MRK, 14, 72), new BibleRef(BookId.MAT, 14, 54) };

        // Act
        var read = verses.Select(peterDenies.Reads).ToList();

        // Assert
        Assert.Equal([true, false, true, false], read);
    }

    internal static TextSpan Span(BookId book, (int Chapter, int Verse) from, (int Chapter, int Verse) to) =>
        new(from: new TextPoint(unit: new BibleRef(book, from.Chapter, from.Verse), word: null), to: new TextPoint(unit: new BibleRef(book, to.Chapter, to.Verse), word: null));

    private static string Page(int? next, params string[] entries) =>
        $$"""{"kind":"attested-in","entries":[{{string.Join(",", entries)}}],"next":{{(next is int n ? n.ToString() : "null")}},"version":"v"}""";

    private static string Unplaced(string verse, string loci) =>
        $$"""{"edge":{"id":"attests:{{verse}}","kind":"attested-in","label":"{{verse}}"},"node":{"id":"text-unit:{{verse}}","kind":"TextUnit","label":"{{verse}}"},"loci":{{loci}},"note":null}""";

    private static string Attestation(string verse, TextSpan run, string? note) =>
        $$"""{"edge":{"id":"attests:{{verse}}","kind":"attested-in","label":"{{verse}}"},"node":{"id":"text-unit:{{verse}}","kind":"TextUnit","label":"{{verse}}"},"loci":[{{WholeValue.Of(run)}}],"note":{{WholeValue.Of(note)}}}""";
}
