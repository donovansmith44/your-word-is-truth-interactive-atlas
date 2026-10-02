#!/bin/sh
set -e
[ -d "$HOME/.dotnet" ] && export DOTNET_ROOT="$HOME/.dotnet" PATH="$HOME/.dotnet:$PATH"
cd "$(dirname "$0")/../.."
printf '{ "ApiBase": "http://localhost:%s" }\n' "${ATLAS_API_PORT:?}" > client/wwwroot/appsettings.Ux.json
exec dotnet run --project client --no-launch-profile -p:WasmApplicationEnvironmentName=Ux --urls "http://localhost:${ATLAS_CLIENT_PORT:?}"
