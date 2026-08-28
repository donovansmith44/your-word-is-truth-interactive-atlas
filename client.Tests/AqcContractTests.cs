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
}
