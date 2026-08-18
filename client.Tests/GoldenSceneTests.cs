using System.Text.Json;

namespace BibleAtlas.Client.Tests;

public class GoldenSceneTests
{
    [Fact]
    public void GoldenFixtureRoundTrips()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "../../../../tests/fixtures/golden-scene.json");
        var json = File.ReadAllText(path);
        var scene = JsonSerializer.Deserialize<Scene>(json, Wire.Options)!;

        Assert.Equal("time", scene.Mode);
        Assert.Equal(-1450, scene.Window!.FromYear);
        Assert.Equal("jericho", scene.Places[0].Id);
        Assert.Equal(27, scene.Places[0].Events[0].VerseGroups[0].Count);
        Assert.Equal("conquest", scene.Arrows[0].Narrative);
        Assert.Equal(2, scene.Arrows[0].Order);

        var back = JsonSerializer.Serialize(scene, Wire.Options);
        var reparsed = JsonSerializer.Deserialize<Scene>(back, Wire.Options)!;

        // Record-generated equality compares List<> members by reference, not
        // content, so `Assert.Equal(scene, reparsed with { })` would fail on
        // this fixture's populated lists regardless of content equality.
        // Compare via re-serialized JSON strings instead (brief's correction
        // to its own Step 1 sketch).
        Assert.Equal(JsonSerializer.Serialize(scene, Wire.Options), JsonSerializer.Serialize(reparsed, Wire.Options));
    }
}
