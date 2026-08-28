namespace BibleAtlas.Client;

/// <summary>
/// Batch AQC-1 (design spec §2's versioning law): the client's own
/// compiled AQC version, and the ONE semver-range check the fail-loud
/// startup gate (<see cref="Pages.ContractMismatch"/> / <c>App.razor</c>'s
/// own startup check) and the C# contract harness (client.ContractTests,
/// versioning.feature) both call -- one implementation, not a duplicate
/// per caller. <see cref="ClientVersion"/> is a compile-time constant,
/// hand-kept in lockstep with <c>contracts/atlas-query-contract/VERSION</c>
/// (the SAME release-process discipline
/// <c>server/atlas-server/src/contract.rs</c>'s own
/// <c>MIN_SUPPORTED_VERSION</c>/<c>MAX_SUPPORTED_VERSION</c> constants
/// carry -- see that file's own doc comment).
/// </summary>
public static class AqcContract
{
    public const string ClientVersion = "0.1.0";

    /// <summary>
    /// True iff <see cref="ClientVersion"/> falls within the server's own
    /// advertised <c>[MinVersion, MaxVersion]</c> range (inclusive),
    /// compared as MAJOR.MINOR.PATCH triples -- the same semver-range
    /// check the Rust cucumber harness's own <c>satisfies</c> fn proves
    /// independently (glossary.md's own phrase-parity pairing).
    /// </summary>
    public static bool Satisfies(ContractDto contract)
    {
        var client = ParseSemver(ClientVersion);
        var min = ParseSemver(contract.MinVersion);
        var max = ParseSemver(contract.MaxVersion);
        return Compare(client, min) >= 0 && Compare(client, max) <= 0;
    }

    private static (int Major, int Minor, int Patch) ParseSemver(string s)
    {
        var parts = s.Split('.');
        if (parts.Length != 3 || !int.TryParse(parts[0], out var major) || !int.TryParse(parts[1], out var minor) || !int.TryParse(parts[2], out var patch))
        {
            throw new FormatException($"AqcContract: '{s}' is not a MAJOR.MINOR.PATCH semver string");
        }
        return (major, minor, patch);
    }

    private static int Compare((int Major, int Minor, int Patch) a, (int Major, int Minor, int Patch) b)
    {
        if (a.Major != b.Major) return a.Major.CompareTo(b.Major);
        if (a.Minor != b.Minor) return a.Minor.CompareTo(b.Minor);
        return a.Patch.CompareTo(b.Patch);
    }
}
