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
/// is the ONE place that sets this true.
///
/// FIX ROUND 2 (review Critical N-1 -- a FALSE PREMISE corrected): fix
/// round 1 licensed unconditional first-to-last coalescing on the claim
/// that a witness's multi-VerseGroup shape "is ALWAYS one continuous
/// account... never a coincidence of storage." The real corpus refutes
/// that: 14 of the 35 multi-range curated `[[witness]]` rows
/// (`data/curated/event-witnesses.toml`) are genuinely NON-contiguous
/// (e.g. `rob_peter_denies`'s own `["MRK.14.54", "MRK.14.66-72"]`;
/// `theo-188`'s own `["2KI.1.17", "2KI.8.16-24"]`), and 21 more
/// `events-extra.toml` verse lists flow through the synthesized-witness
/// path with real gaps too. THE COMPLETE RULE (the owner's, both halves):
/// within one account (one witness row), adjacent ranges merge ONLY where
/// genuinely contiguous -- versification-aware adjacency, chapter
/// boundaries included (MAT.5.48 -&gt; MAT.6.1 joins because Matthew 5
/// really has 48 verses, per <see cref="Canon"/>); where a real gap
/// exists, the account stays ONE account (one block, one unit -- never
/// fake separate accounts) but its ref renders as the honest COMPOUND
/// list of its actual ranges (e.g. "MRK.14.54, 66-72") -- never a
/// fabricated envelope span, never invented or dropped verses.
/// <see cref="PassageBlockBuilder.BuildCoalescedBlock"/> implements this.
/// Two SEPARATE witness ROWS for the SAME book (`psa_014`'s own Psalm 14 +
/// Psalm 53 witnesses, "a second, distinct psalm recounting the same
/// substance, not a copy of the same container") are two SEPARATE
/// <see cref="PassageSourceUnit"/>s already (one per <c>EventWitnessDto</c>)
/// and still never merge -- coalescing remains strictly within-unit.
/// TODO(FQ-1): the server's own `compose_frontier` (the frontier-query
/// contract batch) should inherit this coalescing rule at ITS OWN seam
/// (an event witness's own account span belongs on the wire pre-coalesced,
/// not re-derived client-side) -- this client-side fix is the honest,
/// disclosed interim seam per the controller's own fix-round instruction,
/// not a permanent architectural home.
/// </param>
/// <param name="Canon">
/// Fix round 2 (N-1): the chapter-length lookup that decides whether a
/// chapter-boundary crossing inside a coalesced unit is genuinely
/// contiguous (see <see cref="Versification"/>'s own doc comment). Only
/// consulted when <see cref="CoalesceAcrossChapters"/> is true; null (the
/// default, and the degrade when the books TOC fetch fails) means NO
/// chapter boundary is ever treated as contiguous -- the honest compound
/// rendering, never a guessed join.
/// </param>
public sealed record PassageSourceUnit(IReadOnlyList<PassageListVerse> Verses, string? Caption = null, bool CoalesceAcrossChapters = false, Versification? Canon = null);

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
/// <param name="FirstRangeEndVref">
/// Fix round 2 (N-1, secondary symptom): the TRUE last vref of this
/// block's own FIRST contiguous range -- set only by
/// <see cref="PassageBlockBuilder.BuildCoalescedBlock"/> (null for every
/// grouped block, whose first range IS the whole block). What
/// <c>PassageList.razor</c>'s own <c>FocalToOf</c> highlights against, so
/// expanding a gapped account (e.g. "MRK.14.54, 66-72") highlights only
/// the verses the account's own first range actually contains -- never
/// straight through a gap into verses that belong to a DIFFERENT event.
/// </param>
public sealed record PassageBlockData(string Span, IReadOnlyList<PassageListVerse> Verses, string? Caption, int TruncatedBy = 0, string? FirstRangeEndVref = null)
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

    /// Fix round 2 (N-6, review Low): the ONE witness -&gt; verse-list
    /// flattening both render surfaces share (`WitnessUnitsResolver` for
    /// the landed frontier's PARALLEL ACCOUNTS, <see cref="Explore.ArrowNav"/>'s
    /// own `SelectRefsFromWitnesses` for the refs under a prior/following
    /// button) -- previously the identical one-liner written twice, which
    /// left ACCT-SET-MISMATCH-1's "one derivation, two render sites"
    /// guarantee conventional rather than structural (a future filter/sort
    /// applied to one copy and not the other would silently re-open the
    /// exact divergence that ticket exists to close). GroupCount carries
    /// each VerseGroup's own TRUE total (HOTFIX-4's honest truncation
    /// signal), exactly as both call sites always set it.
    public static List<PassageListVerse> FlattenWitness(EventWitnessDto witness) =>
        witness.VerseGroups.SelectMany(g => g.Verses.Select(v => new PassageListVerse(v, "", g.Count))).ToList();

    /// One honest, genuinely-contiguous range of a coalesced account --
    /// possibly crossing chapter boundaries (only where the canon's own
    /// chapter lengths prove the crossing contiguous), never crossing a
    /// real gap. First/Last verse numbers are TRUE extents (cap-corrected),
    /// not merely delivered ones.
    private sealed record AccountRange(string Book, int FirstChapter, int FirstVerse, int LastChapter, int LastVerse)
    {
        public string FirstVref => $"{Book}.{FirstChapter}.{FirstVerse}";
        public string LastVref => $"{Book}.{LastChapter}.{LastVerse}";
    }

    /// A single-chapter contiguous run of a unit's delivered verses, with
    /// its TRUE end (the wire-cap remainder folded in -- see TrueRunsOf).
    private sealed record TrueRun(string Book, int Chapter, int FirstVerse, int LastVerse);

    /// ACCT-COALESCE-1 (fix round 2, review Critical N-1 -- the COMPLETE
    /// rule this time, both halves): builds the ONE block a
    /// `CoalesceAcrossChapters` unit renders as. One unit (one witness row)
    /// is always ONE account -- exactly one block -- but its SPAN is now
    /// derived from the unit's own genuinely-contiguous ranges:
    ///
    ///   1. Delivered verses split into consecutive same-chapter runs
    ///      (TrueRunsOf), each run's end corrected for the server's own
    ///      20-verse-per-chapter cap (HOTFIX-4): the cap keeps the
    ///      LOWEST-numbered verses of a group, so any undelivered
    ///      remainder continues after the group's own LAST delivered verse
    ///      -- the same "identity never narrows" (PERF-3) correction fix
    ///      round 1 made, now applied per-run instead of assuming the
    ///      whole group is one run (the assumption that produced
    ///      `rob_peter_denies`'s own false "MRK.14.54-61", a span claiming
    ///      verses that belong to a DIFFERENT event while dropping the
    ///      delivered 62-72).
    ///   2. Runs merge across a chapter boundary ONLY where the canon
    ///      itself proves contiguity (unit.Canon -- prior run ends at its
    ///      chapter's own true last verse AND the next starts at verse 1
    ///      of the very next chapter): the Sermon on the Mount's own
    ///      MAT.5.48 -&gt; 6.1 -&gt; 7.1 still coalesces to the one honest
    ///      "MAT.5.1-7.29"; `theo-188`'s own 2KI.1.17 + 8.16-24 never
    ///      welds into the fictional seven-chapter "2KI.1.17-8.24" again.
    ///   3. Where a real gap remains, the span renders as the honest
    ///      COMPOUND list of the account's actual ranges ("MRK.14.54,
    ///      66-72") -- never a fabricated envelope, never invented or
    ///      dropped verses. Span and <see cref="PassageBlockData.LastVref"/>
    ///      agree by construction now: the final range always ends at (or
    ///      cap-extends past) the last delivered verse.
    ///
    /// PUBLIC (ACCT-SET-MISMATCH-1): reused directly by
    /// <see cref="Explore.ArrowNav"/> so the refs under a prior/following
    /// button and the landed frontier's own PARALLEL ACCOUNTS are one
    /// derivation, never two. Truncation stays the flat every-distinct-
    /// chapter-group sum (mirrors <c>ArrowNav.ComputeTruncatedBy</c>).
    public static PassageBlockData BuildCoalescedBlock(PassageSourceUnit unit)
    {
        var ranges = StitchRuns(TrueRunsOf(unit.Verses), unit.Canon);
        var span = AccountSpan(ranges);

        // HOTFIX-4 requirement 7: honest truncation, summed across every
        // distinct (book,chapter) group actually present in this unit.
        var byGroup = new Dictionary<(string Book, int Chapter), (int Delivered, int? TrueCount)>();
        foreach (var v in unit.Verses)
        {
            var (book, chapter, _) = CanonRef.ParseVerse(v.Vref);
            var existing = byGroup.GetValueOrDefault((book, chapter), (0, null));
            byGroup[(book, chapter)] = (existing.Delivered + 1, v.GroupCount ?? existing.TrueCount);
        }
        var truncatedBy = 0;
        foreach (var (_, entry) in byGroup)
        {
            if (entry.TrueCount is int trueCount && trueCount > entry.Delivered)
            {
                truncatedBy += trueCount - entry.Delivered;
            }
        }

        return new PassageBlockData(span, unit.Verses, unit.Caption, truncatedBy, ranges[0].LastVref);
    }

    /// Step 1 of BuildCoalescedBlock (see its doc comment): the unit's own
    /// delivered verses as TRUE single-chapter runs, in delivered order.
    /// Adjacency here is by LIST POSITION within each (book,chapter)
    /// segment -- the same discipline PassageGrouping.Groups follows -- so
    /// a gap the server actually delivered around (MRK.14.54 then
    /// MRK.14.66) is a run boundary, never papered over.
    private static List<TrueRun> TrueRunsOf(IReadOnlyList<PassageListVerse> verses)
    {
        var runs = new List<TrueRun>();
        var i = 0;
        while (i < verses.Count)
        {
            // One (book, chapter) segment: consecutive list entries sharing book+chapter.
            var (book, chapter, _) = CanonRef.ParseVerse(verses[i].Vref);
            int? trueCount = null;
            var nums = new List<int>();
            while (i < verses.Count)
            {
                var v = CanonRef.ParseVerse(verses[i].Vref);
                if (v.Book != book || v.Chapter != chapter)
                {
                    break;
                }
                nums.Add(v.Verse);
                trueCount = verses[i].GroupCount ?? trueCount;
                i++;
            }

            var segmentRuns = new List<(int First, int Last)>();
            var (runStart, runEnd) = (nums[0], nums[0]);
            for (var k = 1; k < nums.Count; k++)
            {
                if (nums[k] == runEnd + 1)
                {
                    runEnd = nums[k];
                    continue;
                }
                segmentRuns.Add((runStart, runEnd));
                (runStart, runEnd) = (nums[k], nums[k]);
            }
            segmentRuns.Add((runStart, runEnd));

            // Wire-cap correction (PERF-3, "identity never narrows"): the
            // server's own take(20) keeps the LOWEST-numbered verses, so
            // the undelivered remainder -- if any -- continues after the
            // segment's own LAST delivered verse. (Where the true shape of
            // that undelivered tail has further gaps, the wire genuinely
            // cannot say; continuing the final run is the only honest math
            // available and matches the delivered-portion evidence.)
            var undelivered = Math.Max(0, (trueCount ?? nums.Count) - nums.Count);
            if (undelivered > 0)
            {
                var last = segmentRuns[^1];
                segmentRuns[^1] = (last.First, last.Last + undelivered);
            }

            runs.AddRange(segmentRuns.Select(r => new TrueRun(book, chapter, r.First, r.Last)));
        }
        return runs;
    }

    /// Step 2 of BuildCoalescedBlock: merge adjacent runs into ranges ONLY
    /// where genuinely contiguous. A chapter boundary joins iff the canon
    /// itself says the prior run ends at its chapter's own real last verse
    /// and the next run opens the very next chapter at verse 1; an unknown
    /// chapter length (canon null / book or chapter missing) NEVER joins --
    /// the conservative, honest degrade (Versification's own doc comment).
    private static List<AccountRange> StitchRuns(List<TrueRun> runs, Versification? canon)
    {
        var ranges = new List<AccountRange>();
        AccountRange? current = null;
        foreach (var run in runs)
        {
            if (current is not null && run.Book == current.Book)
            {
                var sameChapterAdjacent = run.Chapter == current.LastChapter && run.FirstVerse == current.LastVerse + 1;
                var boundaryAdjacent = run.Chapter == current.LastChapter + 1 && run.FirstVerse == 1
                    && canon?.VersesIn(run.Book, current.LastChapter) == current.LastVerse;
                if (sameChapterAdjacent || boundaryAdjacent)
                {
                    current = current with { LastChapter = run.Chapter, LastVerse = run.LastVerse };
                    continue;
                }
            }
            if (current is not null)
            {
                ranges.Add(current);
            }
            current = new AccountRange(run.Book, run.Chapter, run.FirstVerse, run.Chapter, run.LastVerse);
        }
        if (current is not null)
        {
            ranges.Add(current);
        }
        return ranges;
    }

    /// Step 3 of BuildCoalescedBlock: the honest span text. One range keeps
    /// the exact shapes PassageGrouping.SpanRef already renders; multiple
    /// ranges render as a comma-joined compound list in Scripture's own
    /// conventional abbreviation (each later range drops whatever context
    /// -- book, then chapter -- it shares with the range before it):
    /// "MRK.14.54, 66-72"; "2KI.1.17, 8.16-24"; "PSA.96.1-13, 105.1-15,
    /// 106.1, 47-48". Never an envelope over a gap.
    private static string AccountSpan(List<AccountRange> ranges)
    {
        var sb = new System.Text.StringBuilder();
        AccountRange? prev = null;
        foreach (var r in ranges)
        {
            if (prev is null)
            {
                sb.Append(PassageGrouping.SpanRef(r.FirstVref, r.LastVref));
            }
            else
            {
                sb.Append(", ");
                if (r.Book != prev.Book)
                {
                    sb.Append(PassageGrouping.SpanRef(r.FirstVref, r.LastVref));
                }
                else if (r.FirstChapter != r.LastChapter)
                {
                    sb.Append($"{r.FirstChapter}.{r.FirstVerse}-{r.LastChapter}.{r.LastVerse}");
                }
                else if (r.FirstChapter == prev.LastChapter)
                {
                    sb.Append(r.FirstVerse == r.LastVerse ? $"{r.FirstVerse}" : $"{r.FirstVerse}-{r.LastVerse}");
                }
                else
                {
                    sb.Append(r.FirstVerse == r.LastVerse ? $"{r.FirstChapter}.{r.FirstVerse}" : $"{r.FirstChapter}.{r.FirstVerse}-{r.LastVerse}");
                }
            }
            prev = r;
        }
        return sb.ToString();
    }
}
