using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class TypefaceLawTests
{
    private const string HeadingToken = "--font-heading";
    private const string BodyToken = "--font-body";

    private static readonly IReadOnlyDictionary<string, string> FamilyByToken = new Dictionary<string, string>
    {
        [HeadingToken] = "Overpass",
        [BodyToken] = "Atkinson Hyperlegible",
    };

    private static readonly Regex CssComment = new(@"/\*.*?\*/", RegexOptions.Compiled | RegexOptions.Singleline);
    private static readonly Regex FontFaceBlock = new(@"@font-face\s*\{[^}]*\}", RegexOptions.Compiled);
    private static readonly Regex Declaration = new(@"(?<property>--?[A-Za-z][A-Za-z0-9-]*|[A-Za-z][A-Za-z-]*)\s*:\s*(?<value>[^;{}]+);", RegexOptions.Compiled);

    private static string Css() =>
        CssComment.Replace(File.ReadAllText(Path.Combine(ConformanceTests.ClientRoot, "wwwroot", "css", "app.css")), string.Empty);

    private static IEnumerable<(string Property, string Value)> Declarations(string css) =>
        Declaration.Matches(css).Select(match => (match.Groups["property"].Value, match.Groups["value"].Value.Trim()));

    private static string FirstFamily(string stack) => stack.Split(',')[0].Trim().Trim('"', '\'');

    [Fact]
    public void AppCssNamesAFontFamilyOnlyThroughTheHeadingAndBodyTokens()
    {
        // Arrange
        var css = Css();
        var faces = FontFaceBlock.Matches(css).Select(match => match.Value).ToList();
        var rules = FontFaceBlock.Replace(css, string.Empty);
        var allowed = FamilyByToken.Keys.Select(token => $"var({token})").Append("inherit").ToHashSet();

        // Act
        var tokens = Declarations(rules).Where(d => d.Property.StartsWith("--font-")).ToDictionary(d => d.Property, d => FirstFamily(d.Value));
        var offenders = Declarations(rules)
            .Where(d => d.Property is "font-family" or "font" && !allowed.Contains(d.Value))
            .Select(d => $"{d.Property}: {d.Value}")
            .ToList();
        var faceFamilies = faces.SelectMany(Declarations).Where(d => d.Property == "font-family").Select(d => FirstFamily(d.Value)).ToHashSet();

        // Assert
        Assert.Equal(FamilyByToken, tokens);
        Assert.Empty(offenders);
        Assert.Equal(FamilyByToken.Values.ToHashSet(), faceFamilies);
    }
}
