#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
BACKEND_SCRIPT="$REPO_DIR/scripts/run_backend.sh"
FRONTEND_DIR="$REPO_DIR/crates/frontend-web"
TRUNK_BIN=${TRUNK_BIN:-$(command -v trunk || true)}
RUSTUP_BIN=${RUSTUP_BIN:-$(command -v rustup || true)}
BACKEND_LOG=${BACKEND_LOG:-$REPO_DIR/logs/backend.log}
FRONTEND_LOG=${FRONTEND_LOG:-$REPO_DIR/logs/frontend.log}
HEALTH_URL=${ISHOWTTS_HEALTH_URL:-http://127.0.0.1:27121/api/health}
FRONTEND_PORT=${ISHOWTTS_FRONTEND_PORT:-8080}
WAIT_SECONDS=${ISHOWTTS_WAIT_SECONDS:-600}
TAIL_LOGS=1

TRUNK_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-tail|--quiet)
      TAIL_LOGS=0
      shift
      ;;
    --wait)
      if [[ -n "${2:-}" ]]; then
        WAIT_SECONDS="$2"
        shift 2
      else
        echo "error: --wait requires an integer argument" >&2
        exit 1
      fi
      ;;
    --wait=*)
      WAIT_SECONDS="${1#*=}"
      shift
      ;;
    *)
      TRUNK_ARGS+=("$1")
      shift
      ;;
  esac
done

set -- "${TRUNK_ARGS[@]}"

mkdir -p "$REPO_DIR/logs"

if [[ ! -x "$BACKEND_SCRIPT" ]]; then
  echo "error: backend launcher '$BACKEND_SCRIPT' not found or not executable" >&2
  exit 1
fi

if [[ ! -d "$FRONTEND_DIR" ]]; then
  echo "error: frontend crate directory '$FRONTEND_DIR' is missing" >&2
  exit 1
fi

if [[ -z "$TRUNK_BIN" ]]; then
  echo "warning: trunk not found; start_all.sh will skip frontend." >&2
fi

cleanup() {
  local code=$?
  if [[ -n "${FRONTEND_PID:-}" ]] && kill -0 "$FRONTEND_PID" 2>/dev/null; then
    kill "$FRONTEND_PID" 2>/dev/null || true
    wait "$FRONTEND_PID" 2>/dev/null || true
  fi
  if [[ -n "${BACKEND_PID:-}" ]] && kill -0 "$BACKEND_PID" 2>/dev/null; then
    kill "$BACKEND_PID" 2>/dev/null || true
    wait "$BACKEND_PID" 2>/dev/null || true
  fi
  exit $code
}
trap cleanup EXIT INT TERM

: > "$BACKEND_LOG"
: > "$FRONTEND_LOG"

# Launch backend
(
  cd "$REPO_DIR"
  "$BACKEND_SCRIPT"
) >>"$BACKEND_LOG" 2>&1 &
BACKEND_PID=$!

echo "backend starting (pid $BACKEND_PID), logs -> $BACKEND_LOG"

# Wait for backend health
echo -n "waiting for backend"
ready=false
for ((i = 0; i < WAIT_SECONDS; i++)); do
  if curl -fs --max-time 2 "$HEALTH_URL" >/dev/null 2>&1; then
    ready=true
    break
  fi
  sleep 1
  if (( i % 5 == 4 )); then
    echo -n "."
  fi
done
echo
if [[ "$ready" != true ]]; then
  echo "error: backend not ready after ${WAIT_SECONDS}s; check $BACKEND_LOG" >&2
  exit 1
fi
echo "backend ready -> $HEALTH_URL"

# Launch frontend (Trunk serve)
if [[ -n "$TRUNK_BIN" ]]; then
(
  cd "$FRONTEND_DIR"
  "$TRUNK_BIN" serve --port "$FRONTEND_PORT" "$@"
) >>"$FRONTEND_LOG" 2>&1 &
FRONTEND_PID=$!

echo "frontend starting (pid $FRONTEND_PID), logs -> $FRONTEND_LOG"
else
  FRONTEND_PID=""
  echo "frontend skipped (trunk unavailable)"
fi

echo "---"
echo "Backend: $HEALTH_URL"
echo "Frontend: http://127.0.0.1:$FRONTEND_PORT"
echo "Press Ctrl+C to stop both processes."

if [[ "$TAIL_LOGS" -eq 1 ]]; then
  if [[ -n "$FRONTEND_PID" ]]; then
    tail --follow=name --retry --lines=0 --quiet "$BACKEND_LOG" "$FRONTEND_LOG" &
  else
    tail --follow=name --retry --lines=0 --quiet "$BACKEND_LOG" &
  fi
  TAIL_PID=$!
  wait $BACKEND_PID
  kill "$TAIL_PID" 2>/dev/null || true
  wait "$TAIL_PID" 2>/dev/null || true
else
  wait $BACKEND_PID
fi
