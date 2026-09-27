using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class AuthorNode : IExplorable
{
    private readonly string _bookCode;
    private readonly AsyncMemo<VerseDetail> _detail = new();

    public AuthorNode(string bookCode) => _bookCode = bookCode;

    public string Title => _bookCode;
    public string Kind => "Author";

    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var meta = (await Load(api)).BookMeta;
        if (meta.WritePlace is null || meta.WriteFrom is not int from || meta.WriteTo is not int to)
        {
            return Array.Empty<Exploration>();
        }

        return new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"from={from}&to={to}")),
        };
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var meta = (await Load(api)).BookMeta;

        string? placeName = null;
        if (meta.WritePlace is string slug)
        {
            try
            {
                placeName = (await api.Place(slug)).Name;
            }
            catch (Exception)
            {
                placeName = CanonRef.Humanize(slug);
            }
        }

        var years = meta.WriteFrom is int wf && meta.WriteTo is int wt ? YearText.FormatRange(wf, wt) : null;

        RenderFragment fragment = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "popover-meta");
            builder.AddContent(seq++, $"By {meta.Author}.");
            builder.CloseElement();

            if (placeName is not null || years is not null)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-meta");
                var text = (placeName, years) switch
                {
                    (not null, not null) => $"Written from {placeName}, {years}.",
                    (not null, null) => $"Written from {placeName}.",
                    (null, not null) => $"Written {years}.",
                    _ => "",
                };
                builder.AddContent(seq++, text);
                builder.CloseElement();
            }
        };
        return fragment;
    }

    private Task<VerseDetail> Load(AtlasClient api) => _detail.Get(() => api.Verse($"{_bookCode}.1.1"));
}
