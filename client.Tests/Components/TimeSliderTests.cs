using System.Globalization;
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using Bunit;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;

namespace BibleAtlas.Client.Tests;

public sealed class TimeSliderTests : BunitContext
{
    private const double TrackWidthPx = 520.0;

    private static readonly TimeRange Unread = new(from: new Year(label: "AD 1", value: 1), label: "AD 1", to: new Year(label: "AD 1", value: 1));

    private static readonly List<Era> Eras = new()
    {
        new(fromYear: -2166, id: "b", name: "B", node: new NodeRef(id: "Era:b", kind: NodeKind.Era, label: "B"), toYear: -1877, window: Unread),
        new(fromYear: -1876, id: "c", name: "C", node: new NodeRef(id: "Era:c", kind: NodeKind.Era, label: "C"), toYear: -1407, window: Unread),
        new(fromYear: -1406, id: "d", name: "D", node: new NodeRef(id: "Era:d", kind: NodeKind.Era, label: "D"), toYear: -1051, window: Unread),
        new(fromYear: -1050, id: "e", name: "E", node: new NodeRef(id: "Era:e", kind: NodeKind.Era, label: "E"), toYear: -932, window: Unread),
        new(fromYear: -931, id: "f", name: "F", node: new NodeRef(id: "Era:f", kind: NodeKind.Era, label: "F"), toYear: -587, window: Unread),
    };

    private static TimeRange Range(int from, int to) =>
        new(from: new Year(label: from.ToString(CultureInfo.InvariantCulture), value: from), label: $"{from}..{to}", to: new Year(label: to.ToString(CultureInfo.InvariantCulture), value: to));

    private static string Px(double value) => value.ToString(CultureInfo.InvariantCulture);

    private IRenderedComponent<TimeSlider> Slider(
        int from,
        int to,
        TimeRange? bounds = null,
        TimeRange? band = null,
        Action<(int From, int To)>? onChange = null,
        Action<(int From, int To)>? onDrag = null,
        Action<ArrowDirection>? onCross = null) =>
        Render<TimeSlider>(p =>
        {
            p.Add(s => s.Eras, Eras);
            p.Add(s => s.From, from);
            p.Add(s => s.To, to);
            p.Add(s => s.Bounds, bounds);
            p.Add(s => s.Band, band);
            if (onChange is not null)
            {
                p.Add(s => s.OnWindowChange, EventCallback.Factory.Create(this, onChange));
            }

            if (onDrag is not null)
            {
                p.Add(s => s.OnWindowDrag, EventCallback.Factory.Create(this, onDrag));
            }

            if (onCross is not null)
            {
                p.Add(s => s.OnCross, EventCallback.Factory.Create(this, onCross));
            }
        });

    [Fact]
    public void A_slider_without_bounds_draws_no_bounds_band_or_crossing()
    {
        // Arrange
        var slider = Slider(-1800, -1100);

        // Act
        var drawn = slider.FindAll("[data-testid^='world-']").Count;

        // Assert
        Assert.Equal(0, drawn);
    }

    [Fact]
    public async Task A_bounded_slider_cannot_be_dragged_past_its_bounds_and_reports_the_crossing()
    {
        // Arrange
        var bounds = Range(-1876, -1051);
        var dragged = new List<(int From, int To)>();
        var changed = new List<(int From, int To)>();
        var crossed = new List<ArrowDirection>();
        var slider = Slider(-1800, -1100, bounds, onChange: changed.Add, onDrag: dragged.Add, onCross: crossed.Add);
        var beyondX = SliderScale.YearToX(-900, Eras, TrackWidthPx);

        // Act
        await slider.Find(".slider-handle-to").TriggerEventAsync("onpointerdown", new PointerEventArgs());
        await slider.Find(".slider-track").TriggerEventAsync("onpointermove", new PointerEventArgs { OffsetX = beyondX });
        await slider.Find(".slider-track").TriggerEventAsync("onpointerup", new PointerEventArgs { OffsetX = beyondX });
        slider.Render(p => p.Add(s => s.To, -1051));
        slider.Find("[data-testid='world-cross-next']").Click();

        // Assert
        Assert.Equal(((-1800, -1051), (-1800, -1051), "Next"), (dragged[^1], changed.Single(), string.Join(",", crossed)));
    }

    [Fact]
    public void The_crossing_is_offered_only_at_the_bound_it_leads_past()
    {
        // Arrange
        var bounds = Range(-1876, -1051);

        // Act
        var inside = Slider(-1800, -1100, bounds);
        var atEnd = Slider(-1800, -1051, bounds);
        var atStart = Slider(-1876, -1100, bounds);

        // Assert
        Assert.Equal(
            ("", "world-cross-next", "world-cross-previous"),
            (Crossings(inside), Crossings(atEnd), Crossings(atStart)));
    }

    [Fact]
    public void A_band_is_drawn_under_the_window()
    {
        // Arrange
        var band = Range(-1700, -1200);
        var left = SliderScale.YearToX(-1700, Eras, TrackWidthPx);
        var width = SliderScale.YearToX(-1200, Eras, TrackWidthPx) - left;

        // Act
        var slider = Slider(-1800, -1100, Range(-1876, -1051), band);

        // Assert
        Assert.Equal($"left: {Px(left)}px; width: {Px(width)}px;", slider.Find("[data-testid='world-slider-band']").GetAttribute("style")!.Trim());
    }

    [Fact]
    public void The_bounds_are_drawn_on_the_track()
    {
        // Arrange
        var left = SliderScale.YearToX(-1876, Eras, TrackWidthPx);
        var width = SliderScale.YearToX(-1051, Eras, TrackWidthPx) - left;

        // Act
        var slider = Slider(-1800, -1100, Range(-1876, -1051));

        // Assert
        Assert.Equal($"left: {Px(left)}px; width: {Px(width)}px;", slider.Find("[data-testid='world-slider-bounds']").GetAttribute("style")!.Trim());
    }

    [Fact]
    public async Task An_era_wider_than_the_bounds_selects_only_the_bounded_part()
    {
        // Arrange
        var changed = new List<(int From, int To)>();
        var slider = Slider(-1800, -1100, Range(-1700, -1100), onChange: changed.Add);

        // Act
        await slider.Find("[data-testid='slider-era-c']").ClickAsync(new MouseEventArgs());

        // Assert
        Assert.Equal((-1700, -1407), changed.Single());
    }

    private static string Crossings(IRenderedComponent<TimeSlider> slider) =>
        string.Join(",", slider.FindAll("[data-testid^='world-cross-']").Select(e => e.GetAttribute("data-testid")!));
}
