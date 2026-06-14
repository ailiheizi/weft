#!/usr/bin/env bash
# Build every official WASM package to wasm32-wasip1 and place the resulting
# package.wasm next to each package's manifest.
#
# Usage: bash scripts/build-wasm-packages.sh [output_dir]
#   output_dir (optional): also copy each <pkg>/package.wasm into
#                          output_dir/<pkg>/package.wasm for release bundling.
set -euo pipefail

cd "$(dirname "$0")/.."
OUT_DIR="${1:-}"

rustup target add wasm32-wasip1 >/dev/null 2>&1 || true

built=0
for manifest in packages/official/*/Cargo.toml; do
    pkg_dir="$(dirname "$manifest")"
    pkg_name="$(basename "$pkg_dir")"

    # Only build packages that declare package.wasm as their entry.
    if ! grep -q 'entry *= *"package.wasm"' "$pkg_dir/package.toml" 2>/dev/null; then
        continue
    fi

    echo ">> building $pkg_name"
    cargo build --manifest-path "$manifest" --target wasm32-wasip1 --release

    # crate name is package-<dir>; cargo emits package_<dir>.wasm
    wasm_src="$pkg_dir/target/wasm32-wasip1/release/package_${pkg_name//-/_}.wasm"
    if [[ ! -f "$wasm_src" ]]; then
        # fall back to whatever single .wasm was produced
        wasm_src="$(find "$pkg_dir/target/wasm32-wasip1/release" -maxdepth 1 -name '*.wasm' | head -1)"
    fi
    [[ -f "$wasm_src" ]] || { echo "ERROR: no wasm produced for $pkg_name" >&2; exit 1; }

    cp "$wasm_src" "$pkg_dir/package.wasm"
    echo "   -> $pkg_dir/package.wasm"

    if [[ -n "$OUT_DIR" ]]; then
        mkdir -p "$OUT_DIR/$pkg_name"
        cp "$pkg_dir/package.wasm" "$OUT_DIR/$pkg_name/package.wasm"
        cp "$pkg_dir/package.toml" "$OUT_DIR/$pkg_name/package.toml" 2>/dev/null || true
    fi

    built=$((built + 1))
done

echo "built $built wasm package(s)"
