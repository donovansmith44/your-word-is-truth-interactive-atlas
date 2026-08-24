namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch F2, 6-ARCH: one already-resolved verse (ref + full KJV text) going
/// into a <see cref="PassageSourceUnit"/> -- the common currency every
/// verse-list surface (cross-references, THE SCRIPTURES, place est/dest)
/// hands to <see cref="PassageGrouping"/>/<c>PassageList.razor</c>.
/// </summary>
/// <param name="GroupCount">
/// Batch HOTFIX-4 requirement 7 (honest truncation signal): this verse's
/// own server-side (book,chapter) <c>VerseGroup</c>'s TRUE total verse
/// count (<c>VerseGroup.Count</c>), when this verse came from one --
/// <c>null</c> for every OTHER caller (cross-references, THE SCRIPTURES,
/// place est/dest are never <c>VerseGroup</c>-sourced, so this stays
/// <c>null</c> and <see cref="PassageBlockBuilder"/>'s own truncation pass
/// is a no-op for them, by construction, not a per-caller flag). Set by
/// EVENT witness/PRIOR-FOLLOWING callers, which ARE <c>VerseGroup</c>-shaped
/// and can silently truncate at the server's own 20-verse-per-chapter cap
/// (`scene::verse_groups_for`'s own <c>take(20)</c>).
/// </param>
/// <param name="Places">
/// M-D4 fix round 1 (R-M1, review Important-1 -- decision 3, "name links
/// everywhere... render wherever verse text renders"): this verse's own
/// attested place mentions, when the producer had them on hand from a real
/// <c>ChapterOut.Verses</c> row (<see cref="CrossRefsSection"/>,
/// <see cref="VerseTextResolver"/> -- the SAME chapter fetch those already
/// make for the verse's own TEXT, just no longer dropping this field too).
/// Null (never an empty array standing in for "none found") when the
/// producer's own source genuinely has no such row to read from
/// (<see cref="CrossRefOut.Preview"/>'s own cross-chapter-target fallback
/// text, which was never chapter-sourced even for <c>Text</c>) -- distinct
/// from an empty array, which means "resolved, attests nothing," matching
/// <see cref="Components.MentionText"/>'s own "empty is a valid, honest
/// answer" contract. <c>PassageList.razor</c> coalesces null to empty at
/// the render site (never surfaces the distinction to the component).
/// </param>
/// <param name="Persons">The <see cref="Places"/> sibling -- same rule.</param>
public sealed record PassageListVerse(string Vref, string Text, int? GroupCount = null, IReadOnlyList<PlaceRefDto>? Places = null, IReadOnlyList<PersonRefDto>? Persons = null);

/// <summary>
/// One INDEPENDENT source of verses to group into passage blocks -- e.g.
/// one cross-reference target's own member verses, one catechism
/// question's own verses, or a place's own item-level embedded citation.
/// Grouping (<see cref="PassageGrouping.Groups"/>) runs SEPARATELY per
/// unit and never merges across two different units, even if their own
/// verses happen to be numerically adjacent -- two different xref targets,
/// or two different catechism questions, are two different pieces of
/// context and must never silently read as one contiguous passage.
/// <see cref="Caption"/> (e.g. a catechism question's own title) renders on
/// every block this unit produces, when present.
/// </summary>
public sealed record PassageSourceUnit(IReadOnlyList<PassageListVerse> Verses, string? Caption = null);

/// <summary>
/// One renderable passage/lone-verse block -- <see cref="PassageList.razor"/>'s
/// own unit of display, truncation-counting, and per-entry mini-reader
/// expansion. <see cref="Span"/> is the block's own ref-range (or bare
/// vref for a lone verse); <see cref="Verses"/> is 1 (lone verse) or more
/// (passage) already-resolved verses, in order.
/// </summary>
/// <param name="TruncatedBy">
/// Batch HOTFIX-4 requirement 7: how many MORE verses this block's own
/// server-side group truly has beyond what's delivered here (0 -- the
/// overwhelming majority of blocks -- when not truncated, or when this
/// unit carries no <see cref="PassageListVerse.GroupCount"/> at all).
/// Computed once, by <see cref="PassageBlockBuilder.Build"/>, never by a
/// per-caller flag.
/// </param>
public sealed record PassageBlockData(string Span, IReadOnlyList<PassageListVerse> Verses, string? Caption, int TruncatedBy = 0)
{
    public bool IsPassage => Verses.Count >= 2;
    public string FirstVref => Verses[0].Vref;
    public string LastVref => Verses[^1].Vref;
}

