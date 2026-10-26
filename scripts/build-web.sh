#!/bin/sh
# Compile the shared browser engine and package the Firefox extension.
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked -p slml-web --target wasm32-unknown-unknown
python3 scripts/package-firefox.py
