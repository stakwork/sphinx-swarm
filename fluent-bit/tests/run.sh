#!/usr/bin/env bash
# CI-runnable busted suite for fluent-bit/rate_limit.lua
#
# Install (not present in the repo CI image by default):
#   apt-get update && apt-get install -y lua5.1 luarocks
#   luarocks install busted
# Or with Lua 5.1 explicitly:
#   luarocks --lua-version=5.1 install busted
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v busted >/dev/null 2>&1; then
  echo "busted is not installed." >&2
  echo "Install:" >&2
  echo "  apt-get update && apt-get install -y lua5.1 luarocks" >&2
  echo "  luarocks install busted" >&2
  echo "Or:" >&2
  echo "  luarocks --lua-version=5.1 install busted" >&2
  exit 1
fi

busted tests/rate_limit_spec.lua
