using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
using System.Reflection;
using System.Text.Json;

var expected = new[] { "Chapter(ChapterReference)", "ChapterText(ChapterReference)", "KretzmannChapter(ChapterReference)", "SceneScripture(BibleReference)" };
var names = new[] { "Chapter", "ChapterText", "KretzmannChapter", "SceneScripture" };
var actual = typeof(AtlasClient).GetMethods().Where(method => names.Contains(method.Name))
    .Select(method => $"{method.Name}({string.Join(",", method.GetParameters().Select(parameter => parameter.ParameterType.Name))})").Order().ToArray();
if (!actual.SequenceEqual(expected)) throw new Exception(JsonSerializer.Serialize(actual));
Console.WriteLine(JsonSerializer.Serialize(actual));
var api = new AtlasClient(new HttpClient());
var chapterReference = JsonSerializer.Deserialize<ChapterReference>("\"JHN.3\"")!;
var bibleReference = JsonSerializer.Deserialize<BibleReference>("\"JHN.3.16\"")!;
Func<Task<Chapter>> chapter = () => api.Chapter(chapterReference);
Func<Task<BibleAtlas.Client.Exploring.ChapterText>> text = () => api.ChapterText(chapterReference);
Func<Task<KretzmannChapter>> commentary = () => api.KretzmannChapter(chapterReference);
Func<Task<Scene>> scene = () => api.SceneScripture(bibleReference);
Console.WriteLine("The four valid nominal delegates compile; none is invoked and no HTTP request is made.");
var identities = typeof(ArtifactRoot).Assembly.GetTypes()
    .Where(type => type.Namespace == "BibleAtlas.Client.Contract" && type.GetCustomAttribute<System.Text.Json.Serialization.JsonConverterAttribute>() is not null && type.GetProperty("Value") is not null)
    .Select(type => (Name: type.Name, Guard: type.GetProperty("Value")!.GetCustomAttribute<System.Diagnostics.CodeAnalysis.ExperimentalAttribute>()?.DiagnosticId))
    .OrderBy(identity => identity.Name).ToArray();
var expectedNames = new[] { "ArtifactRoot", "BibleReference", "ChapterReference", "ConcordReference", "ContentsReference", "CrossReferenceTarget", "EdgeId", "EdgePageCursor", "ElementId", "ElementPageCursor", "EraId", "NarrativeId", "NodeId", "PassageReference", "PolityId", "TextWindowReference", "UnitReference", "VerseRangeReference", "VerseReference", "VerseSpanReference" };
if (!identities.SequenceEqual(expectedNames.Select(name => (Name: name, Guard: (string?)"ATLASWIRE")))) throw new Exception(string.Join(";", identities));
Console.WriteLine(JsonSerializer.Serialize(identities.Select(identity => new { identity.Name, identity.Guard })));
