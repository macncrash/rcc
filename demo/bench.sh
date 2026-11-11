#!/bin/bash
# Compile ccc's vulkan-ocean example with ccc-rustc, with the stage-C ccc of
# the CCGEN chain and with clang (-O2, the example's flags), run each with
# --bench N --threads 1 (no window or GPU needed) and compare.
#   usage: CCGEN=ccc|clang BENCH_FRAMES=300 COMPILE_RUNS=3 bench.sh
. "$(dirname "$0")/common.sh"
N=${BENCH_FRAMES:-300}
RUNS=${COMPILE_RUNS:-3}
OUT=$V/bench.txt
require_stage_c > "$OUT" || { cat "$OUT"; exit 1; }
prepare_ocean || { echo "could not prepare ocean.c and its shaders" | tee "$OUT"; exit 1; }

TAGS="ccc-rustc $(basename "$CCC_C") clang"
compiler_of() { case "$1" in ccc-rustc) echo "$CCC_RUSTC" ;; clang) echo "$CLANG -arch arm64" ;; *) echo "$CCC_C" ;; esac; }
title_of() { case "$1" in ccc-rustc) echo "$NAME_RUSTC" ;; clang) echo clang ;; *) echo "$NAME_C" ;; esac; }

echo "ocean.c: compiling with each compiler (-O2, best wall time of $RUNS), then --bench $N --threads 1"
for t in $TAGS; do
  best=
  for i in $(seq "$RUNS"); do
    t0=$(now)
    with_timeout 600 $(compiler_of $t) -O2 $OCEAN_CPPFLAGS "$OCEAN_VKDEF" -DCOMPILER_NAME="\"$(title_of $t)\"" \
      -o "$OCEAN/ocean-$t" "$OCEAN/ocean.c" $OCEAN_LDLIBS > "$OCEAN/compile-$t.log" 2>&1
    st=$?; t1=$(now)
    if [ $st != 0 ]; then
      echo "FAILED: $t could not compile ocean.c (status $st):"; tail -5 "$OCEAN/compile-$t.log"
      echo "reproduce: $(compiler_of $t) -O2 $OCEAN_CPPFLAGS '$OCEAN_VKDEF' -o /tmp/ocean $OCEAN/ocean.c $OCEAN_LDLIBS"
      exit 1
    fi
    best=$($PERL -e 'my ($b, $x) = @ARGV; print(($b eq "" || $x < $b) ? $x : $b)' "$best" "$(elapsed $t0 $t1)")
  done
  echo "$best" > "$OCEAN/compile-$t.time"
  printf '  %-20s %5s s\n' "$t" "$best"
done
for t in $TAGS; do
  echo "  running ocean-$t --bench $N --threads 1"
  "$OCEAN/ocean-$t" --bench "$N" --threads 1 > "$OCEAN/bench-$t.txt" 2>&1 || { echo "FAILED: ocean-$t exited with $?"; cat "$OCEAN/bench-$t.txt"; exit 1; }
done

(
  echo
  printf '%-22s %-43s %9s %9s %9s %-17s %s\n' binary "compiler chain" "compile" "ms/frame" "best" heights checksum
  printf '%-22s %-43s %9s %9s %9s %-17s %s\n' "" "" "ocean.c" "avg" "" "" ""
  for t in $TAGS; do
    f=$OCEAN/bench-$t.txt
    avg=$(sed -n 's/^sim time: *\([0-9.]*\) ms avg.*/\1/p' "$f")
    bst=$(sed -n 's/^sim time:.*ms avg, *\([0-9.]*\) ms best.*/\1/p' "$f")
    hts=$(sed -n 's/^heights: *//p' "$f")
    sum=$(sed -n 's/^checksum: *//p' "$f")
    printf '%-22s %-43s %7s s %9s %9s %-17s %s\n' "ocean-$t" "$(title_of $t)" "$(cat "$OCEAN/compile-$t.time")" "$avg" "$bst" "$hts" "$sum"
    echo "$hts / $sum" >> "$OCEAN/results.$$"
  done
  echo
  if [ "$(sort -u "$OCEAN/results.$$" | wc -l | tr -d ' ')" = 1 ]; then
    echo "PASS: heights and checksum identical across all $(echo $TAGS | wc -w | tr -d ' ') binaries ($N frames, 1 thread)"
    rc=0
  else
    echo "FAIL: heights/checksum differ between the binaries"
    rc=1
  fi
  rm -f "$OCEAN/results.$$"
  exit $rc
) > "$OUT"
rc=$?
cat "$OUT"
exit $rc
