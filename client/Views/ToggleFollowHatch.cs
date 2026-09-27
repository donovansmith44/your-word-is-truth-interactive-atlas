using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.Views;

public sealed class ToggleFollowHatch : IEscapeHatch
{
    private readonly Func<Task> _invoke;

    public ToggleFollowHatch(string ownerView, Func<Task> invoke)
    {
        OwnerView = ownerView;
        _invoke = invoke;
    }

    public string Kind => HatchKinds.ToggleFollow;

    public string OwnerView { get; }

    public Task Invoke() => _invoke();
}
