# Build ENGRAM

The repository is a Rust workspace. The desktop uses native `egui`/`eframe`; it does not require a Node.js frontend or a local web server.

Install the current stable Rust toolchain compatible with the workspace's declared minimum and dependency requirements using the [official Rust installer](https://www.rust-lang.org/tools/install). SQLite is built through the bundled crate feature, so a platform C compiler is required.

## Windows

The standard path is Rust's `x86_64-pc-windows-msvc` toolchain and the Microsoft C++ Build Tools with the Windows SDK. A configured GNU Rust toolchain with a compatible MinGW compiler is another build route. CI uses the standard Windows runner toolchain.

```powershell
cargo build -p engram-cli --release
cargo build -p engram-desktop --release
.\target\release\engram.exe doctor
.\target\release\engram-desktop.exe --demo
```

Keep any required runtime DLLs with a GNU-built executable when distributing it. Do not claim a single-file portable build unless that exact package has been tested.

## macOS

Install Rust and the Xcode Command Line Tools. Build on the intended architecture; the initial manual artifact workflow uses the architecture supplied by the selected GitHub runner.

```bash
xcode-select --install
cargo build -p engram-desktop --release
./target/release/engram-desktop --demo
```

Developer Preview binaries are not notarized application bundles. The source build is available; packaging, signing and notarization are separate distribution work.

## Linux

An example Ubuntu/Debian dependency set for native desktop building:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev \
  libgl1-mesa-dev libgtk-3-dev libdbus-1-dev
cargo build -p engram-desktop --release
./target/release/engram-desktop --demo
```

A graphical session and a compatible graphics backend are needed to run the native UI. Core and CLI tests do not require a display server.

## Development checks

```bash
cargo fmt --all -- --check
cargo test -p engram-core -p engram-cli
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p engram-desktop
```

Tests generate synthetic sources in temporary directories. Do not point development scripts at personal backups. Review [methodology](METHODOLOGY.md) when adding or testing a reader.

## Build artifacts

The manually dispatched [Build artifacts workflow](../.github/workflows/build-artifacts.yml) compiles native executables and uploads platform archives with SHA-256 checksums. It does not create a release, sign executables or publish a package. A successful build is not a substitute for running and visually testing the packaged executable on its target platform.
