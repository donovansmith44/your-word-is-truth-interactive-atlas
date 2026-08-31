#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Support the user-local installs produced by rustup and dotnet-install even
# when a fresh shell has not added them to PATH yet.
export PATH="$HOME/.cargo/bin:$HOME/.dotnet:$PATH"
if [[ -x "$HOME/.dotnet/dotnet" ]]; then
  export DOTNET_ROOT="$HOME/.dotnet"
fi

for command_name in cargo dotnet curl; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    echo "See README.md's Prerequisites section, then rerun scripts/dev.sh." >&2
    exit 1
  fi
done

dotnet_version="$(dotnet --version)"
if [[ "$dotnet_version" != 10.* ]]; then
  echo "This project requires .NET 10; found $dotnet_version." >&2
  exit 1
fi

required_resources=(
  "client/wwwroot/vendor/leaflet/leaflet.js"
  "client/wwwroot/vendor/leaflet/leaflet.css"
  "data/compiled/canon.json"
  "data/compiled/land-mask.json"
  "data/compiled/catechism.json"
  # M-C2 retired places.json/events.json (and narratives/eras/verses-kjv/
  # cross-refs) into the one serialized graph artifact below.
  "data/compiled/graph.bin"
)

for relative_path in "${required_resources[@]}"; do
  if [[ ! -s "$repo_dir/$relative_path" ]]; then
    echo "Missing runtime resource: $relative_path" >&2
    echo "Run 'git pull --ff-only' to restore committed runtime resources." >&2
    exit 1
  fi
done

for port in 8000 5000; do
  if command -v lsof >/dev/null 2>&1 && lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "Port $port is already in use. Stop its owning process and rerun scripts/dev.sh." >&2
    lsof -nP -iTCP:"$port" -sTCP:LISTEN >&2
    exit 1
  fi
done

api_pid=""
cleanup() {
  if [[ -n "$api_pid" ]] && kill -0 "$api_pid" 2>/dev/null; then
    kill "$api_pid" 2>/dev/null || true
    wait "$api_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

echo "Starting API at http://localhost:8000"
(
  cd "$repo_dir/server"
  exec cargo run -p atlas-server -- --data-dir ../data/compiled --port 8000
) &
api_pid=$!

for _ in {1..120}; do
  if ! kill -0 "$api_pid" 2>/dev/null; then
    wait "$api_pid"
    exit 1
  fi
  if curl --noproxy '*' --fail --silent http://127.0.0.1:8000/health >/dev/null; then
    break
  fi
  sleep 0.25
done

if ! curl --noproxy '*' --fail --silent http://127.0.0.1:8000/health >/dev/null; then
  echo "API did not become healthy within 30 seconds." >&2
  exit 1
fi

echo "Starting client at http://localhost:5000"
cd "$repo_dir"
dotnet run --project client --launch-profile http
