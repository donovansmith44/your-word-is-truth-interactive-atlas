namespace BibleAtlas.Client;

// Not persisted to localStorage (a hard refresh starts fresh). Restored on init and synced
// continuously at each mutation -- never captured at dispose, since a concurrent dispose+mount
// navigation guarantees no ordering between the old instance's DisposeAsync and the new
// instance's mount.
public sealed class ViewStateService
{
    public MapViewState Map { get; } = new();
    public ReaderViewState Reader { get; } = new();

    // Reference-counted, not a single value or plain HashSet: more than one reading surface
    // (Reader.razor, Kretzmann.razor) can mount the SAME chapter simultaneously (a Following
    // split), and two surfaces can also show DIFFERENT chapters independently -- a last-writer-
    // wins value or an uncounted set would let one instance's unmount wrongly hide a chapter
    // another instance is still genuinely showing. Each instance increments its own chapter on a
    // successful mount and decrements on unmount; a chapter counts as mounted while any count is
    // nonzero. Mounting only happens on a genuinely successful load -- a failed fetch never
    // mounts anything -- and an instance unmounts its own previously-mounted chapter before
    // starting a new fetch, so it can never leak a stale mounted count.
    private readonly Dictionary<(string Book, int Chapter), int> _mountedReaderChapterCounts = new();

    public void MountReaderChapter(string book, int chapter)
    {
        var key = (book, chapter);
        _mountedReaderChapterCounts[key] = _mountedReaderChapterCounts.GetValueOrDefault(key) + 1;
    }

    // Safe to call on a chapter this caller never actually mounted -- a no-op rather than
    // throwing or going negative.
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

    public bool IsReaderChapterMounted(string book, int chapter) => _mountedReaderChapterCounts.ContainsKey((book, chapter));
}

// HasData is false until the first real write -- distinguishes "never visited this session"
// (fall back to the default Gospels-era window) from "visited, and genuinely left sitting at a
// window/ref" (restore that, even where it coincides with the default numbers). ScriptureRef
// non-null means the atlas was left in scripture mode -- From/To are only meaningful when it's
// null (time mode); a scripture-mode restore does not also restore a saved camera, since a
// scripture jump is a deliberate destination, not a snapshot to recreate pixel-for-pixel.
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

public sealed class ReaderViewState
{
    public bool HasData { get; set; }
    public string Book { get; set; } = "GEN";
    public int Chapter { get; set; } = 1;
    public double ScrollY { get; set; }
}
