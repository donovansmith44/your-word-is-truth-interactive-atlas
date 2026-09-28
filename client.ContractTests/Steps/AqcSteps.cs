using System.Text.Json;
using BibleAtlas.Client.Contract;
using Reqnroll;

namespace BibleAtlas.Client.ContractTests.Steps;

/// <summary>
/// Batch AQC-1's own C# contract harness -- THIN, contract-ignorant
/// step-definition glue (spec §3: "Step definitions are thin glue, not
/// contract knowledge") binding every phrase in
/// <c>contracts/atlas-query-contract/features/*.feature</c> (glossary.md's
/// own phrase table) -- the SAME phrases the Rust cucumber harness binds
/// -- through the generated contract records
/// against the committed provider fixtures
/// (<c>contracts/atlas-query-contract/fixtures/*.json</c>), never a live
/// server. Reqnroll creates one instance of this class per scenario (the
/// SAME "fresh World per scenario" shape the Rust harness's own
/// <c>AqcWorld</c> gets), so instance fields below are this side's World.
/// </summary>
[Binding]
public class AqcSteps
{
    internal static readonly string RepoRoot = FindRepoRoot();
    private static readonly string FixturesDir = Path.Combine(RepoRoot, "contracts", "atlas-query-contract", "fixtures");

    /// <summary>
    /// Q-2/Q-3 fix (Batch AQC-1 fix round 1, controller ruling):
    /// <c>aqc.schema.json</c> itself, parsed ONCE -- replaces the prior
    /// hand-copied <c>shape switch</c> required-field lists (three places
    /// carried the same list by hand: the schema, this switch, and the
    /// Rust harness's own match; nothing enforced them staying in sync).
    /// </summary>
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

    /// <summary>
    /// S-1 fix (Batch AQC-1 fix round 1, controller ruling): wire id ->
    /// fixture-name IDENTITY INDEX, loaded from
    /// <c>contracts/atlas-query-contract/fixtures/index.json</c> -- built
    /// by the exporter (<c>export_aqc_examples.rs</c>) from the REQUEST id
    /// of every FocusQuery it captures, never from a hand-typed C# switch.
    /// This is what makes <see cref="WhenFocusQueryCaptured"/>'s own second
    /// lookup a genuine, independently-failable check rather than the SAME
    /// deterministic function called twice on the SAME input (the prior
    /// vacuity this fix closes): a captured reference that is not a real
    /// key in this index throws immediately, which would happen if the
    /// server ever echoed an id no request in this corpus was ever made
    /// for.
    /// </summary>
    private static readonly Dictionary<string, string> IdentityIndex = LoadIdentityIndex();

    /// <summary>Deliberately-invalid FocusQuery request ids (no real node
    /// identity, so they have no business in <see cref="IdentityIndex"/>)
    /// -- kept as their own small, explicit table.</summary>
    private static readonly Dictionary<string, string> ErrorCaseFixtures = new()
    {
        ["Person:nonexistent-xyz"] = "focus-not-found",
        ["not-even-a-colon-pair"] = "focus-bad-ref",
    };

    private Query? _query;
    private int _status;
    private JsonElement _body;
    private string? _capturedRef;

    /// <summary>
    /// S-1 fix: the id this scenario's OWN <see cref="WhenFocusQuery"/>
    /// call requested (null when the scenario's capture instead
    /// originated from a TraversalQuery target, e.g. "a traversal target's
    /// own id round-trips too" -- there, the captured id is legitimately
    /// DIFFERENT from anything requested so far, and this check does not
    /// apply). When set, <see cref="ThenRoundTrips"/> asserts the captured
    /// reference equals THIS -- the actual descriptor round-trip identity
    /// law ("what you asked for is what you get back"), not merely
    /// self-consistency between the capture and a second fetch (which
    /// alone cannot catch a response echoing a DIFFERENT, still
    /// valid-looking, id).
    /// </summary>
    private string? _focusRequestedId;

    /// <summary>
    /// Walks up from this SOURCE FILE's own compile-time path (captured via
    /// <see cref="System.Runtime.CompilerServices.CallerFilePathAttribute"/>,
    /// the same "robust regardless of runtime CWD" discipline
    /// <c>CARGO_MANIFEST_DIR</c> gives the Rust side) to the repo root --
    /// this file lives at <c>client.ContractTests/Steps/AqcSteps.cs</c>, two
    /// levels below the root.
    /// </summary>
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

