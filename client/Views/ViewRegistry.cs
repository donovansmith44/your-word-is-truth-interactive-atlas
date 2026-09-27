using System.Diagnostics.CodeAnalysis;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Views;

public sealed record ViewMountContext(
    bool SplitMode,
    EventCallback OnRequestClose,
    Action<Func<string, Task>>? RegisterQueryHandler);

public sealed class RegisteredView : IView
{
    public RegisteredView(string name, ViewCapabilities capabilities, Func<ViewMountContext, RenderFragment> mount, IReadOnlyList<IEscapeHatch> escapeHatches)
    {
        Name = name;
        Capabilities = capabilities;
        Mount = mount;
        EscapeHatches = escapeHatches;
        Components = Array.Empty<IViewComponent>();
    }

    public string Name { get; }

    public ViewCapabilities Capabilities { get; }

    public Func<ViewMountContext, RenderFragment> Mount { get; }

    public IReadOnlyList<IViewComponent> Components { get; }

    public IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}

public sealed record CompositionLayout(string Kind) : ICompositionLayout;

public sealed class LiveComposition : IViewComposition
{
    public LiveComposition(ViewArrangement arrangement, ViewRegistry registry)
    {
        Layout = new CompositionLayout(arrangement.LayoutKind);
        Members = arrangement.Members.Select(name => (IView)registry.Get(name)).ToList();
    }

    public string Name => string.Join("+", Members.Select(m => m.Name));

    public IReadOnlyList<IViewComponent> Components => Array.Empty<IViewComponent>();

    public IReadOnlyList<IEscapeHatch> EscapeHatches => Members.SelectMany(m => m.EscapeHatches).ToList();

    public IReadOnlyList<IView> Members { get; }

    public ICompositionLayout Layout { get; }
}

public sealed class ViewRegistry
{
    private readonly Dictionary<string, RegisteredView> _views;

    public ViewRegistry(IReadOnlyList<RegisteredView> views)
    {
        _views = views.ToDictionary(v => v.Name);
    }

    public IReadOnlyCollection<RegisteredView> All => _views.Values;

    public RegisteredView Get(string name) =>
        _views.TryGetValue(name, out var view)
            ? view
            : throw new InvalidOperationException($"ViewRegistry: '{name}' is not a registered view. Registered names: {string.Join(", ", _views.Keys)}.");

    public bool TryGet(string name, [MaybeNullWhen(false)] out RegisteredView view) => _views.TryGetValue(name, out view);

    public ViewCapabilities CapabilitiesOf(string name) => TryGet(name, out var view) ? view.Capabilities : ViewCapabilities.None;

    public IViewComposition ComposeFrom(ViewArrangement arrangement) => new LiveComposition(arrangement, this);
}

public static class ViewCompositionExtensions
{
    public static bool IsHostedBy(this IViewComposition composition, string viewName) =>
        composition.Layout.Kind == LayoutKinds.SplitH
        && composition.Members.Count > 0
        && composition.Members[0].Name == viewName;
}
