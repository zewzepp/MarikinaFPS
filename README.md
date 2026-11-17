# Rusty FPS Sketch

A third-person shooter sketch in Rust (macroquad 3D) set in a large procedurally generated city (about 300 m x 300 m, 9x9 blocks): streets with lane markings and crosswalks, sidewalks, parks, parking lots, parked cars, street lamps, and buildings from low-rise to downtown towers. All textures (asphalt, sidewalk, concrete, grass, brick / concrete / glass facades, wooden crates) are generated in code at start-up, so there are no asset files. Distant geometry fades into haze. Moving targets are scattered along the streets.

## Run it

Install Rust, then run:

```sh
cargo run
```

## Controls

- **Touch:** floating joystick on the left (touch anywhere on the left side), drag the right side to look. Buttons: Fire (hold), Jump, Rel (reload), Scope (toggle) and **Snk/Slide**.
- **Snk/Slide:** walking shows **Snk** (tap to toggle sneak). Push the joystick all the way out to run and the button turns into **Slide**. Pick tap-to-slide or hold-to-slide in Settings.
- **Settings** (octagon, top right): slide mode, and **Customize HUD** with a floating window (drag its title bar to move it). **Drag any button or the joystick anywhere on the screen** to place it, use < > to pick one, then change size and opacity (or nudge it). Layout resets when the app restarts.
- **Desktop:** WASD move, hold Shift to run, hold right mouse and move to look, left mouse fire, Space jump, R reload, F scope, C sneak/slide, Ctrl sneak, Esc quit. Click the Settings octagon for the menu.

Crates are low enough to jump over or stand on; cars, trees and buildings are not. Targets respawn after a few seconds. Start-up takes a moment while the city and textures are generated.

## Android APK

The repository includes a GitHub Actions workflow that builds an Android APK on pushes to `main`/`master` or when run manually from the Actions tab. Download the `marikina-fps-android-apk` artifact from the completed run and install it on an ARM64 Android device for a test run.

To build locally, install the Android SDK and NDK, set `ANDROID_HOME` and `NDK_HOME`, then install `cargo-quad-apk` and run:

```sh
rustup target add aarch64-linux-android
cargo install --git https://github.com/not-fl3/cargo-quad-apk.git --rev d411c8fe1c08339e46d7dc6b40ac04ea5e3a01b6 --locked
cargo quad-apk build --release
```

The APK is written under `target/android-artifacts/release/apk/`.

## GitHub Pages / WebAssembly

For a quick local browser build, install `wasm32-unknown-unknown` and `miniquad`'s `cargo web` runner (or use `basic-http-server` to serve the generated files):

```sh
rustup target add wasm32-unknown-unknown
cargo install basic-http-server
cargo build --release --target wasm32-unknown-unknown
```

Macroquad can also package a web build with `macroquad`'s documented `miniquad` web packaging workflow. A GitHub Pages deployment workflow can be added once you choose whether to publish the desktop-first or browser build.

## License

MIT