/// <summary>
/// Batch F2, 6-ARCH: builds the full, ordered list of renderable
/// <see cref="PassageBlockData"/> from one or more <see cref="PassageSourceUnit"/>s
/// -- the one function every verse-list-with-grouping surface in this app
/// calls (<c>PassageList.razor</c>'s own resolve step; <c>PlaceCard.razor</c>
/// calls <see cref="PassageGrouping.Groups"/> directly instead, since its
/// own reveal-by-count mechanism -- Batch D, untouched by this batch -- is a
/// different truncation model from the down-arrow cap this batch adds; see
/// that file's own comment).
/// </summary>
/// <summary>
/// Batch F2, requirement 6b: resolves FULL KJV text for an already-
/// individually-canonical, ordered list of verse refs (e.g. a place's own
/// curated <c>established</c>/<c>destroyed</c> claim verses) via the
/// existing chapter fetch + LRU cache (<see cref="AtlasClient.Chapter"/>) --
/// the SAME "fetch whole chapters, slice out what's needed" pattern
/// <c>PlaceCard.razor</c>'s own <c>LoadChaptersFor</c> already established,
/// reused here rather than re-implemented (house verse rendering: ref +
/// FULL text, never truncated/invented). A verse whose own chapter fetch
/// fails, or that doesn't turn up in the fetched chapter, is simply
/// omitted -- graceful degrade, never a placeholder or fabricated text.
/// </summary>
public static class VerseTextResolver
{
    public static async Task<List<PassageListVerse>> ResolveAsync(AtlasClient api, IReadOnlyList<string> vrefs)
    {
        var pairs = vrefs.Select(CanonRef.ParseVerse).Select(p => (p.Book, p.Chapter)).Distinct().ToList();
        var chapters = new Dictionary<(string, int), ChapterOut>();
        try
        {
            var fetched = await Task.WhenAll(pairs.Select(p => api.Chapter(p.Book, p.Chapter)));
            foreach (var (pair, chapter) in pairs.Zip(fetched))
            {
                chapters[pair] = chapter;
            }
        }
        catch (Exception)
        {
            // graceful degrade -- verses below simply resolve to nothing found
        }

        var result = new List<PassageListVerse>();
        foreach (var vref in vrefs)
        {
            var (book, chapter, verse) = CanonRef.ParseVerse(vref);
            if (!chapters.TryGetValue((book, chapter), out var c))
            {
                continue;
            }
            // M-D4 fix round 1 (R-M1): the SAME VerseOut row already fetched
            // for .Text -- Places/Persons were always sitting right there,
            // just never read. Threading them through costs nothing extra
            // (no new fetch) and is what lets PassageList.razor's own
            // preview text carry in-text mention links, same as every other
            // surface this batch already unified.
            var cv = c.Verses.FirstOrDefault(v => v.Verse == verse);
            if (cv is not null)
            {
                result.Add(new PassageListVerse(vref, cv.Text, Places: cv.Places, Persons: cv.Persons));
            }
        }
        return result;
    }

    /// Batch HOTFIX-4 requirement 7 (honest truncation signal): the SAME
    /// text resolution as <see cref="ResolveAsync"/> above, PLUS each
    /// resolved verse's own <see cref="PassageListVerse.GroupCount"/>
    /// (attached from whichever `VerseGroup` it came from) -- the ONE
    /// shared helper both PRIOR/FOLLOWING resolvers (narrative-scoped and
    /// global-timeline-scoped, `PopoverSectionProviders.cs`) use, so the
    /// "resolve a VerseGroup list's own text, honestly" step is wired
    /// exactly once, not duplicated between them. `EventWitnessesSection`
    /// does its own equivalent inline (a per-witness-unit shape this flat
    /// helper doesn't fit), setting `GroupCount` the identical way.
    public static async Task<List<PassageListVerse>> ResolveGroupsAsync(AtlasClient api, IReadOnlyList<VerseGroup> groups)
    {
        var countByVref = new Dictionary<string, int>();
        foreach (var g in groups)
        {
            foreach (var v in g.Verses)
            {
                countByVref[v] = g.Count;
            }
        }
        var resolved = await ResolveAsync(api, groups.SelectMany(g => g.Verses).ToList());
        return resolved.Select(v => v with { GroupCount = countByVref.TryGetValue(v.Vref, out var c) ? c : null }).ToList();
    }
}

public static class PassageBlockBuilder
{
    public static List<PassageBlockData> Build(IReadOnlyList<PassageSourceUnit> units)
    {
        var blocks = new List<PassageBlockData>();
        foreach (var unit in units)
        {
            if (unit.Verses.Count == 0)
            {
                continue;
            }

            var vrefs = unit.Verses.Select(v => v.Vref).ToList();
            var unitBlocks = new List<PassageBlockData>();
            foreach (var run in PassageGrouping.Groups(vrefs))
            {
                var members = unit.Verses.Skip(run.Start).Take(run.Length).ToList();
                var span = PassageGrouping.SpanRef(members[0].Vref, members[^1].Vref);
                unitBlocks.Add(new PassageBlockData(span, members, unit.Caption));
            }

            // Batch HOTFIX-4 requirement 7 (honest truncation signal): find,
            // per distinct (book,chapter) GroupCount actually present in
            // THIS unit, the block reaching that group's own HIGHEST
            // verse number -- the server's own cap (`take(20)`, ascending)
            // always keeps the LOWEST-numbered verses, so the missing tail
            // always follows the highest one actually delivered. Every
            // verse within one block is already the SAME book+chapter, by
            // `PassageGrouping.Groups`'s own construction (never crosses a
            // book/chapter boundary), so that block's own delivered count
            // for the group is simply its own `Verses.Count` -- no
            // per-verse re-filtering needed. Units with no GroupCount at
            // all (cross-references, THE SCRIPTURES, place est/dest) never
            // populate this map, so this whole pass is a no-op for them.
            var highestVerseForGroup = new Dictionary<(string Book, int Chapter), (int Verse, int TrueCount)>();
            foreach (var v in unit.Verses)
            {
                if (v.GroupCount is not int trueCount)
                {
                    continue;
                }
                var vid = CanonRef.ParseVerse(v.Vref);
                var key = (vid.Book, vid.Chapter);
                if (!highestVerseForGroup.TryGetValue(key, out var existing) || vid.Verse > existing.Verse)
                {
                    highestVerseForGroup[key] = (vid.Verse, trueCount);
                }
            }

            foreach (var block in unitBlocks)
            {
                var last = CanonRef.ParseVerse(block.LastVref);
                if (highestVerseForGroup.TryGetValue((last.Book, last.Chapter), out var top) && last.Verse == top.Verse)
                {
                    var truncatedBy = top.TrueCount - block.Verses.Count;
                    blocks.Add(truncatedBy > 0 ? block with { TruncatedBy = truncatedBy } : block);
                }
                else
                {
                    blocks.Add(block);
                }
            }
        }

        return blocks;
    }
}
