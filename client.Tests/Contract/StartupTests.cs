using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class StartupTests
{
    private const string TheRouterAlone =
        """
        <Router AppAssembly="@typeof(App).Assembly" NotFoundPage="typeof(Pages.NotFound)">
            <Found Context="routeData">
                <RouteView RouteData="@routeData" DefaultLayout="@typeof(MainLayout)"/>
                <FocusOnNavigate RouteData="@routeData" Selector="h1" />
            </Found>
        </Router>

        """;

    [Fact]
    public void The_app_is_the_router_alone_with_no_contract_check_before_it()
    {
        // Arrange
        var appPath = Path.Combine(ConformanceTests.ClientRoot, "App.razor");
        // Act
        var app = File.ReadAllText(appPath).ReplaceLineEndings("\n");
        // Assert
        Assert.Equal(TheRouterAlone, app);
    }

    [Fact]
    public void No_source_of_the_startup_contract_check_remains()
    {
        // Arrange
        var startupCheckSources = new[]
        {
            Path.Combine(ConformanceTests.ClientRoot, "Pages", "ContractMismatch.razor"),
            Path.Combine(ConformanceTests.ClientRoot, "AqcContract.cs"),
        };
        // Act
        var remaining = startupCheckSources.Where(File.Exists).ToList();
        // Assert
        Assert.Empty(remaining);
    }
}
