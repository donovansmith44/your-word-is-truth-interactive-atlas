using System.Reflection;
using System.Reflection.Emit;
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Exploring;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Rendering;

namespace BibleAtlas.Client.Tests;

public sealed class ComponentParameterLawTests
{
    private const BindingFlags Declared = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static | BindingFlags.Instance | BindingFlags.DeclaredOnly;

    private static readonly Dictionary<short, OpCode> Codes = typeof(OpCodes)
        .GetFields(BindingFlags.Public | BindingFlags.Static)
        .Select(field => (OpCode)field.GetValue(null)!)
        .ToDictionary(code => code.Value);

    private static readonly string[] Opens = [nameof(RenderTreeBuilder.OpenElement), nameof(RenderTreeBuilder.OpenRegion)];
    private static readonly string[] Closes = [nameof(RenderTreeBuilder.CloseElement), nameof(RenderTreeBuilder.CloseRegion), nameof(RenderTreeBuilder.CloseComponent)];
    private static readonly string[] Gives = [nameof(RenderTreeBuilder.AddAttribute), nameof(RenderTreeBuilder.AddComponentParameter)];

    private sealed record Given(MethodBase Site, Type Component, string Parameter)
    {
        public override string ToString() => $"{Site.DeclaringType?.FullName}.{Site.Name} gives {Component.Name} the parameter {Parameter}";
    }

    [Fact]
    public void Every_component_the_client_renders_is_given_only_parameters_it_declares()
    {
        // Arrange
        var assemblies = new[] { typeof(RefsList).Assembly, typeof(Paging).Assembly };

        // Act
        var given = assemblies.SelectMany(assembly => assembly.GetTypes()).SelectMany(type => type.GetMethods(Declared).Cast<MethodBase>().Concat(type.GetConstructors(Declared))).SelectMany(ParametersGiven).ToList();

        // Assert
        Assert.Contains(given, g => g.Component == typeof(RefsList) && g.Parameter == nameof(RefsList.TestIdPrefix));
        Assert.Contains(given, g => g.Component == typeof(CouldNotLoad) && g.Parameter == nameof(CouldNotLoad.OnRetry));
        var undeclared = given.Where(g => !Declares(g.Component, g.Parameter)).ToList();
        Assert.True(undeclared is [], string.Join(Environment.NewLine, undeclared));
    }

    private static bool Declares(Type component, string parameter) =>
        component.GetProperties(BindingFlags.Public | BindingFlags.Instance).Any(property =>
            property.GetCustomAttribute<ParameterAttribute>() is { } declared
            && (declared.CaptureUnmatchedValues || string.Equals(property.Name, parameter, StringComparison.OrdinalIgnoreCase)));

    private static IEnumerable<Given> ParametersGiven(MethodBase method)
    {
        var il = method.GetMethodBody()?.GetILAsByteArray();
        if (il is null)
        {
            return [];
        }

        var typeArguments = method.DeclaringType is { IsGenericType: true } declaring ? declaring.GetGenericArguments() : null;
        var methodArguments = method.IsGenericMethod ? method.GetGenericArguments() : null;
        var open = new Stack<Type?>();
        var given = new List<Given>();
        string? named = null;
        foreach (var (code, token) in Instructions(il))
        {
            if (code == OpCodes.Ldstr)
            {
                named ??= method.Module.ResolveString(token);
            }
            else if ((code == OpCodes.Call || code == OpCodes.Callvirt) && method.Module.ResolveMethod(token, typeArguments, methodArguments) is { } called && called.DeclaringType == typeof(RenderTreeBuilder))
            {
                if (called.Name == nameof(RenderTreeBuilder.OpenComponent))
                {
                    open.Push(called.IsGenericMethod ? called.GetGenericArguments()[0] : null);
                }
                else if (Opens.Contains(called.Name))
                {
                    open.Push(null);
                }
                else if (Closes.Contains(called.Name))
                {
                    open.TryPop(out _);
                }
                else if (Gives.Contains(called.Name) && called.GetParameters() is [_, { ParameterType: var name }, ..] && name == typeof(string) && named is not null && open.TryPeek(out var component) && component is not null)
                {
                    given.Add(new Given(method, component, named));
                }

                named = null;
            }
        }

        return given;
    }

    private static IEnumerable<(OpCode Code, int Token)> Instructions(byte[] il)
    {
        var at = 0;
        while (at < il.Length)
        {
            var wide = il[at] == 0xFE;
            var code = Codes[wide ? unchecked((short)(0xFE00 | il[at + 1])) : il[at]];
            at += wide ? 2 : 1;
            var size = code.OperandType switch
            {
                OperandType.InlineNone => 0,
                OperandType.ShortInlineBrTarget or OperandType.ShortInlineI or OperandType.ShortInlineVar => 1,
                OperandType.InlineVar => 2,
                OperandType.InlineI8 or OperandType.InlineR => 8,
                OperandType.InlineSwitch => 4 + (4 * BitConverter.ToInt32(il, at)),
                _ => 4,
            };
            yield return (code, size == 4 ? BitConverter.ToInt32(il, at) : 0);
            at += size;
        }
    }
}
