namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch R requirement 5 (place-in-verse hover -> marker blink): one slice
/// of a scanned verse's text -- either plain prose (<see cref="PlaceId"/>
/// null) or a place mention (<see cref="PlaceId"/>/<see cref="PlaceName"/>
/// set, <see cref="Text"/> is the exact matched substring, byte-for-byte, so
/// the mini-reader never re-derives or re-cases what it displays).
/// </summary>
public readonly record struct MentionSegment(string Text, string? PlaceId, string? PlaceName);

/// <summary>
/// Splits a verse's own rendered text into plain-prose and place-mention
/// segments by a plain, case-insensitive substring search against each
/// candidate place's own curated <see cref="PlaceRefDto.Name"/> -- this
/// app's ONLY mention-detection mechanism (there is no richer per-mention
/// character-offset data anywhere in the pipeline; <c>VerseOut.Places</c>,
/// server: <c>AtlasData.places_for_verse</c>, only ever says WHICH places a
/// verse links to, never WHERE in the text). An approximation, not a claim
/// of perfect recall/precision -- a place named by a pronoun, or under a
/// curated name the verse's own KJV wording doesn't literally use, is
/// simply not found; that is an accepted, disclosed limitation (see the
/// batch report), not a bug this class tries to solve.
/// </summary>
public static class PlaceMentions
{
    public static IReadOnlyList<MentionSegment> Scan(string text, IReadOnlyList<PlaceRefDto> places)
    {
        if (string.IsNullOrEmpty(text) || places.Count == 0)
        {
            return new[] { new MentionSegment(text, null, null) };
        }

        // Every occurrence of every place's own name, found independently
        // (a place's name can legitimately appear more than once in one
        // verse). Order doesn't matter yet -- the acceptance pass below
        // re-sorts and resolves overlaps.
        var candidates = new List<(int Start, int Length, string Id, string Name)>();
        foreach (var place in places)
        {
            if (string.IsNullOrEmpty(place.Name) || place.Name.Length > text.Length)
            {
                continue;
            }
            var searchFrom = 0;
            while (searchFrom <= text.Length - place.Name.Length)
            {
                var idx = text.IndexOf(place.Name, searchFrom, StringComparison.OrdinalIgnoreCase);
                if (idx < 0)
                {
                    break;
                }
                candidates.Add((idx, place.Name.Length, place.Id, place.Name));
                searchFrom = idx + place.Name.Length;
            }
        }

        // Greedy, longest-match-wins acceptance: sorted by start position
        // then by DESCENDING length, so a longer name that genuinely
        // CONTAINS a shorter one as a real substring (e.g. "Beersheba"
        // contains "Sheba") claims the span first; the shorter candidate at
        // (or overlapping) the same position is then rejected by the
        // overlap check below, never wrapped a second time.
        var accepted = new List<(int Start, int Length, string Id, string Name)>();
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
            segments.Add(new MentionSegment(text.Substring(m.Start, m.Length), m.Id, m.Name));
            cursor = m.Start + m.Length;
        }
        if (cursor < text.Length || segments.Count == 0)
        {
            segments.Add(new MentionSegment(text[cursor..], null, null));
        }
        return segments;
    }
}
