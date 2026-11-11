#!/usr/bin/env bash
# Core regression tests: tests/core/NAME.rs + NAME.expected (stdout of the
# rustc-built program; NAME.exitcode when nonzero).
#   tests/core.sh [PATTERN]      run (RCC, RCC_CC from the environment)
#   tests/core.sh --add F.rs     copy F.rs into tests/core and generate its
#                                .expected with real rustc
# Tests without a .expected file (c00_exit.rs) only have to compile and run.
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIR="$ROOT/tests/core"
RCC=${RCC:-$ROOT/build/rcc}
RUSTC=${RUSTC:-/opt/homebrew/bin/rustc}
WORK="$ROOT/build/core-work"
mkdir -p "$WORK"
if [ "${1:-}" = "--add" ]; then
  shift
  for f in "$@"; do
    n=$(basename "$f" .rs)
    [ "$f" -ef "$DIR/$n.rs" ] || cp "$f" "$DIR/$n.rs"
    $RUSTC --edition 2021 -C opt-level=1 -C debug-assertions=off -C overflow-checks=off -A warnings \
      "$DIR/$n.rs" -o "$WORK/$n.ref" || { echo "rustc failed: $n"; exit 1; }
    "$WORK/$n.ref" > "$DIR/$n.expected" 2>/dev/null; st=$?
    rm -f "$DIR/$n.exitcode"; [ $st -ne 0 ] && echo $st > "$DIR/$n.exitcode"
    echo "added $n (exit $st)"
  done
  exit 0
fi
pat=${1:-}
pass=0; fail=0
for src in "$DIR"/*.rs; do
  n=$(basename "$src" .rs)
  case "$n" in *"$pat"*) ;; *) continue ;; esac
  if ! "$RCC" "$src" -o "$WORK/$n.bin" > "$WORK/$n.log" 2>&1; then
    echo "COMPILE_FAIL $n: $(head -1 "$WORK/$n.log")"; fail=$((fail+1)); continue
  fi
  "$WORK/$n.bin" > "$WORK/$n.out" 2>/dev/null; st=$?
  exp=0; [ -f "$DIR/$n.exitcode" ] && exp=$(cat "$DIR/$n.exitcode")
  if [ -f "$DIR/$n.expected" ] && ! cmp -s "$WORK/$n.out" "$DIR/$n.expected"; then
    echo "RUN_FAIL $n: stdout differs"; fail=$((fail+1)); continue
  fi
  if [ "$st" != "$exp" ]; then echo "RUN_FAIL $n: exit $st, expected $exp"; fail=$((fail+1)); continue; fi
  pass=$((pass+1))
done
echo "core: $pass passed, $fail failed"
[ $fail -eq 0 ]
