namespace BibleAtlas.Client.Tests;

public class SliderScaleTests {
    static readonly List<EraDto> Eras = new() {
        new("a","A",-4004,-2167), new("b","B",-2166,-1877), new("c","C",-1876,-1407),
        new("d","D",-1406,-1051), new("e","E",-1050,-932), new("f","F",-931,-587),
        new("g","G",-586,-539), new("h","H",-538,-6), new("i","I",-5,29), new("j","J",30,100) };

    [Fact]
    public void RoundTripEveryYearInSpan() {
        for (int y = -4004; y <= 100; y++) {
            if (y == 0) continue;
            var x = SliderScale.YearToX(y, Eras, 1000.0);
            Assert.Equal(y, SliderScale.XToYear(x, Eras, 1000.0));
        }
    }
    [Fact]
    public void EveryEraGetsUsableWidth() {
        for (int i = 0; i < Eras.Count; i++) {
            var w0 = SliderScale.YearToX(Eras[i].FromYear, Eras, 1000.0);
            var w1 = SliderScale.YearToX(Eras[i].ToYear, Eras, 1000.0);
            Assert.True(w1 - w0 >= 1000.0 / (Eras.Count * 2) - 1e-6);
        }
    }
}
