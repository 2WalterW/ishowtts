#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# Build backend (release)
(cd "$REPO_DIR" && cargo build -p ishowtts-backend --release)

# Build frontend (wasm dist)
if command -v trunk >/dev/null 2>&1; then
  (cd "$REPO_DIR/crates/frontend-web" && trunk build --release)
else
  echo "warning: trunk not found; skipping frontend build" >&2
fi
