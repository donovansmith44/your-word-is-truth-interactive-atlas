using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class ViewStyleLawTests
{
    private static readonly Regex ClassAttribute = new("class=\"([^\"]*)\"", RegexOptions.Compiled);
    private static readonly Regex ComponentTag = new(@"<([A-Z][A-Za-z0-9]*)[\s/>]", RegexOptions.Compiled);
    private static readonly Regex CssComment = new(@"/\*.*?\*/", RegexOptions.Compiled | RegexOptions.Singleline);
    private static readonly Regex CssClassSelector = new(@"\.(-?[A-Za-z_][A-Za-z0-9_-]*)", RegexOptions.Compiled);

    private static string Client => ConformanceTests.ClientRoot;

    private static IReadOnlyDictionary<string, string> RazorByComponent() =>
        Directory.EnumerateFiles(Client, "*.razor", SearchOption.AllDirectories)
            .Where(file => !file.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}") && !file.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}"))
            .GroupBy(Path.GetFileNameWithoutExtension)
            .ToDictionary(group => group.Key!, group => group.Single());

    private static IReadOnlySet<string> ViewsAndTheirChildren()
    {
        var byComponent = RazorByComponent();
        var pending = new Queue<string>(Directory.EnumerateFiles(Path.Combine(Client, "Views"), "*.razor"));
        var seen = new HashSet<string>();
        while (pending.TryDequeue(out var file))
        {
            if (!seen.Add(file))
            {
                continue;
            }

            foreach (Match tag in ComponentTag.Matches(File.ReadAllText(file)))
            {
                if (byComponent.TryGetValue(tag.Groups[1].Value, out var child))
                {
                    pending.Enqueue(child);
                }
            }
        }

        return seen;
    }

    private static IEnumerable<(string File, string Class)> EmittedClasses(string file) =>
        ClassAttribute.Matches(File.ReadAllText(file))
            .SelectMany(match => match.Groups[1].Value.Split(' ', StringSplitOptions.RemoveEmptyEntries))
            .Where(token => !token.StartsWith('@'))
            .Select(token => (Path.GetFileName(file), token));

    private static IReadOnlySet<string> DefinedClasses()
    {
        var css = CssComment.Replace(File.ReadAllText(Path.Combine(Client, "wwwroot", "css", "app.css")), string.Empty);
        return CssClassSelector.Matches(css).Select(match => match.Groups[1].Value).ToHashSet();
    }

    [Fact]
    public void EveryClassAViewOrItsChildEmits_IsDefinedInAppCss()
    {
        // Arrange
        var files = ViewsAndTheirChildren();
        var defined = DefinedClasses();

        // Act
        var emitted = files.SelectMany(EmittedClasses).Distinct().ToList();
        var undefined = emitted.Where(entry => !defined.Contains(entry.Class)).Select(entry => $"{entry.File}: .{entry.Class}").ToList();

        // Assert
        Assert.Contains(files, file => Path.GetFileName(file) == "FocusView.razor");
        Assert.Contains(files, file => Path.GetFileName(file) == "PageControls.razor");
        Assert.NotEmpty(emitted);
        Assert.Empty(undefined);
    }
}
