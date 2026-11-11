#!/usr/bin/env bash
# Expander tests. The oracle is real rustc.
#   tests/expand.sh [--rcc PATH] [-j N] [-k PATTERN] [-v] [GROUP...]
# GROUPs (default: unit run):
#   unit  tests/expand/*.rs and tests/expand/*/main.rs. For each program P:
#           expand   `rcc --expand P` succeeds and its output re-parses
#                    (`rcc --parse`), i.e. full lowering per the std contract;
#           compat   `rcc --expand --expand-compat P` compiled by rustc
#                    behaves like P compiled by rustc (stdout + exit code);
#           mock     `rcc --expand --fmt-root ::std_mock P` compiled by
#                    rustc against tests/expand/support/std_mock (the
#                    formatting contract implemented on real std) behaves
#                    like P.
#         A first-line comment `// rcc-flags: ...` gives extra --cfg/--env
#         options (passed to rustc as --cfg / environment too);
#         `// rcc-skip: mock` skips a sub-test.
#   run   tests/run/*.rs: expand (full) + re-parse, and compat and mock
#         round trips compared with NAME.expected / NAME.exitcode (mock is
#         SKIPped for programs that pass std::fmt::Arguments to user code).
#   ccc   expand ~/dev/ccc (lib + bin) under the host cfg and under
#         target_os="linux", target_arch="x86_64"; check no macro is left.
#         (The compat cargo build of ccc is tests/expand_ccc.sh.)
# Exit status is nonzero if anything fails.
set -u
TESTS_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$TESTS_DIR/.." && pwd)
WORK=${EXPAND_WORK:-$ROOT/build/expand-work}
RUSTC_FLAGS="--edition 2021 -C opt-level=1 -C debug-assertions=off -C overflow-checks=off -A warnings"
RUN_TIMEOUT=${RUN_TIMEOUT:-10}

with_timeout() {
  perl -e '
    my $t = shift @ARGV;
    my $pid = fork();
    die "fork: $!" unless defined $pid;
    if ($pid == 0) { setpgrp(0, 0); exec @ARGV or do { print STDERR "exec $ARGV[0]: $!\n"; exit 127 }; }
    local $SIG{ALRM} = sub { kill "KILL", -$pid; kill "KILL", $pid; waitpid($pid, 0); exit 124; };
    alarm $t;
    waitpid($pid, 0);
    my $s = $?;
    alarm 0;
    exit(($s & 127) ? 128 + ($s & 127) : $s >> 8);
  ' "$@"
}

# run BIN OUTFILE [ARGSFILE STDINFILE ENVFILE] -> prints exit status
run_prog() {
  local bin=$1 out=$2 argsf=${3:-} stdinf=${4:-/dev/null} envf=${5:-}
  local args=() envs=()
  if [ -n "$argsf" ] && [ -f "$argsf" ]; then
    while IFS= read -r line || [ -n "$line" ]; do args+=("$line"); done < "$argsf"
  fi
  if [ -n "$envf" ] && [ -f "$envf" ]; then
    while IFS= read -r line || [ -n "$line" ]; do [ -n "$line" ] && envs+=("$line"); done < "$envf"
  fi
  [ -f "$stdinf" ] || stdinf=/dev/null
  local wd; wd=$(mktemp -d "$WORK/cwd.XXXXXX")
  ( cd "$wd" && with_timeout "$RUN_TIMEOUT" env ${envs[@]+"${envs[@]}"} "$bin" ${args[@]+"${args[@]}"} \
      < "$stdinf" > "$out" 2>/dev/null )
  local st=$?
  rm -rf "$wd"
  echo $st
}

