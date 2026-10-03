# Parallel F# client

Work in progress on CX-FSHARP. This is an independent Bolero client using Elmish model–view–update and generated immutable F# contract types. The C# client remains the reference and remains runnable. Rendered slices include Sources/header, Reader/Concord text and anchors, Concord Next/Retry, and cards/text in the exploration popover. These are incomplete routes; frontiers, pickers, storage, maps and split composition remain required work.

Run commands from the repository root after `. ~/.bible-atlas-env`:

```sh
dotnet test client-fsharp.Tests
dotnet build client-fsharp
dotnet run --project client-fsharp --no-launch-profile --no-build --urls http://127.0.0.1:5100
dotnet publish client-fsharp -c Release -m:4 -p:RunAOTCompilation=true
python3 scripts/fsharp-client/serve-published.py
```

Development API configuration uses port 8100. The published client uses its own origin unless ApiBase is configured. Stop the development host before serving the published client on the same port.

```sh
npm ci --prefix tests/parity-fsharp
npx --prefix tests/parity-fsharp playwright test --config tests/parity-fsharp/playwright.config.ts
FSHARP_PUBLISHED=1 npx --prefix tests/parity-fsharp playwright test --config tests/parity-fsharp/playwright.config.ts
python3 scripts/fsharp-client/parity-ledger.py
python3 scripts/fsharp-client/parity-ledger.py --require-complete
```

The six browser tests mock API responses and prove real WASM/AOT startup, whole source/text rendering, visible focus, page Retry, malformed-answer rejection and shared asset bytes. They do not prove side-by-side client parity. The completion gate currently fails with 217 pending inventory entries. Full integration/parity/resource/usage/mutation gates and license reconciliation remain open.
