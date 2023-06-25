#!/bin/bash

# WARNING! The GitHub CI configuration does *not* delegate to this file.
# If you edit this file, make sure to keep `.github/workflows/release.yml` in sync.

set -eu
cd $(dirname $0)

if ! which wasm-bindgen >/dev/null 2>/dev/null
then
    echo >&2 "Can't find \`wasm-bindgen\` on path."
    echo >&2 "Do you need to call \`cargo install [--root ...] wasm-bindgen-cli\` first?"
    exit 1
fi

cargo build \
    --release \
    --target wasm32-unknown-unknown

wasm-bindgen \
    --out-name browser_maze_bevy \
    --out-dir www/scripts \
    --target web \
    target/wasm32-unknown-unknown/release/browser-maze-bevy.wasm

find www -type f -name '*.ts' -delete

echo "Success! Now copy \`./www\` to your web root."
