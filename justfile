# Lists available recipes.
default:
    @just --list

# Starts the armory registry.
develop:
    cargo run --bin armory-registry

# Builds release artifacts.
[linux]
build:
    cargo build --release --target=x86_64-unknown-linux-musl

# Builds release artifacts.
[macos]
build:
    cargo build --release --target=aarch64-apple-darwin

# Builds release artifacts.
[windows]
build:
    cargo build --release --target=x86_64-pc-windows-msvc

# Publishes the CLI to the Armory registry.
[linux]
publish: build
    armory publish --triple x86_64_linux
    scp target/x86_64-unknown-linux-musl/release/armory \
        msmoiz.com:/home/msmoiz/armory/download/armory-x86_64-linux

# Publishes the CLI to the Armory registry.
[macos]
publish: build
    armory publish --triple aarch64_darwin
    scp target/aarch64-apple-darwin/release/armory \
        armory:/home/msmoiz/armory/download/armory-aarch64-darwin

# Publishes the CLI to the Armory registry.
[windows]
publish: build
    armory publish --triple x86_64_windows
    scp target/x86_64-pc-windows-msvc/release/armory.exe \
        msmoiz.com:/home/msmoiz/armory/download/armory-x86_64-windows
