#!/usr/bin/env bash
# Optimistically navigate from game start through menus to the battle screen.
# Assumes Chrome is already running and the game is loaded at the main menu.
#
# Usage: ./skip-to-battle.sh [character-name]
#
# This sends inputs without stopping to screenshot/verify, so it's fast.
# Take a screenshot AFTER this script completes to confirm you're in battle.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INPUT="$SCRIPT_DIR/input.js"
WAIT="$SCRIPT_DIR/wait-for-log.js"
NAME="${1:-Hero}"

echo ">> Skipping to battle (character: $NAME)..." >&2

# Main Menu: "Play Demo" is highlighted by default — confirm it
$INPUT key Space 2>&1 | tail -1
$WAIT "[GAME_STATE] entered JoinGame" --timeout 5000

# Join Game: press J to join as keyboard player
$INPUT key KeyJ 2>&1 | tail -1
sleep 0.5

# Player panel: "New Character" is highlighted — confirm it
$INPUT key Space 2>&1 | tail -1
sleep 0.5

# Character creation screen:
# 1. Name input is highlighted — toggle focus on, type name, toggle focus off
$INPUT key Space 2>&1 | tail -1
sleep 0.3
$INPUT type "$NAME" 2>&1 | tail -1
sleep 0.1
$INPUT key Space 2>&1 | tail -1
sleep 0.3

# 2. Navigate down past: job selector, color selector, to "Create Character"
$INPUT key KeyS 2>&1 | tail -1
sleep 0.2
$INPUT key KeyS 2>&1 | tail -1
sleep 0.2
$INPUT key KeyS 2>&1 | tail -1
sleep 0.2

# 3. Confirm "Create Character"
$INPUT key Space 2>&1 | tail -1
sleep 0.5

# Unit preview: "Ready!" button is the only option — confirm it
$INPUT key Space 2>&1 | tail -1

# Wait for battle scene to load (map generation, assets, phase init)
echo ">> Waiting for battle to load..." >&2
$WAIT "[GAME_STATE] entered Battle" --timeout 15000

echo ">> Done — should be in battle now. Take a screenshot to verify." >&2
