using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
using System.Text.Json;

var methods = typeof(AtlasClient).GetMethods().Where(method => new[] { "Chapter", "ChapterText", "KretzmannChapter", "SceneScripture" }.Contains(method.Name))
    .Select(method => $"{method.Name}({string.Join(",", method.GetParameters().Select(parameter => parameter.ParameterType.Name))})").Order().ToArray();
Console.WriteLine(string.Join("\n", methods));
var root = JsonSerializer.Deserialize<ArtifactRoot>("\"fa95e31a4c84a41ac12339853cefc795\"")!;
var api = new AtlasClient(new HttpClient());
Func<Task<Chapter>> chapter = () => api.Chapter(root.Value, 1);
Func<Task<BibleAtlas.Client.Exploring.ChapterText>> text = () => api.ChapterText(root.Value, 1);
Func<Task<KretzmannChapter>> commentary = () => api.KretzmannChapter(root.Value, 1);
Func<Task<Scene>> scene = () => api.SceneScripture(root.Value);
Console.WriteLine("All four reference-read delegates compile with an artifact root's primitive as the reference; delegates are not invoked and no HTTP request is made.");
