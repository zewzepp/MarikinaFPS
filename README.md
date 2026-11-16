# Rusty FPS Sketch

A small third-person shooter sketch in Rust (macroquad 3D). Over-the-shoulder camera, a blocky character, a training yard with crates and moving targets.

## Run it

Install Rust, then run:

```sh
cargo run
```

## Controls

- **Touch:** floating joystick on the left (touch anywhere on the left side), drag the right side to look. Buttons: FIRE (hold), JUMP, RELOAD, SCOPE (toggle), SNEAK (toggle), SLIDE.
- **Desktop:** WASD move, hold right mouse and move to look, left mouse fire, Space jump, R reload, F scope, C or Ctrl sneak, Shift slide, Esc quit.

Crates are low enough to jump over or stand on. Targets respawn after a few seconds.

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