# ---- worker: one unit test ----------------------------------------------------
if [ "${1:-}" = "--unit" ]; then
  src=$2; name=$3
  d="$WORK/unit/$name"; rm -rf "$d"; mkdir -p "$d"
  flags=$(sed -n '1s#^// rcc-flags:##p' "$src")
  skip=$(grep -m1 '^// rcc-skip:' "$src" | sed 's#^// rcc-skip:##')
  rcc_opts=(); rustc_opts=(); envs=()
  set -- $flags
  while [ $# -gt 0 ]; do
    case "$1" in
      --cfg) rcc_opts+=(--cfg "$2"); rustc_opts+=(--cfg "$2"); shift 2 ;;
      --env) rcc_opts+=(--env "$2"); envs+=("$2"); shift 2 ;;
      *) shift ;;
    esac
  done
  res() { printf '%s\t%s\t%s\n' "$1" "$name/$2" "${3:-}" >> "$d/results"; }
  # oracle
  if ! env ${envs[@]+"${envs[@]}"} rustc $RUSTC_FLAGS ${rustc_opts[@]+"${rustc_opts[@]}"} -o "$d/orig" "$src" > "$d/orig.log" 2>&1; then
    res FAIL oracle "rustc rejects the original: $(head -1 "$d/orig.log")"; exit 0
  fi
  st0=$(run_prog "$d/orig" "$d/orig.out")
  # full expansion + reparse
  if "$RCC" --expand "$src" ${rcc_opts[@]+"${rcc_opts[@]}"} > "$d/full.rs" 2> "$d/full.log"; then
    if "$RCC" --parse "$d/full.rs" > "$d/reparse.log" 2>&1; then res PASS expand
    else res FAIL expand "output does not re-parse: $(head -1 "$d/reparse.log")"; fi
  else res FAIL expand "$(head -1 "$d/full.log")"; fi
  # compat and mock round trips
  for mode in compat mock; do
    case " $skip " in *" $mode "*) continue ;; esac
    if [ $mode = compat ]; then mo=(--expand-compat); ro=(); else mo=(--fmt-root ::std_mock); ro=(--extern "std_mock=$WORK/libstd_mock.rlib"); fi
    if ! "$RCC" --expand "$src" ${rcc_opts[@]+"${rcc_opts[@]}"} "${mo[@]}" > "$d/$mode.rs" 2> "$d/$mode.log"; then
      res FAIL $mode "rcc: $(head -1 "$d/$mode.log")"; continue
    fi
    if ! env ${envs[@]+"${envs[@]}"} rustc $RUSTC_FLAGS ${rustc_opts[@]+"${rustc_opts[@]}"} ${ro[@]+"${ro[@]}"} -o "$d/$mode" "$d/$mode.rs" > "$d/$mode.rustc.log" 2>&1; then
      res FAIL $mode "rustc rejects expansion: $(grep -m1 '^error' "$d/$mode.rustc.log")"; continue
    fi
    st=$(run_prog "$d/$mode" "$d/$mode.out")
    if [ "$st" != "$st0" ]; then res FAIL $mode "exit $st, original $st0"
    elif ! cmp -s "$d/orig.out" "$d/$mode.out"; then res FAIL $mode "stdout differs: $(diff "$d/orig.out" "$d/$mode.out" | sed -n 2p | cut -c1-80)"
    else res PASS $mode; fi
  done
  exit 0
fi

# ---- worker: one tests/run program --------------------------------------------
if [ "${1:-}" = "--run" ]; then
  name=$2; R="$ROOT/tests/run"; src="$R/$name.rs"
  d="$WORK/run/$name"; rm -rf "$d"; mkdir -p "$d"
  res() { printf '%s\t%s\t%s\n' "$1" "$name/$2" "${3:-}" >> "$d/results"; }
  if "$RCC" --expand "$src" > "$d/full.rs" 2> "$d/full.log"; then
    if "$RCC" --parse "$d/full.rs" > "$d/reparse.log" 2>&1; then res PASS expand
    else res FAIL expand "output does not re-parse: $(head -1 "$d/reparse.log")"; fi
  else res FAIL expand "$(head -1 "$d/full.log")"; fi
  exp=0; [ -f "$R/$name.exitcode" ] && exp=$(tr -d ' \n' < "$R/$name.exitcode")
  for mode in compat mock; do
    if [ $mode = compat ]; then mo=(--expand-compat); ro=(); else mo=(--fmt-root ::std_mock); ro=(--extern "std_mock=$WORK/libstd_mock.rlib"); fi
    if ! "$RCC" --expand "$src" "${mo[@]}" > "$d/$mode.rs" 2> "$d/$mode.log"; then
      res FAIL $mode "rcc: $(head -1 "$d/$mode.log")"; continue
    fi
    if ! rustc $RUSTC_FLAGS ${ro[@]+"${ro[@]}"} -o "$d/$mode" "$d/$mode.rs" > "$d/$mode.rustc.log" 2>&1; then
      if [ $mode = mock ] && grep -q 'std_mock::fmt::Arguments.*std::fmt::Arguments\|std::fmt::Arguments.*std_mock::fmt::Arguments' "$d/$mode.rustc.log"; then
        res SKIP $mode "uses std::fmt::Arguments directly (not mockable)"; continue
      fi
      res FAIL $mode "rustc rejects expansion: $(grep -m1 '^error' "$d/$mode.rustc.log")"; continue
    fi
    st=$(run_prog "$d/$mode" "$d/$mode.out" "$R/$name.args" "$R/$name.stdin" "$R/$name.env")
    if [ "$st" != "$exp" ]; then res FAIL $mode "exit $st, expected $exp"
    elif ! cmp -s "$R/$name.expected" "$d/$mode.out"; then res FAIL $mode "stdout differs from $name.expected"
    else res PASS $mode; fi
  done
  exit 0
fi

# ---- driver -------------------------------------------------------------------
RCC=${RCC:-$ROOT/build/rcc}
JOBS=$(sysctl -n hw.ncpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)
PATTERN=""; VERBOSE=0; GROUPS_=()
while [ $# -gt 0 ]; do
  case "$1" in
    --rcc) RCC=$2; shift 2 ;;
    -j) JOBS=$2; shift 2 ;;
    -k) PATTERN=$2; shift 2 ;;
    -v) VERBOSE=1; shift ;;
    unit|run|ccc) GROUPS_+=("$1"); shift ;;
    -h|--help) sed -n '2,29p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
