# Devlog

I like keep tracking of what I think I need to do in a list. For now, this tracks it!

## The TODO List

### Projectier Things
- [ ] Handle Gamepad Inputs
  - [ ] Create some form of screen for joining game etc

- [ ] Expand on the Combat System
  - [ ] Ranged Attacks?
  - [ ] Damage / AP Calculations?
  - [ ] Multiple Types of Moves?
  - [ ] Allow the unit to pick a FacedDirection on Wait
  - [x] UI Support for more complex choices of "actions"?

- [ ] Animation Data
  - [ ] Update derived animation data to populate expected texture atlas indices
  - [ ] Actually use derived animation data instead of hardcoding values

- [ ] Map Data / Battle Scene Data
  - [ ] The "load_demo_battle_scene" and "spawn_unit" fns are getting a lil out of control. 
  - [ ] Maps / Battles as Data?
    - [ ] Procedurally Generated Levels?

- [ ] Outside of Battle...

- [ ] Items and Interactables?

- [ ] The Progression?

- [ ] The Meta Progression?

- [ ] Save Games?
  - [ ] Load Games??
  
- [ ] Add the idea of Height to the Grid??!

- [ ] Camera Management?
  - [ ] If the levels stay small this might not be a big deal honestly

- [ ] Make the UI not look like shit

### Smaller, Bug kind of focus
- [ ] Ensure assets are loaded before moving to new scene
- [ ] Sometimes there are weird lines running through the fonts?
- [ ] Downed Characters should not be allowed to attack lol
- [ ] The UI should have "greyed out" options if they can't be taken
- [ ] Despawn the MainMenu (and anything else associated with the state) when we go into the BattleState
- [ ] If the Battle ends, but a player happens to have an open menu, the player can still move cursor on menu as it's an "ActiveMenu"
- [ ] Players can still do stuff with a unit after they've "waited"
- [ ] Players can still act after they've waited
- [ ] If you end the phase with a menu open (or maybe between messaging frames?) things can get pretty bad.

## Archive

- [x] A GridManager for tracking all of the "entities" I want
  - [x] I'll need a bevy_system for tracking entities
  - [x] And some startup system to add all the things from the setup
  - [x] I'd like to create a few integration tests here to help me get a feel for that too.

- [x] A way of going from a GridPosition to a WorldPosition for rendering

- [x] These iso_color things are great! Let's use them for showing attack and movement options
  - [x] Figure out how to adjust opacity of stuff

- [x] Use leafwing to support multiple players and abstract input

- [x] Build a simple demo of moving a few characters around!
  - [x] Create a pixel image for a cursor
  - [x] Create a movement system for the cursor (Use the leafwing_input library)
  - [x] Make the "spawn overlay" tie to the cursor!
  - [x] Consolidate bounds checking / valid grid stuff to the grid library
    - [x] Use this in the cursor movement code
  - [x] Use the select action to highlight a "Unit"
  - [x] Pathfinding and Unit Movement
    - [x] Move the unit movement code into it's own lil library
    - [x] Use pathfinding / search with "valid" movement tiles
    - [x] Only highlight "valid" tiles
    - [x] Make the unit movement more testable!
    - [x] Create an "obstacle"
    - [x] Create a "passable, but not landable tile"
    - [x] Ensure that the Transform mutation logic follows "valid" paths
  - [x] Basic Animation
    - [x] Write some code to load in an animated character
    - [x] Write some asset specific code for "tinytactics_battlekiti"
    - [x] Handle FacingDirections
      - [x] Do some research to see what people do here!
  - [x] Download some characters, and figure out how to render them
  - [x] Use the Movement system and tune the lerping constants
  - [x] Center the camera on the map
    - [x] I could naively just put it on the center but it'd be nice if we make bigger maps to have it move around.
  - [x] Get a background image
  - [x] Create a goal for someone to move to (like in a tutorial!)

- [x] Animations
  - [x] Write an animation controller for switching to the right animation state
    - [x] Might scale better if we use a MessageReader / Writer?
  - [x] Fix bug in Spritesheet creation that causes images to be flipped incorrectly


- [x] Make the game code easier to share / collaborate on (and clean it up!)
  - [x] remove any assets we aren't using
  - [x] Use Git LFS or something else for the assets you are actually using
  - [x] Add a README for how to setup the project
  - [x] Fix all compiler warnings
  - [x] Clippy + Rustfmt

- [x] Build a basic Menu screen for the demo
  - [x] Build a screen following example
  - [x] Customize the UI a bit
  - [x] Make the screen work with Gamepad / Keyboard

- [x] Do some UI Research into how to build bevy_uis for selecting characters / viewing stats
  - [x] Build a smol UI for looking at Units on the battlefield based on the current cursor position?
  - [x] Figure out a simple communication system between UI and movement code
    - [x] Ensure that cursor locks only when we choose a valid Unit
    - [x] Ensure that we only perform a move action when the player clicks Move
  - [x] Build a smol UI for attacking
  - [x] Build a smol UI for waiting

- [x] Create a Phased (for now) turn system
  - [x] Track Action Points
  - [x] Movement Points
  - [x] Create a component to manage a "Phase"
  - [x] Create a system to renew the phase

- [x] Platforms
  - [x] Get the demo running on the Steam Deck
  - [x] Get the demo running on the Web!

- [x] Finish "Gameifying" the Demo
  - [x] Victory or Defeat Calculations
  - [x] Advance out of BattleState
    - [x] Maybe with a "Play Again?" button?

- [x] Handle Z Index Correctly
  - [x] Do some maths to specify what the Z index of everything should be
  - [x] Spawn objects from map direclty probably instead of reading from tiled
