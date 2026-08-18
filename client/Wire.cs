using System.Text.Json;

namespace BibleAtlas.Client;

/// <summary>
/// Shared JSON wire options for talking to atlas-server: snake_case property
/// names on both serialize and deserialize, matching atlas-core's serde
/// output exactly (e.g. `FromYear` &lt;-&gt; `from_year`, `Ref` &lt;-&gt;
/// `ref`). One singleton instance -- every (de)serialization of a wire DTO
/// in the client goes through <see cref="Options"/>, never a per-call
/// <c>JsonSerializerOptions</c>.
/// </summary>
public static class Wire
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };
}
