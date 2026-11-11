#!/usr/bin/env bash
# Regenerate tests/run/NAME.expected (and NAME.exitcode when nonzero) from the
# real rustc oracle.  Usage: tests/gen_expected.sh [-j N] [PATTERN...]
# PATTERN is a substring of the test name (e.g. t03_ or t06_vec_sort).
# Each oracle binary is run twice in a fresh scratch dir; differing output is
# reported as nondeterminism and the test is marked failed.
set -u
TESTS_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$TESTS_DIR/.." && pwd)
RUN_DIR=$(cd "${RCC_RUN_DIR:-$TESTS_DIR/run}" && pwd); export RCC_RUN_DIR="$RUN_DIR"  # e.g. RCC_RUN_DIR=tests/std_os
OUT="$ROOT/build/oracle"
RUSTC=${RUSTC:-rustc}
RUSTFLAGS_ORACLE="--edition 2021 -C opt-level=1 -C debug-assertions=off -C overflow-checks=off"

ncpu() { sysctl -n hw.ncpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4; }

# ---- worker mode: generate one test ------------------------------------------
if [ "${1:-}" = "--one" ]; then
  name=$2
  src="$RUN_DIR/$name.rs"
  bin="$OUT/bin/$name"
  log="$OUT/logs/$name.log"
  mkdir -p "$OUT/bin" "$OUT/logs"
  if ! $RUSTC $RUSTFLAGS_ORACLE "$src" -o "$bin" >"$log" 2>&1; then
    echo "FAIL $name: rustc failed: $(grep -m1 -E '^error' "$log")"
    exit 1
  fi
  args=()
  if [ -f "$RUN_DIR/$name.args" ]; then
    while IFS= read -r line || [ -n "$line" ]; do args+=("$line"); done < "$RUN_DIR/$name.args"
  fi
  stdin=/dev/null
  [ -f "$RUN_DIR/$name.stdin" ] && stdin="$RUN_DIR/$name.stdin"
  envs=()
  if [ -f "$RUN_DIR/$name.env" ]; then
    while IFS= read -r line || [ -n "$line" ]; do [ -n "$line" ] && envs+=("$line"); done < "$RUN_DIR/$name.env"
  fi
  for i in 1 2; do
    work="$OUT/work/$name.$i"
    rm -rf "$work"; mkdir -p "$work"
    ( cd "$work" && env ${envs[@]+"${envs[@]}"} perl -e 'alarm 20; exec @ARGV or die' "$bin" ${args[@]+"${args[@]}"} \
        <"$stdin" >"$OUT/logs/$name.out$i" 2>"$OUT/logs/$name.err$i" )
    echo $? > "$OUT/logs/$name.code$i"
    rm -rf "$work"
  done
  if ! cmp -s "$OUT/logs/$name.out1" "$OUT/logs/$name.out2" || \
     ! cmp -s "$OUT/logs/$name.code1" "$OUT/logs/$name.code2"; then
    echo "FAIL $name: nondeterministic output"
    exit 1
  fi
  code=$(cat "$OUT/logs/$name.code1")
  if [ "$code" = 142 ]; then echo "FAIL $name: oracle timed out"; exit 1; fi
  if [ -s "$OUT/logs/$name.err1" ] && [ "$code" = 0 ]; then
    : # stderr output is allowed; it is not compared
  fi
  cp "$OUT/logs/$name.out1" "$RUN_DIR/$name.expected"
  if [ "$code" != 0 ]; then echo "$code" > "$RUN_DIR/$name.exitcode"; else rm -f "$RUN_DIR/$name.exitcode"; fi
  if grep -q -E '^warning' "$log"; then
    echo "ok   $name (exit $code) [rustc warnings, see $log]"
  else
    echo "ok   $name (exit $code)"
  fi
  exit 0
fi

# ---- driver -------------------------------------------------------------------
JOBS=$(ncpu)
pats=()
while [ $# -gt 0 ]; do
  case "$1" in
    -j) JOBS=$2; shift 2 ;;
    -j*) JOBS=${1#-j}; shift ;;
    -h|--help) sed -n '2,6p' "$0"; exit 0 ;;
    *) pats+=("$1"); shift ;;
  esac
done
mkdir -p "$OUT"
list="$OUT/list.$$.txt"
: > "$list"
for f in "$RUN_DIR"/*.rs; do
  n=$(basename "$f" .rs)
  if [ ${#pats[@]} -eq 0 ]; then echo "$n" >> "$list"; continue; fi
  for p in "${pats[@]}"; do case "$n" in *"$p"*) echo "$n" >> "$list"; break ;; esac; done
done
results="$OUT/results.$$.txt"
xargs -P "$JOBS" -n 1 "${BASH:-bash}" "$0" --one < "$list" | tee "$results"
nfail=$(grep -c '^FAIL' "$results")
ntot=$(wc -l < "$list" | tr -d ' ')
rm -f "$results" "$list"
echo "generated: $((ntot - nfail))/$ntot ok, $nfail failed"
[ "$nfail" -eq 0 ]
