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
/// <param name="WordsOfChrist">
/// Batch RED-1 (owner order 2026-08-25, "Red letters on Jesus' words in
/// every translation"): the <see cref="Places"/>/<see cref="Persons"/>
/// sibling for red-letter spans -- same null-vs-empty rule (null when the
/// producer had no <c>ChapterOut.Verses</c> row to read from; an empty
/// array when it did and the verse simply carries no red-letter span).
/// <c>PassageList.razor</c>/<c>ArrowNav.razor</c> coalesce null to empty at
/// the render site, same as the other two.
/// </param>
public sealed record PassageListVerse(string Vref, string Text, int? GroupCount = null, IReadOnlyList<PlaceRefDto>? Places = null, IReadOnlyList<PersonRefDto>? Persons = null, IReadOnlyList<WordsOfChristSpanDto>? WordsOfChrist = null);

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
/// <param name="CoalesceAcrossChapters">
/// ACCT-COALESCE-1 (owner bug report, verbatim: "in parallel accounts
/// (sermon on the mount in particular), accounts from the same book +
/// chapter are listed. makes no sense."): DIAGNOSIS -- an event WITNESS
/// (one curated `[[witness]]` TOML row = ONE account, per
/// EVENT-ACCOUNTS-1's own "events are keys that map to sets of Biblical
/// accounts") is stored on the wire as one <see cref="VerseGroup"/> PER
/// CHAPTER it spans (`data/curated/event-witnesses.toml`'s own Sermon-on-
/// the-Mount comment: "written as 3 same-chapter ranges, this project's
/// own curator-friendly range syntax not spanning a chapter boundary in
/// one string" -- a STORAGE artifact, not three separate accounts). The
/// pre-existing default (false, every other consumer -- cross-references,
/// THE SCRIPTURES, place est/dest) is CORRECT there: those units
/// deliberately never coalesce across a chapter boundary (two different
/// xref targets that happen to land in adjacent chapters are two
/// different pieces of context). <see cref="Explore.WitnessUnitsResolver"/>
/// is the ONE place that sets this true -- an event witness's own
/// multi-VerseGroup shape is ALWAYS one continuous account, by
/// construction of the curation process itself, never a coincidence of
/// storage. THE RULE (owner-dictated): "an account = a coalesced
/// contiguous span per book/narrative locus... never per-storage-row."
/// Two SEPARATE witness ROWS for the SAME book (a real, curated case --
/// `psa_014`'s own Psalm 14 + Psalm 53 witnesses, "a second, distinct
/// psalm recounting the same substance, not a copy of the same
/// container") are two SEPARATE <see cref="PassageSourceUnit"/>s already
/// (one per <c>EventWitnessDto</c>) and stay separate under this rule
/// WITHOUT any gap-detection logic needed -- the curation-level boundary
/// (one witness row = one account) already IS the correct boundary.
/// TODO(FQ-1): the server's own `compose_frontier` (the frontier-query
/// contract batch) should inherit this coalescing rule at ITS OWN seam
/// (an event witness's own account span belongs on the wire pre-coalesced,
/// not re-derived client-side) -- this client-side fix is the honest,
/// disclosed interim seam per the controller's own fix-round instruction,
/// not a permanent architectural home.
/// </param>
public sealed record PassageSourceUnit(IReadOnlyList<PassageListVerse> Verses, string? Caption = null, bool CoalesceAcrossChapters = false);

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
                result.Add(new PassageListVerse(vref, cv.Text, Places: cv.Places, Persons: cv.Persons, WordsOfChrist: cv.WordsOfChrist));
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

            // ACCT-COALESCE-1: a unit that opts in renders as EXACTLY ONE
            // block spanning its own full first-to-last range, regardless
            // of how many chapters it crosses -- bypasses
            // PassageGrouping.Groups entirely (that method's own
            // same-chapter-only adjacency rule is exactly what this unit
            // needs to NOT apply). See PassageSourceUnit.CoalesceAcrossChapters's
            // own doc comment for the full "why" story.
            if (unit.CoalesceAcrossChapters)
            {
                blocks.Add(BuildCoalescedBlock(unit));
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

    /// ACCT-COALESCE-1: builds the ONE block a `CoalesceAcrossChapters`
    /// unit renders as -- the full first-to-last span over EVERY verse in
    /// the unit, regardless of chapter. PUBLIC (fix round 2,
    /// ACCT-SET-MISMATCH-1): reused directly by
    /// <see cref="Explore.ArrowNav"/>'s own refs-list resolver so the SET
    /// and ORDER of refs shown under a prior/following button are computed
    /// by the IDENTICAL coalescing logic the landed frontier's own
    /// PARALLEL ACCOUNTS list uses -- one function, two render sites,
    /// never two derivations that could silently disagree. Mirrors
    /// <c>ArrowNav.ComputeTruncatedBy</c>'s own "flat, no block grouping of
    /// its own" truncation math (same shape, same reasoning: sum, across
    /// every DISTINCT (book,chapter) group actually present in this one
    /// block, how many more verses that group's own server-side Count says
    /// exist beyond what was delivered) rather than the per-chapter-block
    /// check immediately above (which assumes one block == one chapter --
    /// no longer true here by construction).
    ///
    /// FIX ROUND 1 (real, live-caught bug, PERF-3's own "identity never
    /// narrows" law): the block's own SPAN must reflect the TRUE end of
    /// whichever chapter it lands in, never merely the last verse the
    /// server's own 20-verse-per-chapter cap happened to deliver -- the
    /// Sermon on the Mount's own real MAT witness ends in Matthew 7 (29
    /// real verses, only 20 delivered on the wire), so
    /// <c>unit.Verses[^1]</c> alone would silently narrow the account's
    /// own honest identity to "MAT.5.1-7.20". The LAST distinct
    /// (book,chapter) group's own TRUE last verse number is computed the
    /// SAME way <see cref="Explore.ArrowNav.SelectRefs"/> already computes
    /// an honest span for a single-group case: that chapter's own first
    /// DELIVERED verse number + its own true Count - 1 (the server's cap
    /// always keeps the LOWEST-numbered verses, so the FIRST delivered
    /// verse of any group is always honest; only the LAST can ever be
    /// short). The unit's own overall FIRST verse needs no such
    /// correction for the identical reason.
    public static PassageBlockData BuildCoalescedBlock(PassageSourceUnit unit)
    {
        var byGroup = new Dictionary<(string Book, int Chapter), (int Delivered, int? TrueCount, string FirstVref)>();
        foreach (var v in unit.Verses)
        {
            var (book, chapter, _) = CanonRef.ParseVerse(v.Vref);
            var key = (book, chapter);
            var existing = byGroup.GetValueOrDefault(key, (0, null, v.Vref));
            byGroup[key] = (existing.Delivered + 1, v.GroupCount ?? existing.TrueCount, existing.FirstVref);
        }

        var (lastBook, lastChapter, _) = CanonRef.ParseVerse(unit.Verses[^1].Vref);
        var lastGroup = byGroup[(lastBook, lastChapter)];
        var (_, _, lastGroupFirstVerseNum) = CanonRef.ParseVerse(lastGroup.FirstVref);
        var trueLastCount = Math.Max(lastGroup.TrueCount ?? 0, lastGroup.Delivered);
        var trueLastVref = $"{lastBook}.{lastChapter}.{lastGroupFirstVerseNum + trueLastCount - 1}";
        var span = PassageGrouping.SpanRef(unit.Verses[0].Vref, trueLastVref);

        var truncatedBy = 0;
        foreach (var (_, entry) in byGroup)
        {
            if (entry.TrueCount is int trueCount && trueCount > entry.Delivered)
            {
                truncatedBy += trueCount - entry.Delivered;
            }
        }

        return new PassageBlockData(span, unit.Verses, unit.Caption, truncatedBy);
    }
}
