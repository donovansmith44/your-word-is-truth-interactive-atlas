using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.Views;

public sealed class EnterSplitHatch : IEscapeHatch
{
    private readonly Func<string, Task> _invokeWith;
    private readonly Func<string, string>? _guestLabel;

    public string GuestLabel(string guestView) => _guestLabel?.Invoke(guestView) ?? guestView;

    public EnterSplitHatch(string ownerView, string partnerView, string hostView, Func<Task> invoke)
        : this(ownerView, new[] { partnerView }, hostView, _ => invoke())
    {
    }

    public EnterSplitHatch(string ownerView, IReadOnlyList<string> partnerViews, string hostView, Func<string, Task> invokeWith, Func<string, string>? guestLabel = null)
    {
        _guestLabel = guestLabel;
        if (partnerViews.Count == 0)
        {
            throw new ArgumentException("an enter-split hatch offers at least one guest", nameof(partnerViews));
        }

        OwnerView = ownerView;
        PartnerViews = partnerViews;
        HostView = hostView;
        _invokeWith = invokeWith;
    }

    public string Kind => HatchKinds.EnterSplit;

    public string OwnerView { get; }

    public string PartnerView => PartnerViews[0];

    public IReadOnlyList<string> PartnerViews { get; }

    public string HostView { get; }

    public Task Invoke() => _invokeWith(PartnerViews[0]);

    public Task InvokeWith(string guestView)
    {
        if (!PartnerViews.Contains(guestView))
        {
            throw new ArgumentException($"'{guestView}' is not a guest this hatch offers ({string.Join(", ", PartnerViews)})", nameof(guestView));
        }

        return _invokeWith(guestView);
    }
}
