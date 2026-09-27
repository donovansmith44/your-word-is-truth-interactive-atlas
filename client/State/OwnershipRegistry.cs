namespace BibleAtlas.Client.State;

public sealed class OwnershipRegistry
{
    private readonly Dictionary<string, object> _owners = new();

    public OwnershipClaim Claim(string name)
    {
        var token = new object();
        _owners[name] = token;
        return new OwnershipClaim(this, name, token);
    }

    internal bool IsCurrent(string name, object token) =>
        _owners.TryGetValue(name, out var owner) && ReferenceEquals(owner, token);

    internal void Release(string name, object token)
    {
        if (_owners.TryGetValue(name, out var owner) && ReferenceEquals(owner, token))
        {
            _owners.Remove(name);
        }
    }
}

public sealed class OwnershipClaim : IDisposable
{
    private readonly OwnershipRegistry _registry;
    private readonly string _name;
    private readonly object _token;
    private bool _disposed;

    internal OwnershipClaim(OwnershipRegistry registry, string name, object token)
    {
        _registry = registry;
        _name = name;
        _token = token;
    }

    public bool IsCurrent => !_disposed && _registry.IsCurrent(_name, _token);

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _registry.Release(_name, _token);
    }
}
