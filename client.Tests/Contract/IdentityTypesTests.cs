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

    [Fact]
    public async Task No_transport_read_takes_a_primitive_where_the_document_names_an_identity()
    {
        // Arrange
        var document = await Published();
        var identities = IdentityTypes.Of(document).Values.ToHashSet();
        var reads = typeof(AtlasClient).GetMethods(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly)
            .Concat(typeof(IExplorableClient).GetMethods())
            .Where(method => typeof(Task).IsAssignableFrom(method.ReturnType));

        // Act
        var primitives = reads
            .SelectMany(method => method.GetParameters().Where(parameter => Primitive(parameter.ParameterType)).Select(parameter => $"{method.DeclaringType!.Name}.{method.Name}({parameter.Name})"))
            .Order(StringComparer.Ordinal)
            .ToList();
        var identitiesAsked = PrimitiveReads.Values
            .Where(asked => document.Operations.Single(operation => operation.Operation.OperationId == asked.Operation).Operation.ActualParameters.Single(parameter => parameter.Name == asked.Parameter) is { } parameter
                && (parameter.Schema ?? parameter).ActualSchema is var schema && document.Definitions.Any(definition => ReferenceEquals(definition.Value, schema) && identities.Contains(definition.Key)))
            .Select(asked => $"{asked.Operation} {asked.Parameter}")
            .ToList();

        // Assert
        Assert.Equal((WholeValue.Of(PrimitiveReads.Keys.Order(StringComparer.Ordinal)), None), (WholeValue.Of(primitives), string.Join(", ", identitiesAsked)));
    }

    private static readonly IReadOnlyDictionary<string, (string Operation, string Parameter)> PrimitiveReads = new Dictionary<string, (string, string)>
    {
        ["AtlasClient.CatechismItem(id)"] = ("catechism_item", "id"),
        ["AtlasClient.Event(id)"] = ("event", "id"),
        ["AtlasClient.NarrativeEventPositions(eventId)"] = ("narrative_event_positions", "id"),
        ["AtlasClient.Polities(from)"] = ("polities", "from"),
        ["AtlasClient.Polities(to)"] = ("polities", "to"),
        ["AtlasClient.SceneTime(from)"] = ("scene_time", "from"),
        ["AtlasClient.SceneTime(to)"] = ("scene_time", "to"),
        ["IExplorableClient.Edges(limit)"] = ("node_edges", "limit"),
        ["IExplorableClient.Reading(n)"] = ("text_window", "n"),
    };

    private static bool Primitive(Type type) => (Nullable.GetUnderlyingType(type) ?? type) is var bare && (bare == typeof(string) || bare == typeof(int));

    [Fact]
    public async Task An_identity_s_wire_value_is_read_only_by_the_transport()
    {
        // Arrange
        var document = await Published();
        var generated = IdentityTypes.Of(document).Values.Select(name => typeof(NodeId).Assembly.GetType($"{ContractNamespace}.{name}")!).ToList();
        var clientProjects = Directory.GetFiles(ConformanceTests.ClientRoot, "*.csproj", SearchOption.AllDirectories);

        // Act
        var unfenced = generated.Where(identity => identity.GetProperty(nameof(NodeId.Value))?.GetCustomAttribute<System.Diagnostics.CodeAnalysis.ExperimentalAttribute>()?.DiagnosticId != IdentityTypes.TransportOnly).Select(identity => identity.Name).ToList();
        var optedIn = ConformanceTests.ClientSourceFiles().Concat(clientProjects).Where(file => File.ReadAllText(file).Contains(IdentityTypes.TransportOnly)).Select(file => Path.GetRelativePath(ConformanceTests.ClientRoot, file).Replace('\\', '/')).Order(StringComparer.Ordinal).ToList();

        // Assert
        Assert.Equal((None, WholeValue.Of(Transport)), (string.Join(", ", unfenced), WholeValue.Of(optedIn)));
    }

    private static readonly string[] Transport = ["AtlasClient.cs", "GraphExplorableClient.cs"];

    private static Task<OpenApiDocument> Published() =>
        OpenApiYamlDocument.FromFileAsync(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "openapi.yaml"));

    private static string NameOf(OpenApiDocument document, JsonSchema schema) =>
        document.Definitions.Single(definition => ReferenceEquals(definition.Value, schema)).Key;
}
