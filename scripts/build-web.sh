#!/usr/bin/env bash
# Build the web game into dist/: index.html + sit_in.wasm, and dist/single.html with the
# module inlined (one file: for pages that can't serve a second file).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown --lib
target="${CARGO_TARGET_DIR:-target}"
mkdir -p dist
cp "$target/wasm32-unknown-unknown/release/sit_in.wasm" dist/
cp web/index.html dist/index.html
python3 - <<'PY'
import base64
b64 = base64.b64encode(open("dist/sit_in.wasm", "rb").read()).decode()
page = open("web/index.html").read()
open("dist/single.html", "w").write(page.replace("const WASM_B64 = null;", f'const WASM_B64 = "{b64}";', 1))
PY
ls -la dist
