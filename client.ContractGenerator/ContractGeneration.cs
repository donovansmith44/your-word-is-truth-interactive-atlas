namespace BibleAtlas.Client.ContractGenerator;

public static class ContractGeneration
{
    // The client never sends /api/text's optional `scope`, and nothing $refs TextScope.
    public static readonly IReadOnlySet<string> Unread = new HashSet<string> { "TextScope" };
}
