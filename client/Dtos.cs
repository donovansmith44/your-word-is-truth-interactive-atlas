using System.Text.Json;

namespace BibleAtlas.Client;

/// <summary>
/// Wire DTOs mirroring the JSON shapes atlas-server serves (see
/// server/atlas-core/src/wire.rs, server/atlas-core/src/data.rs and
/// server/atlas-server/src/handlers.rs). Deserialize/serialize these with
/// <see cref="Wire.Options"/> (snake_case property names) -- never per-call
/// options. Every record name and shape here is pinned by the Task 7 brief;
/// later client tasks depend on these exact names.
/// </summary>
public sealed record Scene(
    string Mode,
    TimeRangeDto? Window,
    string? Ref,
    List<ScenePlace> Places,
    // Batch E2 (the ever-present graph): every event-bearing place NOT in
    // Places for this window -- always present (never null), always empty
    // for a scripture-mode scene (server: wire.rs's own Scene::quiet_places
    // doc comment has the full "always an array" rationale).
    // NOTE: nullable here as a workaround for platform-specific macOS
    // serialization where this field is omitted from the wire JSON.
    List<QuietPlace>? QuietPlaces = null,
    List<SceneArrow>? Arrows = null,
    List<SceneNarrative>? Narratives = null)

public sealed record TimeRangeDto(int FromYear, int ToYear);

public sealed record ScenePlace(
    string Id,
    string Name,
    // Batch E: the period-true name resolved for the scene's own window
    // (server: atlas_core::history::resolve_display_name). Always present
    // -- equal to Name whenever this place has no curated history or none
    // of its curated name ranges intersects the window.
    string DisplayName,
    double Lat,
    double Lon,
    int Brightness,
    List<SceneEvent> Events,
    // Batch H (existence gating, deferred from E2): curated established/
    // destroyed YEARS (server: atlas_core::history::resolve_existence) --
    // not the richer PlaceDateClaim shape (that's PlaceHistoryOut's own
    // Established/Destroyed, with verses/note; this is deliberately lean,
    // matching QuietPlace's own "no events on the wire unless load-bearing"
    // philosophy). Both null when this place has no curated history, or a
    // curated history with neither claim -- "always labels" per the brief.
    // Trailing + optional so every existing positional-record construction
    // site (e.g. World.razor's QuietAsScenePlace) keeps compiling unchanged.
    int? ExistenceFrom = null,
    int? ExistenceTo = null);

/// Batch E2: one "quiet" place -- an event-bearing place not lit (not in
/// <see cref="Scene.Places"/>) for this window. Deliberately lean (no
/// events/verse groups at all -- server: wire.rs's own QuietPlace doc
/// comment). <see cref="DisplayName"/> uses the SAME window-resolution
/// rules as <see cref="ScenePlace.DisplayName"/>; <see cref="TotalEvents"/>
/// is the place's ALL-TIME count, not scoped to this window.
public sealed record QuietPlace(
    string Id,
    string DisplayName,
    double Lat,
    double Lon,
    int TotalEvents,
    // Batch H: same fields, same rule, as ScenePlace.ExistenceFrom/-To above
    // -- a quiet place is exactly where this matters most (the ever-present
    // graph shows a dot for every event-bearing place regardless of window).
    int? ExistenceFrom = null,
    int? ExistenceTo = null);

public sealed record SceneEvent(
    string Id,
    string Label,
    TimeRangeDto When,
    List<VerseGroup> VerseGroups);

public sealed record VerseGroup(string Book, int Chapter, List<string> Verses, int Count);

public sealed record SceneArrow(
    string Narrative,
    string Color,
    string FromPlace,
    string ToPlace,
    string FromEvent,
    string ToEvent,
    int Order);

public sealed record SceneNarrative(string Id, string Name, string Color, int LegsInScene);

public sealed record BookTocEntry(string Code, string Name, List<int> Chapters);

public sealed record EraDto(string Id, string Name, int FromYear, int ToYear);

public sealed record ChapterOut(string Ref, string Book, int Chapter, List<VerseOut> Verses);

