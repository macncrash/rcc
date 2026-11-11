#!/usr/bin/env bash
# Whole-program expander check against real rustc/cargo:
#   tests/expand_ccc.sh [--rcc PATH] [N_C_PROGRAMS]
# 1. `rcc --expand --expand-compat` the ccc lib crate and every bin into a
#    copy of the ccc project under /private/tmp/ccc-expanded (single-file
#    crates: all modules inline). ~/dev/ccc is never modified.
# 2. `cargo build --release` the copy.
# 3. Compile (with -O2, plus NAME.helper.c if present) and run N (default
#    all) C programs from ccc's tests/difftest/cases with the original ccc and with the rebuilt one;
#    both the programs' exit status and stdout must be identical.
set -u
TESTS_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$TESTS_DIR/.." && pwd)
RCC=$ROOT/build/rcc
if [ "${1:-}" = "--rcc" ]; then RCC=$2; shift 2; fi
N=${1:-100000}
SRC=${CCC_SRC:-$HOME/dev/ccc}
DST=${CCC_EXPANDED:-/private/tmp/ccc-expanded}
ORIG=${CCC_ORIG:-$SRC/target/release/ccc}
rm -rf "$DST"
mkdir -p "$DST/src/bin"
cp "$SRC/Cargo.toml" "$DST/"
[ -f "$SRC/Cargo.lock" ] && cp "$SRC/Cargo.lock" "$DST/"
cp -R "$SRC/include" "$DST/include"
opts=(--expand-compat --crate-name ccc --env "CARGO_MANIFEST_DIR=$DST")
"$RCC" --expand "$SRC/src/lib.rs" "${opts[@]}" > "$DST/src/lib.rs" || { echo "expand lib.rs failed"; exit 1; }
"$RCC" --expand "$SRC/src/main.rs" "${opts[@]}" > "$DST/src/main.rs" || { echo "expand main.rs failed"; exit 1; }
for b in "$SRC"/src/bin/*.rs; do
  "$RCC" --expand "$b" "${opts[@]}" > "$DST/src/bin/$(basename "$b")" || { echo "expand $b failed"; exit 1; }
done
echo "expanded: $(wc -l < "$DST/src/lib.rs") lines in src/lib.rs"
( cd "$DST" && cargo build --release 2>&1 | grep -E '^(error|warning: unused)|Finished|error\[' | head -20 )
NEW=$DST/target/release/ccc
[ -x "$NEW" ] || { echo "cargo build of the expanded ccc failed"; exit 1; }
W=$(mktemp -d /private/tmp/ccc-cmp.XXXXXX)
pass=0; fail=0; tried=0
for c in $(ls "$SRC"/tests/difftest/cases/*.c | grep -v '\.helper\.c$' | sort); do
  [ $tried -ge "$N" ] && break
  tried=$((tried + 1))
  n=$(basename "$c" .c)
  files=("$c"); [ -f "${c%.c}.helper.c" ] && files+=("${c%.c}.helper.c")
  "$ORIG" -O2 -o "$W/$n.orig" "${files[@]}" > "$W/$n.orig.log" 2>&1; s1=$?
  "$NEW" -O2 -o "$W/$n.new" "${files[@]}" > "$W/$n.new.log" 2>&1; s2=$?
  if [ $s1 -ne $s2 ]; then echo "FAIL $n: compile status $s1 vs $s2"; fail=$((fail + 1)); continue; fi
  if [ $s1 -ne 0 ]; then echo "SAME $n: both fail to compile"; pass=$((pass + 1)); continue; fi
  "$W/$n.orig" > "$W/$n.orig.out" 2>&1 < /dev/null; r1=$?
  "$W/$n.new" > "$W/$n.new.out" 2>&1 < /dev/null; r2=$?
  if [ $r1 -eq $r2 ] && cmp -s "$W/$n.orig.out" "$W/$n.new.out"; then
    echo "PASS $n (exit $r1, $(wc -l < "$W/$n.orig.out" | tr -d ' ') lines)"; pass=$((pass + 1))
  else
    echo "FAIL $n: exit $r1 vs $r2 or output differs"; fail=$((fail + 1))
  fi
done
rm -rf "$W"
echo "ccc rebuilt from expanded source: $pass / $tried C programs behave identically"
[ $fail -eq 0 ]
