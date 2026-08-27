using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.Views;

/// <summary>
/// R4: "Enter-split is a declared escape hatch, not page markup ... (Kind
/// 'enter-split', partner view as data) -- any view can declare one with any
/// partner." The ONE concrete <see cref="IEscapeHatch"/> implementation
/// every enter-split button in this app invokes -- Reader's "Open the map
/// beside the text," World's "Read beside the map," and Sources' new
/// "Read beside the reader" proof are all instances of this SAME class,
/// differing only in their own (owner, partner, invoke) data, never in
/// code shape.
///
/// <see cref="OwnerView"/>/<see cref="PartnerView"/> are the "partner view
/// as data" R4 asks for -- NOT part of the compiled <see cref="IEscapeHatch"/>
/// contract itself (that interface, extend-only, declares only Kind/Invoke)
/// but real, inspectable data on THIS concrete type, which is what
/// client.Tests' own hatch-conformance test (R6: "every declared enter-split
/// hatch resolves both its views in the registry") actually reads.
///
/// <see cref="Invoke"/> is expressed purely in terms of DI-singleton
/// services (the shared <c>StateAtom&lt;ViewArrangement&gt;</c>,
/// <c>ViewStateService</c>, <c>StateAtom&lt;Locus&gt;</c>,
/// <c>NavigationManager</c> -- see <see cref="ViewRegistrySetup.Build"/>'s
/// own construction site) rather than a live component instance's own
/// method -- every one of OpenSplit/OpenReadBesideMap/the new Sources hatch
/// turns out to need nothing instance-specific at all, so a hatch can be
/// constructed ONCE, at registry-build time, and invoked from ANY live
/// button that happens to render it.
/// </summary>
public sealed class EnterSplitHatch : IEscapeHatch
{
    private readonly Func<Task> _invoke;

    public EnterSplitHatch(string ownerView, string partnerView, Func<Task> invoke)
    {
        OwnerView = ownerView;
        PartnerView = partnerView;
        _invoke = invoke;
    }

    public string Kind => HatchKinds.EnterSplit;

    public string OwnerView { get; }

    public string PartnerView { get; }

    public Task Invoke() => _invoke();
}
