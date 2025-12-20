# Devlog

I like keep tracking of what I think I need to do in a flat list. For now, this tracks it!

## The TODO List

- [x] A GridManager for tracking all of the "entities" I want
  - [x] I'll need a bevy_system for tracking entities
  - [x] And some startup system to add all the things from the setup
  - [x] I'd like to create a few integration tests here to help me get a feel for that too.

- [x] A way of going from a GridPosition to a WorldPosition for rendering
  - [ ] Need to figure out how to deal with Z values w.r.t bevy_ecs_tiled

- [ ] These iso_color things are great! Let's use them for showing attack and movement options
  - [x] Figure out how to adjust opacity of stuff
  - [ ] Would be fun to make the squares a little smaller than the tiles they sit above.

- [x] Use leafwing to support multiple players and abstract input

- [ ] Build a simple demo of moving a few characters around!
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
  - [ ] Get a background image
  - [ ] Create a goal for someone to move to (like in a tutorial!)

- [ ] Handle Gamepad Inputs

- [ ] Animations
  - [ ] Write an animation controller for switching to the right animation state
    - [ ] Might scale better if we use a MessageReader / Writer?
  - [ ] Update derived animation data to populate expected texture atlas indices
  - [ ] Actually use derived animation data instead of hardcoding values

- [x] Make the game code easier to share / collaborate on (and clean it up!)
  - [x] remove any assets we aren't using
  - [x] Use Git LFS or something else for the assets you are actually using
  - [x] Add a README for how to setup the project
  - [x] Fix all compiler warnings
  - [x] Clippy + Rustfmt

- [ ] Build a basic Menu screen for the demo
  - [x] Build a screen following example
  - [ ] Customize the UI a bit
  - [ ] Make the screen work with Gamepad / Keyboard

- [ ] Do some UI Research into how to build bevy_uis for selecting characters / viewing stats
  - [ ] Build a smol UI for looking at Units on the battlefield based on the current cursor position?
  - [ ] Build a smol UI for attacking
  - [ ] Build a smol UI for waiting (and choosing a faced direction)

- [ ] Have a think through the Unit types and what should be on them
  - [ ] Create a combat system
  - [ ] Create a skill system
  - [ ] Create an inventory system

- [ ] Do some further research into Resource / Game State Tracking
  - [ ] Ensure assets are loaded before moving to scene
    - [ ] Learn basic Bevy UIs

- [ ] Create some silly game music, and play it in the different states
  - [ ] Create some silly music to play in different things

- [ ] Get the demo running on the Steam Deck

- [ ] Code organization
  - [x] Create plugins for systems that are associated with eachother?
  - [ ] Pull all of the Battle stuff into it's own module?

- [ ] Pathfinding and Unit Movement in Multiplayer
  - [ ] If two units are moving at the same time, how do I ensure they can't move to the same spot? When should I do the movement calculation? How can I refresh / lock?



- [ ] Create a camera manager that balances where to focus based on player movement

- [ ] Another demo!

- [ ] "Map" Data structures
  - [ ] At the moment, map data is pretty hardcoded. It'd be great to have a data representation for this.
  - [ ] Experiment with using Tiled as a way of pushing data into the maps themselves? Or just add some JSON / RON alongside some maps maybe for an equivalent without needing to learn too much.

- [ ] How would savegames work? What data does one save? 
