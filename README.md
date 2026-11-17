# Marikina City FPS

A third-person shooter sketch in Rust (macroquad 3D) set in a 200 x 200 m city: avenues with zebra crossings and parked cars, office towers, shop rows, houses, parks with a basketball court, a plaza fountain and a street market. Pedestrian targets patrol the roads.

## Run it

```sh
cargo run --release
```

## What's new

- **Vast city map** instead of the training yard (36 blocks, textured facades, sky, sun, clouds, distance fog, baked lighting).
- **Character physics**: momentum with acceleration and friction, gravity, step-up on curbs, wall sliding, landing impact and camera dip, air control. Targets are knocked into ragdoll pieces that tumble and bounce; shell casings and sparks have physics too.
- **No backward slide**: sprint and slide only work with the stick pushed forward. Walking backward is slower. A slide ends if you turn around.
- **Drag to place buttons**: Settings > Customize HUD, then drag any button (or the joystick) anywhere on screen. The nudge arrows are only for fine tuning. Positions are stored as screen fractions so they survive rotation.
- **Better scope**: first-person lens view, 2x-9x magnification (+/- pads on touch, Q/E or mouse wheel on desktop), breathing sway (steadier when crouched), mil-dot reticle, range finder, lens vignette.
- **Sound FX**: gunshot, dry fire, reload, footsteps, jump, landing, slide, hit marker, headshot ding, bullet impact, scope click, shell casings, city ambience. All synthesized at runtime (no asset files). Volume in Settings.
- Mini-map, headshot detection.

## Controls

- **Touch:** floating joystick on the left, drag the right side to look. Fire (hold), Jump, Rel, Scope, Snk/Slide. Push the stick fully forward to run; the Snk button becomes Slide.
- **Desktop:** WASD move, Shift run, hold right mouse to look, left mouse fire, Space jump, R reload, F scope, Q/E or wheel scope zoom, C sneak/slide, Ctrl sneak, Esc quit.

## Android APK

Same GitHub Actions workflow as before (`.github/workflows/android-apk.yml`).

## License

MIT
