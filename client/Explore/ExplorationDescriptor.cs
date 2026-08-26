namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch G2 decision 3 -- the serialization seam a saved exploration trail
/// (decision 2) and a tray selection (decision 6) both ride: a minimal,
/// JSON-friendly snapshot of one <see cref="IExplorable"/> node, `{ Kind,
/// Key, Title }`. <see cref="Kind"/> is exactly the string <see cref="IExplorable.Kind"/>
/// already returns for that node (the same literal every
/// <c>PopoverSectionRegistry</c> provider's own <c>AppliesTo</c> already
/// switches on); <see cref="Key"/> is the shortest string
/// <see cref="Reconstruct"/> needs to rebuild an equivalent node -- for most
/// kinds, exactly the same string the node's own constructor already takes
/// (a vref, a place id, ...); <see cref="Title"/> is a cached copy of
/// <see cref="IExplorable.Title"/>, kept purely so a saved trail can be
/// LISTED (the hamburger panel's own trail rows) without reconstructing --
/// let alone re-fetching -- a single node just to name it.
///
/// ONE factory, <see cref="Reconstruct"/>, with an exhaustive switch over
/// every concrete <see cref="IExplorable"/> this app defines (12 classes
/// across Explore/*.cs, grep-confirmed: Verse/Passage/Chapter/Book/Author/
/// Place/TimeAndPlace/Year/Catechism/Event/PolityDelta/Person). There is no
/// sealed/closed hierarchy over <see cref="IExplorable"/> for the compiler
/// to enforce exhaustiveness on (it's a plain interface) -- the `switch`
/// below instead throws <see cref="NotSupportedException"/> on a `Kind` it
/// doesn't recognize, a runtime-visible gap rather than a silent drop; the
/// batch report's own descriptor table names every kind actually handled
/// (all twelve, today -- no genuinely non-reconstructible kind was found).
///
/// Most kinds need NO fetch at all to reconstruct: their constructors
/// already take just an id/ref plus a label, and <see cref="Title"/> (cached
/// right here) supplies that label directly -- <c>Verse</c>/<c>Chapter</c>/
/// <c>Book</c>/<c>Author</c>/<c>Catechism</c>/<c>Event</c>/<c>Place</c>/
/// <c>Person</c> all reconstruct synchronously this way (wrapped in
/// <see cref="Task"/> only so the ONE method signature stays uniform for the
/// four kinds that genuinely do need one). Four kinds need a real,
/// documented fetch because their live construction sites hand them data
/// gathered elsewhere (a fetched chapter's own verse text, a place's own
/// curated event/date-claim records) rather than storing it on the node
/// itself: <c>Passage</c> (re-fetches its one chapter, re-slices the same
/// verse range <c>Reader.razor.OpenPassage</c> already slices),
/// <c>TimeAndPlace</c> (re-fetches the place, re-finds the matching
/// <see cref="SceneEvent"/> by id), <c>Year</c> (re-fetches the place,
/// re-reads whichever of <c>History.Established</c>/<c>.Destroyed</c>
/// matches the saved label), and <c>PolityDelta</c> (re-fetches
/// `/api/polities` for the saved window, re-locates the matching era by
/// name+from+to -- see that case's own comment for the graceful-degrade
/// path when a boundary can no longer be found).
/// </summary>
public sealed record ExplorationDescriptor(string Kind, string Key, string Title)
{
    /// <summary>
    /// Captures a descriptor for any <see cref="IExplorable"/> node this app
    /// can push -- the inverse of <see cref="Reconstruct"/>, always
    /// synchronous (every field a descriptor needs is already sitting on the
    /// live node instance; no node's own <c>Title</c>/id fields require a
    /// fetch to read).
    /// </summary>
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
        EventNode ev => new ExplorationDescriptor("Event", ev.EventId, ev.Title),
        PolityDeltaNode pd => new ExplorationDescriptor("PolityDelta", $"{pd.PolityName}|{pd.DeltaKind}|{pd.FromYear}|{pd.ToYear}", pd.Title),
        PersonNode pn => new ExplorationDescriptor("Person", pn.PersonId, pn.Title),
        _ => throw new NotSupportedException($"ExplorationDescriptor.Capture: unrecognized IExplorable kind '{node.Kind}' ({node.GetType().Name})."),
    };

    /// <summary>
    /// Rebuilds an equivalent <see cref="IExplorable"/> from a saved
    /// descriptor -- see this record's own header comment for which kinds
    /// need a fetch and why. A descriptor whose underlying curated data has
    /// since disappeared (a deleted place, a boundary that no longer parses
    /// the same way) throws rather than silently returning a wrong or
    /// half-built node -- callers (the hamburger panel's own "continue")
    /// wrap this in a try/catch and surface the app's existing toast/error
    /// affordance, the same graceful-degradation stance every other
    /// best-effort fetch in this app already takes.
    /// </summary>
    public static async Task<IExplorable> Reconstruct(ExplorationDescriptor descriptor, AtlasClient api)
    {
        switch (descriptor.Kind)
        {
            case "Verse":
                return new VerseNode(descriptor.Key);

            case "Passage":
            {
                // Mirrors Reader.razor.OpenPassage's own text-slicing exactly
                // (see that method's own comment): a passage sref is always
                // single-chapter ("BOOK.N.From-To" or "BOOK.N.V"), so
                // CanonRef.TargetSpan already parses it; re-fetching the one
                // chapter and re-joining its member verses' own text costs
                // one round trip, never a second endpoint this app doesn't have.
                var span = CanonRef.TargetSpan(descriptor.Key)
                    ?? throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: unparseable Passage key '{descriptor.Key}'.");
                var chapter = await api.Chapter(span.Book, span.Chapter);
                var text = string.Join(" ", chapter.Verses
                    .Where(v => v.Verse >= span.FromVerse && v.Verse <= span.ToVerse)
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
                return new EventNode(descriptor.Key, descriptor.Title);

            case "PolityDelta":
            {
                var parts = descriptor.Key.Split('|');
                var polityName = parts[0];
                var deltaKind = parts[1];
                var fromYear = int.Parse(parts[2]);
                var toYear = int.Parse(parts[3]);
                // Graceful degrade, not a throw: map.js's own click payload
                // (World.razor.OnPolityDeltaClick) never carried the
                // polity's own STABLE id to .NET, only this era's display
                // Name -- re-locating by Name+From+To is the best available
                // key without widening that JS interop call. If a curator
                // has since renamed/reshaped this exact boundary, this
                // degrades to the SAME "minimal popover" state
                // PolityDeltaNode already documents as a first-class,
                // supported shape for "an uneventful boundary" -- never a
                // thrown error for what is, from this node kind's own point
                // of view, an ordinary conditional-absence case.
                PolityDeltaDto? delta = null;
                try
                {
                    var polities = await api.Polities(fromYear, toYear);
                    var era = polities.Polities.FirstOrDefault(e => e.Name == polityName && e.From == fromYear && e.To == toYear);
                    delta = deltaKind == "fall" ? era?.Fall : era?.Transition;
                }
                catch (Exception)
                {
                    // Same graceful-degradation stance as every other
                    // best-effort fetch in this app -- falls through to the
                    // minimal-popover construction below.
                }
                return new PolityDeltaNode(polityName, deltaKind, fromYear, toYear, delta?.Event, delta?.Verses ?? new List<string>(), delta?.RefNote);
            }

            case "Person":
                return new PersonNode(descriptor.Key, descriptor.Title);

            default:
                throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: unrecognized descriptor Kind '{descriptor.Kind}'.");
        }
    }
}
