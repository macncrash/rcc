#!/bin/sh
# Regenerate the raw data behind docs/INVENTORY.md. Works on a COPY of ccc;
# never touches ~/dev/ccc. Usage: tools/inventory/run.sh [SCRATCH_DIR]
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
W=${1:-${TMPDIR:-/tmp}/rcc-inventory}
mkdir -p "$W/out"
rsync -a --delete --exclude target --exclude tests --exclude .git ~/dev/ccc/ "$W/ccc/"
# 1. MIR of the release lib (cfg(test) off, debug_assertions off, no features)
(cd "$W/ccc" && touch src/lib.rs && RUSTC_BOOTSTRAP=1 cargo rustc --lib --release -- \
    -Zunpretty=mir -Zmir-opt-level=0 -Ztrim-diagnostic-paths=false -Zmir-include-spans=yes) > "$W/mir.txt"
python3 "$HERE/mir_inventory.py" "$W/mir.txt" "$W/out"      # calls/casts/binops/types/sites tsv
python3 "$HERE/report.py" "$W/out" > "$W/api.md"            # std API by type (markdown)
python3 "$HERE/instances.py" "$W/mir.txt" > "$W/instances.txt"
# 2. Source-level counts on the test-stripped tree (checked: it still compiles)
rm -rf "$W/stripped" && mkdir -p "$W/stripped"
python3 "$HERE/strip_tests.py" ~/dev/ccc "$W/stripped"
cp ~/dev/ccc/Cargo.toml "$W/stripped/" && (cd "$W/stripped" && cargo check --release)
python3 "$HERE/features.py" "$W/stripped" > "$W/features.txt"
python3 "$HERE/fmt_specs.py" "$W/stripped" > "$W/fmt.txt"
echo "outputs in $W"
