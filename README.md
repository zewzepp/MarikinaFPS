# Rusty FPS Sketch

A small, browser-buildable FPS sketch in Rust. It uses a simple raycaster so the prototype stays self-contained and easy to change while you develop the Rusty Launcher game logic.

## Run it

Install Rust, then run:

```sh
cargo run
```

## Controls

- **Desktop:** WASD move, mouse look, Space jump, Shift slide, Ctrl sneak, right mouse scope, left mouse fire, R reload.
- **Touch:** left joystick moves; drag the right side to look; use the labeled buttons for slide, jump, scope, fire, reload, and sneak.

The game is an early gameplay sketch: it has a small arena, moving targets, a magazine, reload timing, jumping, sliding, sneaking, and scoped aim. Tweak the constants and arena in `src/main.rs` as the launcher design takes shape.

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
