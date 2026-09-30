namespace BibleAtlas.Client;

public sealed record Draft(string Shown, string? Typed)
{
    public static readonly Draft Empty = new("", null);

    public string Text => Typed ?? Shown;

    public Draft Served(string shown) => this with { Shown = shown };

    public Draft Type(string text) => this with { Typed = text };

    public Draft Committed() => new(Text, null);

    public Draft Discarded() => this with { Typed = null };
}
