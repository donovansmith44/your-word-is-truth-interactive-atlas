using System.Text.Json;
using Reqnroll;

namespace BibleAtlas.Client.ContractTests.Steps;

/// <summary>
/// Batch AQC-1's own C# contract harness -- THIN, contract-ignorant
/// step-definition glue (spec §3: "Step definitions are thin glue, not
/// contract knowledge") binding every phrase in
/// <c>contracts/atlas-query-contract/features/*.feature</c> (glossary.md's
/// own phrase table) -- the SAME phrases the Rust cucumber harness binds
/// -- through the client's own DTO deserialization (<see cref="Wire.Options"/>)
/// against the committed provider fixtures
/// (<c>contracts/atlas-query-contract/fixtures/*.json</c>), never a live
/// server. Reqnroll creates one instance of this class per scenario (the
/// SAME "fresh World per scenario" shape the Rust harness's own
/// <c>AqcWorld</c> gets), so instance fields below are this side's World.
/// </summary>
[Binding]
public class AqcSteps
{
    private static readonly string RepoRoot = FindRepoRoot();
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

    private string _advertisedMin = "";
    private string _advertisedMax = "";

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

    [Given("the server advertises AQC version \"([^\"]+)\" through \"([^\"]+)\"")]
    public void GivenAdvertisedRange(string min, string max)
    {
        _advertisedMin = min;
        _advertisedMax = max;
    }

    // ---------------------------------------------------------------
    // When
    // ---------------------------------------------------------------

    [When("I run FocusQuery for \"([^\"]+)\"")]
    public void WhenFocusQuery(string id)
    {
        (_status, _body) = LoadFixture(FocusFixtureNameForRequest(id));
        _focusRequestedId = id;
    }

    [When("I run FocusQuery again for the captured reference")]
    public void WhenFocusQueryCaptured()
    {
        if (_capturedRef is null) throw new InvalidOperationException("no focus reference was captured yet");
        (_status, _body) = LoadFixture(FocusFixtureNameForCapturedIdentity(_capturedRef));
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
        (_status, _body) = LoadFixture(name);
    }

    [When("I run TraversalQuery for \"([^\"]+)\" frontier \"([^\"]+)\" with limit (\\d+)")]
    public void WhenTraversalQueryLimit(string id, string kind, int limit)
    {
        var name = (id, kind, limit) switch
        {
            ("text-unit:JHN.3.16", "cites", 1) => "traversal-cites-limit1",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TraversalQuery '{id}'/'{kind}' limit {limit}."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I run TextWindowQuery for \"([^\"]+)\" radius (\\d+)")]
    public void WhenTextWindow(string sref, int n)
    {
        var name = (sref, n) switch
        {
            ("JHN.3.16", 1) => "text-window-single",
            ("JHN.3.16", 3) => "text-window-multi",
            ("MAT.4.19", 1) => "text-window-mat-4-19",
            ("MAT.5.4", 1) => "text-window-mat-5-4",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TextWindowQuery '{sref}' radius {n}."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I run TextWindowQuery for \"([^\"]+)\" radius (\\d+) with corpus \"([^\"]+)\"")]
    public void WhenTextWindowCorpus(string sref, int n, string corpus)
    {
        var name = (sref, n, corpus) switch
        {
            ("JHN.3.16", 1, "not-a-real-corpus") => "text-window-bad-corpus",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for TextWindowQuery '{sref}' radius {n} corpus '{corpus}'."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I run a chapter-scoped TextWindowQuery for \"([^\"]+)\" with dir \"([^\"]+)\"")]
    public void WhenTextWindowChapterDir(string cref, string dir)
    {
        var name = (cref, dir) switch
        {
            ("JHN.3", "backward") => "text-window-chapter-backward-bad-dir",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for chapter-scoped TextWindowQuery '{cref}' dir '{dir}'."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I run SceneQuery for the time window \"([^\"]+)\"-\"([^\"]+)\"")]
    public void WhenSceneTime(string from, string to)
    {
        var name = (from, to) switch
        {
            ("-2100", "-2000") => "scene-time",
            ("100", "-100") => "scene-bad-window",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for SceneQuery time window '{from}'-'{to}'."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I run SceneQuery for scripture ref \"([^\"]+)\"")]
    public void WhenSceneScripture(string sref)
    {
        var name = sref switch
        {
            "JHN.3.16" => "scene-scripture",
            "not-a-ref-at-all" => "scene-bad-ref",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for SceneQuery scripture ref '{sref}'."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I query \"([^\"]+)\"")]
    public void WhenQueryPath(string path)
    {
        var name = path switch
        {
            "/api/contract" => "contract",
            _ => throw new NotSupportedException($"AqcSteps: no fixture mapped for path '{path}'."),
        };
        (_status, _body) = LoadFixture(name);
    }

