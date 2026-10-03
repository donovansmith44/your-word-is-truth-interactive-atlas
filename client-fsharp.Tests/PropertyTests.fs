module BibleAtlas.FSharp.Tests.PropertyTests

open System.Reflection
open FsCheck.Xunit
open Xunit

[<Property(MaxTest = 1)>]
let ``every compiled FSharp test is a property including inherited or aliased test attributes`` () =
    let methods = Assembly.GetExecutingAssembly().GetTypes() |> Array.collect (fun shape -> shape.GetMethods(BindingFlags.Public ||| BindingFlags.NonPublic ||| BindingFlags.Static ||| BindingFlags.Instance ||| BindingFlags.DeclaredOnly))
    let properties = methods |> Array.filter (fun method -> method.IsDefined(typeof<PropertyAttribute>, true))
    let tests = methods |> Array.filter (fun method -> method.IsDefined(typeof<FactAttribute>, true))
    let examples = tests |> Array.filter (fun method -> not (method.IsDefined(typeof<PropertyAttribute>, true))) |> Array.map (fun method -> method.DeclaringType.FullName + "." + method.Name) |> Array.sort |> Array.toList
    Assert.NotEmpty(properties)
    Assert.Equal((properties.Length, []), (tests.Length, examples))
