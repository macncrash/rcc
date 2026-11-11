#!/bin/sh
# Parser acceptance test: `rcc --parse` every .rs file under the ccc source
# tree (default ~/dev/ccc/src) plus tests/parse/*.rs.
#   tests/parse.sh [RCC_BINARY] [SRC_DIR]
# Prints failing files with their error and a pass/fail summary.
# Exit status 0 iff every file parses.
RCC=${1:-build/rcc}
SRC=${2:-$HOME/dev/ccc/src}
DIR=$(cd "$(dirname "$0")" && pwd)
pass=0
fail=0
tmp=${TMPDIR:-/tmp}/rcc-parse.$$
for f in $(find "$SRC" "$DIR/parse" -name '*.rs' | sort); do
    if "$RCC" --parse "$f" >"$tmp" 2>&1; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        echo "FAIL $f"
        sed 's/^/    /' "$tmp" | head -3
    fi
done
# Golden AST dumps: tests/parse/X.rs with a reviewed tests/parse/X.ast.
gpass=0
gfail=0
for f in "$DIR"/parse/*.rs; do
    exp=${f%.rs}.ast
    [ -f "$exp" ] || continue
    if "$RCC" --dump-ast "$f" 2>&1 | diff -u "$exp" - >"$tmp"; then
        gpass=$((gpass + 1))
    else
        gfail=$((gfail + 1))
        echo "AST MISMATCH $f"
        head -20 "$tmp"
    fi
done
rm -f "$tmp"
echo "parse: $pass passed, $fail failed, $((pass + fail)) total"
echo "golden ast: $gpass passed, $gfail failed"
[ "$fail" -eq 0 ] && [ "$gfail" -eq 0 ]
