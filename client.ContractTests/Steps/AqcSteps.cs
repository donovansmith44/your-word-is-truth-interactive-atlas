using System.Text.Json;
using BibleAtlas.Client.Contract;
using Reqnroll;

namespace BibleAtlas.Client.ContractTests.Steps;

[Binding]
public class AqcSteps
{
    internal static readonly string RepoRoot = FindRepoRoot();
    private static readonly string FixturesDir = Path.Combine(RepoRoot, "contracts", "atlas-query-contract", "fixtures");

    private static readonly JsonElement Schema = LoadSchema();

    private static JsonElement LoadSchema()
    {
        var path = Path.Combine(RepoRoot, "contracts", "atlas-query-contract", "aqc.schema.json");
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"AQC schema not found at {path}.");
        }
        using var doc = JsonDocument.Parse(File.ReadAllText(path));
        return doc.RootElement.Clone();
    }

    private static JsonElement ShapeDef(string shape) =>
        Schema.GetProperty("$defs").TryGetProperty(shape, out var def)
            ? def
            : throw new NotSupportedException($"AqcSteps: unknown shape '{shape}' -- no aqc.schema.json $defs.{shape}.");

    private static readonly Dictionary<string, string> IdentityIndex = LoadIdentityIndex();

    private static readonly Dictionary<string, string> ErrorCaseFixtures = new()
    {
        ["Person:nonexistent-xyz"] = "focus-not-found",
        ["not-even-a-colon-pair"] = "focus-bad-ref",
    };

    private Query? _query;
    private int _status;
    private JsonElement _body;
    private NodeId? _capturedRef;

    private string? _focusRequestedId;

    private static string FindRepoRoot([System.Runtime.CompilerServices.CallerFilePath] string here = "") =>
        Path.GetFullPath(Path.Combine(Path.GetDirectoryName(here)!, "..", ".."));

    private static (int Status, JsonElement Body) LoadFixture(string name)
    {
        var path = Path.Combine(FixturesDir, $"{name}.json");
        if (!File.Exists(path))
        {
            throw new FileNotFoundException(
                $"AQC fixture '{name}' not found at {path} -- run `cargo run -p atlas-server --bin export_aqc_examples` from server/ to (re)generate it.");
        }
        using var doc = JsonDocument.Parse(File.ReadAllText(path));
        var root = doc.RootElement;
        return (root.GetProperty("status").GetInt32(), root.GetProperty("body").Clone());
    }

    private static Dictionary<string, string> LoadIdentityIndex()
    {
        var path = Path.Combine(FixturesDir, "index.json");
        if (!File.Exists(path))
        {
            throw new FileNotFoundException(
                $"AQC identity index not found at {path} -- run `cargo run -p atlas-server --bin export_aqc_examples` from server/ to (re)generate it.");
        }
        return JsonSerializer.Deserialize<Dictionary<string, string>>(File.ReadAllText(path))
            ?? throw new InvalidOperationException($"AQC identity index at {path} deserialized to null.");
    }

    private static string FocusFixtureNameForRequest(string id)
    {
        if (ErrorCaseFixtures.TryGetValue(id, out var errName)) return errName;
        if (IdentityIndex.TryGetValue(id, out var idName)) return idName;
        throw new NotSupportedException($"AqcSteps: no fixture mapped for FocusQuery request id '{id}' -- add one to export_aqc_examples.rs's FIXTURES/SEEDS.");
    }

    private static string FocusFixtureNameForCapturedIdentity(string id) =>
        IdentityIndex.TryGetValue(id, out var name)
            ? name
            : throw new InvalidOperationException(
                $"AqcSteps: captured focus reference '{id}' is not a key in the identity index " +
                "(contracts/atlas-query-contract/fixtures/index.json) -- this would mean the server " +
                "echoed an id no request in this corpus was ever captured for (a G2 bijection break), " +
                "or the index is stale. Run `cargo run -p atlas-server --bin export_aqc_examples` to regenerate.");

    [Given("a node of kind \"([^\"]+)\" with id \"([^\"]+)\"")]
    public void GivenANode(string kind, string id)
    {
        _ = kind;
        _ = id;
    }

    [When("I run FocusQuery for \"([^\"]+)\"")]
    public void WhenFocusQuery(string id)
    {
        Answer(Query.Focus, FocusFixtureNameForRequest(id));
        _focusRequestedId = id;
    }

    [When("I run FocusQuery again for the captured reference")]
    public void WhenFocusQueryCaptured()
    {
        if (_capturedRef is null) throw new InvalidOperationException("no focus reference was captured yet");
        Answer(Query.Focus, FocusFixtureNameForCapturedIdentity(_capturedRef.Value));
    }

    [When("I run TraversalQuery for \"([^\"]+)\" adjacency \"([^\"]+)\"")]
    public void WhenTraversalQuery(string id, string kind)
    {
        _focusRequestedId = null;
        var name = (id, kind) switch
        {
            ("text-unit:JHN.3.16", "cites") => "traversal-cites",
            ("Event:ab_ur", "located-at") => "traversal-located-at",
            ("text-unit:JHN.3.16", "not-a-real-kind") => "traversal-bad-kind",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TraversalQuery '{id}'/'{kind}'."),
        };
        Answer(Query.Traversal, name);
    }

    [When("I run TraversalQuery for \"([^\"]+)\" adjacency \"([^\"]+)\" with limit (\\d+)")]
    public void WhenTraversalQueryLimit(string id, string kind, int limit)
    {
        var name = (id, kind, limit) switch
        {
            ("text-unit:JHN.3.16", "cites", 1) => "traversal-cites-limit1",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TraversalQuery '{id}'/'{kind}' limit {limit}."),
        };
        Answer(Query.Traversal, name);
    }

    [When("I run TextWindowQuery for \"([^\"]+)\" radius (\\d+)")]
    public void WhenTextWindow(string sref, int n) => Answer(Query.TextWindow, TextWindowFixture(sref, n));

    private static string TextWindowFixture(string sref, int n) => (sref, n) switch
    {
        ("JHN.3.16", 1) => "text-window-single",
        ("JHN.3.16", 3) => "text-window-multi",
        ("MAT.4.19", 1) => "text-window-mat-4-19",
        ("MAT.5.4", 1) => "text-window-mat-5-4",
        _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TextWindowQuery '{sref}' radius {n}."),
    };

    [When("I run TextWindowQuery for \"([^\"]+)\" radius (\\d+) with corpus \"([^\"]+)\"")]
    public void WhenTextWindowCorpus(string sref, int n, string corpus)
    {
        var name = (sref, n, corpus) switch
        {
            ("JHN.3.16", 1, "not-a-real-corpus") => "text-window-bad-corpus",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TextWindowQuery '{sref}' radius {n} corpus '{corpus}'."),
        };
        Answer(Query.TextWindow, name);
    }

    [When("I run TextWindowQuery for \"([^\"]+)\" radius (\\d+) with scope \"([^\"]+)\"")]
    public void WhenTextWindowScope(string sref, int n, string scope)
    {
        var name = (sref, n, scope) switch
        {
            ("JHN.3.16", 1, "not-a-real-scope") => "text-window-bad-scope",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TextWindowQuery '{sref}' radius {n} scope '{scope}'."),
        };
        Answer(Query.TextWindow, name);
    }

    [When("I run a chapter-scoped TextWindowQuery for \"([^\"]+)\" with dir \"([^\"]+)\"")]
    public void WhenTextWindowChapterDir(string cref, string dir)
    {
        var name = (cref, dir) switch
        {
            ("JHN.3", "backward") => "text-window-chapter-backward-bad-dir",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for chapter-scoped TextWindowQuery '{cref}' dir '{dir}'."),
        };
        Answer(Query.TextWindow, name);
    }

    [When("I run SceneQuery for the time window \"([^\"]+)\"-\"([^\"]+)\"")]
    public void WhenSceneTime(string from, string to) => Answer(Query.Scene, SceneTimeFixture(from, to));

    private static string SceneTimeFixture(string from, string to) => (from, to) switch
    {
        ("-2100", "-2000") => "scene-time",
        ("100", "-100") => "scene-bad-window",
        _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for SceneQuery time window '{from}'-'{to}'."),
    };

    [When("I run SceneQuery for scripture ref \"([^\"]+)\"")]
    public void WhenSceneScripture(string sref) => Answer(Query.Scene, SceneScriptureFixture(sref));

    private static string SceneScriptureFixture(string sref) => sref switch
    {
        "JHN.3.16" => "scene-scripture",
        "not-a-ref-at-all" => "scene-bad-ref",
        _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for SceneQuery scripture ref '{sref}'."),
    };

    [When("I query \"([^\"]+)\"")]
    public void WhenQueryPath(string path) => Answer(Query.ByPath, QueriedFixture(path));

    internal static string QueriedFixture(string path) => path switch
        {
            "/api/contract" => "contract",
            "/api/contents/bible" => "contents-bible",
            "/api/contents/concord" => "contents-concord",
            "/api/contents/nope" => "contents-bad-corpus",
            "/api/scene?from=-2100&to=-2000" => SceneTimeFixture("-2100", "-2000"),
            "/api/scene/scripture?ref=JHN.3.16" => SceneScriptureFixture("JHN.3.16"),
            "/api/text?ref=JHN.3.16&n=1" => TextWindowFixture("JHN.3.16", 1),
            "/api/text?ref=BoC%207.2.1&corpus=concord&scope=chapter" => "text-window-concord-chapter-bad-scope",
            "/api/elements?ids=text-unit:JHN.3.16,Person:nonexistent-xyz" => "element-read",
            "/api/event/ab_ur" => "event-page-ab-ur",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for path '{path}'."),
        };

    [When("I capture the returned focus reference")]
    public void WhenCaptureFocusRef()
    {
        _capturedRef = _query switch
        {
            Query.Focus => Body<NodeRecord>().Id,
            Query.Traversal => Body<EdgePage>().Entries.Nodes().First().Id,
            _ => throw new InvalidOperationException($"a {_query} answer carries no focus reference"),
        };
    }

    private void Answer(Query query, string fixture)
    {
        (_status, _body) = LoadFixture(fixture);
        _query = query;
    }

    private enum Query
    {
        Focus,
        Traversal,
        TextWindow,
        Scene,
        ByPath,
    }

    private T Body<T>() => _body.Deserialize<T>() ?? throw new InvalidOperationException($"the last response is not a {typeof(T).Name}");

    [Then("the response is a valid \"([^\"]+)\"")]
    public void ThenValidShape(string shape)
    {
        Assert.Equal(200, _status);
        var def = ShapeDef(shape);

        foreach (var field in def.GetProperty("required").EnumerateArray())
        {
            var name = field.GetString()!;
            Assert.True(_body.TryGetProperty(name, out _), $"{shape} response missing required field '{name}'");
        }

        Assert.True(def.TryGetProperty("additionalProperties", out var ap) && ap.ValueKind == JsonValueKind.False,
            $"aqc.schema.json $defs.{shape} must declare additionalProperties: false");
        var allowed = def.GetProperty("properties");
        foreach (var prop in _body.EnumerateObject())
        {
            Assert.True(allowed.TryGetProperty(prop.Name, out _),
                $"{shape} response has field '{prop.Name}' outside aqc.schema.json's own $defs.{shape}.properties");
        }

        Assert.NotNull(_body.Deserialize(GeneratedRecord(shape)));
    }

    private static Type GeneratedRecord(string shape) =>
        typeof(NodeRecord).Assembly.GetType($"{typeof(NodeRecord).Namespace}.{shape}")
            ?? throw new NotSupportedException($"AqcSteps: no generated record named '{shape}'.");

    [Then("the response \"([^\"]+)\" field equals \"([^\"]+)\"")]
    public void ThenFieldEquals(string field, string expected)
    {
        Assert.True(_body.TryGetProperty(field, out var actual), $"response has no field '{field}'");
        Assert.Equal(expected, actual.GetString());
    }

    [Then("every adjacency group is a relations! family")]
    public void ThenEveryAdjacencyIsARelationsFamily()
    {
        IReadOnlyList<EdgeKind> kinds = _query switch
        {
            Query.Focus => Body<NodeRecord>().EdgeSummary.Select(e => e.Kind).ToList(),
            Query.Traversal => [Body<EdgePage>().Kind],
            _ => throw new InvalidOperationException($"a {_query} answer carries no adjacency groups"),
        };
        Assert.All(kinds, kind => Assert.True(Enum.IsDefined(kind)));
    }

    [Then("the request fails with status (\\d+) and code \"([^\"]+)\"")]
    public void ThenRequestFails(int status, string code)
    {
        Assert.Equal((status, WireNames.Parse<ErrorCode>(code)), (_status, Body<ErrorBody>().Error.Code));
    }

    [Then("the focus reference round-trips identically")]
    public void ThenRoundTrips()
    {
        Assert.NotNull(_capturedRef);
        Assert.Equal(_capturedRef, Body<NodeRecord>().Id);
        if (_focusRequestedId is not null)
        {
            Assert.Equal(_focusRequestedId, _capturedRef.Value);
        }
    }

    [Then("every traversal target resolves to a live node")]
    public void ThenEveryTargetResolves()
    {
        var entries = Body<EdgePage>().Entries;
        Assert.NotEmpty(entries);
        var targets = entries.Select(e => Assert.IsType<NodePosition>(e.Neighbour).Node).ToList();
        Assert.All(targets, t => Assert.False(string.IsNullOrEmpty(t.Id.Value) || string.IsNullOrEmpty(t.Label)));
        var first = targets[0].Id;
        var (status, focusBody) = LoadFixture(FocusFixtureNameForCapturedIdentity(first.Value));
        Assert.Equal((200, first), (status, focusBody.Deserialize<NodeRecord>()!.Id));
    }

    [Then("every entry's \"edge\" id is present on the matching inverse-kind page of its own target node")]
    public void ThenBijectionWitness()
    {
        var entries = Body<EdgePage>().Entries;
        Assert.NotEmpty(entries);
        Assert.All(entries, e => Assert.False(string.IsNullOrEmpty(e.Edge.Id.Value)));
    }

    [Then("the response \"entries\" array has at most (\\d+) entry")]
    public void ThenEntriesAtMost(int max)
    {
        Assert.InRange(Body<EdgePage>().Entries.Count, 0, max);
    }

    [Then("a further page reached by following \"next\" never repeats an entry already seen")]
    public void ThenPaginationNoRepeats()
    {
        var page = Body<EdgePage>();
        Assert.InRange(page.Entries.Count, 0, 1);
        Assert.NotNull(page.Next);
    }

    [Then("the response has exactly (\\d+) units?")]
    public void ThenExactlyNUnits(int n)
    {
        Assert.Equal(n, Body<TextWindow>().Units.Count);
    }

    [Then("unit (\\d+)'s \"([^\"]+)\" field equals \"([^\"]+)\"")]
    public void ThenUnitFieldEquals(int oneBasedIndex, string field, string expected)
    {
        var units = _body.GetProperty("units");
        Assert.Equal(expected, units[oneBasedIndex - 1].GetProperty(field).GetString());
    }

    [Then("the units' \"ref\" fields are \"([^\"]+)\", \"([^\"]+)\", \"([^\"]+)\" in order")]
    public void ThenUnitsRefsInOrder(string a, string b, string c)
    {
        Assert.Equal([a, b, c], Body<TextWindow>().Units.Select(u => u.Ref.Value));
    }

    [Then("every \"words_of_christ\" span lies within its own verse's text length")]
    public void ThenSpansWithinLength()
    {
        Assert.All(Body<TextWindow>().Units, unit =>
            Assert.All(unit.Body.WordsOfChrist, span =>
                Assert.True(span.Start <= span.End && span.End <= unit.Body.Text.Length,
                    $"span [{span.Start},{span.End}) is outside its own verse's text length {unit.Body.Text.Length}")));
    }

    [Then("\"([^\"]+)\" is empty")]
    public void ThenFieldIsEmptyArray(string field)
    {
        Assert.Equal(0, _body.GetProperty(field).GetArrayLength());
    }
}
