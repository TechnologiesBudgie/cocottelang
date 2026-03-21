# HelloWindow

A minimal Cocotte GUI application using Charlotte (egui).

## Requirements

The `cocotte` binary must be built with GUI support:

```sh
# In the cocotte source tree:
# 1. Uncomment eframe and egui in Cargo.toml
# 2. Build with:
cargo build --release --features gui
```

## Run

```sh
cocotte run
```

## Build (native binary)

```sh
cocotte build --release
./dist/HelloWindow
```

## Cross-compile for Linux ARM64

```sh
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
cocotte exec build-linux-arm64
```
