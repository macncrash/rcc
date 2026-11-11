#!/bin/bash
# The bootstrap chain.  usage: CCGEN=ccc|clang chain.sh
#
#   A  ccc-rustc    ccc built by cargo/rustc
#   B  rcc-by-ccc   rcc's C sources compiled by ccc-rustc
#   C  ccc-by-rcc   ccc's Rust compiled by rcc-by-ccc; the generated C is
#                   compiled by ccc-rustc (RCC_CC)
#   D  rcc-by-ccc2  rcc's C compiled by ccc-by-rcc, then
#      ccc-by-rcc2  ccc's Rust compiled by rcc-by-ccc2, C compiled by ccc-by-rcc
#
# With CCGEN=clang, clang compiles rcc in stage B and the generated C of ccc
# in stages C and D (binaries get the suffix .clang); stage D's rcc is still
# compiled by the stage-C ccc.
#
# A stage is skipped when its inputs (source snapshot, compilers) did not
# change; FORCE=1 rebuilds everything.  Every stage logs to
# build/chain-$CCGEN/logs/ and, on failure, prints a command that reproduces it.
. "$(dirname "$0")/common.sh"
NCPU=$(sysctl -n hw.ncpu)
STAGE_TIMEOUT=${STAGE_TIMEOUT:-1800}
SMOKE_TIMEOUT=${SMOKE_TIMEOUT:-60}
L=$V/logs
mkdir -p "$L"
SUMMARY=$V/summary.txt
: > "$SUMMARY"
CHAINLOG=$V/chain.log
FAILED=0

if [ "$CCGEN" = clang ]; then GEN_CC=$CLANG; else GEN_CC=$CCC_RUSTC; fi

"$DEMO/snapshot.sh" || { echo "snapshot failed"; exit 1; }
FP_CCC=$(fingerprint "$CCC_SRC/src" "$CCC_SRC/include" "$CCC_SRC/Cargo.toml")
FP_RCC=$(fingerprint "$RCC_SRC/src" "$RCC_SRC/lib" "$RCC_SRC/rt" "$RCC_SRC/Makefile")

echo
echo "bootstrap chain, CCGEN=$CCGEN ($([ "$CCGEN" = clang ] && echo 'clang compiles all C' || echo 'ccc compiles all C; no clang'))"
echo "  ccc sources: $(cat "$CCC_SRC/.rev")    rcc sources: snapshot of $(cat "$RCC_SRC/.time")"
echo

# ---- helpers -----------------------------------------------------------------

# up_to_date ID KEY OUTPUT: true if stage ID last succeeded with the same key
up_to_date() {
  local d=${4:-$V}
  [ "${FORCE:-0}" != 1 ] && [ -e "$3" ] && [ "$(cat "$d/key.$1" 2>/dev/null)" = "$2" ]
}

# wall time of the last build of an up-to-date stage
lasttime() { cat "$1" 2>/dev/null || echo "-"; }

# line ID NAME DESCRIPTION SECONDS RESULT
line() {
  printf '  %-2s %-17s %-58s %7s  %s\n' "$1" "$2" "$3" "$4" "$5"
  printf '%s|%s|%s|%s|%s\n' "$1" "$2" "$3" "$4" "$5" >> "$SUMMARY"
}

# report_failure LOG STATUS REPRO: details of a failed step
report_failure() {
  local why
  case "$2" in
    124) why="timed out (killed after the time limit)" ;;
    1[3-9][0-9]) why="killed by signal $(( $2 - 128 ))" ;;
    *) why="exit status $2" ;;
  esac
  echo "     FAILED: $why. Last lines of $1:"
  tail -n 15 "$1" | sed 's/^/       | /'
  echo "     reproduce with:"
  printf '%s\n' "$3" | sed 's/^/       $ /'
  FAILED=1
}

# smoke_ccc CCC LOG: compile and run a small loop with the given ccc
smoke_ccc() {
  local d=$V/smoke-$(basename "$1")
  mkdir -p "$d"
  cat > "$d/loop.c" <<'EOF'
#include <stdio.h>
int sum(const int *p, int n) { int s = 0; for (int i = 0; i < n; i++) s += p[i]; return s; }
int main(void) { int a[100]; for (int i = 0; i < 100; i++) a[i] = i; printf("sum %d\n", sum(a, 100)); return 0; }
EOF
  rm -f "$d/loop"
  with_timeout "$SMOKE_TIMEOUT" "$1" -O2 "$d/loop.c" -o "$d/loop" >> "$2" 2>&1 || return $?
  [ "$("$d/loop")" = "sum 4950" ]
}

