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
    List<SceneArrow> Arrows,
    List<SceneNarrative> Narratives);

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
    List<SceneEvent> Events);

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

public sealed record VerseOut(int Verse, string Text);

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
    List<CrossRefOut> CrossRefs);

public sealed record BookMetaDto(string Author, string? WritePlace, int? WriteFrom, int? WriteTo);

public sealed record CrossRefOut(string Target, int Votes, string Preview);

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

/// <c>GET /api/borders?from=&amp;to=</c>. <see cref="Geojson"/> stays a raw
/// <see cref="JsonElement"/> rather than a typed geometry model -- the
/// client never inspects border geometry, only forwards it to map.js
/// (mirrors atlas-core's own <c>serde_json::Value</c> pass-through).
public sealed record BordersOut(int? SnapshotYear, JsonElement Geojson);

/// <c>GET /api/landmarks</c> array element.
public sealed record LandmarkDto(string Name, string Kind, double Lat, double Lon);
