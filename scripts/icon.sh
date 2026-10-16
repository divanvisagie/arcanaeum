#!/bin/sh
# Renders the app icon (packaging/linux/arcanaeum.svg) to docs/icon.png at
# 120 px, for the README, through the render_icon example (resvg), so it
# needs no system SVG tools. SIZE and OUT change either.
set -eu
cd "$(dirname "$0")/.."
mkdir -p docs
cargo run --quiet --example render_icon -- "${SIZE:-120}" "${OUT:-docs/icon.png}"
