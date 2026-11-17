# Marikina FPS

A compact 3D cargo yard shooter built with Rust and Macroquad. The playable arena is 52 × 52 m, with staggered cargo-container walls around the outside, collision bounds along the edges, 5 red AI opponents, and 4 blue teammates. The map reuses the supplied ResoForge low-poly asset; its attribution is in `assets/dockyard/ATTRIBUTION.txt`.

## Run it

```sh
cargo run --release
```

## Controls

- **Desktop:** WASD move, Shift sprint, hold right mouse to look, left mouse fire, V switch rifle/knife, Space jump, R reload, F aim, Q/E or mouse wheel adjust magnification, C sneak/slide, Ctrl sneak, Esc quit.
- **Touch:** floating joystick on the left, drag the right side to look. Fire, Jump, Rel, Scope and Snk/Slide are on-screen controls. Use the weapon button at the bottom to switch rifle/knife.
- Push the stick fully forward or hold Shift to run. The Snk button becomes Slide while sprinting.
- Settings > Customize HUD lets you drag and scale touch controls.

## Android APK

The GitHub Actions workflow is `.github/workflows/android-apk.yml`.

## License

The game source is MIT licensed. The included low-poly map asset retains its separate CC-BY-4.0 attribution in `assets/dockyard/ATTRIBUTION.txt`.
