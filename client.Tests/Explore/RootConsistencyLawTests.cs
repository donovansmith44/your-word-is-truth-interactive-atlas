using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class RootConsistencyLawTests
{
    private const int Size = 300;
    private const string RootA = "root-a";
    private const string RootB = "root-b";
    private const string TheRawOpening = "PageWindow`1.Opened";
    private static readonly Regex OpensARawWindow = new(@"PageWindow<[^>]*>\.Opened\(", RegexOptions.Compiled);
    private static readonly string[] TheDoorSource = ["Paging.cs"];

    private static readonly Dictionary<string, Func<PresentationRequest, Task<PageWindow<Entry>>>> Doors = new()
    {
        ["Paging.Window(PresentationRequest, EdgeKind)"] = presenting => Paging.Window(presenting, EdgeKind.MentionedIn),
    };

    private enum Moment
    {
        Opening,
        More,
        Less,
        Retry,
        Revisit,
    }

    [Fact]
    public void Every_production_page_window_is_opened_through_a_root_aware_door_this_law_walks()
    {
        // Arrange
        var assemblies = new[] { typeof(Paging).Assembly, typeof(PersonMentionsList).Assembly };

        // Act
        var openers = assemblies.SelectMany(assembly => assembly.GetTypes())
            .Where(type => !type.IsDefined(typeof(CompilerGeneratedAttribute), false))
            .SelectMany(type => type.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static | BindingFlags.Instance | BindingFlags.DeclaredOnly))
            .Where(method => !method.IsDefined(typeof(CompilerGeneratedAttribute), false) && OpensAWindow(method.ReturnType))
            .Select(method => (Name: Signature(method), method.IsPublic))
            .Order()
            .ToList();

        // Assert
        Assert.Equal(
            Doors.Keys.Select(door => (door, true)).Append((TheRawOpening, false)).Order(),
            openers.Select(opener => (opener.Name.StartsWith(TheRawOpening) ? TheRawOpening : opener.Name, opener.IsPublic)));
    }

    [Fact]
    public void Only_the_door_opens_a_window_over_a_raw_read()
    {
        // Arrange
        var sources = ConformanceTests.ClientSourceFiles();

        // Act
        var opening = sources.Where(file => OpensARawWindow.IsMatch(File.ReadAllText(file))).Select(Path.GetFileName).Order();

        // Assert
        Assert.Equal(TheDoorSource, opening);
    }

    [Fact]
    public async Task Every_door_shows_one_artifact_root_whenever_the_root_moves_and_hears_that_it_moved()
    {
        // Arrange
        var cases = Doors.SelectMany(door => Enum.GetValues<Moment>().Select(moment => (Door: door, Moment: moment)));

        // Act
        var offenders = new List<string>();
        foreach (var (door, moment) in cases)
        {
            if (await Offence(door.Value, moment) is { } offence)
            {
                offenders.Add($"{door.Key} at {moment}: {offence}");
            }
        }

        // Assert
        Assert.Empty(offenders);
    }

    private static async Task<string?> Offence(Func<PresentationRequest, Task<PageWindow<Entry>>> door, Moment moment)
    {
        var graph = new RootedMentions(Size);
        var presenting = await graph.Presenting();
        if (moment == Moment.Opening)
        {
            graph.Root = RootB;
            return await Refused(() => door(presenting)) && presenting.Element.Moved ? null : "a window opened over another artifact's page";
        }

        var window = await door(presenting);
        var seen = new List<IReadOnlyList<string>> { Roots(window) };
        foreach (var turn in TurnsAt[moment](new Walk(graph, presenting, door, opened => window = opened)))
        {
            await turn(window);
            seen.Add(Roots(window));
        }

        return seen.All(roots => roots.SequenceEqual([RootA])) && window.Moved == !ReadsOnlyHeldPages.Contains(moment) ? null : $"showed roots {string.Join(" then ", seen.Select(roots => string.Join("+", roots)))}, moved {window.Moved}";
    }

    private static readonly Moment[] ReadsOnlyHeldPages = [Moment.Less];

    private sealed record Walk(RootedMentions Graph, PresentationRequest Presenting, Func<PresentationRequest, Task<PageWindow<Entry>>> Door, Action<PageWindow<Entry>> Reopened);

    private static readonly Dictionary<Moment, Func<Walk, IReadOnlyList<Func<PageWindow<Entry>, Task>>>> TurnsAt = new()
    {
        [Moment.Opening] = _ => [],
        [Moment.More] = walk => [_ => Moved(walk.Graph), window => window.More()],
        [Moment.Less] = walk => [window => window.More(), window => window.More(), window => window.More(), _ => Moved(walk.Graph), window => window.Fewer(), window => window.Fewer()],
        [Moment.Retry] = walk => [_ => Task.FromResult(walk.Graph.Failing(1)), window => window.More(), _ => Moved(walk.Graph), window => window.Resume()],
        [Moment.Revisit] = walk => [window => window.More(), _ => Moved(walk.Graph), async _ => walk.Reopened(await walk.Door(walk.Presenting)), window => window.More(), window => window.More()],
    };

    private static Task Moved(RootedMentions graph)
    {
        graph.Root = RootB;
        return Task.CompletedTask;
    }

    private static async Task<bool> Refused(Func<Task<PageWindow<Entry>>> opening)
    {
        try
        {
            await opening();
            return false;
        }
        catch (ArtifactMoved)
        {
            return true;
        }
    }

    private static IReadOnlyList<string> Roots(PageWindow<Entry> window) =>
        window.Shown.Select(entry => Positions.Of(entry.Neighbour.Target).Label.Split(':')[0]).Distinct().ToList();

    private static bool OpensAWindow(Type returned) =>
        returned.IsGenericType && (returned.GetGenericTypeDefinition() == typeof(PageWindow<>) || (returned.GetGenericTypeDefinition() == typeof(Task<>) && OpensAWindow(returned.GetGenericArguments()[0])));

    private static string Signature(MethodInfo method) =>
        $"{method.DeclaringType!.Name}.{method.Name}({string.Join(", ", method.GetParameters().Select(parameter => parameter.ParameterType.Name))})";
}
