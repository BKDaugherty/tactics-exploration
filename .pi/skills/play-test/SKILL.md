---
name: play-test
description: "Build the tactics-exploration game for WebAssembly, serve it locally, and play-test it in a Chrome browser to verify gameplay mechanics are working. Use after code changes to validate main menu, character creation, battle combat, and battle resolution flows."
---

# Play-Test Skill

Build the game for WASM, launch it in Chrome, and interactively verify gameplay mechanics work end-to-end.

## Prerequisites

- **web-browser** skill for Chrome automation (screenshot, eval, logs)
- `wasm-bindgen` CLI, Rust target `wasm32-unknown-unknown`, Python 3

## Fast Path — Get to Battle

Build, launch, and skip straight to the battle screen in one go. All state transitions use event-driven log waits — no blind sleeps.

```bash
# 1. Build and serve
.pi/skills/play-test/scripts/build-and-serve.sh

# 2. Launch Chrome and navigate
WEB=~/.pi/agent/git/github.com/mitsuhiko/agent-stuff/skills/web-browser/scripts
$WEB/start.js
$WEB/nav.js http://127.0.0.1:8843

# 3. Wait for game to fully load (WASM init + asset loading)
WAIT=".pi/skills/play-test/scripts/wait-for-log.js"
$WAIT "[GAME_STATE] entered MainMenu" --timeout 15000

# 4. Skip menus → battle (creates an Archer named "Hero")
# Uses log markers like [GAME_STATE] entered JoinGame/Battle for deterministic waits
.pi/skills/play-test/scripts/skip-to-battle.sh
# Optional: pass a custom name
# .pi/skills/play-test/scripts/skip-to-battle.sh Gandalf

# 5. Verify you're in battle
$WEB/screenshot.js    # Read the returned file to confirm battle loaded
```

You should see an isometric tactical grid with units, "Objective: Defeat all Enemies", and the battle menu (Move/Skills/Wait/View Map).

**If the screenshot doesn't show battle**, something went wrong in the menu flow. Use the manual steps in "Detailed Menu Navigation" below to diagnose.

## Quick Reference — Input Helper

All game interaction goes through `./scripts/input.js` which sends real browser-level keyboard/mouse events via CDP. The script **automatically focuses the canvas** on every call.

```bash
INPUT=".pi/skills/play-test/scripts/input.js"

# Single key press
$INPUT key Space          # Select / confirm
$INPUT key ShiftLeft      # Deselect / back
$INPUT key KeyW           # Grid y-1 (up-left on screen)
$INPUT key KeyS           # Grid y+1 (down-right on screen)
$INPUT key KeyA           # Grid x-1 (down-left on screen)
$INPUT key KeyD           # Grid x+1 (up-right on screen)
$INPUT key KeyQ           # Zoom in
$INPUT key KeyE           # Zoom out
$INPUT key KeyJ           # Join game (JoinGame screen only)

# Multiple keys in sequence (150ms gap)
$INPUT keys KeyS KeyS Space

# Click at viewport coordinates
$INPUT click 960 540

# Type text (for name entry fields)
$INPUT type Hero
```

## Quick Reference — Browser Tools

```bash
WEB="~/.pi/agent/git/github.com/mitsuhiko/agent-stuff/skills/web-browser/scripts"
WAIT=".pi/skills/play-test/scripts/wait-for-log.js"

$WEB/start.js              # Launch Chrome
$WEB/nav.js <url>          # Navigate to URL
$WEB/screenshot.js         # Take screenshot (returns path, read it to see the image)
$WEB/logs-tail.js          # Dump browser console logs
$WEB/eval.js '<js>'        # Run JS in page
```

### Event-Driven Waits

**All** state transitions and action completions use `wait-for-log.js` which listens on CDP for log markers in real time. Never use blind sleeps for state waits.

```bash
# Game state transitions
$WAIT "[GAME_STATE] entered MainMenu" --timeout 15000
$WAIT "[GAME_STATE] entered JoinGame" --timeout 5000
$WAIT "[GAME_STATE] entered Battle" --timeout 15000
$WAIT "[GAME_STATE] entered BattleResolution" --timeout 30000

# Unit positions at battle start (emitted once when battle loads)
$WAIT "[UNIT_POS] team=player" --timeout 5000   # e.g. [UNIT_POS] team=player name=" hero" x=1 y=5
$WAIT "[UNIT_POS] team=enemy" --timeout 5000     # e.g. [UNIT_POS] team=enemy name="Jimothy Timbers" x=7 y=3

# Battle phase transitions
$WAIT "Advancing To Next Phase: Player" --timeout 20000
$WAIT "Advancing To Next Phase: Enemy" --timeout 10000

# Unit action completion (movement or attack animation finished)
$WAIT "Unit Action Completed" --timeout 10000

# Cursor movement — always use SPECIFIC expected coordinates
$WAIT "[CURSOR_POS] player=1 x=3 y=5" --timeout 3000
```

