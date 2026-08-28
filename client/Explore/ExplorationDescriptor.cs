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
public sealed record ExplorationDescriptor(string Kind, string Key, string Title, bool IsGeneralKind = false)
{
    // Batch PERI-1 (PRESENTATION CATEGORY LAW): trailing, default-`false`,
    // ADDITIVE -- client-only (this record is serialized to localStorage
    // ONLY, SavedExplorationsService, never the HTTP wire), so an
    // already-saved descriptor from before this batch simply deserializes
    // with `false`, unchanged rendering, same as every OTHER kind ever
    // reconstructs. Set true only for an "Event"-kind descriptor whose
    // captured node's own EventNode.CachedKind reads "general" (see
    // Capture's own EventNode case below) -- ExplorationListItem.razor
    // reads this to render "Passage" instead of the raw Kind string.

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
        EventNode ev => new ExplorationDescriptor("Event", ev.EventId, ev.Title, IsGeneralKind: ev.CachedKind == "general"),
        // G2-m1: Key now leads with the polity's own stable id (5 fields,
        // was 4) -- Reconstruct below still accepts the pre-G2-m1 4-field
        // form (a descriptor saved before this batch) and falls back to the
        // original Name+From+To re-location for it, so no previously saved
        // exploration breaks.
        PolityDeltaNode pd => new ExplorationDescriptor("PolityDelta", $"{pd.PolityId}|{pd.PolityName}|{pd.DeltaKind}|{pd.FromYear}|{pd.ToYear}", pd.Title),
        PersonNode pn => new ExplorationDescriptor("Person", pn.PersonId, pn.Title),
        // Batch CORP-1: CommentaryItem's own Key is the wire node id
        // ("CommentaryItem:kretzmann/0.1.0") -- Reconstruct needs nothing
        // more (no fetch: the constructor takes id+heading only, see that
        // class's own header for why it deliberately holds no
        // IExplorableClient). ConcordUnit's own Key is its citation
        // ("BoC 7.2.1"); Reconstruct DOES need a fetch there (unlike every
        // fetch-free case above) -- see that case's own comment.
        CommentaryItemNode ci => new ExplorationDescriptor("CommentaryItem", ci.Id, ci.Title),
        ConcordUnitNode cu => new ExplorationDescriptor("ConcordUnit", cu.Title, cu.Title),
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
    ///
    /// Batch CORP-1 widens this signature with a SECOND client
    /// (<paramref name="graph"/>, <see cref="IExplorableClient"/>) --
    /// disclosed, not silent: every case above this batch's own additions
    /// needs only <paramref name="api"/> (the bespoke client), but
    /// reconstructing a saved "ConcordUnit" needs the generic reading-spine
    /// endpoint (<c>Reading(...,corpus:"concord")</c>), which only
    /// <see cref="IExplorableClient"/> exposes -- <c>AtlasClient</c> has no
    /// Concord-reading method of its own, and duplicating one there just for
    /// this ONE reconstruction path would be a second representation of the
    /// same call. <c>MainLayout.razor</c>'s own "Continue" is the one real
    /// call site, updated to inject and pass both clients.
    /// </summary>
    public static async Task<IExplorable> Reconstruct(ExplorationDescriptor descriptor, AtlasClient api, IExplorableClient graph)
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
                // PERI-1 fix round 1 (S-1a/Q-1a): seeds knownKind from the
                // SAVED descriptor's own IsGeneralKind -- without this, a
                // "Continue" reopen re-captures every seeded node fresh
                // (SeedFromTrail.Apply -> Visit.Apply -> Capture, FocusStack.cs),
                // which would silently regress an already-correct saved
                // "Passage" badge back to "Event" the instant the trail is
                // reopened (Capture would again read a not-yet-fetched
                // EventNode). See EventNode.cs's own _knownKind doc comment.
                return new EventNode(descriptor.Key, descriptor.Title, descriptor.IsGeneralKind ? "general" : "event");

            case "PolityDelta":
            {
                // G2-m1: two supported Key shapes. 5 fields (current) leads
                // with the polity's own STABLE id -- re-locate by Id, immune
                // to a curator rename. 4 fields (legacy -- a descriptor
                // saved to localStorage before this batch, when map.js's own
                // click payload never carried the id at all) falls back to
                // the original Name+From+To re-location. Either way, a
                // boundary that can no longer be found degrades to the SAME
                // "minimal popover" state PolityDeltaNode already documents
                // as a first-class, supported shape for "an uneventful
                // boundary" -- never a thrown error for what is, from this
                // node kind's own point of view, an ordinary
                // conditional-absence case.
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

                PolityDeltaDto? delta = null;
                PolityEraOut? era = null;
                try
                {
                    var polities = await api.Polities(fromYear, toYear);
                    era = polityId is not null
                        ? polities.Polities.FirstOrDefault(e => e.Id == polityId)
                        : polities.Polities.FirstOrDefault(e => e.Name == polityName && e.From == fromYear && e.To == toYear);
                    delta = deltaKind == "fall" ? era?.Fall : era?.Transition;
                }
                catch (Exception)
                {
                    // Same graceful-degradation stance as every other
                    // best-effort fetch in this app -- falls through to the
                    // minimal-popover construction below.
                }
                return new PolityDeltaNode(polityId ?? era?.Id ?? "", era?.Name ?? polityName, deltaKind, fromYear, toYear, delta?.Event, delta?.Verses ?? new List<string>(), delta?.RefNote);
            }

            case "Person":
                return new PersonNode(descriptor.Key, descriptor.Title);

            case "CommentaryItem":
                // No fetch -- CommentaryItemNode's own constructor takes
                // id+heading only (see that class's own header for why).
                return new CommentaryItemNode(descriptor.Key, descriptor.Title);

            case "ConcordUnit":
            {
                // One reading-window fetch (n=1, corpus=concord) re-resolves
                // this exact paragraph's own full text -- the same call
                // Pages/Concord.razor itself makes to build its own rows.
                var window = await graph.Reading(descriptor.Key, 1, "onward", corpus: "concord");
                var unit = window.Units.FirstOrDefault(u => u.Ref == descriptor.Key)
                    ?? throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: Concord paragraph '{descriptor.Key}' no longer resolves.");
                return new ConcordUnitNode(unit.Ref, unit.Text);
            }

            default:
                throw new NotSupportedException($"ExplorationDescriptor.Reconstruct: unrecognized descriptor Kind '{descriptor.Kind}'.");
        }
    }
}
