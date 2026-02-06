#!/usr/bin/env bash
# Stop the play-test HTTP server if running.
set -euo pipefail

if [ -f /tmp/tactics-play-test.pid ]; then
    PID=$(cat /tmp/tactics-play-test.pid)
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "Stopped play-test server (PID $PID)"
    else
        echo "Server (PID $PID) was not running"
    fi
    rm -f /tmp/tactics-play-test.pid
else
    echo "No play-test server PID file found"
fi