### Cursor Position Logging

During battle, the game emits `[CURSOR_POS] player=<id> x=<col> y=<row>` every time the cursor moves to a new grid tile.

**Important:** Always wait for the **specific expected coordinates**, not a bare `[CURSOR_POS]` pattern. When the cursor is unlocked (e.g. selecting Move or a Skill), an initialization event fires at the current position *before* your keypress takes effect. A bare pattern will match that stale event instead of the movement you intended. Example:

```bash
# BAD — matches the unlock event, not your D-press
$WAIT "[CURSOR_POS]" --timeout 3000

# GOOD — only matches when cursor actually reaches (3,5)
$WAIT "[CURSOR_POS] player=1 x=3 y=5" --timeout 3000
```

### Isometric Cursor Mapping

The game uses a diamond isometric projection. WASD keys map to **grid** coordinates, which appear diagonal on screen:

```
        W (y-1)
        ↗
A (x-1) ←  → D (x+1)
        ↘
        S (y+1)
```

| Key | Grid effect | Screen direction |
|-----|------------|-----------------|
| **W** | y - 1 | up-left |
| **S** | y + 1 | down-right |
| **A** | x - 1 | down-left |
| **D** | x + 1 | up-right |

To navigate from hero at (hx, hy) to enemy at (ex, ey):
- Press **D** `(ex - hx)` times if ex > hx, or **A** `(hx - ex)` times if hx > ex
- Press **W** `(hy - ey)` times if hy > ey, or **S** `(ey - hy)` times if ey > hy

Example: hero (1,5) → enemy (7,3): press D×6 then W×2.

## Battle Interaction

Once in battle, here's how to control the game. All waits are event-driven using log markers.

### Moving a unit

```bash
$INPUT key Space          # Select "Move" (highlighted by default)
sleep 0.3                 # Wait for cursor unlock event to flush
# Move cursor to destination — use specific coordinates
$INPUT key KeyD
$WAIT "[CURSOR_POS] player=1 x=2 y=5" --timeout 3000
$INPUT key KeyD
$WAIT "[CURSOR_POS] player=1 x=3 y=5" --timeout 3000
$INPUT key Space          # Confirm movement
$WAIT "Unit Action Completed" --timeout 10000
```

### Attacking with a skill

**Important: Always screenshot before selecting a skill!** The skill sub-menu order (e.g. Poison Shot vs Stun Shot) is **not fixed** and may vary between runs. Take a screenshot to visually confirm which skill is highlighted before pressing Space.

**Important: Range check!** Skills have a `TargetInRange(N)` that limits how far they can reach from the unit's current position. If the enemy is outside this range, you **must move the unit closer first** using the Move command, then use the skill.

General flow:
1. Check enemy distance from your unit (use `[CURSOR_POS]` log markers to verify positions)
2. If out of range → Move closer first, then re-open battle menu to attack
3. If in range → Use skill directly

```bash
$INPUT key KeyS           # Navigate to "Skills"
$INPUT key Space          # Open skill categories
$INPUT key Space          # Select first category (e.g. "Attack")

# SCREENSHOT to verify which skill is highlighted before selecting!
$WEB/screenshot.js

$INPUT key Space          # Select the skill (after verifying it's the right one)
sleep 0.3                 # Wait for cursor unlock event to flush

# Navigate cursor to enemy position using specific coordinates
# Calculate direction from UNIT_POS logs: e.g. hero (1,5) → enemy (3,5) = D×2
$INPUT key KeyD
$WAIT "[CURSOR_POS] player=1 x=2 y=5" --timeout 3000
$INPUT key KeyD
$WAIT "[CURSOR_POS] player=1 x=3 y=5" --timeout 3000

# Verify cursor is on the enemy before confirming
$WEB/screenshot.js

$INPUT key Space          # Confirm attack on target
$WAIT "Unit Action Completed" --timeout 10000
```

### Waiting (end unit's turn)

