using BibleAtlas.Client.Contracts;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Views;

/// <summary>
/// R1: the member-context parameters a mounted view needs, whichever ROLE it
/// is playing this render:
/// <list type="bullet">
/// <item><see cref="SplitMode"/> -- true whenever this instance is embedded
/// in a composition (host OR guest), false when it fills the page alone.
/// Deliberately reuses the pre-existing PARAMETER NAME `SplitMode` already
/// on <c>World.razor</c> (Batch H) rather than renaming it to something more
/// abstract like "Embedded" -- <c>World.razor</c> has ~30 internal call
/// sites keyed off that exact identifier; keeping the name is zero-churn on
/// this batch's single highest-blast-radius file, disclosed here rather
/// than silently reusing a "legacy" name without explanation.</item>
/// <item><see cref="IsHost"/> -- meaningful only while SplitMode; true for
/// Members[0] (left pane, owns the divider + <see cref="Guest"/> slot),
/// false for Members[1] (right pane, gets <see cref="OnRequestClose"/>
/// wired to <c>CloseGuest</c>).</item>
/// <item><see cref="OnRequestClose"/> -- "close ME." For the guest, dispatches
/// <c>CloseGuest</c>; for the host, dispatches <c>EnterSingle(guestName)</c>
/// (both computed by <see cref="Components.CompositionHost"/>, never by the
/// view itself -- a view only ever needs "call this when my own close
/// affordance fires," never which intent that means).</item>
/// <item><see cref="RegisterQueryHandler"/> -- the generalized "no
/// nested-popup rule" (Batch H) plumbing: a GUEST that supports being the
/// target of a popover-originated scene query (today: World's own
/// <c>ApplyExternalQuery</c>) hands its own instance delegate UP through
/// this callback. Whichever HOST embeds a <see cref="Components.CompositionHost"/>
/// for its own guest slot reads the composed result back via that child's
/// own <c>@ref</c> (<c>Components.CompositionHost.NavigateGuestInSplit</c>)
/// -- a delegate handoff, not a typed `@ref` on the guest itself, so it
/// stays fully generic across ANY guest type (no hardcoded "World" anywhere
/// in this mechanism). A guest that does not support query-routing
/// (Sources, Reader-as-guest) simply never calls this.</item>
/// </list>
/// A view playing HOST does not receive a mount context at all in this
/// batch's design -- it is reached by ordinary Blazor routing (its own
/// `@page`), never mounted via the registry, and embeds its OWN
/// <see cref="Components.CompositionHost"/> directly to resolve its guest
/// slot. See that component's own header for why.
/// </summary>
public sealed record ViewMountContext(
    bool SplitMode,
    bool IsHost,
    EventCallback OnRequestClose,
    Action<Func<string, Task>>? RegisterQueryHandler);

/// <summary>
/// R1 / Total-capture (§0): a registered view arrives as a real
/// <see cref="IView"/> implementation, not an ad-hoc tuple. <see cref="Components"/>
/// is disclosed-empty (this batch models the VIEW/COMPOSITION layer per spec
/// §4b -- registering every child widget as its own <see cref="IViewComponent"/>
/// is out of scope here, not silently skipped) -- see this batch's own report
/// for the disclosure. <see cref="Capabilities"/>/<see cref="Mount"/> are
/// this concrete type's own additive data (R1's mount-factory + capability
/// requirements), beyond what <see cref="IView"/> itself declares.
/// </summary>
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

/// <summary>
/// R1: "a ViewRegistry ... mapping stable view NAMES to (a) a mount factory
/// ... and (b) declared CAPABILITIES as data." One singleton instance
/// (registered in Program.cs, built by <see cref="ViewRegistrySetup.Build"/>),
/// holding exactly the three registrations this batch ships
/// (<see cref="ViewNames.Reader"/>/<see cref="ViewNames.World"/>/
/// <see cref="ViewNames.Sources"/>) -- a future view registers here and
/// nowhere else (<see cref="Components.CompositionHost"/> never hardcodes a
/// view name).
/// </summary>
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

    public bool TryGet(string name, out RegisteredView view) => _views.TryGetValue(name, out view!);

    public ViewCapabilities CapabilitiesOf(string name) => TryGet(name, out var view) ? view.Capabilities : ViewCapabilities.None;
}
