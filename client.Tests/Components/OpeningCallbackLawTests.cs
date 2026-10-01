using System.Reflection;
using BibleAtlas.Client.Explore;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Tests;

public sealed class OpeningCallbackLawTests
{
    [Fact]
    public void No_component_hands_its_host_a_bare_legacy_node()
    {
        // Arrange
        var parameters = typeof(IExplorable).Assembly.GetTypes()
            .Where(type => typeof(ComponentBase).IsAssignableFrom(type))
            .SelectMany(type => type.GetProperties(BindingFlags.Public | BindingFlags.Instance))
            .Where(property => property.GetCustomAttribute<ParameterAttribute>() is not null);

        // Act
        var carryingLegacyNodes = parameters
            .Where(property => property.PropertyType.IsGenericType
                && property.PropertyType.GetGenericTypeDefinition() == typeof(EventCallback<>)
                && typeof(IExplorable).IsAssignableFrom(property.PropertyType.GetGenericArguments()[0]))
            .Select(property => $"{property.DeclaringType!.Name}.{property.Name}")
            .ToList();

        // Assert
        Assert.Empty(carryingLegacyNodes);
    }
}