/// Batch R requirement 5 (place-in-verse hover -> marker blink):
/// <see cref="Places"/> is the reverse-index list <c>GET /api/chapter/{cref}</c>
/// now carries per verse -- every curated place whose own <c>verse_links</c>
/// names this verse. Always present (possibly empty); the mini-reader
/// (Explore/VerseTextSection) is what decides, per verse, whether to wrap any
/// substring as a hoverable mention -- see that file's own comment for why a
/// plain-text name search against <see cref="PlaceRefDto.Name"/> is this
/// app's only mention-detection mechanism (there is no richer per-mention
/// character-offset data anywhere in this pipeline).
public sealed record VerseOut(int Verse, string Text, List<PlaceRefDto> Places);

public sealed record PlaceRefDto(string Id, string Name);

/// <summary>
/// <c>GET /api/verse/{vref}</c>. Its wire <c>events</c> array is
/// SceneEvent-shaped plus an extra <c>places</c> field per event (so the
/// server side can reuse to_scene_event); System.Text.Json ignores unmapped
/// JSON members by default, so <c>places</c> is silently dropped here --
/// this reuses <see cref="SceneEvent"/> rather than a separate type,
/// exactly as the brief's record list specifies.
/// </summary>
public sealed record VerseDetail(
    string Ref,
    string Text,
    BookMetaDto BookMeta,
    List<SceneEvent> Events,
    List<CrossRefOut> CrossRefs,
    // Batch F ("the small catechism"): catechism items citing this verse --
    // shares this ALREADY-fetched verse-detail response (server:
    // handlers::verse's own doc comment) rather than a second round trip,
    // "one fetch, not four." Always present, possibly empty.
    List<CatechismRefDto> Catechism);

public sealed record BookMetaDto(string Author, string? WritePlace, int? WriteFrom, int? WriteTo);

public sealed record CrossRefOut(string Target, int Votes, string Preview);

/// Batch F: one catechism item citing a verse/span -- id + display name
/// only (no preview text, unlike <see cref="CrossRefOut"/>: requirement 4's
/// own UI lists citing items as plain named entries). Shared shape for
/// <see cref="VerseDetail.Catechism"/> and <c>GET /api/catechism/{sref}</c>'s
/// own array response.
public sealed record CatechismRefDto(string Id, string Name);

/// Batch F: one resolved proof verse -- <see cref="AtlasClient.CatechismItem"/>'s
/// own <see cref="CatechismItemDetail.Verses"/> entries, each carrying its
/// OWN FULL KJV text (design-direction.md's house rendering, not a
/// truncated preview). Property named <see cref="Vref"/> (one leading
/// capital, not "VRef") deliberately -- mirrors <c>ChapterOut.Ref</c>'s own
/// naming exactly, since <c>Wire.Options</c>' snake_case policy maps a
/// two-capital acronym-like run ("VRef") to "v_ref", not the server's own
/// literal `vref` JSON key (a real bug caught live: the proof-verse button
/// silently rendered with an EMPTY testid/ref before this fix, since a
/// missing JSON property just deserializes to the type's default, `null`
/// for a string, and interpolating `null` produces no text at all rather
/// than throwing).
public sealed record CatechismProofVerseDto(string Vref, string Text);

/// <c>GET /api/catechism/item/{id}</c>. <see cref="Text"/> is null for an
/// item with no separate prompt distinct from its own explanation (every
/// Baptism/Confession/Sacrament-of-the-Altar item -- server:
/// <c>CatechismItem.text</c>'s own doc comment); <see cref="WhereWritten"/>
/// is null when Luther's text has no distinct "Where is this written?"
/// proof-citation for this item.
public sealed record CatechismItemDetail(
    string Id,
    string Name,
    string PartTitle,
    string? Text,
    string ExplanationHeading,
    string Explanation,
    string? WhereWritten,
    List<CatechismProofVerseDto> Verses);

