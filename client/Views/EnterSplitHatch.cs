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
///
/// <see cref="HostView"/> (fix round 1, controller ruling 2): "the declared
/// enter-split HATCH is the hosting declaration" -- distinct from
/// <see cref="OwnerView"/> (who DECLARES/renders the button) because they
/// are not always the same view. World's own hatch is declared BY World
/// ("Read beside the map" lives on World's own page) but its OWN Invoke()
/// makes READER the host (R7: byte-identical to pre-VC-1 -- see
/// ViewRegistrySetup's own EnterSplitWorldRequestsReader). Reader's and
/// Sources' own hatches have OwnerView == HostView (the declaring page
/// becomes the host). This is what the R6 hatch-conformance test's own
/// CompositionSplit tripwire (client.Tests) checks against: EVERY hatch's
/// own HostView must render through <c>&lt;CompositionSplit HostName=... /&gt;</c>
/// somewhere in that view's own component -- "no separate CanHost flag: the
/// hatch vocabulary IS the hosting declaration."
/// </summary>
public sealed class EnterSplitHatch : IEscapeHatch
{
    private readonly Func<Task> _invoke;

    public EnterSplitHatch(string ownerView, string partnerView, string hostView, Func<Task> invoke)
    {
        OwnerView = ownerView;
        PartnerView = partnerView;
        HostView = hostView;
        _invoke = invoke;
    }

    public string Kind => HatchKinds.EnterSplit;

    public string OwnerView { get; }

    public string PartnerView { get; }

    public string HostView { get; }

    public Task Invoke() => _invoke();
}
