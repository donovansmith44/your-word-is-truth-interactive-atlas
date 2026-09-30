#!/bin/sh
set -e
[ -d "$HOME/.dotnet" ] && export DOTNET_ROOT="$HOME/.dotnet" PATH="$HOME/.dotnet:$PATH"
cd "$(dirname "$0")/../.."
exec dotnet run --project client --launch-profile http