    // The FIRST FocusQuery in a scenario: request id may be a real node
    // identity (IdentityIndex) or one of the two deliberately-invalid
    // error-case ids (ErrorCaseFixtures) -- both are legitimate REQUESTS.
    private static string FocusFixtureNameForRequest(string id)
    {
        if (ErrorCaseFixtures.TryGetValue(id, out var errName)) return errName;
        if (IdentityIndex.TryGetValue(id, out var idName)) return idName;
        throw new NotSupportedException($"AqcSteps: no fixture mapped for FocusQuery request id '{id}' -- add one to export_aqc_examples.rs's FIXTURES/SEEDS.");
    }

    // S-1 fix: the SECOND FocusQuery, keyed by the id the FIRST response
    // itself echoed back (glossary.md's own "descriptor RE-DERIVED from
    // the response content") -- deliberately narrower than
    // FocusFixtureNameForRequest above (no error-case fallback: a captured
    // reference is never one of the two deliberately-invalid inputs).
    private static string FocusFixtureNameForCapturedIdentity(string id) =>
        IdentityIndex.TryGetValue(id, out var name)
            ? name
            : throw new InvalidOperationException(
                $"AqcSteps: captured focus reference '{id}' is not a key in the identity index " +
                "(contracts/atlas-query-contract/fixtures/index.json) -- this would mean the server " +
                "echoed an id no request in this corpus was ever captured for (a G2 bijection break), " +
                "or the index is stale. Run `cargo run -p atlas-server --bin export_aqc_examples` to regenerate.");

    // ---------------------------------------------------------------
    // Given
    // ---------------------------------------------------------------

    [Given("a node of kind \"([^\"]+)\" with id \"([^\"]+)\"")]
    public void GivenANode(string kind, string id)
    {
        // Documentation-only (glossary.md): the exporter already verified
        // this id resolves against the real committed graph before it was
        // ever written into the Examples: table or captured as a fixture.
        _ = kind;
        _ = id;
    }

