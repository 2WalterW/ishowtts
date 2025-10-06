#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
VLLM_DIR="$REPO_DIR/third_party/index-tts-vllm"
REPO_URL=${ISHOWTTS_VLLM_REPO:-"https://github.com/2WalterW/index-tts-vllm.git"}
PORT=${ISHOWTTS_VLLM_PORT:-6006}
MODEL=${ISHOWTTS_VLLM_MODEL:-"IndexTeam/IndexTTS-1.5"}
GPU_MEMORY=${ISHOWTTS_VLLM_GPU_MEMORY_UTILIZATION:-0.25}
MODEL_DIR=${ISHOWTTS_VLLM_MODEL_DIR:-"checkpoints"}
DOWNLOAD_MODEL=${ISHOWTTS_VLLM_DOWNLOAD:-1}
CONVERT_MODEL=${ISHOWTTS_VLLM_CONVERT:-1}
ENV_FILE="$VLLM_DIR/.env"

if ! command -v git >/dev/null 2>&1; then
  echo "error: git is required to fetch index-tts-vllm" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is required for index-tts-vllm" >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "error: docker compose plugin is required" >&2
  exit 1
fi

mkdir -p "$REPO_DIR/third_party"
if [[ ! -d "$VLLM_DIR" ]]; then
  echo "cloning index-tts-vllm into $VLLM_DIR"
  git clone "$REPO_URL" "$VLLM_DIR"
else
  echo "updating index-tts-vllm in $VLLM_DIR"
  git -C "$VLLM_DIR" pull --ff-only || true
fi

cat >"$ENV_FILE" <<EOV
MODEL=$MODEL
VLLM_USE_MODELSCOPE=1
DOWNLOAD_MODEL=$DOWNLOAD_MODEL
CONVERT_MODEL=$CONVERT_MODEL
MODEL_DIR=$MODEL_DIR
PORT=$PORT
GPU_MEMORY_UTILIZATION=$GPU_MEMORY
EOV

cleanup() {
  docker compose down --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

cd "$VLLM_DIR"

# shellcheck disable=SC2068
exec docker compose up --build --pull missing $@
