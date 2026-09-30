using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Link(EdgeKind Kind, Explorable Target);