# smoke_rcc RCC CC LOG: compile and run a small Rust program
smoke_rcc() {
  local d=$V/smoke-$(basename "$1")
  mkdir -p "$d"
  cat > "$d/hello.rs" <<'EOF'
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<String, usize> = HashMap::new();
    for w in "the quick brown fox jumps over the lazy dog the end".split(' ') {
        *m.entry(w.to_string()).or_insert(0) += 1;
    }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort();
    println!("{:?}", &v[..3]);
}
EOF
  rm -f "$d/hello"
  RCC_CC=$2 with_timeout "$SMOKE_TIMEOUT" "$1" "$d/hello.rs" -o "$d/hello" >> "$3" 2>&1 || return $?
  [ "$("$d/hello")" = '[("brown", 1), ("dog", 1), ("end", 1)]' ]
}

setstatus() { echo "$2" > "$V/status.$1"; }

# ---- stage A: ccc built by rustc -----------------------------------------------
stage_A() {
  local log=$L/A.log key="$FP_CCC" t0 t1 st src
  if up_to_date A "$key" "$CCC_RUSTC" "$B"; then
    setstatus A ok
    line A ccc-rustc "ccc built by cargo/rustc" "$(lasttime "$B/time.A")" "ok (up to date)"; return 0
  fi
  : > "$log"; t0=$(now)
  # Use the binary in the ccc checkout if it is current for this snapshot:
  # snapshot of HEAD, clean source tree, binary newer than the commit.
  src="cargo build --release into build/cargo"
  local user=$CCC_DIR/target/release/ccc
  if [ -x "$user" ] && [ "$(cat "$CCC_SRC/.rev")" = "$(git -C "$CCC_DIR" rev-parse --short HEAD)" ] &&
     [ -z "$(git -C "$CCC_DIR" status --porcelain -- src include Cargo.toml)" ] &&
     [ "$(stat -f %m "$user")" -gt "$(git -C "$CCC_DIR" log -1 --format=%ct)" ]; then
    cp "$user" "$CCC_RUSTC"; st=$?; src="current ~/dev/ccc/target/release/ccc"
  else
    (cd "$CCC_SRC" && with_timeout "$STAGE_TIMEOUT" cargo build --release --bin ccc --target-dir "$B/cargo") >> "$log" 2>&1
    st=$?
    [ $st = 0 ] && cp "$B/cargo/release/ccc" "$CCC_RUSTC"
  fi
  t1=$(now)
  if [ $st = 0 ] && smoke_ccc "$CCC_RUSTC" "$log"; then
    setstatus A ok; echo ok > "$B/status.A"; echo "$key" > "$B/key.A"
    echo "$(elapsed $t0 $t1) s" > "$B/time.A"
    line A ccc-rustc "ccc built by cargo/rustc ($src)" "$(elapsed $t0 $t1) s" ok
  else
    setstatus A FAILED; echo FAILED > "$B/status.A"
    line A ccc-rustc "ccc built by cargo/rustc" "$(elapsed $t0 $t1) s" FAILED
    report_failure "$log" "${st:-1}" "cd $CCC_SRC && cargo build --release --bin ccc --target-dir $B/cargo"
    return 1
  fi
}

# ---- stage B: rcc's C compiled by ccc-rustc (or clang) -------------------------------
stage_B() {
  local log=$L/B.log cc key t0 t1 st desc
  if [ "$CCGEN" = clang ]; then cc=$CLANG; desc="rcc's C compiled by clang"
  else cc=$CCC_RUSTC; desc="rcc's C compiled by ccc-rustc"; fi
  key="$FP_RCC $(filesum "$cc")"
  if up_to_date B "$key" "$RCC_B"; then setstatus B ok; line B "$(basename "$RCC_B")" "$desc" "$(lasttime "$V/time.B")" "ok (up to date)"; return 0; fi
  : > "$log"; rm -rf "$V/rcc-B"; t0=$(now)
  with_timeout "$STAGE_TIMEOUT" make -C "$RCC_SRC" -j"$NCPU" BUILD="$V/rcc-B" CC="$cc" >> "$log" 2>&1
  st=$?; t1=$(now)
  [ $st = 0 ] && cp "$V/rcc-B/rcc" "$RCC_B"
  if [ $st = 0 ] && smoke_rcc "$RCC_B" "$GEN_CC" "$log"; then st=0; else [ $st = 0 ] && st=1; fi
  if [ $st = 0 ]; then
    setstatus B ok; echo "$key" > "$V/key.B"; echo "$(elapsed $t0 $t1) s" > "$V/time.B"
    line B "$(basename "$RCC_B")" "$desc" "$(elapsed $t0 $t1) s" ok
  else
    setstatus B FAILED; line B "$(basename "$RCC_B")" "$desc" "$(elapsed $t0 $t1) s" FAILED
    report_failure "$log" $st "make -C $RCC_SRC BUILD=$V/rcc-B CC=$cc"
    return 1
  fi
}

