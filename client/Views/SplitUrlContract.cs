namespace BibleAtlas.Client.Views;

/// <summary>
/// Batch CORPREAD-1a, DELIVERABLE 0a (owner standing order, verbatim:
/// "henceforth you write the contracts first"; spec §4d: "URLs/deep links
/// are projections"). The declared vocabulary <see cref="Components.CompositionSplit"/>'s
/// own URL-projection logic (that component's own header, "RULING 5" --
/// added this batch) reads and writes -- the ONE place either half of this
/// grammar is spelled as a literal query-string KEY, so a future edit that
/// hand-writes "split"/"follow" as a bare string literal anywhere else is a
/// visible drift, not a silent second vocabulary.
///
/// GRAMMAR:
///   `?split={a ViewNames constant}` -- the GUEST view's own stable name
///   (e.g. <see cref="ViewNames.World"/>), never a boolean "1" -- see the
///   MIGRATION note below for why this superseded the pre-existing boolean.
///   Read ONLY against a currently-registered view name
///   (<see cref="ViewRegistry.TryGet"/>) different from the reading
///   component's own <c>HostName</c> -- anything else (missing, unknown,
///   self-referential) is treated as "no split," fail-soft, never a thrown
///   fault or a stuck restore.
///
///   `&amp;follow=1` -- present (the literal <see cref="FollowTrueValue"/>)
///   iff <see cref="State.ViewArrangement.Follow"/> is true for the CURRENT
///   split; absent means false. There is no "&amp;follow=0" form -- same
///   "presence is the true value" shape the retired `?split=1` boolean
///   already used elsewhere in this app (e.g. the reader's own picker-jump
///   query keys). A `follow=1` with no `split` present alongside it is never
///   PRODUCED by this app and is simply ignored on read.
///
/// LAW (CONTRACT.md, "Split-URL projection"): the URL is a PROJECTION of the
/// live <see cref="State.ViewArrangement"/> atom, never a second writer.
/// Exactly two directions of data flow exist, and no third:
///   1. RESTORE-ONCE-ON-LOAD -- read exactly once per host-page component
///      INSTANCE (a real guard field, never re-armed by a later, unrelated
///      parameter set), dispatching <see cref="State.EnterSplit"/> or
///      <see cref="State.EnterSingle"/> to make the atom match the URL that
///      was actually navigated to.
///   2. PROJECT-ON-CHANGE -- every time the atom's own value changes
///      (regardless of which dispatch site caused it -- a hatch click, a
///      guest's own close, a picker-jump, ...), the URL is resynced
///      (`NavigationManager.NavigateTo(..., replace: true)`), guarded by a
///      same-URL no-op check so a resync that would not actually change
///      anything never re-enters Blazor's own navigation pipeline.
/// There is no THIRD path that re-reads the query string LATER in an
/// already-restored instance's lifetime and re-dispatches from it -- that
/// would be exactly the router-echo loop spec §4d's no-echo law forbids
/// (compare <see cref="State.LinkDerivedIntent{T}"/>'s own Origin-stamping
/// discipline for the analogous atom-to-atom case; the URL itself carries no
/// such stamp because there is only ever one restore per instance to guard
/// against, not a chain of atoms that could re-derive into each other).
///
/// MIGRATION (SPLIT-PERSIST-1): supersedes the pre-existing `?split=1`
/// boolean (Batch H/F2/ST-2/VC-1), which only ever worked for the ONE
/// reader-hosts-world pairing (Reader.razor's own retired `SplitQuery`
/// parameter) and left Sources/Kretzmann/Concord hosting with a disclosed
/// "does not survive refresh" limitation (see ViewRegistrySetup.cs's own
/// EnterSplitSourcesHostsReader/EnterSplitKretzmannHostsReader/
/// EnterSplitConcordHostsReader comments, pre-CORPREAD-1a). There is only
/// ONE vocabulary now, not two (owner order, verbatim: "unify onto the new
/// contract, do not leave two URL vocabularies") -- `split=1` is no longer a
/// recognized value under this grammar (it is not a registered
/// <see cref="ViewNames"/> constant), so a stale bookmark/link carrying it
/// fails soft exactly like a missing param: no split, no exception.
/// </summary>
public static class SplitUrlContract
{
    public const string SplitParam = "split";
    public const string FollowParam = "follow";
    public const string FollowTrueValue = "1";
}
