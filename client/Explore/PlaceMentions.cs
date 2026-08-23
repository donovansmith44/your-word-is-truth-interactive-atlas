namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch R requirement 5 (place-in-verse hover -> marker blink), widened
/// M-D3/U5 ("in-text mentions-attested links") to a SECOND entity kind: one
/// slice of a scanned verse's text -- either plain prose (<see cref="PlaceId"/>
/// AND <see cref="PersonId"/> both null) or an attested mention of exactly
/// ONE kind (<see cref="PlaceId"/>/<see cref="PlaceName"/> set XOR
/// <see cref="PersonId"/>/<see cref="PersonName"/> set -- mirrors the
/// server's own <c>PlaceOrPerson</c> mentions-edge variant, graph-types::
/// edge). <see cref="Text"/> is the exact matched substring, byte-for-byte,
/// so a caller never re-derives or re-cases what it displays.
/// </summary>
public readonly record struct MentionSegment(string Text, string? PlaceId, string? PlaceName, string? PersonId = null, string? PersonName = null);

/// <summary>
/// Splits a verse's own rendered text into plain-prose and entity-mention
/// segments by a plain, CASE-SENSITIVE substring search against each
/// candidate place's/person's own curated name (<see cref="PlaceRefDto.Name"/>/
/// <see cref="PersonRefDto.Name"/>) -- this app's ONLY mention-detection
/// mechanism (there is no richer per-mention character-offset data anywhere
/// in the pipeline; <c>VerseOut.Places</c>/<c>VerseOut.Persons</c>, server:
/// <c>persons_by_verse</c>/legacy <c>places_for_verse</c>, only ever say
/// WHICH entities a verse is attested to mention, never WHERE in the text).
/// An approximation, not a claim of perfect recall/precision -- an entity
/// named by a pronoun, or under a curated name the verse's own KJV wording
/// doesn't literally use (e.g. GEN.28.19's real text reads "Beth&#8211;el",
/// an en dash, against the curated name "Bethel", no dash -- a real, known
/// miss), is simply not found; that is an accepted, disclosed limitation
/// (see the batch report), not a bug this class tries to solve.
///
/// M-D3/U5 CORRECTED the comparison itself from case-INSENSITIVE to
/// case-SENSITIVE (<see cref="StringComparison.Ordinal"/>, was
/// <see cref="StringComparison.OrdinalIgnoreCase"/>) -- the prior
/// insensitive search made the place "Sin" (a real wilderness/city name,
/// EXO.16.1/EZE.30.15) indistinguishable from the common English word "sin",
/// which appears constantly and is never an attested place mention. Every
/// candidate name entering this scan is already curated/attested (from
/// <c>VerseOut.Places</c>/<c>Persons</c>, themselves graph-mentions-backed --
/// never a raw dictionary lookup), so exact-case matching is strictly a
///
/// A REAL, empirically-confirmed same-verse ambiguity worth disclosing
/// (not hypothetical): GEN.28.1's own attested data carries BOTH a Place
/// "Canaan" (the land) AND a Person "Canaan" (Ham's son, Genesis 9-10) for
/// the SAME verse, and its own text ("...a wife of the daughters of
/// Canaan") contains exactly one literal "Canaan" span -- identical start
/// position, identical length, from both pools. The acceptance pass below
/// is a STABLE sort (LINQ OrderBy/ThenBy), and place candidates are always
/// enqueued before person candidates (see Scan's own two foreach loops in
/// order), so a true tie always resolves to the PLACE, deterministically --
/// which happens to be linguistically correct here ("a wife of the
/// daughters of [the land] Canaan", not of the man), but that is a
/// fortunate coincidence of this one example, not a claim this class
/// performs any real sense-disambiguation. A future verse where a tied
/// name's PERSON sense is the one the prose actually means would resolve
/// wrong, silently, the same disclosed-approximation way the en-dash/
/// pronoun misses above already are.
/// PRECISION fix, not a new source of false negatives beyond the
/// already-disclosed literal-substring limitation above: real KJV proper
/// nouns are consistently capitalized, and so are curated names.
/// </summary>
public static class PlaceMentions
{
    public static IReadOnlyList<MentionSegment> Scan(string text, IReadOnlyList<PlaceRefDto> places, IReadOnlyList<PersonRefDto> persons)
    {
        if (string.IsNullOrEmpty(text) || (places.Count == 0 && persons.Count == 0))
        {
            return new[] { new MentionSegment(text, null, null) };
        }

        // Every occurrence of every place's/person's own name, found
        // independently (a name can legitimately appear more than once in
        // one verse, and a place name and a person name never collide --
        // curated data, not user text). PersonName/PlaceId/PersonId carry
        // which pool a candidate came from; order doesn't matter yet -- the
        // acceptance pass below re-sorts and resolves overlaps across BOTH
        // pools together, exactly as if they were one.
        var candidates = new List<(int Start, int Length, string? PlaceId, string? PlaceName, string? PersonId, string? PersonName)>();
        foreach (var place in places)
        {
            foreach (var idx in FindAll(text, place.Name))
            {
                candidates.Add((idx, place.Name.Length, place.Id, place.Name, null, null));
            }
        }
        foreach (var person in persons)
        {
            foreach (var idx in FindAll(text, person.Name))
            {
                candidates.Add((idx, person.Name.Length, null, null, person.Id, person.Name));
            }
        }

        // Greedy, longest-match-wins acceptance: sorted by start position
        // then by DESCENDING length, so a longer name that genuinely
        // CONTAINS a shorter one as a real substring (e.g. "Beersheba"
        // contains "Sheba") claims the span first; the shorter candidate at
        // (or overlapping) the same position is then rejected by the
        // overlap check below, never wrapped a second time. This holds
        // ACROSS kinds too -- a place name containing a person name (or vice
        // versa) resolves the identical way, one combined arbitration, not
        // two independent scans stitched together afterward.
        var accepted = new List<(int Start, int Length, string? PlaceId, string? PlaceName, string? PersonId, string? PersonName)>();
        foreach (var c in candidates.OrderBy(c => c.Start).ThenByDescending(c => c.Length))
        {
            var overlaps = accepted.Any(a => c.Start < a.Start + a.Length && a.Start < c.Start + c.Length);
            if (!overlaps)
            {
                accepted.Add(c);
            }
        }
        accepted.Sort((a, b) => a.Start.CompareTo(b.Start));

        var segments = new List<MentionSegment>();
        var cursor = 0;
        foreach (var m in accepted)
        {
            if (m.Start > cursor)
            {
                segments.Add(new MentionSegment(text[cursor..m.Start], null, null));
            }
            segments.Add(new MentionSegment(text.Substring(m.Start, m.Length), m.PlaceId, m.PlaceName, m.PersonId, m.PersonName));
            cursor = m.Start + m.Length;
        }
        if (cursor < text.Length || segments.Count == 0)
        {
            segments.Add(new MentionSegment(text[cursor..], null, null));
        }
        return segments;
    }

