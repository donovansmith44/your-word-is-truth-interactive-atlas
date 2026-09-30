namespace BibleAtlas.Client.Tests;

public sealed class DraftTests
{
    [Fact]
    public void A_served_label_fills_a_readout_nobody_has_typed_in()
    {
        // Arrange
        var readout = Draft.Empty;

        // Act
        var served = readout.Served("5 BC – AD 33");

        // Assert
        Assert.Equal((new Draft("5 BC – AD 33", null), "5 BC – AD 33"), (served, served.Text));
    }

    [Fact]
    public void A_label_served_while_the_user_is_typing_leaves_the_typed_text_in_place()
    {
        // Arrange
        var typing = Draft.Empty.Served("2426 – 411 BC").Type("785 – 2 BC");

        // Act
        var served = typing.Served("2426 – 411 BC");

        // Assert
        Assert.Equal((new Draft("2426 – 411 BC", "785 – 2 BC"), "785 – 2 BC"), (served, served.Text));
    }

    [Fact]
    public void Committed_text_stays_shown_until_the_next_label_is_served()
    {
        // Arrange
        var typed = Draft.Empty.Served("2426 – 411 BC").Type("785 – 2 BC");

        // Act
        var committed = typed.Committed();
        var relabelled = committed.Served("785 – 2 BC");

        // Assert
        Assert.Equal((new Draft("785 – 2 BC", null), new Draft("785 – 2 BC", null)), (committed, relabelled));
    }

    [Fact]
    public void A_discarded_draft_shows_the_last_served_label_again()
    {
        // Arrange
        var typed = Draft.Empty.Served("2426 – 411 BC").Type("78");

        // Act
        var discarded = typed.Discarded();

        // Assert
        Assert.Equal(new Draft("2426 – 411 BC", null), discarded);
    }
}
