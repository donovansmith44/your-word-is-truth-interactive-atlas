using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-1: the canonical "where we are in Scripture" value (spec §4d,
/// <see cref="AtomNames.Locus"/>) -- the single shared source SYNC-1's own
/// five divergent copies (Reader's `_book`/`_chapterNum`, World's `FollowRef`
/// chain, ViewStateService's `Reader`/`Map.ScriptureRef` halves,
/// `MountedReaderChapter`) all collapse into. See the batch report's own
/// divergent-copy retirement table for what replaced each one.
///
/// SHAPE: book code + chapter only, for now. Every chapter-navigation path
/// this batch migrates (both ScripturePicker mounts, `reader-prev`/
/// `reader-next`, deep links, "read in context" hatches) already bottoms out
/// at a book+chapter -- none of them round-trip a specific verse THROUGH
/// shared state today (ScripturePicker's own optional verse-from/-to fields
/// stay local UI state, consumed only by the ref string `Apply` composes;
/// VerseLine/ExplorerPopover's own verse-anchor scrolling is a separate,
/// unmigrated mechanism -- see the report's disclosed-unmigrated-writers
/// table). EXTENSIBLE toward verse/range: a later batch that needs to share
/// a verse-level position can add a nullable `VerseRange? Verse` member here
/// (extend-only, per the Contracts/ discipline this record itself is not
/// bound by -- Locus is OUR OWN new type, not a Contracts/ member) rather
/// than reaching for a second, parallel atom.
/// </summary>
public sealed record Locus(string Book, int Chapter)
{
    /// <summary>GEN 1 -- the same fallback every pre-ST-1 copy of this state
    /// already used independently (Reader's own field initializers,
    /// ReaderViewState's own defaults, ScripturePicker's own "no
    /// CurrentBook/CurrentChapter" fallback to the TOC's first book).</summary>
    public static readonly Locus Default = new("GEN", 1);

    /// <summary>The canonical "BOOK.chapter" ref string -- the exact shape
    /// World.razor's own scripture-mode fetch (`Atlas.SceneScripture`) and
    /// the follow-chip's own label ("Following {ref}") already use.</summary>
    public string Ref => $"{Book}.{Chapter}";
}

/// <summary>
/// Batch ST-1: the Locus atom's one seed intent. Idempotent BY CONSTRUCTION
/// (law 2): <see cref="Apply"/> ignores <paramref name="current"/> and always
/// returns the same fresh <see cref="Locus"/> record for a given
/// (Book, Chapter) -- and <see cref="Locus"/>'s own record value-equality
/// means dispatching this twice in a row is a no-op the SECOND time
/// (<see cref="StateAtom{T}.Dispatch"/>'s own equality check catches it),
/// with no special-casing needed here at all.
/// </summary>
public sealed record SetLocus(string Book, int Chapter, string? Origin = null) : IIntent<Locus>
{
    public string Name => "set-locus";

    public Locus Apply(Locus current) => new(Book, Chapter);
}
