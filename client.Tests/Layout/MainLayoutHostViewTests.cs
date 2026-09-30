using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class MainLayoutHostViewTests
{
    [Fact]
    public void The_layout_hands_its_popover_the_host_view_from_the_one_url_signal_it_has()
    {
        // Arrange
        var layout = File.ReadAllLines(Path.Combine(ConformanceTests.ClientRoot, "Layout", "MainLayout.razor"));

        // Act
        var popover = layout.Single(line => line.Contains("<ExplorerPopover "));

        // Assert
        Assert.Contains("HostView=\"@(IsWorld ? ViewNames.World : ViewNames.Reader)\"", popover);
    }
}
