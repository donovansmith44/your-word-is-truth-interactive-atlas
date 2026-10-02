using System.Reflection;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class ExploringBoundaryLawTests
{
    private const BindingFlags Declared = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.DeclaredOnly;

    [Fact]
    public void Only_the_exploring_core_builds_an_explorable_or_a_walk_or_resolves_through_the_explorer()
    {
        // Arrange
        var guarded = typeof(Explorable).GetConstructors(Declared).Cast<MethodBase>()
            .Concat(typeof(Explore<Unit>).GetConstructors(Declared))
            .Concat(typeof(IExplorer).GetMethods(Declared))
            .Where(member => !member.IsPrivate);

        // Act
        var reach = guarded.Select(member => $"{member.Module.Assembly.GetName().Name}: {member.DeclaringType!.Name}.{member.Name}({string.Join(", ", member.GetParameters().Select(parameter => parameter.ParameterType.Name))}) {(member.IsAssembly ? "internal" : "public")}").ToList();

        // Assert
        Assert.Equal(
            [
                "BibleAtlas.Client.Exploring: Explorable..ctor(NodeRecord, String, ServedPages) internal",
                "BibleAtlas.Client.Exploring: Explorable..ctor(EdgeRecord, String, ServedPages) internal",
                "BibleAtlas.Client.Exploring: Explore`1..ctor(Func`3) internal",
                "BibleAtlas.Client.Exploring: IExplorer.Resolve(IReadOnlyList`1) internal",
            ],
            reach);
    }
}
