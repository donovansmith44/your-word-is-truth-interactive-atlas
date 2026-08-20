namespace BibleAtlas.Client;

/// <summary>
/// Batch H (split-view study): a lightweight, in-memory, app-lifetime
/// singleton (registered <c>AddSingleton</c> in Program.cs, same treatment
/// as <see cref="AtlasClient"/> -- Blazor WASM has exactly one DI scope for
/// the app's whole life, so "scoped" and "singleton" coincide here) that
/// remembers where the reader and the atlas were each left, across ordinary
/// page navigation AND split open/close. NOT persisted to localStorage --
/// explicitly out of scope this batch (a hard refresh starts fresh, per the
/// brief).
///
/// Two independent halves, deliberately not coupled to each other by this
/// service itself -- World.razor and Reader.razor each read/write their OWN
/// half, on their own lifecycle (restore on init, synced continuously at
/// each mutation -- never captured at dispose, since a concurrent
/// dispose+mount navigation guarantees no ordering between the old
/// instance's DisposeAsync and the new instance's mount); a
/// split host composes the two pages, but nothing here has any notion of
/// "split" -- it is just "the atlas's last position" and "the reader's last
/// position," full stop, exactly as true for two ordinary full-page visits
/// as for a split session.
/// </summary>
public sealed class ViewStateService
{
    public MapViewState Map { get; } = new();
    public ReaderViewState Reader { get; } = new();
}

/// <summary>
/// The atlas's last-known position. <see cref="HasData"/> is false until the
/// first real write (World.razor's SyncViewState, at each window/mode
/// mutation; the camera fields via OnCameraChanged) -- distinguishes "never
/// visited this session" (fall back to the DefaultFrom/DefaultTo Gospels-era
/// default) from "visited, and genuinely left sitting at a window/ref" (restore
/// THAT, even where it happens to coincide with the default numbers).
/// <see cref="ScriptureRef"/> non-null means the atlas was left in scripture
/// mode -- <see cref="From"/>/<see cref="To"/> are only meaningful when it's
/// null (time mode), the same one-or-the-other discipline World.razor's own
/// <c>_scriptureRef</c> field already keeps; a scripture-mode restore does
/// not also restore a saved camera (see World.razor's own RestoreMapState) --
/// re-entering scripture mode always re-fits, exactly like a fresh
/// <c>/world?ref=</c> deep link would, since a scripture jump is a
/// deliberate destination, not a snapshot to recreate pixel-for-pixel.
/// <see cref="Follow"/> is a split-only concept (standalone /world has no
/// toggle chip to ever change it), but travels here rather than being scoped
/// to the split pane specifically, per the brief's "one map state, it is the
/// same atlas" -- only a split-mode instance's own SyncViewState ever
/// writes it, though, so an intervening standalone visit can never reset it.
/// </summary>
public sealed class MapViewState
{
    public bool HasData { get; set; }
    public int From { get; set; }
    public int To { get; set; }
    public string? ScriptureRef { get; set; }
    public bool Follow { get; set; } = true;
    public double? CenterLat { get; set; }
    public double? CenterLon { get; set; }
    public double? Zoom { get; set; }
}

/// <summary>
/// The reader's last-known position. <see cref="HasData"/> mirrors
/// <see cref="MapViewState.HasData"/>'s own "never visited yet" distinction
/// -- it is what lets World.razor's own "Read beside the map" affordance
/// fall back to Genesis 1 rather than a meaningless GEN/0 on a session that
/// never opened the reader at all this visit. <see cref="ScrollY"/> is a
/// plain pixel offset (the lightweight option -- no verse-anchored scroll
/// restore), captured/restored only when landing back on the SAME
/// book+chapter that was last left (Reader.razor's own restore guard) -- a
/// DIFFERENT chapter always starts at its own top, exactly like ordinary
/// browser navigation already would.
/// </summary>
public sealed class ReaderViewState
{
    public bool HasData { get; set; }
    public string Book { get; set; } = "GEN";
    public int Chapter { get; set; } = 1;
    public double ScrollY { get; set; }
}
