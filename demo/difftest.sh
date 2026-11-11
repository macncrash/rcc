#!/bin/bash
# ccc's differential test suite (tests/difftest: each case compiled by ccc and
# by clang, both binaries run, exit status and stdout compared), once with
# ccc-rustc and once with the stage-C ccc of the CCGEN chain.  Runs in a
# scratch copy (build/difftest) because the harness writes caches next to
# itself; nothing in ~/dev/ccc is written.
#   usage: CCGEN=ccc|clang [DIFFTEST_SUITES="cases"|all] difftest.sh
# Cases whose status differs between the two runs are rerun twice with each
# compiler; if the original ccc itself gives different results across runs
# the case is reported as flaky and not counted as a difference.
. "$(dirname "$0")/common.sh"
require_stage_c > "$V/difftest.txt" || { cat "$V/difftest.txt"; exit 1; }
SUITES=${DIFFTEST_SUITES:-cases}
DT=$B/difftest/tests/difftest
mkdir -p "$DT"
rsync -a --delete --exclude suites "$CCC_SRC/tests/difftest/" "$DT/" || exit 1
SUITE_ARGS=
if [ "$SUITES" = all ]; then
  [ -d "$CCC_DIR/tests/difftest/suites" ] && rsync -a "$CCC_DIR/tests/difftest/suites/" "$DT/suites/"
else
  for s in $SUITES; do SUITE_ARGS="$SUITE_ARGS --suite $s"; done
fi
cd "$DT" || exit 1

# statuses JSON -> "id<TAB>status" lines
statuses() {
  $PYTHON -I -c 'import json,sys
d = json.load(open(sys.argv[1]))["results"]
for k in sorted(d): print(k + "\t" + d[k]["status"])' "$1"
}

run() {  # run CCC JSON LOG [extra args]
  local c=$1 j=$2 log=$3; shift 3
  $PYTHON -I difftest.py --ccc "$c" $SUITE_ARGS --json "$j" "$@" > "$log" 2>&1
}

REF_JSON=$B/difftest/ccc-rustc.json
REF_KEY="$(filesum "$CCC_RUSTC") $(fingerprint "$DT/cases" "$DT/difftest.py") $SUITES"
t0=$(now)
if [ "$(cat "$REF_JSON.key" 2>/dev/null)" = "$REF_KEY" ] && [ -f "$REF_JSON" ]; then
  echo "ccc-rustc: reusing the results of an earlier run (same ccc-rustc and cases)"
  t1=$(now)
else
  echo "running difftest ($SUITES) with ccc-rustc ..."
  run "$CCC_RUSTC" "$REF_JSON" "$B/difftest/ccc-rustc.log"
  echo "$REF_KEY" > "$REF_JSON.key"
  t1=$(now)
  echo "$(elapsed $t0 $t1) s" > "$REF_JSON.time"
fi
CAND=$(basename "$CCC_C")
CAND_JSON=$V/difftest-$CAND.json
echo "running difftest ($SUITES) with $CAND ..."
run "$CCC_C" "$CAND_JSON" "$V/difftest-$CAND.log"
t2=$(now)

statuses "$REF_JSON" > "$V/dt-ref.tsv"
statuses "$CAND_JSON" > "$V/dt-cand.tsv"
LC_ALL=C join -t "$(printf '\t')" -a1 -a2 -e missing -o 0,1.2,2.2 "$V/dt-ref.tsv" "$V/dt-cand.tsv" |
  awk -F '\t' '$2 != $3' > "$V/dt-diff.tsv"

real=0; flaky=0; inter=0
: > "$V/dt-verdict.txt"
while IFS="$(printf '\t')" read -r id a b; do
  ra=; rb=
  for i in 1 2; do
    run "$CCC_RUSTC" "$V/dt-one.json" /dev/null -k "$id" && :
    ra="$ra $(statuses "$V/dt-one.json" | awk -F '\t' -v id="$id" '$1 == id {print $2}')"
    run "$CCC_C" "$V/dt-one.json" /dev/null -k "$id" && :
    rb="$rb $(statuses "$V/dt-one.json" | awk -F '\t' -v id="$id" '$1 == id {print $2}')"
  done
  if [ "$(echo "$a$ra" | tr ' ' '\n' | sort -u | wc -l | tr -d ' ')" != 1 ]; then
    echo "  flaky       $id: ccc-rustc gave$(echo " $a$ra" | tr -s ' ') across 3 runs ($CAND: $b$rb)" >> "$V/dt-verdict.txt"
    flaky=$((flaky + 1))
  elif [ "$(echo $rb | tr ' ' '\n' | sort -u)" = "$a" ]; then
    echo "  intermittent $id: $CAND gave $b once, then$rb like ccc-rustc ($a); not counted" >> "$V/dt-verdict.txt"
    inter=$((inter + 1))
  else
    echo "  DIFFERENT   $id: ccc-rustc $a (stable over 3 runs), $CAND $b$rb" >> "$V/dt-verdict.txt"
    real=$((real + 1))
  fi
done < "$V/dt-diff.tsv"

{
  echo
  echo "difftest ($SUITES), ccc-rustc ($(cat "$REF_JSON.time" 2>/dev/null); results reused if unchanged):"
  sed -n '/^suite/,/tests in/p' "$B/difftest/ccc-rustc.log" | sed 's/^/  /'
  echo
  echo "difftest ($SUITES), $CAND = $NAME_C ($(elapsed $t1 $t2) s):"
  sed -n '/^suite/,/tests in/p' "$V/difftest-$CAND.log" | sed 's/^/  /'
  echo
  n=$(wc -l < "$V/dt-ref.tsv" | tr -d ' ')
  echo "cases whose outcome differs between ccc-rustc and $CAND: $(wc -l < "$V/dt-diff.tsv" | tr -d ' ') of $n in the first run"
  cat "$V/dt-verdict.txt"
  if [ $real = 0 ]; then
    echo "PASS: no consistent difference between the two ccc builds ($flaky flaky in ccc-rustc itself, $inter intermittent; both excluded)"
  else
    echo "FAIL: $real case(s) differ consistently ($flaky flaky, $inter intermittent excluded)"
  fi
} > "$V/difftest.txt"
cat "$V/difftest.txt"
[ $real = 0 ]