    /// Every start index where `name` occurs in `text` as a whole word,
    /// case-sensitive, left to right, overlaps allowed (the caller's own
    /// acceptance pass resolves those) -- the one shared search primitive
    /// both candidate loops above use, so the Place and Person pools are
    /// found by the EXACT same rule, never two subtly different
    /// implementations.
    ///
    /// A REAL, empirically-caught bug (not hypothetical, found by
    /// MENTION-3's own Playwright test against real data, popover-sections.
    /// spec.ts): a bare substring search, even case-sensitive, finds "Sin"
    /// not only as EXO.16.1's own genuine standalone mention but ALSO as
    /// the first three letters of the SAME verse's own "Sinai" a few words
    /// later -- a real place, wrongly split out of the middle of an
    /// unrelated word, TWO separate "Sin" mention spans where there is
    /// truly only one. The word-boundary check below (the character
    /// immediately before/after a candidate match, if any, must not itself
    /// be a letter) rejects that second, spurious match while leaving every
    /// other candidate in this file untouched -- every real occurrence this
    /// class already found (Canaan, Aaron, ...) sits between spaces/
    /// punctuation, never mid-word.
    private static IEnumerable<int> FindAll(string text, string name)
    {
        if (string.IsNullOrEmpty(name) || name.Length > text.Length)
        {
            yield break;
        }
        var searchFrom = 0;
        while (searchFrom <= text.Length - name.Length)
        {
            var idx = text.IndexOf(name, searchFrom, StringComparison.Ordinal);
            if (idx < 0)
            {
                yield break;
            }
            var leftBoundaryOk = idx == 0 || !char.IsLetter(text[idx - 1]);
            var rightBoundaryOk = idx + name.Length == text.Length || !char.IsLetter(text[idx + name.Length]);
            if (leftBoundaryOk && rightBoundaryOk)
            {
                yield return idx;
            }
            // Advance by ONE, not name.Length -- even for a REJECTED
            // (mid-word) match, so a genuine later occurrence that happens
            // to start within the rejected span's own characters is never
            // silently skipped. Verse text is at most a few hundred
            // characters; the extra re-scanning is immaterial.
            searchFrom = idx + 1;
        }
    }
}
