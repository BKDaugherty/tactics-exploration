---
name: bevy-tactics
description: "Project-specific Bevy 0.17 skill for tactics-exploration. Enforces this repo's state-driven plugin architecture, message-based combat orchestration, and scheduling conventions."
---

## Purpose

Use this skill whenever modifying Bevy gameplay, UI, combat, input, state transitions, or systems in this repository.

Target project: `tactics-exploration`  
Engine: **Bevy 0.17.3**

This skill codifies how the maintainer structures Bevy code in this repo so changes remain idiomatic and low-risk.

---

## Project Architecture (Must Follow)

### 1) State-first flow
Global state is in `src/lib.rs` (`GameState`).
- Setup work belongs in `OnEnter(GameState::...)`
- Runtime behavior belongs in `Update` with `run_if(in_state(...))`
- Teardown belongs in `OnExit(GameState::...)`
- Transitions use `ResMut<NextState<GameState>>`

Do not introduce ad-hoc boolean gate resources when existing state/phase predicates can be used.

### 2) Plugin modularity
Main app composition is in `src/main.rs`. Feature logic is grouped into feature plugins:
- `join_game_plugin`
- `main_menu_plugin`
- `battle_plugin`

When adding a new feature, prefer adding/updating a plugin module over bloating `main.rs`.

### 3) Messages for cross-domain decoupling
This repo uses Bevy messages heavily in battle/combat.
- Define messages with `#[derive(Message)]`
- Register in plugin with `app.add_message::<T>()`
- Use `MessageReader<T>` / `MessageWriter<T>` in systems

Use messages when coupling would otherwise cross boundaries (combat↔animation, UI↔action dispatch, projectile↔damage, audio triggers, etc.).

### 4) Deterministic scheduling
Preserve ordering using:
- `.chain()` for strict sequential pipelines
- `.after(system)` for dependencies
- `run_if(...)` for phase/state constraints

Never rely on implicit scheduling for gameplay-critical logic.

### 5) Cleanup discipline
State-scoped entities should have marker components (e.g. `BattleEntity`) and be despawned in `OnExit`.
Avoid leaks across state transitions.

---

## Existing Maintainer Conventions

### Battle plugin organization (`src/battle.rs`)
The battle plugin is organized into conceptual groups:
1. Message registrations
2. Asset/plugin setup
3. `OnEnter(Battle)` setup pipeline
4. Update loops by domain:
   - stat derivation
   - phase progression
   - enemy phase start
   - movement/cursor/UI/combat
   - animation systems
   - combat timeline systems
   - enemy AI chain
   - projectile systems
   - interaction + audio resolver systems
5. `OnEnter(BattleResolution)` UI setup
6. `OnExit(BattleResolution)` cleanup

When adding systems, place them in the nearest existing group and preserve run conditions/order.

### Input
- Uses `leafwing-input-manager` at app/plugin level.
- Avoid bypassing existing input abstractions unless necessary.

### Assets and databases
- Asset loading and DB initialization happen early (`Startup` or `OnEnter`).
- Reuse existing resources (`AnimationDB`, `SpriteDB`, `TinytacticsAssets`, sound/font resources) instead of reloading ad hoc.

### UI
- Bevy UI entities are composed structurally; menu behavior is separated into navigation/interactions systems.
- Reuse menu/navigation primitives in `src/menu.rs` and battle menu modules.

---

## Bevy 0.17 Guidance for This Repo

1. Prefer `add_systems(Schedule, (...))` style already used throughout code.
2. Use modern message APIs (`add_message`, `MessageReader`, `MessageWriter`) consistent with current code.
3. Keep state predicates explicit with `run_if(in_state(...))`.
4. Use observers only where the project already benefits from event-style UI handling (e.g. pointer click handlers).

---

## How to Implement Changes Safely

1. Identify target state and plugin first.
2. Decide if communication should be direct query/resource or message-based.
3. Register new messages in the owning plugin.
4. Add systems with explicit ordering (`chain` / `after`) and state/phase guards.
5. Mark spawned state-bound entities for cleanup.
6. Run checks:
   - `cargo check`
   - `cargo test`
   - run game path if applicable (`cargo run`)

---

## Do / Don’t

### Do
- Follow existing module boundaries (combat, battle_phase, unit, menu, assets, animation).
- Keep systems small and single-purpose.
- Gate systems with state/phase predicates.
- Reuse existing resources and helper systems.

### Don’t
- Add global mutable resources for local flow control when messages/state suffice.
- Introduce unordered system dependencies for core battle logic.
- Spawn persistent entities during battle without cleanup strategy.
- Mix unrelated concerns into `main.rs`.

---

## File Map (High-value references)

- App bootstrap: `src/main.rs`
- Game state enum: `src/lib.rs`
- Battle orchestration: `src/battle.rs`
- Combat timeline + effects: `src/combat.rs`
- Phase logic: `src/battle_phase.rs`
- Menu/nav patterns: `src/menu.rs`, `src/battle_menu.rs`, `src/main_menu.rs`, `src/join_game_menu.rs`
- Grid/cursor: `src/grid.rs`, `src/grid_cursor.rs`
- Assets/sound/fonts: `src/assets.rs`
- Unit behavior/stats/actions: `src/unit.rs`

---

## PR/Change Checklist (Bevy-specific)

- [ ] System added in correct plugin and schedule
- [ ] Correct `GameState`/phase `run_if` guards
- [ ] Message type registered (if used)
- [ ] Ordering constraints (`chain` / `after`) specified
- [ ] State-scoped entities cleaned on exit
- [ ] No duplicate asset/resource initialization
- [ ] `cargo check` and relevant tests pass