public sealed record PlaceDetail(
    string Id,
    string Name,
    double Lat,
    double Lon,
    List<SceneEvent> Events,
    // Batch E: present only when this place has a curated history record
    // (`GET /api/place/{id}?from=&to=`'s optional `history`).
    PlaceHistoryOut? History);

/// Batch E: `PlaceDetail.History`'s shape. <see cref="Blurb"/> is null
/// whenever the request carried no window, or the window matched none of
/// the place's curated blurb ranges (PlaceCard renders NO blurb section in
/// either case -- conditional presence, never an empty placeholder).
public sealed record PlaceHistoryOut(string DisplayName, string? Blurb, DateClaimOut? Established, DateClaimOut? Destroyed);

/// One established/destroyed date claim. <see cref="When"/> reuses
/// <see cref="TimeRangeDto"/>'s own shape -- <see cref="YearText.FormatRange"/>
/// already collapses equal endpoints to a single-year display, so a plain
/// year and a genuine range need no separate wire flag.
public sealed record DateClaimOut(TimeRangeDto When, List<string> Verses, string? Note);

public sealed record NarrativeOut(string Id, string Name, string Color, List<string> Legs);

/// <c>GET /api/polities?from=&amp;to=</c> array element (Batch B2, "borders
/// v2, the cartographer's edition" -- supersedes <c>BordersOut</c>'s own
/// snapshot-year GeoJSON shape entirely). One polity ERA row: <see cref="Id"/>
/// is the polity's own stable id (constant across every era it contributes
/// to a response -- <see cref="ColorKey"/> is hashed from THIS, never the
/// era name, so a polity keeps one plate hue across a rename); <see cref="Name"/>/
/// <see cref="From"/>/<see cref="To"/>/<see cref="Rings"/> are this specific
/// era's own fields. <see cref="Rings"/> stays a raw <see cref="JsonElement"/>
/// (a `[[[lat,lon],...],...]` array of rings) rather than a typed geometry
/// model -- the client never inspects ring coordinates, only forwards them
/// to map.js (mirrors the retired <c>BordersOut.Geojson</c>'s own
/// <see cref="JsonElement"/> pass-through pattern).
public sealed record PolityEraOut(string Id, string Name, int From, int To, JsonElement Rings, int ColorKey);

/// <c>GET /api/polities?from=&amp;to=</c>'s own top-level wire shape --
/// every era (of every polity) whose own <c>[from,to]</c> intersects the
/// requested window, deterministically ordered by id then <c>from</c> (see
/// the server's own <c>handlers::polities</c> doc comment) -- the exact
/// order <see cref="MapInterop.SetPolities"/>/map.js's <c>BorderLayer</c>
/// need to paint an older era under a newer one without re-sorting.
public sealed record PolitiesOut(List<PolityEraOut> Polities);

/// <c>GET /api/landmarks</c> array element. <see cref="Size"/> is the
/// optional Batch C2 far-field size hint ("sm"/"md"/"lg", null for every
/// landmark curated before that batch) -- see atlas-core's
/// <c>Landmark::size</c> doc comment. Forwarded to map.js verbatim by
/// <see cref="MapInterop.SetLandmarks"/>; the client itself never inspects it.
public sealed record LandmarkDto(string Name, string Kind, double Lat, double Lon, string? Size);

/// <c>GET /api/land-mask</c> (Batch R requirement 1, "borders become part of
/// the plate"): the curated land/coastline mask, used ONLY to clip polity
/// washes (map.js's <c>BorderLayer</c>) so they never spill into open sea --
/// static geometry (no <c>from</c>/<c>to</c>, unlike <see cref="PolitiesOut"/>),
/// fetched once and cached forever, same treatment as <see cref="LandmarkDto"/>.
/// <see cref="Rings"/> stays a raw <see cref="JsonElement"/> -- a flat
/// <c>[[[lat,lon],...],...]</c> array -- same "the client never inspects ring
/// coordinates, only forwards them to map.js" reasoning as
/// <see cref="PolityEraOut.Rings"/>.
public sealed record LandMaskOut(JsonElement Rings);
