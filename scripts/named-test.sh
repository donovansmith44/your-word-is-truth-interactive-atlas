#!/usr/bin/env bash
set -euo pipefail
CARGO="${CARGO:-cargo}"

require() {
  local mode="$1" pkg="$2" bin="$3" filter="$4"
  local listed matches
  listed="$("$CARGO" test -p "$pkg" --test "$bin" -- --list 2>/dev/null | sed -nE 's/: test$//p')"
  case "$mode" in
    exact)  matches="$(printf '%s\n' "$listed" | grep -cFx -- "$filter" || true)" ;;
    prefix) matches="$(printf '%s\n' "$listed" | grep -cF -- "$filter" || true)" ;;
    *) echo "usage: $0 require exact|prefix <package> <test-binary> <name>" >&2; exit 2 ;;
  esac
  if [ "$matches" -eq 0 ]; then
    echo "REFUSED: the $mode filter '$filter' matches zero tests in $pkg::$bin" >&2
    exit 1
  fi
}

case "${1:-}" in
  require) [ "$#" -eq 5 ] || { echo "usage: $0 require exact|prefix <package> <test-binary> <name>" >&2; exit 2; }
           require "$2" "$3" "$4" "$5" ;;
  *) echo "usage: $0 require exact|prefix <package> <test-binary> <name>" >&2; exit 2 ;;
esac
