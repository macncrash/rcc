#!/usr/bin/env bash
# rcc runtime test harness.
#   tests/run.sh [-j N] [-k PATTERN] [--cc CC] [--rcc PATH] [-v]
# For each tests/run/NAME.rs: compile with `rcc NAME.rs -o BIN`, run BIN
# (cwd = fresh scratch dir, 10s timeout, optional NAME.args / NAME.stdin /
# NAME.env), compare stdout with NAME.expected and exit status with
# NAME.exitcode (default 0).  Results: PASS COMPILE_FAIL RUN_FAIL TIMEOUT.
# Full logs: build/test-logs/NAME.log.  Exit status nonzero if anything fails.
# Works with bash 3.2 and stock macOS tools (perl for timeouts).
set -u
TESTS_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$TESTS_DIR/.." && pwd)
RUN_DIR=$(cd "${RCC_RUN_DIR:-$TESTS_DIR/run}" && pwd); export RCC_RUN_DIR="$RUN_DIR"  # e.g. RCC_RUN_DIR=tests/std_os
LOG_DIR="${RCC_RUN_LOGS:-$ROOT/build/test-logs}"
WORK="${RCC_RUN_WORK:-$ROOT/build/test-work}"
RUN_TIMEOUT=${RUN_TIMEOUT:-10}
COMPILE_TIMEOUT=${COMPILE_TIMEOUT:-120}

ncpu() { sysctl -n hw.ncpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4; }

# Run "$@" with a timeout of $1 seconds; exit 124 on timeout (kills the
# whole process group), otherwise the child's exit status (128+sig on signal).
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

# ---- worker: one test --------------------------------------------------------
if [ "${1:-}" = "--one" ]; then
  name=$2
  src="$RUN_DIR/$name.rs"
  log="$LOG_DIR/$name.log"
  res="$LOG_DIR/$name.result"
  bin="$WORK/bin/$name"
  mkdir -p "$WORK/bin"
  rm -f "$bin"
  {
    echo "== test $name"
    echo "== compile: $RCC $src -o $bin   (RCC_CC=${RCC_CC:-})"
  } > "$log"
  with_timeout "$COMPILE_TIMEOUT" "$RCC" "$src" -o "$bin" > "$LOG_DIR/$name.cout" 2>&1
  cst=$?
  cat "$LOG_DIR/$name.cout" >> "$log"
  if [ $cst -ne 0 ] || [ ! -x "$bin" ]; then
    if [ $cst -eq 124 ]; then first="compiler timed out after ${COMPILE_TIMEOUT}s"
    else
      first=$(grep -m1 -i -E 'error|fatal|unsupported|panick' "$LOG_DIR/$name.cout")
      [ -z "$first" ] && first=$(grep -m1 . "$LOG_DIR/$name.cout")
      [ -z "$first" ] && first="exit status $cst (no output)"
    fi
    echo "== COMPILE_FAIL (status $cst)" >> "$log"
    printf 'COMPILE_FAIL\t%s\t%s\n' "$name" "$first" > "$res"
    rm -f "$LOG_DIR/$name.cout"
    exit 0
  fi
  rm -f "$LOG_DIR/$name.cout"
  args=()
  if [ -f "$RUN_DIR/$name.args" ]; then
    while IFS= read -r line || [ -n "$line" ]; do args+=("$line"); done < "$RUN_DIR/$name.args"
  fi
  envs=()
  if [ -f "$RUN_DIR/$name.env" ]; then
    while IFS= read -r line || [ -n "$line" ]; do [ -n "$line" ] && envs+=("$line"); done < "$RUN_DIR/$name.env"
  fi
  stdin=/dev/null
  [ -f "$RUN_DIR/$name.stdin" ] && stdin="$RUN_DIR/$name.stdin"
  expcode=0
  [ -f "$RUN_DIR/$name.exitcode" ] && expcode=$(tr -d ' \n' < "$RUN_DIR/$name.exitcode")
  wd="$WORK/cwd/$name"
  rm -rf "$wd"; mkdir -p "$wd"
  out="$LOG_DIR/$name.stdout"; err="$LOG_DIR/$name.stderr"
  ( cd "$wd" && with_timeout "$RUN_TIMEOUT" env ${envs[@]+"${envs[@]}"} "$bin" ${args[@]+"${args[@]}"} \
      < "$stdin" > "$out" 2> "$err" )
  rst=$?
  rm -rf "$wd"
  {
    echo "== run: $bin ${args[*]+${args[*]}}  (exit $rst, expected $expcode)"
    echo "== stdout:"; cat "$out"
    echo "== stderr:"; cat "$err"
  } >> "$log"
  if [ $rst -eq 124 ]; then
    echo "== TIMEOUT" >> "$log"
    printf 'TIMEOUT\t%s\t%s\n' "$name" "no exit after ${RUN_TIMEOUT}s" > "$res"
    exit 0
  fi
  if ! cmp -s "$out" "$RUN_DIR/$name.expected"; then
    first=$(perl -e '
      open(my $e, "<", $ARGV[0]) or die; open(my $a, "<", $ARGV[1]) or die;
      my $n = 0;
      while (1) {
        my $x = <$e>; my $y = <$a>; $n++;
        last if !defined $x && !defined $y;
        if (!defined $x || !defined $y || $x ne $y) {
          for ($x, $y) { if (defined) { chomp; s/[^\x20-\x7e]/?/g; $_ = substr($_, 0, 70) } }
          printf "line %d: expected %s got %s", $n,
            defined $x ? "\"$x\"" : "EOF", defined $y ? "\"$y\"" : "EOF";
          last;
        }
      }' "$RUN_DIR/$name.expected" "$out")
    [ -z "$first" ] && first="stdout differs (trailing bytes)"
    [ "$rst" != "$expcode" ] && first="$first; exit $rst (expected $expcode)"
    echo "== RUN_FAIL" >> "$log"
    diff "$RUN_DIR/$name.expected" "$out" >> "$log"
    printf 'RUN_FAIL\t%s\t%s\n' "$name" "$first" > "$res"
    exit 0
  fi
  if [ "$rst" != "$expcode" ]; then
    first="exit status $rst, expected $expcode"
    [ -s "$err" ] && first="$first; stderr: $(head -c 100 "$err" | head -n 1)"
    echo "== RUN_FAIL" >> "$log"
    printf 'RUN_FAIL\t%s\t%s\n' "$name" "$first" > "$res"
    exit 0
  fi
  echo "== PASS" >> "$log"
  printf 'PASS\t%s\t\n' "$name" > "$res"
  rm -f "$out" "$err" "$bin"
  exit 0
fi

# ---- driver ------------------------------------------------------------------
JOBS=$(ncpu)
PATTERN=""
VERBOSE=0
RCC="${RCC:-$ROOT/build/rcc}"
usage() { sed -n '2,9p' "$0"; }
while [ $# -gt 0 ]; do
  case "$1" in
    -j) JOBS=$2; shift 2 ;;
    -j*) JOBS=${1#-j}; shift ;;
    -k) PATTERN=$2; shift 2 ;;
    --cc) export RCC_CC="$2"; shift 2 ;;
    --cc=*) export RCC_CC="${1#--cc=}"; shift ;;
    --rcc) RCC=$2; shift 2 ;;
    -v) VERBOSE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done
