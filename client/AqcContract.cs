namespace BibleAtlas.Client;

public static class AqcContract
{
    public const string ClientVersion = "0.8.0";

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