[ ${#GROUPS_[@]} -eq 0 ] && GROUPS_=(unit run)
case "$RCC" in /*) ;; *) RCC="$(pwd)/$RCC" ;; esac
[ -x "$RCC" ] || { echo "expand.sh: no rcc at $RCC" >&2; exit 2; }
export RCC WORK RUSTC_FLAGS RUN_TIMEOUT
mkdir -p "$WORK"
fail=0

summary() { # results-file label
  awk -F '\t' -v v="$VERBOSE" -v label="$2" '
    { split($2, a, "/"); k = a[length(a)]; tot[k]++
      if ($1 == "PASS") pass[k]++
      else if ($1 == "SKIP") { skip[k]++; if (v == 1) printf "SKIP %-44s %s\n", $2, $3 }
      else printf "FAIL %-44s %s\n", $2, $3 }
    $1 == "PASS" && v == 1 { printf "PASS %s\n", $2 }
    END { for (k in tot) printf "%s %-7s %4d / %4d passed%s\n", label, k, pass[k] + 0, tot[k] - skip[k], skip[k] ? sprintf(" (%d skipped)", skip[k]) : "" }' "$1" | sort
  ! grep -q '^FAIL' "$1"
}

MOCK_BUILT=0
build_mock() {
  [ $MOCK_BUILT = 1 ] && return 0
  python3 "$TESTS_DIR/expand/support/gen_std_mock.py" "$WORK/std_mock.rs" &&
  rustc --edition 2021 --crate-type rlib --crate-name std_mock -A warnings \
    -o "$WORK/libstd_mock.rlib" "$WORK/std_mock.rs" || { echo "cannot build std_mock"; return 1; }
  MOCK_BUILT=1
}

for g in "${GROUPS_[@]}"; do
  case $g in
  unit)
    build_mock || exit 1
    list="$WORK/unit.list"; : > "$list"
    for f in "$TESTS_DIR"/expand/*.rs "$TESTS_DIR"/expand/*/main.rs; do
      [ -f "$f" ] || continue
      case "$f" in */support/*) continue ;; esac
      n=$(basename "$f" .rs); [ "$n" = main ] && n=$(basename "$(dirname "$f")")
      if [ -n "$PATTERN" ]; then case "$n" in *"$PATTERN"*) ;; *) continue ;; esac; fi
      rm -f "$WORK/unit/$n/results"
      printf '%s %s\n' "$f" "$n" >> "$list"
    done
    xargs -P "$JOBS" -L 1 bash "$0" --unit < "$list"
    cat "$WORK"/unit/*/results > "$WORK/unit.results" 2>/dev/null
    summary "$WORK/unit.results" unit || fail=1
    ;;
  run)
    build_mock || exit 1
    list="$WORK/run.list"; : > "$list"
    for f in "$ROOT"/tests/run/*.rs; do
      n=$(basename "$f" .rs)
      if [ -n "$PATTERN" ]; then case "$n" in *"$PATTERN"*) ;; *) continue ;; esac; fi
      rm -f "$WORK/run/$n/results"
      echo "$n" >> "$list"
    done
    xargs -P "$JOBS" -n 1 bash "$0" --run < "$list"
    cat "$WORK"/run/*/results > "$WORK/run.results" 2>/dev/null
    summary "$WORK/run.results" run || fail=1
    ;;
  ccc)
    CCC=${CCC_SRC:-$HOME/dev/ccc}
    : > "$WORK/ccc.results"
    for cfgset in host linux; do
      opts=(--env "CARGO_MANIFEST_DIR=$CCC")
      [ $cfgset = linux ] && opts+=(--cfg 'target_os="linux"' --cfg 'target_arch="x86_64"' --cfg 'target_vendor="unknown"' --cfg 'target_env="gnu"')
      for root in src/lib.rs src/main.rs src/bin/ccc_x86.rs; do
        n=$(echo "$root" | tr '/.' '__')
        for mode in full compat; do
          mo=(); [ $mode = compat ] && mo=(--expand-compat)
          out="$WORK/ccc_${cfgset}_${n}_$mode.rs"
          if "$RCC" --expand "$CCC/$root" --crate-name ccc "${opts[@]}" ${mo[@]+"${mo[@]}"} > "$out" 2> "$out.log"; then
            if "$RCC" --parse "$out" > "$out.parse.log" 2>&1; then
              printf 'PASS\tccc/%s/%s/%s\t\n' $cfgset "$root" $mode >> "$WORK/ccc.results"
            else
              printf 'FAIL\tccc/%s/%s/%s\treparse: %s\n' $cfgset "$root" $mode "$(head -1 "$out.parse.log")" >> "$WORK/ccc.results"
            fi
          else
            printf 'FAIL\tccc/%s/%s/%s\t%s\n' $cfgset "$root" $mode "$(head -1 "$out.log")" >> "$WORK/ccc.results"
          fi
        done
      done
    done
    summary "$WORK/ccc.results" ccc || fail=1
    ;;
  esac
done
exit $fail