case "$RCC" in /*) ;; *) RCC="$(pwd)/$RCC" ;; esac
export RCC RUN_TIMEOUT COMPILE_TIMEOUT
if [ ! -x "$RCC" ]; then echo "run.sh: compiler not found or not executable: $RCC" >&2; exit 2; fi

mkdir -p "$LOG_DIR" "$WORK"
list="$WORK/list.$$.txt"
: > "$list"
for f in "$RUN_DIR"/*.rs; do
  n=$(basename "$f" .rs)
  if [ -n "$PATTERN" ]; then case "$n" in *"$PATTERN"*) ;; *) continue ;; esac; fi
  rm -f "$LOG_DIR/$n.result"
  echo "$n" >> "$list"
done
total=$(wc -l < "$list" | tr -d ' ')
if [ "$total" -eq 0 ]; then echo "no tests match '$PATTERN'"; exit 2; fi

xargs -P "$JOBS" -n 1 "${BASH:-bash}" "$0" --one < "$list"

all="$WORK/results.$$.txt"
: > "$all"
while IFS= read -r n; do
  if [ -f "$LOG_DIR/$n.result" ]; then cat "$LOG_DIR/$n.result" >> "$all"
  else printf 'COMPILE_FAIL\t%s\t%s\n' "$n" "harness error: no result" >> "$all"; fi
done < "$list"

# failing lines, sorted by name
sort -t "$(printf '\t')" -k2,2 "$all" | awk -F '\t' -v v="$VERBOSE" '
  $1 != "PASS" { printf "%-13s %-40s %s\n", $1, $2, $3 }
  $1 == "PASS" && v == 1 { printf "%-13s %s\n", $1, $2 }'

# per-tier summary
awk -F '\t' '
  { t = substr($2, 1, 3); tiers[t] = 1; c[t, $1]++; tot[$1]++; n[t]++; N++ }
  END {
    printf "\n%-6s %6s %6s %8s %8s %8s\n", "tier", "pass", "total", "compile", "run", "timeout"
    k = 0; for (t in tiers) order[++k] = t
    for (i = 1; i <= k; i++) for (j = i + 1; j <= k; j++) if (order[j] < order[i]) { s = order[i]; order[i] = order[j]; order[j] = s }
    for (i = 1; i <= k; i++) { t = order[i]
      printf "%-6s %6d %6d %8d %8d %8d\n", t, c[t,"PASS"], n[t], c[t,"COMPILE_FAIL"], c[t,"RUN_FAIL"], c[t,"TIMEOUT"] }
    printf "%-6s %6d %6d %8d %8d %8d\n", "total", tot["PASS"], N, tot["COMPILE_FAIL"], tot["RUN_FAIL"], tot["TIMEOUT"]
  }' "$all"
echo "logs: $LOG_DIR/NAME.log"
npass=$(grep -c "^PASS" "$all")
rm -f "$list" "$all"
[ "$npass" -eq "$total" ]
