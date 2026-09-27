using System.Text.Json;

namespace BibleAtlas.Client;

public sealed record Scene(
    string Mode,
    TimeRangeDto? Window,
    string? Ref,
    List<ScenePlace> Places,
    List<QuietPlace> QuietPlaces,
    List<SceneArrow> Arrows,
    List<SceneNarrative> Narratives);

public sealed record TimeRangeDto(int FromYear, int ToYear);

public sealed record ScenePlace(
    string Id,
    string Name,
    string DisplayName,
    double Lat,
    double Lon,
    int Brightness,
    List<SceneEvent> Events,
    int? ExistenceFrom = null,
    int? ExistenceTo = null);

public sealed record QuietPlace(
    string Id,
    string DisplayName,
    double Lat,
    double Lon,
    int TotalEvents,
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

public sealed record KretzmannChapterItemOut(string Id, string? Heading);

public sealed record KretzmannChapterVerseOut(int Verse, List<KretzmannChapterItemOut> Items);

public sealed record KretzmannChapterOut(List<KretzmannChapterVerseOut> Verses, string Version);

public sealed record HeadingDto(string EventId, string Title, string Kind, bool IsContinuation = false);

public sealed record VerseOut(int Verse, string Text, List<PlaceRefDto> Places, List<PersonRefDto> Persons, List<WordsOfChristSpanDto> WordsOfChrist, HeadingDto? Heading = null, int XrefCount = 0);

public sealed record PlaceRefDto(string Id, string Name);

public sealed record PersonRefDto(string Id, string Name);

public sealed record WordsOfChristSpanDto(int Start, int End);

public sealed record VerseEventDto(string Id, string Label, TimeRangeDto? When, List<VerseGroup> VerseGroups, List<string> Places,
    string Kind,
    // Defaults to "" only so a hand-built test fixture need not restate it -- the server always
    // sends a real value, and a blank one is meaningful: it renders a loud unresolved-provenance
    // notice, not silence.
    string Provenance = "");

public sealed record VerseDetail(
    string Ref,
    string Text,
    List<WordsOfChristSpanDto> WordsOfChrist,
    BookMetaDto BookMeta,
    List<VerseEventDto> Events,
    List<CrossRefOut> CrossRefs,
    List<CatechismRefDto> Catechism,
    string Provenance = "",
    List<string>? CrossRefsProvenance = null,
    List<string>? CatechismProvenance = null)
{
    public IReadOnlyList<string> CrossRefsProvenanceOrEmpty => CrossRefsProvenance ?? new List<string>();

    public IReadOnlyList<string> CatechismProvenanceOrEmpty => CatechismProvenance ?? new List<string>();
}

public sealed record NarrativePositionDto(
    string NarrativeId,
    string NarrativeName,
    string EventId,
    string EventLabel,
    NarrativeAdjacentEventDto? Prior,
    NarrativeAdjacentEventDto? Following);

public sealed record NarrativeAdjacentEventDto(string Id, string Label, List<string> Places, List<VerseGroup> VerseGroups);

public sealed record TimelinePositionDto(NarrativeAdjacentEventDto? Prior, NarrativeAdjacentEventDto? Following);

public sealed record NarrativeEventPositionsResult(List<NarrativePositionDto> Narrative, TimelinePositionDto? Timeline = null);

public sealed record EventPlaceDto(string Id, string Name);

public sealed record EventWitnessDto(
    string Book,
    List<VerseGroup> VerseGroups,
    string? RefNote = null,
    string? RobertsonSection = null);

public sealed record EventDetail(
    string Id,
    string Title,
    string Kind,
    TimeRangeDto? When,
    List<EventPlaceDto> Places,
    List<EventWitnessDto> Witnesses,
    string? RobertsonSection = null,
    string? ActsSection = null,
    string? AtlasSection = null,
    string? KjvSuperscription = null,
    string? RefNote = null,
    List<string>? MentionedIn = null,
    List<EventAnalogueDto>? Analogues = null,
    string Provenance = "",
    List<string>? WitnessesProvenance = null,
    List<string>? MentionsProvenance = null)
{
    public IReadOnlyList<string> MentionedInOrEmpty => MentionedIn ?? new List<string>();

    public IReadOnlyList<EventAnalogueDto> AnaloguesOrEmpty => Analogues ?? new List<EventAnalogueDto>();

    public IReadOnlyList<string> WitnessesProvenanceOrEmpty => WitnessesProvenance ?? new List<string>();

    public IReadOnlyList<string> MentionsProvenanceOrEmpty => MentionsProvenance ?? new List<string>();
}

public sealed record EventAnalogueDto(string Id, string Title, string Provenance = "");

public sealed record BookMetaDto(string Author, string? WritePlace, int? WriteFrom, int? WriteTo);

// Provenance is section-level, not per-row: every element of one response carries the identical
// set (the server cannot supply per-row values here). An absent list means "no attribution
// section here," which is not the same as a blank id in a present list.
public sealed record CrossRefOut(string Target, int Votes, string Preview, List<string>? Provenance = null)
{
    public IReadOnlyList<string> ProvenanceOrEmpty => Provenance ?? new List<string>();
}

// Provenance here is likewise section-level: identical across every element of one response,
// never per-row.
public sealed record CatechismRefDto(string Id, string Name, string? Question = null, List<string>? Provenance = null)
{
    public IReadOnlyList<string> ProvenanceOrEmpty => Provenance ?? new List<string>();
}

// Vref, not VRef: Wire.Options's snake_case policy maps a two-capital acronym run ("VRef") to
// "v_ref", not the server's literal `vref` JSON key -- a mismatch that deserializes silently to
// null/empty rather than throwing, so the casing here is load-bearing, not a style choice.
public sealed record CatechismProofVerseDto(string Vref, string Text, string? Question = null);

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
    PlaceHistoryOut? History,
    string? CanonicalName = null);

