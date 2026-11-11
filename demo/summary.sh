#!/bin/bash
# Summary of the last chain/bench/difftest/selftest runs of both chains,
# suitable for pasting into a README.
. "$(dirname "$0")/common.sh"
echo '```'
echo "rcc bootstrap demo, $(date '+%Y-%m-%d %H:%M'), $(sysctl -n machdep.cpu.brand_string), macOS $(sw_vers -productVersion)"
echo "ccc sources: $(cat "$CCC_SRC/.rev" 2>/dev/null)   rcc sources: snapshot of $(cat "$RCC_SRC/.time" 2>/dev/null)"
for g in ccc clang; do
  d=$B/chain-$g
  echo
  if [ $g = ccc ]; then echo "== CCGEN=ccc: every C file compiled by ccc (no clang)"
  else echo "== CCGEN=clang: rcc and the generated C compiled by clang"; fi
  if [ ! -s "$d/summary.txt" ]; then echo "   chain not run"; continue; fi
  awk -F '|' '$1 ~ /^[A-D]$/ { printf "   %s %-18s %-60s %8s  %s\n", $1, $2, $3, $4, $5 }
              $1 == "check" || $1 == "info" { printf "   %-5s %-71s %s\n", $1, $2, $3 }' "$d/summary.txt"
  for t in bench difftest selftest; do
    f=$d/$t.txt
    if [ ! -f "$f" ]; then echo "   $t: not run"; continue; fi
    case $t in
      bench)
        if grep -q '^binary' "$f"; then
          echo "   bench (ocean.c, $(sed -n 's/.*(\([0-9]* frames, 1 thread\)).*/\1/p' "$f")):"
          sed -n '/^binary/,$p' "$f" | grep -v '^$' | sed 's/^/     /'
        else echo "   bench: $(head -1 "$f")"; fi ;;
      difftest)
        if grep -q '^difftest' "$f"; then
          echo "   difftest:"
          grep -m1 '^  suite' "$f" | sed 's/^/     /'
          grep -E '^(difftest|  cases|  c-testsuite|  gcc-torture|  TOTAL|cases whose|  flaky|  intermittent|  DIFFERENT|PASS|FAIL)' "$f" | sed 's/^/     /'
        else echo "   difftest: $(head -1 "$f")"; fi ;;
      selftest)
        echo "   selftest: $(head -1 "$f")"
        grep -E '^  tier|^  total|^(PASS|FAIL)' "$f" | sed 's/^/     /' ;;
    esac
  done
done
echo '```'
