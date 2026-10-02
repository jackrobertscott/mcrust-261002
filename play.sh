#!/bin/sh
# Build (if needed) and launch Minecraft Rust Edition.
cd "$(dirname "$0")" || exit 1
cargo build --release --quiet && exec ./target/release/mcrust "$@"
