using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class PresentationTests
{
    private const string Genesis1Label = "Genesis 1";
    private const string Kjv = "kjv";

    [Fact]
    public void Every_kind_is_a_card_on_the_popover_and_has_its_own_form_on_at_most_one_home_surface()
    {
        // Arrange
        var everyKind = Enum.GetValues<NodeKind>();

        // Act
        var table = everyKind
            .Select(kind => (kind, Presentation.Of(kind, Surface.World), Presentation.Of(kind, Surface.Reader), Presentation.Of(kind, Surface.Popover)))
            .ToList();

        // Assert
        Assert.Equal(
            [
                (NodeKind.TextUnit, null, Presentation.Form.Text, Presentation.Form.Card),
                (NodeKind.Container, null, Presentation.Form.Sequence, Presentation.Form.Card),
                (NodeKind.Event, null, null, Presentation.Form.Card),
                (NodeKind.Narrative, null, null, Presentation.Form.Card),
                (NodeKind.Place, Presentation.Form.Geography, null, Presentation.Form.Card),
                (NodeKind.Person, null, null, Presentation.Form.Card),
                (NodeKind.Anchor, null, null, Presentation.Form.Card),
                (NodeKind.Era, Presentation.Form.Geography, null, Presentation.Form.Card),
                (NodeKind.Polity, Presentation.Form.Geography, null, Presentation.Form.Card),
                (NodeKind.CatechismItem, null, null, Presentation.Form.Card),
                (NodeKind.Source, null, null, Presentation.Form.Card),
                (NodeKind.Translation, null, null, Presentation.Form.Card),
                (NodeKind.PeopleGroup, null, null, Presentation.Form.Card),
                (NodeKind.CommentaryItem, null, null, Presentation.Form.Card),
                (NodeKind.LexiconEntry, null, null, Presentation.Form.Card),
                (NodeKind.Map, Presentation.Form.Geography, null, Presentation.Form.Card),
            ],
            table);
    }

    [Fact]
    public void Every_kind_has_one_home_surface_derived_from_where_it_has_a_form_of_its_own()
    {
        // Arrange
        var everyKind = Enum.GetValues<NodeKind>();

        // Act
        var homes = everyKind.Select(kind => (kind, HomeSurfaces.Of(kind))).ToList();

        // Assert
        Assert.Equal(
            [
                (NodeKind.TextUnit, Surface.Reader),
                (NodeKind.Container, Surface.Reader),
                (NodeKind.Event, Surface.Popover),
                (NodeKind.Narrative, Surface.Popover),
                (NodeKind.Place, Surface.World),
                (NodeKind.Person, Surface.Popover),
                (NodeKind.Anchor, Surface.Popover),
                (NodeKind.Era, Surface.World),
                (NodeKind.Polity, Surface.World),
                (NodeKind.CatechismItem, Surface.Popover),
                (NodeKind.Source, Surface.Popover),
                (NodeKind.Translation, Surface.Popover),
                (NodeKind.PeopleGroup, Surface.Popover),
                (NodeKind.CommentaryItem, Surface.Popover),
                (NodeKind.LexiconEntry, Surface.Popover),
                (NodeKind.Map, Surface.World),
            ],
            homes);
    }

    [Fact]
    public void Two_cards_with_the_same_title_and_fields_are_equal_whatever_lists_hold_them()
    {
        // Arrange
        var provenance = new Presentation.Field("Provenance", Kjv);

        // Act
        var (a, b) = (new Presentation.Card(Genesis1Label, [provenance]), new Presentation.Card(Genesis1Label, new List<Presentation.Field> { provenance }));

        // Assert
        Assert.Equal((true, true), (a == b, a.GetHashCode() == b.GetHashCode()));
    }

    [Fact]
    public void Two_cards_with_different_fields_are_not_equal()
    {
        // Arrange
        var provenance = new Presentation.Field("Provenance", Kjv);

        // Act
        var (a, b) = (new Presentation.Card(Genesis1Label, [provenance]), new Presentation.Card(Genesis1Label, []));

        // Assert
        Assert.NotEqual(a, b);
    }
}
