using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class NamespaceLawTests
{
    private static readonly string[] Reported = ["CompositionSplit", "Contract"];

    [Fact]
    public void No_client_type_is_named_for_a_client_namespace_beyond_the_two_reported()
    {
        // Arrange
        var types = new[] { typeof(Explorable).Assembly, typeof(AtlasClient).Assembly }.SelectMany(assembly => assembly.GetTypes()).Where(type => !type.IsNested && !type.Name.StartsWith('<')).ToList();
        var segments = types.Select(type => type.Namespace).OfType<string>().SelectMany(name => name.Split('.')).ToHashSet();

        // Act
        var clashes = types.Select(type => type.Name.Split('`')[0]).Where(segments.Contains).Distinct().Order().ToList();

        // Assert
        Assert.Equal(Reported, clashes);
    }
}