```bash
$INPUT keys KeyS KeyS     # Navigate to "Wait" (Move > Skills > Wait)
$INPUT key Space          # Confirm wait
# Wait for enemy phase to complete and player phase to begin
$WAIT "Advancing To Next Phase: Player" --timeout 20000
```

### Enemy Phase

After all player units act, the enemy phase runs automatically. Wait for the phase transition log:

```bash
# Wait for enemy phase to end and player phase to begin
$WAIT "Advancing To Next Phase: Player" --timeout 20000
```

Then a new player phase starts. AP and Move reset. HP may have decreased.

### Battle Resolution

When all enemies or all player units are downed, a resolution screen shows **Victory** or **Defeat** with: **Main Menu**, **Quit**.

```bash
$WAIT "[GAME_STATE] entered BattleResolution" --timeout 30000
```

## Verification Checklist

When play-testing after code changes, verify at minimum:

- [ ] Battle loads (isometric tilemap, units, objective, stats bar)
- [ ] Player can select a unit and see the battle menu (Move/Skills/Wait/View Map)
- [ ] Player can move a unit (movement tiles shown, unit moves, Move points decrease)
- [ ] Player can attack an enemy (targeting tiles, combat animation, damage numbers, AP consumed)
- [ ] Enemy phase runs automatically (enemy moves/attacks, player HP changes)
- [ ] Turn cycling works (AP/Move reset each new player phase)
- [ ] Battle resolution screen appears when combat ends (Victory/Defeat)

## Detailed Menu Navigation

If the fast path fails, use these steps with screenshots between each action to find where things broke.

### Main Menu

Game starts with "Couch Tactics" — **Play Demo** (highlighted), Settings, Quit. Navigate with W/S, confirm with Space.

```bash
$INPUT key Space      # Select "Play Demo"
$WAIT "[GAME_STATE] entered JoinGame" --timeout 5000
$WEB/screenshot.js    # Verify JoinGame screen
```

### Join Game

Shows "Press J or LB and RB together to join the game":

```bash
$INPUT key KeyJ       # Join as keyboard player
sleep 0.5             # UI panel animation (no state change to wait on)
$WEB/screenshot.js    # Verify player panel (New Character / Load Character / DEV Delete)
```

### Create a Character

```bash
$INPUT key Space          # Open "New Character" (highlighted by default)
sleep 0.3                 # UI transition

$INPUT key Space          # Focus name text input
sleep 0.2                 # Input focus
$INPUT type Hero          # Type name
$INPUT key Space          # Unfocus text input
sleep 0.2                 # Input unfocus

$INPUT key KeyS           # → Job selector (D/A to cycle: Archer/Knight/Mercenary/Mage)
sleep 0.2                 # Input timing for just_pressed
$INPUT key KeyS           # → Color selector (D/A to cycle: Red/Blue/Green)
sleep 0.2                 # Input timing
$INPUT key KeyS           # → "Create Character" button
sleep 0.2                 # Input timing
$INPUT key Space          # Confirm creation
sleep 0.3                 # Character creation processing
$WEB/screenshot.js        # Verify unit preview
```

Note: The small `sleep` calls (0.2–0.3s) here are **input timing** for leafwing-input-manager's `just_pressed` frame detection, not state waits. They ensure consecutive key presses register as separate events.

### Ready Up

```bash
$INPUT key Space          # Press Ready
$WAIT "[GAME_STATE] entered Battle" --timeout 15000
$WEB/screenshot.js        # Verify battle loaded
```

## Cleanup

```bash
.pi/skills/play-test/scripts/stop-server.sh
```

## Troubleshooting

- **Build fails**: Run `cargo check` first to identify compilation errors
- **Blank screen**: Check `$WEB/logs-tail.js` for WASM errors
- **Input not registering**: `input.js` auto-focuses the canvas, but if Chrome loses focus entirely, click on the Chrome window first
- **Menu not responding to keys**: `leafwing-input-manager` uses `just_pressed` — add `sleep 0.2` between key presses for input timing
- **Text input capturing keys**: When the name input is focused, letter keys go to the text field. Press Space to toggle focus on/off
- **Audio not playing**: Expected — browsers require user interaction. Handled by `fix-audio.js` after first click
- **Port conflict**: `build-and-serve.sh` auto-kills previous servers; if issues persist: `lsof -ti:8843 | xargs kill`
- **404 on .meta files**: Normal Bevy asset server behavior — ignore them
