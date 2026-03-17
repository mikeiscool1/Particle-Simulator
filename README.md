# Particle Simulator
## A simple particle simulator

Includes:
- Gravity
- Velocity & Acceleration
- Collision & Bounce
- Friction
- Mass
- Trails
- Merging
- Grid
- Movable Camera
- Cube/Sphere Rendering

## Tutorial (GUI)
- To run, run `cargo run --release`.
- To move: WASD to move forward, left, back, and right. Right click + drag to look around. E/Q to go up/down.
- Fullscreen: F11
- Start/Stop simulation: Space
- Reset simulation: r
- Reset camera: o
- Show/Hide particle trails: t
- Show/Hide grid: g
- Toggle sphere/cube rendering: c (tip: use cube rendering for many particles to reduce lag)
- Slow time: F1
- Speed up time: F2
- Slow camera speed: F3
- Speed up camera speed: F4

## Tutorial (Code)
State settings are stored in the `state` variable in `main.rs`. Many are togglable or only meant for internal use, but some are not.


To enable **particle merging**, set `min_merge_mass` to a number. This mass is the minimum both particles have to be to merge together once they are touching and
do not have enough velocity to escape. 

To edit how much bounce particles have, change `restitution` from 0 to 1. 

Finally, to make your particles follow a strict function with respect to time (ignoring velocity, acceleration, gravity, etc., useful for 3d shapes like the lissajous curve,
as included in the code as an example), set `use_time_function` to true.

If you're using the time function, edit the `time_function` function to create your function. Otherwise, to set up particles, use the `set_particles` function.

The example code included is a cube of particles with random starting velocities that, on play, will go in all different directions.