#!/usr/bin/env bash
# Regenerates the generated fixture corpus + Aseprite-rendered golden PNGs.
# Requires a working Aseprite binary (dev machines only — CI never runs this;
# the generated outputs are committed).
set -euo pipefail

cd "$(dirname "$0")/../.."
OUT=crates/ase-core/tests/fixtures/generated
mkdir -p "$OUT"

# Resolution order: explicit $ASEPRITE, working system binary, Steam build.
# The Steam build bundles its own deps, so it survives the cmark soname
# bumps that break the AUR package.
if [[ -z "${ASEPRITE:-}" ]]; then
    if command -v aseprite >/dev/null && aseprite -b --version >/dev/null 2>&1; then
        ASEPRITE=aseprite
    else
        for lib in "$HOME/.local/share/Steam" "$HOME/.steam/steam" /mnt/games/SteamLibrary; do
            if [[ -x "$lib/steamapps/common/Aseprite/aseprite" ]]; then
                ASEPRITE="$lib/steamapps/common/Aseprite/aseprite"
                break
            fi
        done
        ASEPRITE=${ASEPRITE:-aseprite}
    fi
fi

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
