using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed class ChapterText
{
    private readonly IReadOnlyDictionary<int, TextUnit> _byVerse;

    public ChapterText(TextWindow window)
    {
        Units = window.Units;
        _byVerse = Units.ToDictionary(unit => ((BibleRef)unit.Locus).Verse);
    }

    public IReadOnlyList<TextUnit> Units { get; }

    public TextUnit Verse(int number) => _byVerse[number];

    public IReadOnlyList<TextUnit> Between(int fromVerse, int toVerse) =>
        Units.Where(unit => ((BibleRef)unit.Locus).Verse >= fromVerse && ((BibleRef)unit.Locus).Verse <= toVerse).ToList();
}
