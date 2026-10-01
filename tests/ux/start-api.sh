#!/bin/sh
set -e
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
cd "$(dirname "$0")/../../server"
exec cargo run --release -p atlas-server -- --data-dir ../data/compiled --port 8000
