using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using static BibleAtlas.Client.Tests.EventAccountsTests;

namespace BibleAtlas.Client.Tests;

public class ArrowNavTests
{
    [Fact]
    public void An_adjacent_events_refs_are_its_served_accounts_each_opening_at_its_first_verse()
    {
        // Arrange
        var accounts = new[]
        {
            new EventAccount([Span(BookId.LUK, (6, 12), (6, 16))], null),
            new EventAccount([Span(BookId.MRK, (14, 54), (14, 54)), Span(BookId.MRK, (14, 66), (14, 72))], null),
            new EventAccount([Span(BookId.MAT, (5, 1), (7, 29))], null),
        };

        // Act
        var refs = ArrowNav.SelectRefs(accounts).Select(r => (r.Ref, ((VerseNode)((PopoverOpening.Legacy)r.Target).Node).Title)).ToList();

        // Assert
        Assert.Equal([("LUK.6.12-16", "LUK.6.12"), ("MRK.14.54, 66-72", "MRK.14.54"), ("MAT.5.1-7.29", "MAT.5.1")], refs);
    }

    [Fact]
    public void An_adjacent_event_with_no_accounts_has_no_refs()
    {
        // Act
        var refs = ArrowNav.SelectRefs([]);

        // Assert
        Assert.Equal([], refs);
    }
}
