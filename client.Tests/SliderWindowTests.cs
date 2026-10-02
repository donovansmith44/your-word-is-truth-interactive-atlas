using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests;

public sealed class SliderWindowTests
{
    private const string GospelsToAd33Scene = """
        {"mode":"time","places":[],"quiet_places":[],"arrows":[],"narratives":[],
         "window":{"from":{"label":"5 BC","value":-5},"label":"5 BC – AD 33","to":{"label":"AD 33","value":33}}}
        """;

    private static readonly TimeRange GospelsToAd33 = new(from: new Year(label: "5 BC", value: -5), label: "5 BC – AD 33", to: new Year(label: "AD 33", value: 33));
    
    [Fact]
    public async Task A_window_no_time_scene_has_served_yet_is_labelled_by_the_server()
    {
        // Arrange
        var atlas = new StubbedAtlas(GospelsToAd33Scene);

        // Act
        var served = await SliderWindow.ServedFor(new RequestSeries().Next(), atlas.Client(), known: null, from: -5, to: 33);

        // Assert
        Assert.Equal(((Outcome<TimeRange?>)new Outcome<TimeRange?>.Arrived(GospelsToAd33), "/api/scene?from=-5&to=33"), (served, string.Join(" ", atlas.Asked)));
    }

    [Fact]
    public async Task A_window_already_served_is_not_asked_for_again()
    {
        // Arrange
        var atlas = new StubbedAtlas(GospelsToAd33Scene);

        // Act
        var served = await SliderWindow.ServedFor(new RequestSeries().Next(), atlas.Client(), known: GospelsToAd33, from: -5, to: 33);

        // Assert
        Assert.Equal(((Outcome<TimeRange?>)new Outcome<TimeRange?>.Arrived(GospelsToAd33), 0), (served, atlas.Asked.Count));
    }

    [Fact]
    public async Task A_window_served_for_other_years_is_asked_for_afresh()
    {
        // Arrange
        var atlas = new StubbedAtlas(GospelsToAd33Scene);
        var endingElsewhere = GospelsToAd33 with { To = new Year(label: "AD 30", value: 30) };
        var startingElsewhere = GospelsToAd33 with { From = new Year(label: "4 BC", value: -4) };

        // Act
        var served = new[]
        {
            await SliderWindow.ServedFor(new RequestSeries().Next(), atlas.Client(), known: endingElsewhere, from: -5, to: 33),
            await SliderWindow.ServedFor(new RequestSeries().Next(), atlas.Client(), known: startingElsewhere, from: -5, to: 33),
        };

        // Assert
        Assert.Equal(((Outcome<TimeRange?>)new Outcome<TimeRange?>.Arrived(GospelsToAd33), (Outcome<TimeRange?>)new Outcome<TimeRange?>.Arrived(GospelsToAd33), "/api/scene?from=-5&to=33 /api/scene?from=-5&to=33"), (served[0], served[1], string.Join(" ", atlas.Asked)));
    }

    [Fact]
    public async Task A_label_answered_after_a_newer_request_went_out_is_not_served()
    {
        // Arrange
        var atlas = new StubbedAtlas(GospelsToAd33Scene);
        var series = new RequestSeries();
        var request = series.Next();
        series.Next();

        // Act
        var served = await SliderWindow.ServedFor(request, atlas.Client(), known: null, from: -5, to: 33);

        // Assert
        Assert.Equal(((Outcome<TimeRange?>)new Outcome<TimeRange?>.Superseded(), "/api/scene?from=-5&to=33"), (served, string.Join(" ", atlas.Asked)));
    }
}
