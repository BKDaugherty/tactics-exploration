TODO!

[x] A GridManager for tracking all of the "entities" I want
  [x] I'll need a bevy_system for tracking entities
  [x] And some startup system to add all the things from the setup
  [x] I'd like to create a few integration tests here to help me get a feel for that too.
[x] A way of going from a GridPosition to a WorldPosition for rendering
    [ ] Need to figure out how to deal with Z values w.r.t bevy_ecs_tiled
[ ] These iso_color things are great! Let's use them for showing attack and movement options
  [x] Figure out how to adjust opacity of stuff
  [ ] Would be fun to make the squares a little smaller than the tiles they sit above.
[x] Use leafwing to support multiple players and abstract input
[ ] Build a simple demo of moving a few characters around!
  [ ] Create a pixel image for a cursor
  [x] Create a movement system for the cursor (Use the leafwing_input library)
  [ ] Consolidate bounds checking / valid grid stuff to the grid library
    [ ] Use this in the cursor movement code
  [ ] Use the select action to highlight a "Unit"
  [ ] Find a way to highlight valid "Ground" tiles for movement (assume constant movement for now)
  [ ] Download some characters, and figure out how to render them
  [ ] Use the Movement system and tune the lerping constants
  [ ] Implement a pathfinding algorithm based on valid movement indices nearby
  [ ] Implement a simple camera manager that moves the camera around, or at least centers on the map?
    [ ] I could naively just put it on the center but it'd be nice if we make bigger maps to have it move around.
  [ ] Figure out a way to tie in GamepadInputs
  [ ] Get a skybox so it looks nice
  [ ] Create a goal for someone to move to (like in a tutorial!)
[ ] Do some research into Resource / Game State Tracking
  [ ] Build a Start Screen, and Loading Screen for the demo above!
    [ ] Learn basic Bevy UIs
    [ ] Do some research into the "Pickable" system for hover, etc / other events.
  [ ] Create some silly music to play in different things
  [ ] Look forward and write out the set of states the Game can be in
[ ] Get the demo running on the Steam Deck
[ ] Cleanup code!
[ ] Do some UI Research into how to build bevy_uis for selecting characters / viewing stats
[ ] Have a think through the Unit types and what should be on them
  [ ] Create a combat system
  [ ] Create a skill system
  [ ] Create an inventory system
[ ] Another demo!
[ ] I want to spend some time learning about Tiled, and creating Custom data on my map
  [ ] It'd be great to use this to be able to load in "Ground" tiles for example.
  [ ] Experiment with using Tiled as a way of pushing data into the maps themselves? Or just add some JSON / RON alongside some maps maybe for an equivalent without needing to learn too much.
[ ] How would savegames work? What data does one save? 