public sealed record PlaceHistoryOut(string DisplayName, string? Blurb, DateClaimOut? Established, DateClaimOut? Destroyed);

public sealed record DateClaimOut(TimeRangeDto When, List<string> Verses, string? Note);

public sealed record NarrativeOut(string Id, string Name, string Color, List<string> Legs);

public sealed record PolityDeltaDto(string Event, List<string> Verses, string RefNote);

public sealed record PolityEraOut(string Id, string Name, int From, int To, JsonElement Rings, int ColorKey, PolityDeltaDto? Transition, PolityDeltaDto? Fall);

public sealed record PolitiesOut(List<PolityEraOut> Polities);

public sealed record LandmarkDto(string Name, string Kind, double Lat, double Lon, string? Size);

public sealed record LandMaskOut(JsonElement Rings);

public sealed record SourcesDocumentOut(
    List<SourceCategoryDto> Categories,
    List<SourceEntryDto> Sources,
    List<ProvenanceEntryDto>? Provenances = null)
{
    public IReadOnlyList<ProvenanceEntryDto> ProvenancesOrEmpty => Provenances ?? new List<ProvenanceEntryDto>();
}

public sealed record ProvenanceEntryDto(string Id, string Source, string Confidence, string? Locator = null);

public sealed record SourceCategoryDto(string Id, string Label);

public sealed record SourceEntryDto(
    string Id,
    string Category,
    string Title,
    string WhatItIs,
    string WhatWeBuilt,
    string License,
    string? Link,
    string LicensesRowKey);

public sealed record EdgeSummaryEntryDto(string Kind, int Count);

public sealed record NodeCardDto(string Id, string Kind, string Label, string Provenance, List<EdgeSummaryEntryDto> EdgeSummary, string Version, string? Description = null, PersonLifeDto? Person = null);

public sealed record PersonLifeDto(string? Gender, int? BirthYear, int? DeathYear, int? FirstYear, int? LastYear, bool Eternal, List<string> EternalGrounds, List<string> AlsoCalled);

public sealed record NodeRefDto(string Id, string Kind, string Label);

public sealed record EdgeEntryDto(string Edge, NodeRefDto Node);

public sealed record EdgePageDto(string Kind, List<EdgeEntryDto> Entries, int? Next, string Version);

public sealed record TextUnitDto(
    string Ref,
    string Text,
    List<WordsOfChristSpanDto> WordsOfChrist,
    List<EdgeSummaryEntryDto> EdgeSummary);

public sealed record TextWindowDto(List<TextUnitDto> Units, string? Next, string Version);

public sealed record ContractDto(string MinVersion, string MaxVersion);

public sealed record ContentsChildOut(string Id, string Title, string Kind, string Ref, int Count);

public sealed record ContentsRootOut(string Id, string Title, string Kind, string? Group, string Ref, List<ContentsChildOut> Children);

public sealed record ContentsOut(string Corpus, string Version, List<ContentsRootOut> Roots);
