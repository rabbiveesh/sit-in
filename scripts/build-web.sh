#!/usr/bin/env bash
# Build the web game into dist/: index.html + sit_in.wasm, and dist/single.html with the
# module and the SoundFont (gzipped) inlined (one file: for pages that can't serve a second file).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown --lib
target="${CARGO_TARGET_DIR:-target}"
mkdir -p dist
cp "$target/wasm32-unknown-unknown/release/sit_in.wasm" dist/
cp web/index.html web/band.sf2 dist/
python3 - <<'PY'
import base64, gzip
b64 = base64.b64encode(open("dist/sit_in.wasm", "rb").read()).decode()
font = base64.b64encode(gzip.compress(open("web/band.sf2", "rb").read(), 9, mtime=0)).decode()
page = open("web/index.html").read().replace("const FONT_GZ_B64 = null;", f'const FONT_GZ_B64 = "{font}";', 1)
open("dist/single.html", "w").write(page.replace("const WASM_B64 = null;", f'const WASM_B64 = "{b64}";', 1))
# The same page as an Artifact body (the viewer adds its own document skeleton).
import re
art = page.replace('const WASM_B64 = null;', f'const WASM_B64 = "{b64}";', 1)
for tag in ["<!doctype html>\n", '<html lang="en">\n', "<head>\n", "</head>\n", "<body>\n", "</body>\n", "</html>\n", '<meta charset="utf-8" />\n']:
    art = art.replace(tag, "", 1)
art = re.sub(r'<meta name="viewport"[^>]*>\n', "", art)
open("dist/artifact.html", "w").write(art)
PY
ls -la dist
