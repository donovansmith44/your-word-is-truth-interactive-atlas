using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

public sealed record Locus(string Book, int Chapter)
{
    public static readonly Locus Default = new("GEN", 1);

    public string Ref => $"{Book}.{Chapter}";
}

public sealed record SetLocus(string Book, int Chapter, string? Origin = null) : IIntent<Locus>
{
    public string Name => "set-locus";

    public Locus Apply(Locus current) => new(Book, Chapter);
}
