# F# client dependencies

This client uses Bolero 0.25.65 and Bolero.Build 0.25.65 (Apache-2.0; Loïc Denuzière and contributors), Elmish 4.0.1 (MIT; Eugene Tolmachev and contributors), FSharp.SystemTextJson 1.4.36 (MIT; Loïc Denuzière and contributors), and FSharp.Core supplied by the .NET SDK (MIT; Microsoft and contributors).

The build generator uses YamlDotNet 18.1.0 (MIT; Antoine Aubry and contributors). The test projects use xUnit, bUnit and Playwright; these are build/test dependencies and are absent from the published client.

The application links the existing client's CSS, fonts, JavaScript and Leaflet assets. Their attribution and the owner's OFL font exception are already recorded with the C# client. None is modified by this migration.

Primary license texts: [Bolero](https://github.com/fsbolero/Bolero/blob/master/LICENSE.md), [Elmish](https://github.com/elmish/elmish/blob/v4.x/LICENSE.md), [FSharp.SystemTextJson](https://github.com/Tarmil/FSharp.SystemTextJson/blob/master/LICENSE), [YamlDotNet](https://github.com/aaubry/YamlDotNet/blob/master/LICENSE.txt), [.NET](https://github.com/dotnet/runtime/blob/main/LICENSE.TXT).

Before the parallel client lands, the new runtime/build dependency attribution must be reconciled into the repository's root LICENSES.md and included with distributed assets. That shared file is currently in Claude's licensing work; this task does not edit it concurrently.
