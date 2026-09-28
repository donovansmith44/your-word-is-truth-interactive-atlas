using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record ExplorationDescriptor(string Kind, string Key, string Title, bool IsGeneralKind = false)
{
    public static ExplorationDescriptor Capture(IExplorable node) => node switch
    {
        VerseNode v => new ExplorationDescriptor("Verse", v.Title, v.Title),
        PassageNode p => new ExplorationDescriptor("Passage", p.Title, p.Title),
        ChapterNode c => new ExplorationDescriptor("Chapter", c.Title, c.Title),
        BookNode b => new ExplorationDescriptor("Book", b.Title, b.Title),
        AuthorNode a => new ExplorationDescriptor("Author", a.Title, a.Title),
        PlaceNode pl => new ExplorationDescriptor("Place", pl.PlaceId, pl.Title),
        TimeAndPlaceNode tp => new ExplorationDescriptor("TimeAndPlace", $"{tp.PlaceId}|{tp.EventId}", tp.Title),
        YearNode y => new ExplorationDescriptor("Year", $"{y.PlaceId}|{y.Label}", y.Title),
        CatechismNode ct => new ExplorationDescriptor("Catechism", ct.Id, ct.Title),
        EventNode ev => new ExplorationDescriptor("Event", ev.EventId, ev.Title, IsGeneralKind: ev.CachedKind == EventKind.General),
        // Key leads with the polity's own stable id so Reconstruct can re-locate
        // it even if a curator later renames it; Reconstruct still accepts the
        // older 4-field (Name+From+To only) form for descriptors saved earlier.
        PolityDeltaNode pd => new ExplorationDescriptor("PolityDelta", $"{pd.PolityId}|{pd.PolityName}|{pd.DeltaKind}|{pd.FromYear}|{pd.ToYear}", pd.Title),
        PersonNode pn => new ExplorationDescriptor("Person", pn.PersonId, pn.Title),
        CommentaryItemNode ci => new ExplorationDescriptor("CommentaryItem", ci.Id, ci.Title),
        ConcordUnitNode cu => new ExplorationDescriptor("ConcordUnit", cu.Title, cu.Title),
        _ => throw new NotSupportedException($"ExplorationDescriptor.Capture: unrecognized IExplorable kind '{node.Kind}' ({node.GetType().Name})."),
    };

    public static async Task<IExplorable> Reconstruct(ExplorationDescriptor descriptor, AtlasClient api, IExplorableClient graph)
    {
        switch (descriptor.Kind)
        {
            case "Verse":
                return new VerseNode(descriptor.Key);

            case "Passage":
            {
                var span = CanonRef.TargetSpan(descriptor.Key)
                    ?? throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: unparseable Passage key '{descriptor.Key}'.");
                var chapter = await api.Chapter(span.Book, span.Chapter);
                var text = string.Join(" ", chapter.Verses
                    .Where(v => v.Number >= span.FromVerse && v.Number <= span.ToVerse)
                    .Select(v => v.Text));
                return new PassageNode(descriptor.Key, text);
            }

            case "Chapter":
            {
                var parts = descriptor.Key.Split('.');
                return new ChapterNode(parts[0], int.Parse(parts[1]));
            }

            case "Book":
                return new BookNode(descriptor.Key);

            case "Author":
                return new AuthorNode(descriptor.Key);

            case "Place":
                return new PlaceNode(descriptor.Key, descriptor.Title);

            case "TimeAndPlace":
            {
                var parts = descriptor.Key.Split('|', 2);
                var placeId = parts[0];
                var eventId = parts[1];
                var detail = await api.PlaceHistory(placeId, null, null);
                var ev = detail.Events.FirstOrDefault(e => e.Id == eventId)
                    ?? throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: event '{eventId}' is no longer recorded at place '{placeId}'.");
                var placeName = detail.History?.DisplayName ?? detail.Name;
                return new TimeAndPlaceNode(placeId, placeName, ev.Id, ev.When, ev.Label, ev.VerseGroups);
            }

            case "Year":
            {
                var parts = descriptor.Key.Split('|', 2);
                var placeId = parts[0];
                var label = parts[1];
                var detail = await api.PlaceHistory(placeId, null, null);
                var claim = label == "Established" ? detail.History?.Established : detail.History?.Destroyed;
                if (claim is null)
                {
                    throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: place '{placeId}' no longer has a curated '{label}' date.");
                }
                return new YearNode(placeId, label, claim.When, claim.Verses, claim.Note);
            }

            case "Catechism":
                return new CatechismNode(descriptor.Key, descriptor.Title);

            case "Event":
                // Seeds knownKind from the saved IsGeneralKind flag: without this, a
                // "Continue" reopen re-captures the node fresh before its detail fetch
                // resolves, which would regress an already-correct saved "Passage"
                // badge back to "Event".
                return new EventNode(descriptor.Key, descriptor.Title, descriptor.IsGeneralKind ? EventKind.General : EventKind.Event);

            case "PolityDelta":
            {
                var parts = descriptor.Key.Split('|');
                string? polityId;
                string polityName;
                string deltaKind;
                int fromYear;
                int toYear;
                if (parts.Length >= 5)
                {
                    polityId = parts[0];
                    polityName = parts[1];
                    deltaKind = parts[2];
                    fromYear = int.Parse(parts[3]);
                    toYear = int.Parse(parts[4]);
                }
                else
                {
                    polityId = null;
                    polityName = parts[0];
                    deltaKind = parts[1];
                    fromYear = int.Parse(parts[2]);
                    toYear = int.Parse(parts[3]);
                }

                PolityDelta? delta = null;
                Polity? era = null;
                try
                {
                    var polities = await api.Polities(fromYear, toYear);
                    era = polityId is not null
                        ? polities.All.FirstOrDefault(e => e.Id == polityId)
                        : polities.All.FirstOrDefault(e => e.Name == polityName && e.From == fromYear && e.To == toYear);
                    delta = deltaKind == "fall" ? era?.Fall : era?.Transition;
                }
                catch (Exception)
                {
                }
                return new PolityDeltaNode(polityId ?? era?.Id ?? "", era?.Name ?? polityName, deltaKind, fromYear, toYear, delta?.Event, delta?.Verses ?? new List<string>(), delta?.RefNote);
            }

            case "Person":
                return new PersonNode(descriptor.Key, descriptor.Title);

            case "CommentaryItem":
                return new CommentaryItemNode(descriptor.Key, descriptor.Title);

            case "ConcordUnit":
            {
                var window = await graph.Reading(descriptor.Key, 1, WindowDir.Onward, corpus: Corpus.Concord);
                var unit = window.Units.FirstOrDefault(u => u.Ref == descriptor.Key)
                    ?? throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: Concord paragraph '{descriptor.Key}' no longer resolves.");
                return new ConcordUnitNode(unit.Ref, unit.Text);
            }

            default:
                throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: unrecognized descriptor Kind '{descriptor.Kind}'.");
        }
    }
}
