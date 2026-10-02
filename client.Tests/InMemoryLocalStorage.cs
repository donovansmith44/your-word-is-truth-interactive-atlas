using Microsoft.JSInterop;

namespace BibleAtlas.Client.Tests;

internal sealed class InMemoryLocalStorage : IJSInProcessRuntime
{
    private const string GetItem = "localStorage.getItem";
    private const string SetItem = "localStorage.setItem";
    private const string RemoveItem = "localStorage.removeItem";

    public SortedDictionary<string, string> Items { get; } = [];

    public TValue Invoke<TValue>(string identifier, params object?[]? args)
    {
        var key = (string)args![0]!;
        switch (identifier)
        {
            case GetItem:
                return (TValue)(object?)Items.GetValueOrDefault(key)!;
            case SetItem:
                Items[key] = (string)args[1]!;
                return default!;
            case RemoveItem:
                Items.Remove(key);
                return default!;
            default:
                throw new NotSupportedException(identifier);
        }
    }

    public ValueTask<TValue> InvokeAsync<TValue>(string identifier, object?[]? args) => throw new NotSupportedException(identifier);

    public ValueTask<TValue> InvokeAsync<TValue>(string identifier, CancellationToken cancellationToken, object?[]? args) =>
        throw new NotSupportedException(identifier);
}
