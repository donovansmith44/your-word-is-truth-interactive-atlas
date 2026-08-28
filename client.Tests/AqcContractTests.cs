namespace BibleAtlas.Client.Tests;

/// <summary>
/// Batch AQC-1: <see cref="AqcContract.Satisfies"/> -- the semver-range
/// check the fail-loud startup gate (App.razor) and the C# contract
/// harness (client.ContractTests, versioning.feature) both call.
/// </summary>
public class AqcContractTests {
    [Fact]
    public void ClientVersionIsWithinItsOwnAdvertisedRange() =>
        Assert.True(AqcContract.Satisfies(new ContractDto(AqcContract.ClientVersion, AqcContract.ClientVersion)));

    [Theory]
    [InlineData("0.1.0", "0.1.0")]
    [InlineData("0.0.9", "0.2.0")]
    public void AcceptsWhenClientVersionFallsWithinRange(string min, string max) =>
        Assert.True(AqcContract.Satisfies(new ContractDto(min, max)));

    [Theory]
    [InlineData("0.2.0", "0.5.0")]
    [InlineData("0.0.1", "0.0.9")]
    public void RejectsWhenClientVersionFallsOutsideRange(string min, string max) =>
        Assert.False(AqcContract.Satisfies(new ContractDto(min, max)));

    [Fact]
    public void MalformedAdvertisedVersionThrowsRatherThanSilentlyPassing() =>
        Assert.Throws<FormatException>(() => AqcContract.Satisfies(new ContractDto("not-a-semver", "0.1.0")));

    /// <summary>
    /// Q-6 fix (Batch AQC-1 fix round 1, controller ruling -- "single-source
    /// [the five hand-kept "0.1.0" copies] or extend the cross-check to all
    /// five"): <see cref="AqcContract.ClientVersion"/> is a compiled
    /// constant (it ships in the WASM bundle -- it cannot read a
    /// repo-relative file at runtime once deployed, unlike the Rust test
    /// harness's own single-sourced mirror). This test is the cross-check:
    /// it must agree with <c>contracts/atlas-query-contract/VERSION</c>'s
    /// own trimmed contents -- the SAME file
    /// <c>aqc_corpus_generation.rs::version_file_and_schema_version_agree_with_the_compiled_server_constants</c>
    /// cross-checks against the compiled SERVER constants. Together the two
    /// tests tie all five copies (VERSION, aqc.schema.json's own
    /// "version", contract.rs's two consts, and this client constant) back
    /// to one file.
    /// </summary>
    [Fact]
    public void ClientVersionAgreesWithTheVersionFile()
    {
        var repoRoot = FindRepoRoot();
        var versionPath = Path.Combine(repoRoot, "contracts", "atlas-query-contract", "VERSION");
        Assert.True(File.Exists(versionPath), $"VERSION file not found at {versionPath}");
        var version = File.ReadAllText(versionPath).Trim();
        Assert.Equal(AqcContract.ClientVersion, version);
    }

    private static string FindRepoRoot([System.Runtime.CompilerServices.CallerFilePath] string here = "") =>
        Path.GetFullPath(Path.Combine(Path.GetDirectoryName(here)!, ".."));
}
