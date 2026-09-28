using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// Batch PERI-1 (PRESENTATION CATEGORY LAW -- owner, verbatim: "NUN is not
/// an event. fix this error and others like it"): pins
/// <see cref="EventMembershipHeading.For"/>'s own exhaustive kind-&gt;heading
/// mapping directly (client/Explore/PopoverSectionProviders.cs) -- the
/// conformance corollary the batch brief's own machine rules ask for: "the
/// section provider's switch is exhaustive over kinds with the general arm
/// mapped to the passage heading, pinned by a client.Tests case." A future
/// edit that silently defaults an unrecognized `Event::kind` instead of
/// throwing (the pre-PERI-1 shape this class replaced) fails this test.
/// </summary>
public class EventMembershipHeadingTests {
    [Fact] public void EventKindMapsToEventHeading() => Assert.Equal("EVENT", EventMembershipHeading.For(EventKind.Event));

    [Fact] public void GeneralKindMapsToPassageHeading() => Assert.Equal("PASSAGE", EventMembershipHeading.For(EventKind.General));

    [Fact]
    public void UndeclaredKindThrowsRatherThanDefaulting() =>
        Assert.Throws<NotSupportedException>(() => EventMembershipHeading.For(UndeclaredKind));

    private static readonly EventKind UndeclaredKind = (EventKind)Enum.GetValues<EventKind>().Length;
}
