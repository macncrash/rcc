#!/bin/bash
# rcc's own runtime corpus (tests/run.sh over tests/run/*.rs, expected output
# from rustc) through the bootstrapped toolchain only: an rcc whose C was
# compiled by ccc, with the stage-C ccc (ccc built by rcc) compiling rcc's
# generated C.
#   CCGEN=ccc:   rcc-by-ccc        + ccc-by-rcc
#   CCGEN=clang: rcc-by-ccc2.clang + ccc-by-rcc.clang  (that rcc was compiled
#                by ccc-by-rcc.clang; clang only built that ccc's own C)
# Runs the snapshot's copy of tests/ with its scratch and log directories
# under build/ (RCC_RUN_WORK / RCC_RUN_LOGS), so it does not share
# build/test-work with other runs in the repository.
#   usage: CCGEN=ccc|clang [SELFTEST_ARGS="-k t0"] selftest.sh
. "$(dirname "$0")/common.sh"
OUT=$V/selftest.txt
require_stage_c > "$OUT" || { cat "$OUT"; exit 1; }
if [ "$CCGEN" = clang ]; then RCC=$RCC_D; else RCC=$RCC_B; fi
[ -x "$RCC" ] || { echo "SKIPPED: $RCC missing (run make chain CCGEN=$CCGEN)" | tee "$OUT"; exit 1; }
S=$V/selftest
rm -rf "$S"; mkdir -p "$S"
export RCC_RUN_WORK=$S/work RCC_RUN_LOGS=$S/logs COMPILE_TIMEOUT=${COMPILE_TIMEOUT:-300}
echo "selftest: $RCC_SRC/tests/run.sh --rcc $(basename "$RCC") --cc $(basename "$CCC_C") ${SELFTEST_ARGS:-}"
t0=$(now)
/bin/bash "$RCC_SRC/tests/run.sh" --rcc "$RCC" --cc "$CCC_C" ${SELFTEST_ARGS:-} > "$S/run.out" 2>&1
st=$?
t1=$(now)
{
  echo "rcc runtime corpus with rcc = $(basename "$RCC"), C compiler = $(basename "$CCC_C") ($(elapsed $t0 $t1) s, COMPILE_TIMEOUT=${COMPILE_TIMEOUT}s)"
  sed 's/^/  /' "$S/run.out"
  total=$(awk '$1 == "total" {print $2 " of " $3}' "$S/run.out")
  if [ $st = 0 ]; then echo "PASS: $total tests pass"; else echo "FAIL: $total tests pass (logs: $S/logs/NAME.log)"; fi
} > "$OUT"
cat "$OUT"
exit $st
