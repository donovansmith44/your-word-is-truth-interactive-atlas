using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public class EventMembershipHeadingTests {
    [Fact] public void EventKindMapsToEventHeading() => Assert.Equal("EVENT", EventMembershipHeading.For(EventKind.Event));

    [Fact] public void GeneralKindMapsToPassageHeading() => Assert.Equal("PASSAGE", EventMembershipHeading.For(EventKind.General));

    [Fact]
    public void UndeclaredKindThrowsRatherThanDefaulting() =>
        Assert.Throws<NotSupportedException>(() => EventMembershipHeading.For(UndeclaredKind));

    private static readonly EventKind UndeclaredKind = (EventKind)Enum.GetValues<EventKind>().Length;
}
