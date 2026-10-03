using System.Reflection;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.ContractGenerator;
using BibleAtlas.Client.Tests.State;
using NJsonSchema;
using NSwag;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class IdentityTypesTests
{
    private const string ContractNamespace = "BibleAtlas.Client.Contract";
    private const string Widen = "op_Implicit";
    private const string Converter = "Json";
    private const string None = "";

    [Fact]
    public async Task Every_named_scalar_and_every_union_of_them_the_document_publishes_is_an_identity()
    {
        // Arrange
        var document = await Published();

        // Act
        var identities = IdentityTypes.Of(document).Values.Order(StringComparer.Ordinal);

        // Assert
        Assert.Equal(
            [
                "ArtifactRoot", "BibleReference", "ChapterReference", "ConcordReference", "ContentsReference", "CrossReferenceTarget", "EdgeId", "EdgePageCursor", "ElementId",
                "ElementPageCursor", "EraId", "NarrativeId", "NodeId", "PassageReference", "PolityId", "TextWindowReference", "UnitReference", "VerseRangeReference", "VerseReference",
                "VerseSpanReference",
            ],
            identities);
    }

    [Fact]
    public async Task Identities_convert_only_along_the_documents_unions()
    {
        // Arrange
        var document = await Published();
        var generated = IdentityTypes.Of(document).Values.Select(name => typeof(NodeId).Assembly.GetType($"{ContractNamespace}.{name}")!).ToList();
        var unions = document.Definitions
            .Where(definition => definition.Value.OneOf.Count > 0 && generated.Any(identity => identity.Name == definition.Key))
            .ToDictionary(definition => definition.Key, definition => definition.Value.OneOf.Select(member => NameOf(document, member.ActualSchema)).ToHashSet());
        var declared = unions.SelectMany(union => union.Value.Select(member => (From: member, To: union.Key)))
            .Concat(unions.SelectMany(narrower => unions.Where(wider => wider.Key != narrower.Key && narrower.Value.IsSubsetOf(wider.Value)).Select(wider => (From: narrower.Key, To: wider.Key))))
            .Order()
            .ToList();

        // Act
        var widenings = generated.SelectMany(identity => identity.GetMethods(BindingFlags.Public | BindingFlags.Static).Where(method => method.Name == Widen).Select(method => (From: method.GetParameters().Single().ParameterType.Name, To: method.ReturnType.Name))).Order().ToList();
        var constructed = generated.Where(identity => identity.GetConstructors(BindingFlags.Public | BindingFlags.Instance).Length > 0).Select(identity => identity.Name).ToList();
        var unified = generated.SelectMany(narrower => generated.Where(wider => wider != narrower && wider.IsAssignableFrom(narrower)).Select(wider => $"{narrower.Name} is a {wider.Name}")).ToList();

        // Assert
        Assert.Equal((WholeValue.Of(declared), None, None), (WholeValue.Of(widenings), string.Join(", ", constructed), string.Join(", ", unified)));
    }

    [Fact]
    public async Task A_page_cursor_s_first_page_is_the_default_its_schema_publishes()
    {
        // Arrange
        var document = await Published();
        var cursors = IdentityTypes.Of(document).Where(identity => identity.Key.Default is not null).ToDictionary(identity => identity.Value, identity => Convert.ToInt32(identity.Key.Default));

        // Act
        var first = new Dictionary<string, int>
        {
            [nameof(EdgePageCursor)] = EdgePageCursor.First.Value,
            [nameof(ElementPageCursor)] = ElementPageCursor.First.Value,
        };

        // Assert
        Assert.Equal(cursors, first);
    }

    [Fact]
    public async Task Every_identity_reads_and_writes_its_wire_form_and_nothing_else()
    {
        // Arrange
        var document = await Published();
        var generated = IdentityTypes.Of(document).Values.Select(name => typeof(NodeId).Assembly.GetType($"{ContractNamespace}.{name}")!).ToList();

        // Act
        var converters = generated.Where(identity => identity.GetCustomAttribute<System.Text.Json.Serialization.JsonConverterAttribute>()?.ConverterType != identity.GetNestedType(Converter)).Select(identity => identity.Name).ToList();
        var refusals = generated.Select(identity => (identity.Name, Refused: Record.Exception(() => System.Text.Json.JsonSerializer.Deserialize("{}", identity)) is System.Text.Json.JsonException)).Where(read => !read.Refused).Select(read => read.Name).ToList();

        // Assert
        Assert.Equal((None, None), (string.Join(", ", converters), string.Join(", ", refusals)));
    }

    [Fact]
    public async Task No_client_source_reads_an_identity_from_text_but_through_the_legacy_door()
    {
        // Arrange
        var document = await Published();
        var reads = new System.Text.RegularExpressions.Regex($@"\bDeserialize<\s*(?:{string.Join("|", IdentityTypes.Of(document).Values)})\b");

        // Act
        var readers = ConformanceTests.ClientSourceFiles().Where(file => reads.IsMatch(File.ReadAllText(file))).Select(file => Path.GetRelativePath(ConformanceTests.ClientRoot, file).Replace('\\', '/')).ToList();

        // Assert
        Assert.Empty(readers);
    }

    private static Task<OpenApiDocument> Published() =>
        OpenApiYamlDocument.FromFileAsync(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "openapi.yaml"));

    private static string NameOf(OpenApiDocument document, JsonSchema schema) =>
        document.Definitions.Single(definition => ReferenceEquals(definition.Value, schema)).Key;
}
