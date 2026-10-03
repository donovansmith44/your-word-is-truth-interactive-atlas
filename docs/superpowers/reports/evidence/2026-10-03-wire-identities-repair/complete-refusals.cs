using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
public static class Forbidden
{
    public static object ExtractArtifactRoot(ArtifactRoot identity) => identity.Value;
    public static object ExtractBibleReference(BibleReference identity) => identity.Value;
    public static object ExtractChapterReference(ChapterReference identity) => identity.Value;
    public static object ExtractConcordReference(ConcordReference identity) => identity.Value;
    public static object ExtractContentsReference(ContentsReference identity) => identity.Value;
    public static object ExtractCrossReferenceTarget(CrossReferenceTarget identity) => identity.Value;
    public static object ExtractEdgeId(EdgeId identity) => identity.Value;
    public static object ExtractEdgePageCursor(EdgePageCursor identity) => identity.Value;
    public static object ExtractElementId(ElementId identity) => identity.Value;
    public static object ExtractElementPageCursor(ElementPageCursor identity) => identity.Value;
    public static object ExtractEraId(EraId identity) => identity.Value;
    public static object ExtractNarrativeId(NarrativeId identity) => identity.Value;
    public static object ExtractNodeId(NodeId identity) => identity.Value;
    public static object ExtractPassageReference(PassageReference identity) => identity.Value;
    public static object ExtractPolityId(PolityId identity) => identity.Value;
    public static object ExtractTextWindowReference(TextWindowReference identity) => identity.Value;
    public static object ExtractUnitReference(UnitReference identity) => identity.Value;
    public static object ExtractVerseRangeReference(VerseRangeReference identity) => identity.Value;
    public static object ExtractVerseReference(VerseReference identity) => identity.Value;
    public static object ExtractVerseSpanReference(VerseSpanReference identity) => identity.Value;
    public static void WrongReferences(AtlasClient api, ArtifactRoot root)
    {
        _ = api.Chapter(root);
        _ = api.ChapterText(root);
        _ = api.KretzmannChapter(root);
        _ = api.SceneScripture(root);
    }
}
