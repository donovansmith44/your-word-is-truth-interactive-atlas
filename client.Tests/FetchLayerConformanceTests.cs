using BibleAtlas.Client.Tests.State;
using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

public class FetchLayerConformanceTests
{
    private static readonly Regex FetchCall = new(@"\b(Atlas|Graph|atlas|Explorer|Maps)\.[A-Z]\w*\(|\.Entries\(", RegexOptions.Compiled);
    private static readonly Regex ThroughARequest = new(@"\.Fetch\(\(\) =>", RegexOptions.Compiled);
    private static readonly Regex TypedInput = new(@"<input[^>]*@oninput", RegexOptions.Compiled | RegexOptions.Singleline);
    private static readonly Regex DraftBound = new(@"value=""@\w+\.Text""", RegexOptions.Compiled);

    private static readonly Regex MapLayerFetchedDirectly = new(@"\bAtlas\.(Scene|Polities)", RegexOptions.Compiled);

    private static IEnumerable<(string File, int Line, string Text)> ComponentLines() =>
        ConformanceTests.ClientSourceFiles()
            .Where(f => f.EndsWith(".razor") || f.EndsWith("SliderWindow.cs"))
            .SelectMany(f => File.ReadAllLines(f).Select((text, i) => (Path.GetFileName(f), i + 1, text)));

    [Fact]
    public void Every_component_fetch_goes_through_a_request_that_drops_a_superseded_answer()
    {
        // Arrange
        var fetches = ComponentLines().Where(l => FetchCall.IsMatch(l.Text) && !l.Text.TrimStart().StartsWith("//"));

        // Act
        var outsideTheRequest = string.Join(", ", fetches.Where(l => !ThroughARequest.IsMatch(l.Text)).Select(l => $"{l.File}:{l.Line}"));

        // Assert
        Assert.Equal((true, ""), (fetches.Any(), outsideTheRequest));
    }

    [Fact]
    public void The_world_view_reads_its_map_layers_through_the_map_source_alone()
    {
        // Arrange
        var world = ComponentLines().Where(l => l.File == "World.razor");

        // Act
        var direct = string.Join(", ", world.Where(l => MapLayerFetchedDirectly.IsMatch(l.Text)).Select(l => $"{l.File}:{l.Line}"));

        // Assert
        Assert.Equal((true, ""), (world.Any(), direct));
    }

    [Fact]
    public void Resolving_following_presenting_and_paging_a_frontier_are_fetches_the_law_sees()
    {
        // Arrange
        var planted = new[]
        {
            "var node = await Explorer.Resolve(target);",
            "var next = await Explorer.Follow(link);",
            "var card = await Explorer.Present(node, Surface.Popover);",
            "var page = await node.Entries(kind, cursor);",
        };

        // Act
        var seen = planted.Select(line => FetchCall.IsMatch(line)).ToList();

        // Assert
        Assert.Equal([true, true, true, true], seen);
    }

    [Fact]
    public void Every_input_the_user_types_into_binds_its_value_to_a_draft()
    {
        // Arrange
        var inputs = ConformanceTests.ClientSourceFiles().Where(f => f.EndsWith(".razor"))
            .SelectMany(f => TypedInput.Matches(File.ReadAllText(f)).Select(m => (File: Path.GetFileName(f), Tag: m.Value)));

        // Act
        var boundElsewhere = string.Join(", ", inputs.Where(i => !DraftBound.IsMatch(i.Tag)).Select(i => i.File));

        // Assert
        Assert.Equal((true, ""), (inputs.Any(), boundElsewhere));
    }
}
