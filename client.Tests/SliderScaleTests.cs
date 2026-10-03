using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests;

public class SliderScaleTests {
    static readonly TimeRange WindowSliderScaleNeverReads = new(from: new Year(label: "AD 1", value: 1), label: "AD 1", to: new Year(label: "AD 1", value: 1));
    static readonly NodeRef NodeSliderScaleNeverReads = new(id: Wire.Node("Era:a"), kind: NodeKind.Era, label: "A");
    static readonly List<Era> Eras = new() {
        new(fromYear: -4004, id: Wire.Read<EraId>("a"), name: "A", toYear: -2167, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -2166, id: Wire.Read<EraId>("b"), name: "B", toYear: -1877, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -1876, id: Wire.Read<EraId>("c"), name: "C", toYear: -1407, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -1406, id: Wire.Read<EraId>("d"), name: "D", toYear: -1051, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -1050, id: Wire.Read<EraId>("e"), name: "E", toYear: -932, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -931, id: Wire.Read<EraId>("f"), name: "F", toYear: -587, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -586, id: Wire.Read<EraId>("g"), name: "G", toYear: -539, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -538, id: Wire.Read<EraId>("h"), name: "H", toYear: -6, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: -5, id: Wire.Read<EraId>("i"), name: "I", toYear: 29, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads),
        new(fromYear: 30, id: Wire.Read<EraId>("j"), name: "J", toYear: 100, window: WindowSliderScaleNeverReads, node: NodeSliderScaleNeverReads) };

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
