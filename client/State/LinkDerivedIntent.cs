using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

public sealed record LinkDerivedIntent<T>(string Origin, T NewValue) : IIntent<T>
{
    public string Name => $"link:{Origin}";

    public T Apply(T current) => NewValue;
}