    [When("I capture the returned focus reference")]
    public void WhenCaptureFocusRef()
    {
        // FocusQuery response: a top-level "id". TraversalQuery response:
        // the FIRST entry's own node.id (glossary.md's own convention,
        // identical to the Rust harness's own step).
        if (_body.TryGetProperty("id", out var idProp))
        {
            _capturedRef = idProp.GetString();
            return;
        }
        if (_body.TryGetProperty("entries", out var entries) && entries.GetArrayLength() > 0)
        {
            _capturedRef = entries[0].GetProperty("node").GetProperty("id").GetString();
            return;
        }
        throw new InvalidOperationException("no capturable focus reference on the last response (expected a top-level 'id' or a non-empty 'entries' array)");
    }

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

        // Also proves CONSUMER PARSING, not just field presence -- the
        // brief's own "proving the consumer parses every shape the
        // provider emits" -- by round-tripping through the client's real
        // DTO + Wire.Options.
        var json = _body.GetRawText();
        object? dto = shape switch
        {
            "NodeCardOut" => JsonSerializer.Deserialize<NodeCardDto>(json, Wire.Options),
            "EdgePageOut" => JsonSerializer.Deserialize<EdgePageDto>(json, Wire.Options),
            "TextWindowOut" => JsonSerializer.Deserialize<TextWindowDto>(json, Wire.Options),
            "Scene" => JsonSerializer.Deserialize<Scene>(json, Wire.Options),
            "ContractOut" => JsonSerializer.Deserialize<ContractDto>(json, Wire.Options),
            _ => null,
        };
        Assert.NotNull(dto);
    }

    [Then("the response \"([^\"]+)\" field equals \"([^\"]+)\"")]
    public void ThenFieldEquals(string field, string expected)
    {
        Assert.True(_body.TryGetProperty(field, out var actual), $"response has no field '{field}'");
        Assert.Equal(expected, actual.GetString());
    }

    [Then("every frontier group is a relations! family")]
    public void ThenEveryFrontierIsARelationsFamily()
    {
        // Thin: the C# side proves it recognizes every label the FIXTURE
        // set itself carries (the Rust side is authoritative for "is this
        // string drawn from RelationId::ALL" -- graph_wire::parse_edge_kind
        // has no C# equivalent, and duplicating the relation manifest here
        // would be a second representation of it). What this DOES prove:
        // every kind label present deserializes as a non-empty string on
        // the client's own EdgeSummaryEntryDto/EdgePageDto.Kind.
        if (_body.TryGetProperty("edge_summary", out var summary))
        {
            foreach (var entry in summary.EnumerateArray())
            {
                Assert.False(string.IsNullOrEmpty(entry.GetProperty("kind").GetString()));
            }
        }
        else if (_body.TryGetProperty("kind", out var kind))
        {
            Assert.False(string.IsNullOrEmpty(kind.GetString()));
        }
        else
        {
            throw new InvalidOperationException("no frontier-bearing field (edge_summary or kind) on the last response");
        }
    }

    [Then("the request fails with status (\\d+) and code \"([^\"]+)\"")]
    public void ThenRequestFails(int status, string code)
    {
        Assert.Equal(status, _status);
        Assert.Equal(code, _body.GetProperty("error").GetProperty("code").GetString());
    }

    [Then("the focus reference round-trips identically")]
    public void ThenRoundTrips()
    {
        Assert.NotNull(_capturedRef);
        var second = _body.GetProperty("id").GetString();
        Assert.Equal(_capturedRef, second);
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
        // The Rust harness proves this LIVE (a real FocusQuery re-fetch per
        // entry, against the real committed graph). A page can carry up to
        // 20 entries by default -- fixturing every one's own FocusQuery
        // individually would multiply the fixture count for a property
        // already proven live, server-side (the SAME reasoning
        // ThenBijectionWitness below discloses). What THIS side proves:
        // every entry deserializes as a well-formed NodeRefDto (non-empty
        // id/kind/label) on the client's own EdgeEntryDto.Node -- one
        // entry's id (the FIRST, "text-unit:ROM.5.8") is additionally
        // cross-checked against its own committed FocusQuery fixture, so
        // at least one target IS proven to independently resolve.
        var entries = _body.GetProperty("entries");
        Assert.True(entries.GetArrayLength() > 0, "test needs at least one real entry to prove resolution over");
        var first = true;
        foreach (var entry in entries.EnumerateArray())
        {
            var node = entry.GetProperty("node");
            var id = node.GetProperty("id").GetString();
            Assert.False(string.IsNullOrEmpty(id));
            Assert.False(string.IsNullOrEmpty(node.GetProperty("kind").GetString()));
            Assert.False(string.IsNullOrEmpty(node.GetProperty("label").GetString()));
            if (first)
            {
                var (status, focusBody) = LoadFixture(FocusFixtureNameForCapturedIdentity(id!));
                Assert.Equal(200, status);
                Assert.Equal(id, focusBody.GetProperty("id").GetString());
                first = false;
            }
        }
    }

    [Then("every entry's \"edge\" id is present on the matching inverse-kind page of its own target node")]
    public void ThenBijectionWitness()
    {
        // The bijection ITSELF is a graph-structural law the Rust harness
        // proves against the live inverse-kind page (this fixture set
        // deliberately carries only ONE direction's fixture per pair --
        // "traversal-located-at", not also its own inverse "site-of" page
        // -- capturing every inverse page too would double the fixture
        // count for a property already proven live, server-side). What
        // THIS side proves instead: every entry's own "edge" id
        // deserializes as a non-empty string on the client's own
        // EdgeEntryDto.Edge -- the wire FIELD the bijection travels on is
        // real and parses.
        var entries = _body.GetProperty("entries");
        Assert.True(entries.GetArrayLength() > 0, "test needs at least one real entry to prove the wire field over");
        foreach (var entry in entries.EnumerateArray())
        {
            Assert.False(string.IsNullOrEmpty(entry.GetProperty("edge").GetString()));
        }
    }

    [Then("the response \"entries\" array has at most (\\d+) entry")]
    public void ThenEntriesAtMost(int max)
    {
        Assert.True(_body.GetProperty("entries").GetArrayLength() <= max);
    }

    [Then("a further page reached by following \"next\" never repeats an entry already seen")]
    public void ThenPaginationNoRepeats()
    {
        // Fixture-only equivalent of the Rust harness's own live-walked
        // proof: this fixture ("traversal-cites-limit1") is a ONE-entry
        // page with a real "next" cursor -- proving the client's own
        // EdgePageDto.Next deserializes as the expected non-null int, the
        // wire FIELD the live pagination walk (Rust side) depends on.
        Assert.True(_body.GetProperty("entries").GetArrayLength() <= 1);
        Assert.True(_body.TryGetProperty("next", out var next));
        if (next.ValueKind != JsonValueKind.Null)
        {
            Assert.True(next.GetInt32() >= 0);
        }
    }

    [Then("the response has exactly (\\d+) units?")]
    public void ThenExactlyNUnits(int n)
    {
        Assert.Equal(n, _body.GetProperty("units").GetArrayLength());
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
        var units = _body.GetProperty("units");
        var refs = units.EnumerateArray().Select(u => u.GetProperty("ref").GetString()).ToArray();
        Assert.Equal(new[] { a, b, c }, refs);
    }

    [Then("every \"words_of_christ\" span lies within its own verse's text length")]
    public void ThenSpansWithinLength()
    {
        foreach (var unit in _body.GetProperty("units").EnumerateArray())
        {
            var len = unit.GetProperty("text").GetString()!.Length;
            foreach (var span in unit.GetProperty("words_of_christ").EnumerateArray())
            {
                var start = span.GetProperty("start").GetInt32();
                var end = span.GetProperty("end").GetInt32();
                Assert.True(start <= end && end <= len, $"span [{start},{end}) is outside its own verse's text length {len}");
            }
        }
    }

    [Then("\"([^\"]+)\" is empty")]
    public void ThenFieldIsEmptyArray(string field)
    {
        Assert.Equal(0, _body.GetProperty(field).GetArrayLength());
    }

    [Then("the server advertises AQC version \"([^\"]+)\" through \"([^\"]+)\"")]
    public void ThenServerAdvertises(string min, string max)
    {
        Assert.Equal(min, _body.GetProperty("min_version").GetString());
        Assert.Equal(max, _body.GetProperty("max_version").GetString());
    }

    [Then("the client accepts the advertised range")]
    public void ThenClientAccepts()
    {
        Assert.True(AqcContract.Satisfies(new ContractDto(_advertisedMin, _advertisedMax)));
    }

    [Then("the client rejects the advertised range")]
    public void ThenClientRejects()
    {
        Assert.False(AqcContract.Satisfies(new ContractDto(_advertisedMin, _advertisedMax)));
    }

    /// <summary>Q-4 fix (fix round 1, controller ruling): a MALFORMED
    /// advertised version must fail LOUD -- the REAL production
    /// <see cref="AqcContract.Satisfies"/> throwing
    /// <see cref="FormatException"/>, the same mechanism App.razor's own
    /// narrowed startup-check catch now routes to
    /// <c>CheckState.Mismatch</c> rather than swallowing into "Ok".</summary>
    [Then("the malformed advertisement fails loud")]
    public void ThenMalformedAdvertisementFailsLoud()
    {
        Assert.Throws<FormatException>(() => AqcContract.Satisfies(new ContractDto(_advertisedMin, _advertisedMax)));
    }
}
