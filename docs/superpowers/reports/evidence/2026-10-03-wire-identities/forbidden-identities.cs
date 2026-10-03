using BibleAtlas.Client.Contract;
public static class WrongIdentities
{
    public static NodeId RootAsNode(ArtifactRoot root) => root;
    public static EdgePageCursor ElementCursorAsEdge(ElementPageCursor cursor) => cursor;
    public static ChapterReference VerseAsChapter(VerseReference verse) => verse;
    public static void Main() {}
}
