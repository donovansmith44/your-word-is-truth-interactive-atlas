using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Tests.State;
using YamlDotNet.RepresentationModel;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class FirstPageTests
{
    [Fact]
    public void Every_paged_read_the_document_publishes_defaults_its_cursor_to_the_generated_first_page()
    {
        // Arrange
        var document = new YamlStream();
        document.Load(new StringReader(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "openapi.yaml"))));
        var paths = (YamlMappingNode)((YamlMappingNode)document.Documents[0].RootNode)["paths"];

        // Act
        var cursors = paths.Children
            .SelectMany(path => ((YamlMappingNode)path.Value).Children.Select(operation => (Operation: $"{operation.Key} {path.Key}", Body: (YamlMappingNode)operation.Value)))
            .SelectMany(operation => Parameters(operation.Body)
                .Where(parameter => parameter["name"].ToString() == "cursor" && parameter["in"].ToString() == "query")
                .Select(cursor => (operation.Operation, Default: Default((YamlMappingNode)cursor["schema"]))))
            .ToList();

        // Assert
        Assert.NotEmpty(cursors);
        Assert.Equal(cursors.Select(cursor => (cursor.Operation, (int?)PagedReads.FirstPage)), cursors.Select(cursor => (cursor.Operation, cursor.Default)));
    }

    private static IEnumerable<YamlMappingNode> Parameters(YamlMappingNode operation) =>
        operation.Children.TryGetValue(new YamlScalarNode("parameters"), out var parameters) ? ((YamlSequenceNode)parameters).Cast<YamlMappingNode>() : [];

    private static int? Default(YamlMappingNode schema) =>
        schema.Children.TryGetValue(new YamlScalarNode("default"), out var given) ? int.Parse(given.ToString()) : null;
}
