using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public readonly record struct MentionSegment(string Text, string? PlaceId, string? PlaceName, string? PersonId = null, string? PersonName = null);

public static class PlaceMentions
{
    public static IReadOnlyList<MentionSegment> Scan(string text, IReadOnlyList<PlaceRef> places, IReadOnlyList<PersonRef> persons)
    {
        if (string.IsNullOrEmpty(text) || (places.Count == 0 && persons.Count == 0))
        {
            return new[] { new MentionSegment(text, null, null) };
        }

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

        // Ties resolve to Place: places are enqueued before persons above and this sort is stable.
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
            // Advance by 1, not name.Length, so a match starting inside a rejected mid-word span isn't skipped.
            searchFrom = idx + 1;
        }
    }
}
