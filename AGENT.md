# AGENT.md

## Scope
This file applies to the entire repository.

## Project
- Engine: **Bevy** (Rust), version `0.17.x`
- Main gameplay architecture is state-driven and plugin-based.

## Required skill usage
Before proposing or implementing code changes, agents **must load and follow**:
- `.pi/skills/bevy-tactics/SKILL.md`

Before testing code changes, agents **must load and follow**:
- `.pi/skills/play-test/SKILL.md`

If the skill is unavailable in-session, read it directly from disk and follow it anyway.

## Code change expectations
- Keep changes aligned with existing Bevy patterns in this repo:
  - `GameState`-driven flow (`OnEnter`, `Update` + `run_if(in_state(...))`, `OnExit`)
  - Feature plugins (`main_menu`, `join_game`, `battle`) over monolithic wiring
  - Message-based decoupling for cross-domain gameplay/UI/combat interactions
  - Explicit scheduling (`chain`, `after`, state/phase run conditions)
- Prefer minimal, localized edits over broad refactors.
- Reuse existing resources, systems, and module boundaries.

## Validation
For meaningful code changes, run:
- `cargo check`
- `cargo test` (or targeted tests when appropriate)
- `play-test` skill and verify expected behavior in game

If runtime behavior is affected, include a short manual verification note.

## References
- App entry: `src/main.rs`
- State enum: `src/lib.rs`
- Battle orchestration: `src/battle.rs`