# compile ccc with RCC, C compiled by CC, into OUT, keeping the C in CDIR
rcc_build_ccc() {
  local rcc=$1 cc=$2 out=$3 cdir=$4 log=$5
  rm -rf "$cdir"; mkdir -p "$cdir"
  RCC_CC=$cc with_timeout "$STAGE_TIMEOUT" "$rcc" $CCC_CRATE_ARGS --emit-c "$cdir/ccc.c" -o "$out" >> "$log" 2>&1
}
repro_rcc_build_ccc() {
  echo "RCC_HOME=$RCC_SRC RCC_CC=$2 $1 $CCC_CRATE_ARGS --emit-c $4/ccc.c -o $3"
}

# ---- stage C: ccc's Rust compiled by the stage-B rcc ----------------------------
stage_C() {
  local log=$L/C.log key t0 t1 st desc
  desc="ccc's Rust compiled by $(basename "$RCC_B"), C by $(basename "$GEN_CC")"
  key="$FP_CCC $(filesum "$RCC_B") $(filesum "$GEN_CC") $FP_RCC"
  if up_to_date C "$key" "$CCC_C"; then setstatus C ok; line C "$(basename "$CCC_C")" "$desc" "$(lasttime "$V/time.C")" "ok (up to date)"; return 0; fi
  : > "$log"; t0=$(now)
  rcc_build_ccc "$RCC_B" "$GEN_CC" "$V/ccc-C" "$V/c-C" "$log"
  st=$?; t1=$(now)
  if [ $st != 0 ]; then
    setstatus C FAILED; line C "$(basename "$CCC_C")" "$desc" "$(elapsed $t0 $t1) s" FAILED
    report_failure "$log" $st "$(repro_rcc_build_ccc "$RCC_B" "$GEN_CC" "$V/ccc-C" "$V/c-C")"
    return 1
  fi
  cp "$V/ccc-C" "$CCC_C"
  smoke_ccc "$CCC_C" "$log"; st=$?
  if [ $st != 0 ]; then
    setstatus C "FAILED (built, but does not work)"
    line C "$(basename "$CCC_C")" "$desc" "$(elapsed $t0 $t1) s" "BUILT, BROKEN"
    echo "     The build succeeded ($(ls "$V"/c-C/ccc.c.*.c | wc -l | tr -d ' ') C units), but the resulting ccc fails a smoke test:"
    echo "     compiling a 3-line C program with a loop (-O2, ${SMOKE_TIMEOUT}s limit)."
    report_failure "$log" $st "$(printf '%s\n' \
      "printf 'int f(int *p, int n) { int s = 0; for (int i = 0; i < n; i++) s += p[i]; return s; }\\n' > /tmp/loop.c" \
      "$CCC_C -O0 -c /tmp/loop.c -o /tmp/loop.o     # fails or never returns")"
    return 1
  fi
  setstatus C ok; echo "$key" > "$V/key.C"; echo "$(elapsed $t0 $t1) s" > "$V/time.C"
  line C "$(basename "$CCC_C")" "$desc" "$(elapsed $t0 $t1) s" ok
  # Same rcc source compiled by another C compiler must emit the same C.
  local other
  if [ "$CCGEN" = clang ]; then other=$B/chain-ccc/c-C; else other=$B/chain-clang/c-C; fi
  if [ -d "$other" ]; then
    if diff -rq "$V/c-C" "$other" > /dev/null 2>&1; then
      echo "     the generated C ($(ls "$V"/c-C | wc -l | tr -d ' ') files) is identical to the other chain's (rcc built by $([ "$CCGEN" = clang ] && echo ccc || echo clang))"
    else
      echo "     note: the generated C differs from $other (sources changed between runs?)"
    fi
  fi
}

