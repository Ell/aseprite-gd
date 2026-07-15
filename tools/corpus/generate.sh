#!/usr/bin/env bash
# Regenerates the generated fixture corpus + Aseprite-rendered golden PNGs.
# Requires a working Aseprite binary (dev machines only — CI never runs this;
# the generated outputs are committed).
set -euo pipefail

cd "$(dirname "$0")/../.."
OUT=crates/ase-core/tests/fixtures/generated
mkdir -p "$OUT"

ASEPRITE=${ASEPRITE:-aseprite}

# Arch/AUR builds can break on cmark patch bumps (libcmark.so.X.Y.Z is
# version-pinned at link time). Shim the soname locally instead of touching
# /usr/lib; a package rebuild is the real fix.
if ! "$ASEPRITE" -b --version >/dev/null 2>&1; then
    missing=$("$ASEPRITE" -b --version 2>&1 | grep -oP 'libcmark\.so\.[0-9]+(\.[0-9]+)*' | head -1 || true)
    actual=$(ls /usr/lib/libcmark.so.*.* 2>/dev/null | head -1)
    if [[ -n "$missing" && -n "$actual" ]]; then
        shim=$(mktemp -d)
        ln -s "$actual" "$shim/$missing"
        export LD_LIBRARY_PATH="$shim${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        echo "note: shimmed $missing -> $actual (rebuild the aseprite package to fix properly)"
    fi
fi

"$ASEPRITE" -b --script-param out="$OUT" --script tools/corpus/gen_corpus.lua
echo "$("$ASEPRITE" -b --version 2>/dev/null)" > "$OUT/GENERATED_BY.txt"
ls "$OUT"/*.aseprite | wc -l
