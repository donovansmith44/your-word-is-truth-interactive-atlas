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
        // Batch E2 (the ever-present graph): quiet_places round-trips too --
        // the "both sides" golden regeneration the brief calls for.
        Assert.Equal("shiloh", scene.QuietPlaces[0].Id);
        Assert.Equal("Shiloh", scene.QuietPlaces[0].DisplayName);
        Assert.Equal(3, scene.QuietPlaces[0].TotalEvents);
        // Batch H (existence gating): shiloh's own real curated bounds
        // (established -1399, destroyed's own upper bound -1050) round-trip
        // onto the wire; jericho (no curated history in this fixture) keeps
        // both null -- pinning "no curated bounds -> always labels" on the
        // client side of the SAME golden regeneration.
        Assert.Equal(-1399, scene.QuietPlaces[0].ExistenceFrom);
        Assert.Equal(-1050, scene.QuietPlaces[0].ExistenceTo);
        Assert.Null(scene.Places[0].ExistenceFrom);
        Assert.Null(scene.Places[0].ExistenceTo);
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
