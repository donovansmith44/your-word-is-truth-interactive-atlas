using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class ConcordUnitNode : IExplorable
{
    private readonly string? _givenText;
    private readonly AsyncMemo<string> _text = new();

    public ConcordUnitNode(string citation, string? text = null)
    {
        Title = citation;
        _givenText = text;
    }

    public string Title { get; }
    public string Kind => "ConcordUnit";

    public string NodeId => $"text-unit:{Title}";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    public Task<string> TextAsync(AtlasClient api) =>
        _givenText is { } given
            ? Task.FromResult(given)
            : _text.Get(async () => (await api.ConcordUnit(Title)).Units.FirstOrDefault()?.Text ?? string.Empty);

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        string text;
        try
        {
            text = await TextAsync(api);
        }
        catch (Exception)
        {
            text = string.Empty;
        }

        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-concord-text");
            builder.AddContent(2, text);
            builder.CloseElement();
        };
        return fragment;
    }
}