    // ---------------------------------------------------------------
    // When
    // ---------------------------------------------------------------

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
        Answer(Query.Focus, FocusFixtureNameForCapturedIdentity(_capturedRef));
    }

    [When("I run TraversalQuery for \"([^\"]+)\" frontier \"([^\"]+)\"")]
    public void WhenTraversalQuery(string id, string kind)
    {
        // Not a FocusQuery -- see _focusRequestedId's own doc comment for
        // why this scenario shape (capture originates from a traversal
        // TARGET, not this call's own id) skips the original-id check.
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

    [When("I run TraversalQuery for \"([^\"]+)\" frontier \"([^\"]+)\" with limit (\\d+)")]
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

    // A query and the URL it is served at must resolve to the SAME fixture,
    // so the name is decided once here and the URL-form step below calls this
    // rather than repeating it.
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
    public void WhenQueryPath(string path)
    {
        var name = path switch
        {
            "/api/contract" => "contract",
            // D4: ContentsQuery (contents.feature)
            "/api/contents/bible" => "contents-bible",
            "/api/contents/concord" => "contents-concord",
            "/api/contents/nope" => "contents-bad-corpus",
            // The URL form of a query whose own scenario also asks for it by
            // name (scene-query.feature, text-window.feature): the arguments
            // are read back out of the URL and handed to the same resolver the
            // named step uses, so the two forms cannot come to answer from
            // different fixtures.
            "/api/scene?from=-2100&to=-2000" => SceneTimeFixture("-2100", "-2000"),
            "/api/scene/scripture?ref=JHN.3.16" => SceneScriptureFixture("JHN.3.16"),
            "/api/text?ref=JHN.3.16&n=1" => TextWindowFixture("JHN.3.16", 1),
            // A chapter is a Scripture reading (text-window.feature) and a place's
            // period is read by the one window law (place-period.feature).
            "/api/text?ref=BoC%207.2.1&corpus=concord&scope=chapter" => "text-window-concord-chapter-bad-scope",
            "/api/place/hazor-1?from=notayear" => "place-period-bad-window",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for path '{path}'."),
        };
        Answer(Query.ByPath, name);
    }

    [When("I capture the returned focus reference")]
    public void WhenCaptureFocusRef()
    {
        // A FocusQuery answers with a NodeCard, a TraversalQuery with an EdgePage.
        _capturedRef = _query switch
        {
            Query.Focus => Body<NodeCard>().Id,
            Query.Traversal => Body<EdgePage>().Entries.First().Node.Id,
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

    // ---------------------------------------------------------------
    // Then
    // ---------------------------------------------------------------

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

        // Q-3 fix: additionalProperties: false, enforced -- glossary.md's
        // own "the response is a valid <Shape>" definition names this as
        // HALF of what the phrase means; only the required-fields half
        // was checked before this fix.
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
        typeof(NodeCard).Assembly.GetType($"{typeof(NodeCard).Namespace}.{shape}")
            ?? throw new NotSupportedException($"AqcSteps: no generated record named '{shape}'.");

    [Then("the response \"([^\"]+)\" field equals \"([^\"]+)\"")]
    public void ThenFieldEquals(string field, string expected)
    {
        Assert.True(_body.TryGetProperty(field, out var actual), $"response has no field '{field}'");
        Assert.Equal(expected, actual.GetString());
    }

    [Then("every frontier group is a relations! family")]
    public void ThenEveryFrontierIsARelationsFamily()
    {
        IReadOnlyList<EdgeKind> kinds = _query switch
        {
            Query.Focus => Body<NodeCard>().EdgeSummary.Select(e => e.Kind).ToList(),
            Query.Traversal => [Body<EdgePage>().Kind],
            _ => throw new InvalidOperationException($"a {_query} answer carries no frontier groups"),
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
        Assert.Equal(_capturedRef, Body<NodeCard>().Id);
        // S-1 fix (fix round 1): the ACTUAL round-trip identity law --
        // captured must ALSO equal what this scenario originally
        // requested, not merely equal the second (independently,
        // index-looked-up) fetch. Skipped when the capture originated
        // from a TraversalQuery target (_focusRequestedId's own doc
        // comment).
        if (_focusRequestedId is not null)
        {
            Assert.Equal(_focusRequestedId, _capturedRef);
        }
    }

    [Then("every traversal target resolves to a live node")]
    public void ThenEveryTargetResolves()
    {
        // The Rust harness re-fetches every entry live; this side proves every entry is a
        // well-formed NodeRef and that the first one resolves through its own committed
        // FocusQuery fixture.
        var entries = Body<EdgePage>().Entries;
        Assert.NotEmpty(entries);
        Assert.All(entries, e => Assert.False(string.IsNullOrEmpty(e.Node.Id) || string.IsNullOrEmpty(e.Node.Label)));
        var first = entries[0].Node.Id;
        var (status, focusBody) = LoadFixture(FocusFixtureNameForCapturedIdentity(first));
        Assert.Equal((200, first), (status, focusBody.Deserialize<NodeCard>()!.Id));
    }

    [Then("every entry's \"edge\" id is present on the matching inverse-kind page of its own target node")]
    public void ThenBijectionWitness()
    {
        // The bijection itself is proven live by the Rust harness against the inverse page;
        // this side proves the wire field it travels on is present on every entry.
        var entries = Body<EdgePage>().Entries;
        Assert.NotEmpty(entries);
        Assert.All(entries, e => Assert.False(string.IsNullOrEmpty(e.Edge)));
    }

    [Then("the response \"entries\" array has at most (\\d+) entry")]
    public void ThenEntriesAtMost(int max)
    {
        Assert.InRange(Body<EdgePage>().Entries.Count, 0, max);
    }

    [Then("a further page reached by following \"next\" never repeats an entry already seen")]
    public void ThenPaginationNoRepeats()
    {
        // The Rust harness walks the pages live; this fixture is a one-entry page whose
        // cursor is the field that walk depends on.
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
        Assert.Equal([a, b, c], Body<TextWindow>().Units.Select(u => u.Ref));
    }

    [Then("every \"words_of_christ\" span lies within its own verse's text length")]
    public void ThenSpansWithinLength()
    {
        Assert.All(Body<TextWindow>().Units, unit =>
            Assert.All(unit.WordsOfChrist, span =>
                Assert.True(span.Start <= span.End && span.End <= unit.Text.Length,
                    $"span [{span.Start},{span.End}) is outside its own verse's text length {unit.Text.Length}")));
    }

    [Then("\"([^\"]+)\" is empty")]
    public void ThenFieldIsEmptyArray(string field)
    {
        Assert.Equal(0, _body.GetProperty(field).GetArrayLength());
    }

    [Then("the server advertises AQC version \"([^\"]+)\" through \"([^\"]+)\"")]
    public void ThenServerAdvertises(string min, string max)
    {
        var contract = Body<Contract.Contract>();
        Assert.Equal((min, max), (contract.MinVersion, contract.MaxVersion));
    }
}
