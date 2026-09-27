using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Components;

public sealed record CompositionSplitContext(bool IsSplitOpen, bool IsHost, EventCallback InvokeHatch, EventCallback RequestClose, string HostPaneClassSuffix, EventCallback<string> InvokeHatchWith = default, IReadOnlyList<string>? PartnerViews = null, Func<string, string>? PartnerLabel = null, bool IsSameView = false, bool Follow = false, EventCallback ToggleFollow = default)
{
    public IReadOnlyList<string> Partners => PartnerViews ?? Array.Empty<string>();

    public string LabelOf(string guest) => PartnerLabel?.Invoke(guest) ?? guest;
}
