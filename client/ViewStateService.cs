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
///
/// Batch ST-1 DEMOTION (spec §4d: "ViewStateService remains the PERSISTENCE
/// layer beneath atoms -- it persists state; atoms OWN it"): this class
/// itself is byte-unchanged by ST-1 -- the demotion is entirely in what
/// reads/writes it. <see cref="ReaderViewState.Book"/>/
/// <see cref="ReaderViewState.Chapter"/>'s one write site
/// (<c>Reader.razor</c>'s <c>OnScroll</c>) now writes values that are
/// themselves thin reads of the shared <c>Locus</c> atom (client/State/
/// Locus.cs) -- satisfied by construction, no code here needed to change.
/// Every OTHER reader of <c>ViewState.Reader</c> was audited to also go
/// through the atom directly rather than this persistence layer (fix round
/// 1, review finding S-5 -- <c>World.OpenReadBesideMap</c> was the one miss,
/// now fixed). <see cref="MapViewState.ScriptureRef"/>/<see cref="MapViewState.Follow"/>
/// stay OUTSIDE this migration entirely, deliberately -- "the atlas's
/// current scene" and "the reader's actual locus" are genuinely different
/// facts (a world-picker Apply can point the scene somewhere the reader
/// never was -- see <c>World.ApplyScriptureRef</c>'s own doc comment) -- so
/// the two halves documented above stay "deliberately not coupled," exactly
/// as before ST-1.
///
/// Batch ST-2 fix round 1 (Adjudication C): <see cref="MapViewState.DividerFraction"/>
/// added -- this class's role for it is IDENTICAL to
/// <see cref="MapViewState.Follow"/>'s own: the root, cross-remount
/// persistence source a fresh <c>ViewArrangement.Split</c> arm is re-seeded
/// from (<c>Reader.OpenSplit</c>/the <c>SplitQuery</c>-consumption block,
/// via <c>EnterSplit</c>'s own <c>DefaultDividerFraction</c> parameter) --
/// see that field's own doc comment for why the arm itself cannot carry it
/// through a <c>ReaderOnly</c>/<c>WorldOnly</c> detour.
/// </summary>
public sealed class ViewStateService
{
    public MapViewState Map { get; } = new();
    public ReaderViewState Reader { get; } = new();

    // M-D3/U6 (owner: "'read the whole chapter' affordance REMOVED when
    // already reading that chapter -- a chapter-aware policy, not a new
    // data path"): DELIBERATELY separate from Reader (above) -- Reader
    // is a PERSISTENT "last known position" record, kept even after
    // Reader.razor unmounts (that class's own doc comment: "never
    // captured at dispose"), so it stays true on /world after navigating
    // away and would wrongly suppress the affordance there too. This
    // signal is the opposite: a chapter counts as "mounted" only while
    // SOME reading surface is ACTIVELY showing it right now.
    //
    // Batch CORPREAD-2 fix round (Q-1/Q-2, review): this was a single
    // nullable `(Book, Chapter)?` with exactly ONE documented writer
    // (Reader.razor). K2 made Kretzmann.razor a second, genuine full
    // reading of the chapter too -- a SECOND simultaneous writer, most
    // visibly in `EnterSplitKretzmannHostsReader` (Kretzmann hosting,
    // Reader as guest, Following, both legitimately showing the SAME
    // chapter at once). A single last-writer-wins value is wrong on BOTH
    // sides of that: (Q-2) while RELEASED, the two panes can legitimately
    // show DIFFERENT chapters, and whichever page wrote last silently wins
    // for both; even while FOLLOWING the same chapter, a plain non-counted
    // `HashSet` would be wrong too -- one pane disposing (or navigating
    // away) would unmount a chapter the OTHER pane is still genuinely
    // showing. A REFERENCE-COUNTED multiset (below) is the minimal correct
    // generalization: each reading-surface INSTANCE increments its own
    // chapter on a successful mount and decrements it on unmount/dispose;
    // the chapter counts as mounted for as long as ANY count is nonzero,
    // so two simultaneous readers of the same chapter (the common
    // Following-split case) never race each other, and two readers on
    // DIFFERENT chapters (the released-split case) are tracked
    // independently by construction, never last-writer-wins.
    //
    // (Q-1) Mounting is now scoped to a GENUINELY SUCCESSFUL load only
    // (Reader.razor's LoadChapter / Kretzmann.razor's LoadChapterAsync,
    // each in their own success branch) -- a failed fetch never mounts
    // anything, matching what is actually on screen (an error toast, not
    // stale or half-loaded chapter content). The PREVIOUS chapter this
    // SAME instance had mounted is unmounted at the top of that same
    // method, before the new fetch starts, keyed off that instance's own
    // `_loadedBook`/`_loadedChapterNum` (the last chapter IT actually
    // finished loading) rather than off the current navigation target --
    // so an instance can never accidentally leak a stale "still mounted"
    // count for a chapter it in fact is no longer showing, regardless of
    // whether the new navigation itself later succeeds or fails.
    private readonly Dictionary<(string Book, int Chapter), int> _mountedReaderChapterCounts = new();

    public void MountReaderChapter(string book, int chapter)
    {
        var key = (book, chapter);
        _mountedReaderChapterCounts[key] = _mountedReaderChapterCounts.GetValueOrDefault(key) + 1;
    }

    // Safe to call on a chapter this caller never actually mounted (e.g. a
    // superseded, discarded navigation, or a defensive double-call across a
    // race window) -- a no-op rather than throwing or going negative.
    public void UnmountReaderChapter(string book, int chapter)
    {
        var key = (book, chapter);
        if (!_mountedReaderChapterCounts.TryGetValue(key, out var count))
        {
            return;
        }

        if (count <= 1)
        {
            _mountedReaderChapterCounts.Remove(key);
        }
        else
        {
            _mountedReaderChapterCounts[key] = count - 1;
        }
    }

    // MiniReaderExpand.razor's own read -- "is the reader genuinely on
    // screen, on this exact chapter, at this exact moment" -- true for a
    // standalone /read/{book}/{chapter} visit, split view's own embedded
    // reader pane, AND Kretzmann's own reading surface (SPLIT-1/K2:
    // Reader.razor and Kretzmann.razor are both reused, not copied),
    // regardless of how many of those are simultaneously true at once.
    public bool IsReaderChapterMounted(string book, int chapter) => _mountedReaderChapterCounts.ContainsKey((book, chapter));
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
///
/// <see cref="DividerFraction"/> (Batch ST-2 fix round 1, Adjudication C):
/// the SAME persistence role as <see cref="Follow"/>, for the split
/// divider's own last-committed position -- written by
/// <c>Reader.OnDividerCommitted</c> (a Reader.razor concern, not really
/// "the atlas's" own state, but placed alongside <see cref="Follow"/> for
/// the identical reason that field already lives here: R5 groups
/// split-layout state with the Map half, not the Reader half). Null until
/// the user has ever actually moved the divider this session.
/// </summary>
public sealed class MapViewState
{
    public bool HasData { get; set; }
    public int From { get; set; }
    public int To { get; set; }
    public string? ScriptureRef { get; set; }
    public bool Follow { get; set; } = true;
    public double? DividerFraction { get; set; }
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
