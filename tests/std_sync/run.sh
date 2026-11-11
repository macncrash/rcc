#!/usr/bin/env bash
# Focused tests for std::sync / std::thread / thread_local! / panic capture.
#   tests/std_sync/run.sh [--gen] [-k PATTERN]
# Same layout as tests/run (NAME.rs, NAME.expected, NAME.exitcode). --gen
# writes the expectations from rustc (RUSTC, default rustc) with the oracle
# flags of tests/gen_expected.sh; otherwise each test is built with rcc
# (RCC, default build/rcc; RCC_CC picks the C compiler) and stdout and the
# exit status are compared. Stderr is not compared.
set -u
DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$DIR/../.." && pwd)
OUT="$ROOT/build/std_sync"
RCC=${RCC:-$ROOT/build/rcc}
RCC_FLAGS=${RCC_FLAGS:-}   # e.g. "--std DIR"
RUSTC=${RUSTC:-rustc}
GEN=0
PAT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --gen) GEN=1 ;;
    -k) PAT=$2; shift ;;
    *) echo "usage: $0 [--gen] [-k PATTERN]" >&2; exit 2 ;;
  esac
  shift
done
mkdir -p "$OUT"

with_timeout() {
  perl -e '
    my $t = shift @ARGV;
    my $pid = fork();
    die "fork: $!" unless defined $pid;
    if ($pid == 0) { setpgrp(0, 0); exec @ARGV or exit 127; }
    local $SIG{ALRM} = sub { kill "KILL", -$pid; kill "KILL", $pid; waitpid($pid, 0); exit 124; };
    alarm $t;
    waitpid($pid, 0);
    my $s = $?;
    alarm 0;
    exit(($s & 127) ? 128 + ($s & 127) : $s >> 8);
  ' "$@"
}

pass=0
fail=0
for src in "$DIR"/*.rs; do
  name=$(basename "$src" .rs)
  case "$name" in *"$PAT"*) ;; *) continue ;; esac
  bin="$OUT/$name"
  rm -f "$bin"
  if [ $GEN = 1 ]; then
    $RUSTC --edition 2021 -C opt-level=1 -C debug-assertions=off -C overflow-checks=off "$src" -o "$bin" || { echo "FAIL $name: rustc"; fail=$((fail+1)); continue; }
    (cd "$OUT" && with_timeout 30 "$bin" > "$DIR/$name.expected" 2> "$OUT/$name.stderr")
    st=$?
    rm -f "$DIR/$name.exitcode"
    [ $st -ne 0 ] && echo $st > "$DIR/$name.exitcode"
    echo "generated $name (exit $st)"
    continue
  fi
  if ! with_timeout 120 "$RCC" $RCC_FLAGS "$src" -o "$bin" > "$OUT/$name.cout" 2>&1 || [ ! -x "$bin" ]; then
    echo "COMPILE_FAIL $name: $(grep -m1 -i -E 'error|fatal' "$OUT/$name.cout")"
    fail=$((fail+1))
    continue
  fi
  (cd "$OUT" && with_timeout 30 "$bin" > "$OUT/$name.out" 2> "$OUT/$name.stderr")
  st=$?
  want=0
  [ -f "$DIR/$name.exitcode" ] && want=$(cat "$DIR/$name.exitcode")
  if [ $st -ne $want ]; then
    echo "RUN_FAIL $name: exit status $st, expected $want"
    fail=$((fail+1))
  elif ! diff -q "$DIR/$name.expected" "$OUT/$name.out" > /dev/null; then
    echo "RUN_FAIL $name: $(diff "$DIR/$name.expected" "$OUT/$name.out" | head -3 | tr '\n' ' ')"
    fail=$((fail+1))
  else
    pass=$((pass+1))
  fi
done
[ $GEN = 1 ] && exit 0
echo "std_sync: $pass passed, $fail failed"
[ $fail -eq 0 ]
