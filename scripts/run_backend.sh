#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# Ensure the backend uses a Python environment with the TTS dependencies installed.
# Priority order:
#   1. Explicit env vars (PYTHONHOME / PYO3_PYTHON)
#   2. ISHOWTTS_PYTHON_ENV (default .venv under repo)
#   3. System python fallback (let pyo3 locate python)

unset PYTHONHOME

if [[ -z "${PYO3_PYTHON:-}" ]]; then
  if [[ -n "${ISHOWTTS_PYTHON_ENV:-}" ]]; then
    CANDIDATE="$ISHOWTTS_PYTHON_ENV"
  elif [[ -d "/opt/miniforge3/envs/ishowtts" ]]; then
    CANDIDATE="/opt/miniforge3/envs/ishowtts"
  else
    CANDIDATE="$REPO_DIR/.venv"
  fi

  if [[ -d "$CANDIDATE" && -x "$CANDIDATE/bin/python" ]]; then
    export PYO3_PYTHON="$CANDIDATE/bin/python"
    export VIRTUAL_ENV="$CANDIDATE"
    export PYTHONHOME="$CANDIDATE"
    SITE_PACKAGES="$CANDIDATE/lib/python3.10/site-packages"
    if [[ -d "$SITE_PACKAGES" ]]; then
      if [[ -z "${PYTHONPATH:-}" ]]; then
        export PYTHONPATH="$SITE_PACKAGES"
      else
        export PYTHONPATH="$SITE_PACKAGES:$PYTHONPATH"
      fi
    fi
  fi
fi

# Allow the caller to layer additional PATH entries (e.g. cargo install bin dir).
export PATH="$HOME/.cargo/bin:$PATH"
export F5_TTS_QUIET=${F5_TTS_QUIET:-0}  # Temporarily set to 0 for debugging

# Fix protobuf compatibility issue on Jetson
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python

# Ensure OpenFST (for IndexTTS/pynini) is visible when the backend launches.
if [[ -z "${OPENFST_ROOT:-}" && -d /opt/openfst-1.8.3 ]]; then
  export OPENFST_ROOT=/opt/openfst-1.8.3
fi
if [[ -n "${OPENFST_ROOT:-}" ]]; then
  export PATH="$OPENFST_ROOT/bin:$PATH"
  export LD_LIBRARY_PATH="$OPENFST_ROOT/lib:${LD_LIBRARY_PATH:-}"
  export LIBRARY_PATH="$OPENFST_ROOT/lib:${LIBRARY_PATH:-}"
  export CPLUS_INCLUDE_PATH="$OPENFST_ROOT/include:${CPLUS_INCLUDE_PATH:-}"
  export C_INCLUDE_PATH="$OPENFST_ROOT/include:${C_INCLUDE_PATH:-}"
fi

echo "Building backend binary (release profile)..."
(cd "$REPO_DIR" && cargo build -p ishowtts-backend --release)

BIN="$REPO_DIR/target/release/ishowtts-backend"

if [[ ! -x "$BIN" ]]; then
  echo "error: backend binary '$BIN' missing after build" >&2
  exit 1
fi

echo "Starting iShowTTS backend..."
exec "$BIN" --config "$REPO_DIR/config/ishowtts.toml" --log-level "${ISHOWTTS_LOG_LEVEL:-info}" "$@"
