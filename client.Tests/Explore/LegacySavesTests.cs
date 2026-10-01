using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class LegacySavesTests
{
    private static readonly DateTimeOffset Saved = DateTimeOffset.Parse("2026-09-01T00:00:00+00:00");

    private static readonly V1Node Verse = new("Verse", "GEN.1.1", "GEN.1.1");
    private static readonly V1Node Chapter = new("Chapter", "GEN.1", "GEN.1");
    private static readonly V1Node Book = new("Book", "GEN", "GEN");
    private static readonly V1Node Place = new("Place", "hazor-1", "Hazor");
    private static readonly V1Node Person = new("Person", "Person:moses_2108", "Moses");
    private static readonly V1Node Event = new("Event", "ab_ur", "Terah's family leaves Ur");
    private static readonly V1Node Author = new("Author", "GEN", "Moses");
    private static readonly V1Node Year = new("Year", "hazor-1|Established", "Established 2000 BC");

    private static readonly NodeRef VerseRef = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1");
    private static readonly NodeRef ChapterRef = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-1", "GEN.1");
    private static readonly NodeRef BookRef = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "GEN");
    private static readonly NodeRef PlaceRef = ServedGraph.Ref(NodeKind.Place, "Place:hazor-1", "Hazor");
    private static readonly NodeRef PersonRef = ServedGraph.Ref(NodeKind.Person, "Person:moses_2108", "Moses");
    private static readonly NodeRef EventRef = ServedGraph.Ref(NodeKind.Event, "Event:ab_ur", "Terah's family leaves Ur");

    [Fact]
    public void Every_v1_step_kind_translates_to_the_node_that_stands_for_it_or_is_dropped()
    {
        // Arrange
        var v1 = new V1Node[]
        {
            Verse,
            new("ConcordUnit", "BoC 7.2.1", "BoC 7.2.1"),
            new("Passage", "GEN.1.1-5", "GEN.1.1-5"),
            Chapter,
            new("Chapter", "GEN", "GEN"),
            Book,
            Place,
            Person,
            Event,
            new("TimeAndPlace", "ur|ab_ur", "Ur, 2000 BC"),
            new("TimeAndPlace", "ur", "Ur"),
            new("Catechism", "commandment-1", "The First Commandment"),
            new("CommentaryItem", "kretzmann/0.1.0", "The Creation of Chaos and Light"),
            Author,
            Year,
            new("PolityDelta", "israel|Israel|transition|-931|-722", "Israel, 931 BC → 722 BC"),
            new("Polity", "Polity:egypt", "Egypt"),
        };

        // Act
        var translated = v1.Select(LegacySaves.Node).ToList();

        // Assert
        Assert.Equal(
            WholeValue.Of(new NodeRef?[]
            {
                VerseRef,
                ServedGraph.Ref(NodeKind.TextUnit, "text-unit:BoC 7.2.1", "BoC 7.2.1"),
                ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1-5"),
                ChapterRef,
                null,
                BookRef,
                PlaceRef,
                PersonRef,
                EventRef,
                ServedGraph.Ref(NodeKind.Event, "Event:ab_ur", "Ur, 2000 BC"),
                null,
                ServedGraph.Ref(NodeKind.CatechismItem, "CatechismItem:commandment-1", "The First Commandment"),
                ServedGraph.Ref(NodeKind.CommentaryItem, "CommentaryItem:kretzmann/0.1.0", "The Creation of Chaos and Light"),
                null,
                null,
                null,
                null,
            }),
            WholeValue.Of(translated));
    }

    [Fact]
    public void A_v1_save_becomes_a_start_and_steps_whose_kinds_are_read_from_their_ends_dropping_what_has_no_node()
    {
        // Arrange
        var v1 = new V1Exploration("seed1", "GEN.1.1 → Moses", Saved, [Verse, Chapter, Verse, Place, Event, Year, Person, Book, Author]);

        // Act
        var translated = LegacySaves.Exploration(v1);

        // Assert
        Assert.Equal(
            WholeValue.Of(new Translated<SavedExploration>(
                [
                    new SavedExploration("seed1", "GEN.1.1 → Moses", Saved, ServedGraph.At(VerseRef),
                    [
                        new Link(EdgeKind.MemberOf, ServedGraph.At(ChapterRef)),
                        new Link(EdgeKind.Contains, ServedGraph.At(VerseRef)),
                        new Link(EdgeKind.Mentions, ServedGraph.At(PlaceRef)),
                        new Link(EdgeKind.SiteOf, ServedGraph.At(EventRef)),
                        new Link(EdgeKind.MemberOf, ServedGraph.At(PersonRef)),
                        new Link(EdgeKind.MemberOf, ServedGraph.At(BookRef)),
                    ]),
                ],
                2)),
            WholeValue.Of(translated));
    }

    [Fact]
    public void A_v1_save_whose_first_steps_have_no_node_starts_at_the_first_that_does()
    {
        // Arrange
        var v1 = new V1Exploration("seed2", "Moses → GEN.1.1", Saved, [Author, Year, Verse]);

        // Act
        var translated = LegacySaves.Exploration(v1);

        // Assert
        Assert.Equal(
            WholeValue.Of(new Translated<SavedExploration>([new SavedExploration("seed2", "Moses → GEN.1.1", Saved, ServedGraph.At(VerseRef), [])], 2)),
            WholeValue.Of(translated));
    }

    [Fact]
    public void A_v1_save_with_no_surviving_step_is_dropped_whole()
    {
        // Arrange
        var v1 = new V1Exploration("seed3", "Moses", Saved, [Author, Year]);

        // Act
        var translated = LegacySaves.Exploration(v1);

        // Assert
        Assert.Equal(WholeValue.Of(new Translated<SavedExploration>([], 2)), WholeValue.Of(translated));
    }

    [Fact]
    public void A_v1_store_translates_every_save_and_sums_the_dropped_steps()
    {
        // Arrange
        var v1 = new V1Exploration[]
        {
            new("seed1", "GEN.1.1", Saved, [Verse, Author]),
            new("seed3", "Moses", Saved, [Author, Year]),
            new("seed4", "Hazor", Saved, [Place]),
        };

        // Act
        var translated = LegacySaves.Explorations(v1);

        // Assert
        Assert.Equal(
            WholeValue.Of(new Translated<SavedExploration>(
                [
                    new SavedExploration("seed1", "GEN.1.1", Saved, ServedGraph.At(VerseRef), []),
                    new SavedExploration("seed4", "Hazor", Saved, ServedGraph.At(PlaceRef), []),
                ],
                3)),
            WholeValue.Of(translated));
    }

    [Fact]
    public void A_v2_save_reads_as_v3_with_every_node_as_a_node_position()
    {
        // Arrange
        var v2 = new V2Exploration("seed5", "GEN.1.1 → Moses", Saved, VerseRef,
        [
            new V2Link(EdgeKind.MemberOf, ChapterRef),
            new V2Link(EdgeKind.SpouseOf, PersonRef),
        ]);

        // Act
        var translated = LegacySaves.Exploration(v2);

        // Assert
        Assert.Equal(
            WholeValue.Of(new SavedExploration("seed5", "GEN.1.1 → Moses", Saved, ServedGraph.At(VerseRef),
            [
                new Link(EdgeKind.MemberOf, ServedGraph.At(ChapterRef)),
                new Link(EdgeKind.SpouseOf, ServedGraph.At(PersonRef)),
            ])),
            WholeValue.Of(translated));
    }

    [Fact]
    public void A_v2_store_reads_as_v3_save_for_save()
    {
        // Arrange
        var v2 = new V2Exploration[]
        {
            new("seed5", "GEN.1.1", Saved, VerseRef, []),
            new("seed6", "Hazor", Saved, PlaceRef, [new V2Link(EdgeKind.MemberOf, ChapterRef)]),
        };

        // Act
        var translated = LegacySaves.Explorations(v2);

        // Assert
        Assert.Equal(
            WholeValue.Of(new[]
            {
                new SavedExploration("seed5", "GEN.1.1", Saved, ServedGraph.At(VerseRef), []),
                new SavedExploration("seed6", "Hazor", Saved, ServedGraph.At(PlaceRef), [new Link(EdgeKind.MemberOf, ServedGraph.At(ChapterRef))]),
            }),
            WholeValue.Of(translated));
    }

    [Fact]
    public void A_v1_selection_keeps_the_nodes_that_translate_and_counts_the_rest()
    {
        // Arrange
        var v1 = new[] { Verse, Author, Place };

        // Act
        var translated = LegacySaves.Nodes(v1);

        // Assert
        Assert.Equal(WholeValue.Of(new Translated<NodeRef>([VerseRef, PlaceRef], 1)), WholeValue.Of(translated));
    }
}
