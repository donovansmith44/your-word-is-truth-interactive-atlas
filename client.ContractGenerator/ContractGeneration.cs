namespace BibleAtlas.Client.ContractGenerator;

public static class ContractGeneration
{
    // No client or pact-test code ever asks for /api/text's optional `scope`
    // (AtlasClient.ConcordUnit always omits it and the server defaults it),
    // so no hand-written code names TextScope and nothing $refs it either.
    public static readonly IReadOnlySet<string> Unread = new HashSet<string> { "TextScope" };
}
