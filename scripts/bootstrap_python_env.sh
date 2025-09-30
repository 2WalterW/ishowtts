#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PY_ENV_DIR=${ISHOWTTS_PYTHON_ENV:-"$REPO_DIR/.venv"}
PYTHON_BIN=${PYTHON_BIN:-python3}
EXTRA_REQUIREMENTS=${ISHOWTTS_EXTRA_REQUIREMENTS:-}

if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
  echo "error: python executable '$PYTHON_BIN' not found. Set PYTHON_BIN or install Python 3.10+." >&2
  exit 1
fi

if [[ ! -d "$PY_ENV_DIR" ]]; then
  echo "Creating virtual environment at $PY_ENV_DIR"
  "$PYTHON_BIN" -m venv "$PY_ENV_DIR"
fi

if [[ ! -x "$PY_ENV_DIR/bin/python" ]]; then
  echo "error: virtual environment missing python executable at $PY_ENV_DIR/bin/python" >&2
  exit 1
fi

source "$PY_ENV_DIR/bin/activate"

python -m pip install --upgrade pip wheel setuptools

# Install F5-TTS in editable mode to reuse its dependencies.
if [[ -d "$REPO_DIR/third_party/F5-TTS" ]]; then
  echo "Installing F5-TTS dependencies"
python -m pip install -e "$REPO_DIR/third_party/F5-TTS"
else
  echo "warning: F5-TTS directory not found; skipping installation" >&2
fi

# Ensure shared audio dependencies are present even if upstream requirements omit them.
python -m pip install soundfile

# Optional extra requirements list (space separated) can be provided via env.
if [[ -n "$EXTRA_REQUIREMENTS" ]]; then
  echo "Installing additional packages: $EXTRA_REQUIREMENTS"
  python -m pip install $EXTRA_REQUIREMENTS
fi

cat <<MSG
Python environment ready -> $PY_ENV_DIR
  Activate with: source "$PY_ENV_DIR/bin/activate"
  Backend will auto-detect it via scripts/run_backend.sh
MSG
