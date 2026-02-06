#!/usr/bin/env bash
# Build the game for WASM and serve it locally for play-testing.
# Usage: ./build-and-serve.sh [--port PORT]
#
# Outputs the URL to stdout on success.
# The HTTP server runs in the background; its PID is written to /tmp/tactics-play-test.pid

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
PORT="${1:-8843}"

cd "$PROJECT_DIR"

# --- Build WASM artifacts ---
echo "::build:: Packaging assets..." >&2
mkdir -p out
cargo run --bin package_assets 2>&1 | tail -5 >&2

echo "::build:: Building WASM target..." >&2
cargo build --bin tactics-exploration --profile release-wasm --target wasm32-unknown-unknown 2>&1 | tail -5 >&2

echo "::build:: Running wasm-bindgen..." >&2
wasm-bindgen --no-typescript --target web \
    --out-dir ./out/ \
    --out-name "tactics-exploration" \
    ./target/wasm32-unknown-unknown/release-wasm/tactics-exploration.wasm 2>&1 >&2

# Patch audio fix into JS
cp web/index.html ./out/index.html
cat web/fix-audio.js ./out/tactics-exploration.js > ./out/tactics-exploration-audio-fix.js
mv ./out/tactics-exploration-audio-fix.js ./out/tactics-exploration.js

# --- Kill any previous server on this port ---
if [ -f /tmp/tactics-play-test.pid ]; then
    OLD_PID=$(cat /tmp/tactics-play-test.pid)
    kill "$OLD_PID" 2>/dev/null || true
    rm -f /tmp/tactics-play-test.pid
fi
# Also kill anything else on the port
lsof -ti:"$PORT" | xargs kill 2>/dev/null || true
sleep 0.5

# --- Start local HTTP server ---
echo "::serve:: Starting HTTP server on port $PORT..." >&2
cd out
python3 -m http.server "$PORT" --bind 127.0.0.1 >/dev/null 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > /tmp/tactics-play-test.pid

# Wait a moment and verify it started
sleep 1
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "ERROR: HTTP server failed to start" >&2
    exit 1
fi

echo "http://127.0.0.1:$PORT"
