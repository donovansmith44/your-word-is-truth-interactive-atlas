using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A book's author/provenance (fetched via BookMeta on "{book}.1.1").
/// Title is the book code -- IExplorable requires Title synchronously, and
/// the real author NAME is only known after the fetch, so the popover
/// header shows the (already ref-shaped) book code the instant this node is
/// pushed, while BodyAsync's prose names the actual author underneath once
/// it resolves.
/// </summary>
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
                // The write-place slug should always resolve (the ETL warns
                // and drops unknown ones before compiling) -- this fallback
                // only guards against an unexpected fetch failure so the
                // popover still shows something readable instead of blowing
                // up the whole body.
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

    // PERF-3 re-review fix round 2 (Major, new -- found by the re-review's
    // own grep sweep after this batch's fix round 1 claimed uniform
    // application and missed this one): this was still the pre-batch
    // value-memoizing idiom, and IS a live race -- ExplorerPopover.LoadCurrent's
    // own non-registry fallback path (Author is one of the node kinds with
    // no section-registry providers) dispatches ExploreAsync and BodyAsync
    // via ONE Task.WhenAll (ExplorerPopover.razor's own `explorationsTask`/
    // `bodyTask`), and BOTH independently call Load -- the exact same
    // synchronous-prefix-then-first-await race this batch's own VerseNode
    // fix was built around, just reached through the ExploreAsync+BodyAsync
    // fallback shape instead of the section-registry Task.WhenAll shape.
    // AsyncMemo-backed now, same fix as every other node in this file.
    private Task<VerseDetail> Load(AtlasClient api) => _detail.Get(() => api.Verse($"{_bookCode}.1.1"));
}
