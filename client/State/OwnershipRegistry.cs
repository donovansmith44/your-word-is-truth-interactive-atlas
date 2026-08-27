namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-3 (R4): a minimal, reusable "one owner per name, latest claim
/// wins" primitive -- the SAME PATTERN <see cref="EffectRegistry"/> (R1) uses
/// for effect ownership, generalized here for a caller that needs
/// claim/release/latest-wins WITHOUT the effect machinery's own
/// AppliesTo/Materialize/Source-atom ceremony. Deliberately NOT built on
/// <see cref="IEffectRegistry"/>/<see cref="Contracts.IStateEffect{T}"/>: an
/// EFFECT (that contract's own doc comment, verbatim) is "the async
/// materialization of an atom's value into the world (a scene fetch, JS
/// interop)" -- FocusStack ownership materializes nothing; it exists purely
/// to decide WHICH of several simultaneously-mountable components is the
/// CURRENT one, for a shared atom to mirror. Forcing that through
/// IStateEffect{T} would mean a permanently-false AppliesTo and a
/// never-meaningfully-called Materialize -- ceremony with no honest content.
///
/// R4's own multi-instance reality check (see the batch report's own
/// evidence): THREE independent sites can each mount an ExplorerPopover --
/// Reader.razor, World.razor (the embedded split pane), and
/// MainLayout.razor's own hamburger "continue" popover -- and nothing in the
/// app cross-clears one when another opens, so more than one CAN be alive at
/// once. This registry is what lets the FocusStack atom hold "the ACTIVE
/// session" (R4 option (b)) -- claimed on a popover's own OnInitializedAsync,
/// released on its own Dispose -- without literally repurposing the effect
/// machinery for a job it doesn't describe.
/// </summary>
public sealed class OwnershipRegistry
{
    private readonly Dictionary<string, object> _owners = new();

    public OwnershipClaim Claim(string name)
    {
        var token = new object();
        _owners[name] = token; // latest claim wins -- supersedes any prior claimant immediately, synchronously
        return new OwnershipClaim(this, name, token);
    }

    internal bool IsCurrent(string name, object token) =>
        _owners.TryGetValue(name, out var owner) && ReferenceEquals(owner, token);

    internal void Release(string name, object token)
    {
        if (_owners.TryGetValue(name, out var owner) && ReferenceEquals(owner, token))
        {
            _owners.Remove(name);
        }
    }
}

/// <summary>
/// The handle <see cref="OwnershipRegistry.Claim"/> returns. <see cref="IsCurrent"/>
/// is read BEFORE every write a caller wants to mirror into a shared atom
/// (see ExplorerPopover.razor's own use) -- once superseded, it stays false
/// forever (a claim, once lost, is never reacquired by the SAME handle;
/// the losing component keeps working fully off its own local state, per
/// the batch report's own R4 design notes, it just stops mirroring into the
/// shared atom). Disposing releases the claim -- idempotent, same discipline
/// as <see cref="EffectClaim"/>.
/// </summary>
public sealed class OwnershipClaim : IDisposable
{
    private readonly OwnershipRegistry _registry;
    private readonly string _name;
    private readonly object _token;
    private bool _disposed;

    internal OwnershipClaim(OwnershipRegistry registry, string name, object token)
    {
        _registry = registry;
        _name = name;
        _token = token;
    }

    public bool IsCurrent => !_disposed && _registry.IsCurrent(_name, _token);

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _registry.Release(_name, _token);
    }
}