# ---- stage D: the fixed point -----------------------------------------------------
stage_D() {
  local log=$L/D.log key t0 t1 t2 st dcc desc1 desc2
  if [ "$CCGEN" = clang ]; then dcc=$CLANG; else dcc=$CCC_C; fi
  desc1="rcc's C compiled by $(basename "$CCC_C")"
  desc2="ccc's Rust compiled by $(basename "$RCC_D"), C by $(basename "$dcc")"
  key="$FP_RCC $FP_CCC $(filesum "$CCC_C") $(filesum "$dcc")"
  if up_to_date D "$key" "$CCC_D"; then
    setstatus D ok
    line D "$(basename "$RCC_D")" "$desc1" "$(lasttime "$V/time.D1")" "ok (up to date)"
    line D "$(basename "$CCC_D")" "$desc2" "$(lasttime "$V/time.D2")" "ok (up to date)"
  else
    : > "$log"; rm -rf "$V/rcc-D"; t0=$(now)
    with_timeout "$STAGE_TIMEOUT" make -C "$RCC_SRC" -j"$NCPU" BUILD="$V/rcc-D" CC="$CCC_C" >> "$log" 2>&1
    st=$?; t1=$(now)
    if [ $st != 0 ]; then
      setstatus D FAILED; line D "$(basename "$RCC_D")" "$desc1" "$(elapsed $t0 $t1) s" FAILED
      report_failure "$log" $st "make -C $RCC_SRC BUILD=$V/rcc-D CC=$CCC_C"
      return 1
    fi
    cp "$V/rcc-D/rcc" "$RCC_D"; echo "$(elapsed $t0 $t1) s" > "$V/time.D1"
    line D "$(basename "$RCC_D")" "$desc1" "$(elapsed $t0 $t1) s" ok
    rcc_build_ccc "$RCC_D" "$dcc" "$V/ccc-D" "$V/c-D" "$log"
    st=$?; t2=$(now)
    if [ $st != 0 ]; then
      setstatus D FAILED; line D "$(basename "$CCC_D")" "$desc2" "$(elapsed $t1 $t2) s" FAILED
      report_failure "$log" $st "$(repro_rcc_build_ccc "$RCC_D" "$dcc" "$V/ccc-D" "$V/c-D")"
      return 1
    fi
    cp "$V/ccc-D" "$CCC_D"
    if smoke_ccc "$CCC_D" "$log"; then
      setstatus D ok; echo "$key" > "$V/key.D"; echo "$(elapsed $t1 $t2) s" > "$V/time.D2"
      line D "$(basename "$CCC_D")" "$desc2" "$(elapsed $t1 $t2) s" ok
    else
      setstatus D FAILED; line D "$(basename "$CCC_D")" "$desc2" "$(elapsed $t1 $t2) s" "BUILT, BROKEN"
      report_failure "$log" 1 "$CCC_D -O2 $V/smoke-$(basename "$CCC_C")/loop.c -o /tmp/loop"
      return 1
    fi
  fi
  check_fixpoint
}

