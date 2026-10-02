using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Tests.State;
using YamlDotNet.RepresentationModel;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class FirstPageTests
{
    private static readonly string[] PagedReads = ["/api/elements", "/api/node/{id}/edges"];

    [Fact]
    public void The_page_store_keys_the_first_page_at_the_cursor_every_paged_read_publishes_as_its_default()
    {
        // Arrange
        var document = new YamlStream();
        document.Load(new StringReader(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "openapi.yaml"))));
        var paths = (YamlMappingNode)((YamlMappingNode)document.Documents[0].RootNode)["paths"];

        // Act
        var defaults = PagedReads
            .Select(path => ((YamlSequenceNode)((YamlMappingNode)((YamlMappingNode)paths[path])["get"])["parameters"])
                .Cast<YamlMappingNode>()
                .Single(parameter => parameter["name"].ToString() == "cursor"))
            .Select(cursor => int.Parse(((YamlMappingNode)cursor["schema"])["default"].ToString()))
            .ToList();

        // Assert
        Assert.Equal(PagedReads.Select(_ => ServedPages.FirstPage), defaults);
    }
}