# ---- checks: is it a fixed point? -------------------------------------------------
check_fixpoint() {
  local n out ref refdir f differ=0 same=0
  echo
  echo "  fixed-point checks ($CCGEN chain):"
  # 1. rcc built by ccc-by-rcc emits the same C for ccc as rcc of stage B
  n=$(ls "$V/c-C" | wc -l | tr -d ' ')
  if diff -rq "$V/c-C" "$V/c-D" > "$V/fixpoint-c.diff" 2>&1; then
    out="identical ($n files, $(cat "$V"/c-C/* | wc -c | tr -d ' ') bytes)"
    setstatus fixpoint_c ok
  else
    out="DIFFERENT: $(wc -l < "$V/fixpoint-c.diff" | tr -d ' ') files differ, see $V/fixpoint-c.diff"
    setstatus fixpoint_c FAILED; FAILED=1
  fi
  echo "    generated C of ccc, stage C vs stage D:                 $out"
  printf 'check|generated C of ccc: stage C vs stage D|%s\n' "$out" >> "$SUMMARY"

  # 2. stage C and stage D ccc are the same program: identical assembly for
  #    the same input.  Also compared with ccc-rustc (informational: see
  #    README, "Why ccc-rustc's output differs in register choice").
  local asm=$V/asm base c f
  rm -rf "$asm"; mkdir -p "$asm"
  prepare_ocean
  local inputs="$OCEAN/ocean.c $(ls "$RCC_SRC"/src/*.c)"
  local cd_same=0 cd_diff=0 r_same=0 r_diff=0 cd_list="" r_list=""
  # compile everything in parallel (the ccc builds through rcc are slower)
  for f in $inputs; do
    for c in "$CCC_RUSTC" "$CCC_C" "$CCC_D"; do
      echo "$c $f $asm/$(basename "$f" .c).$(basename "$c").s"
    done
  done | ASM_FLAGS="-O2 -I$RCC_SRC/src $OCEAN_CPPFLAGS" ASM_VKDEF="$OCEAN_VKDEF" xargs -P "$NCPU" -L 1 /bin/bash -c '
    "$1" $ASM_FLAGS "$ASM_VKDEF" -S "$2" -o "$3" > /dev/null 2>&1 || echo "failed: $?" > "$3"' _
  for f in $inputs; do
    base=$(basename "$f" .c)
    if cmp -s "$asm/$base.$(basename "$CCC_C").s" "$asm/$base.$(basename "$CCC_D").s" &&
       ! grep -q '^failed' "$asm/$base.$(basename "$CCC_C").s"; then cd_same=$((cd_same + 1))
    else cd_diff=$((cd_diff + 1)); cd_list="$cd_list $base.c"; fi
    if cmp -s "$asm/$base.ccc-rustc.s" "$asm/$base.$(basename "$CCC_C").s"; then r_same=$((r_same + 1))
    else r_diff=$((r_diff + 1)); r_list="$r_list $base.c"; fi
  done
  if [ $cd_diff = 0 ]; then out="identical for all $cd_same files"; setstatus same_asm ok
  else out="$cd_diff of $((cd_same + cd_diff)) files DIFFER:$cd_list (see $asm)"; setstatus same_asm FAILED; FAILED=1; fi
  echo "    -O2 -S of ocean.c + rcc's src/*.c, stage C vs stage D ccc:  $out"
  printf 'check|-O2 assembly of ocean.c and rcc src/*.c: stage C ccc vs stage D ccc|%s\n' "$out" >> "$SUMMARY"
  # 3. the stage-C ccc of the other chain was built from the same generated C
  #    by a different C compiler (ccc vs clang): it must behave identically.
  local og ox same=0 diff=0
  if [ "$CCGEN" = clang ]; then og=ccc; ox=ccc-by-rcc; else og=clang; ox=ccc-by-rcc.clang; fi
  if [ -d "$B/chain-$og/asm" ] && diff -rq "$V/c-C" "$B/chain-$og/c-C" > /dev/null 2>&1; then
    for f in $inputs; do
      base=$(basename "$f" .c)
      if cmp -s "$asm/$base.$(basename "$CCC_C").s" "$B/chain-$og/asm/$base.$ox.s"; then same=$((same + 1)); else diff=$((diff + 1)); fi
    done
    if [ $diff = 0 ]; then out="identical for all $same files"; else out="$diff of $((same + diff)) files DIFFER"; FAILED=1; fi
    echo "    the same, $(basename "$CCC_C") vs $ox (same C, compiled by ccc vs clang): $out"
    printf 'check|-O2 assembly: ccc-by-rcc (C by ccc) vs ccc-by-rcc.clang (C by clang)|%s\n' "$out" >> "$SUMMARY"
  fi
  out="$r_same of $((r_same + r_diff)) files byte-identical, $r_diff differ (README: why)"
  [ $r_diff = 0 ] && out="identical for all $r_same files"
  echo "    the same, ccc-rustc vs stage C ccc (informational):          $out"
  printf 'info|-O2 assembly: ccc-rustc vs stage C ccc|%s\n' "$out" >> "$SUMMARY"
}

# ---- run ------------------------------------------------------------------------------
for s in A B C D fixpoint_c same_asm; do echo "not run" > "$V/status.$s"; done
{
  printf '  %-2s %-17s %-58s %7s  %s\n' "" binary "built from / by" "wall" result
  stage_A && stage_B && stage_C && stage_D
  rc=$?
  if [ $rc != 0 ]; then
    for s in B C D; do
      if [ "$(stage_status $s)" = "not run" ]; then
        line "$s" "" "not run: an earlier stage failed" "-" SKIPPED; setstatus $s "skipped (earlier stage failed)"
      fi
    done
  fi
  echo
  if [ $rc = 0 ] && [ $FAILED = 0 ]; then echo "chain ($CCGEN): all stages ok"; else echo "chain ($CCGEN): FAILED (logs in $L)"; fi
} 2>&1 | tee "$CHAINLOG"
# the pipeline loses the subshell's state; recompute from status files
for s in A B C D; do case "$(stage_status $s)" in ok) ;; *) exit 1 ;; esac; done
for s in fixpoint_c same_asm; do case "$(stage_status $s)" in ok) ;; *) exit 1 ;; esac; done
exit 0
