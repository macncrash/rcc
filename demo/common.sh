# Shared settings for the demo scripts (sourced by bash 3.2 scripts).
#
# Nothing here may resolve to /usr/local/bin (its homebrew bash/perl/python3
# hang on this machine), so PATH is fixed to the system tools plus
# /opt/homebrew/bin (SDL2, Vulkan, glslc, cargo, rustc).
PATH=/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin
export PATH
unset CDPATH MAKEFLAGS MFLAGS MAKELEVEL

DEMO=$(cd "$(dirname "$0")" && pwd)
RCC_REPO=$(cd "$DEMO/.." && pwd)
B=${B:-$DEMO/build}
CCC_DIR=${CCC_DIR:-$HOME/dev/ccc}
CCGEN=${CCGEN:-ccc}
CLANG=${CLANG:-/usr/bin/clang}
PERL=/usr/bin/perl
PYTHON=/usr/bin/python3
BIN=$B/bin
LOGS=$B/logs
mkdir -p "$BIN" "$LOGS"

# The demo builds from snapshots of both source trees (snapshot.sh), so
# concurrent edits in ~/dev/ccc or in rcc's src/ cannot change a build
# half-way, and nothing is ever written into either repository.
RCC_SRC=$B/src/rcc
CCC_SRC=$B/src/ccc
export RCC_HOME=$RCC_SRC

# ---- binaries of the chain ---------------------------------------------------
# Shared:                 ccc-rustc   (stage A, cargo + rustc)
# CCGEN=ccc  (default):   rcc-by-ccc  ccc-by-rcc  rcc-by-ccc2  ccc-by-rcc2
# CCGEN=clang:            same names with the suffix .clang; their C (rcc's
#                         sources in stage B, ccc's generated C in stages C
#                         and D) is compiled by clang instead of ccc.
CCC_RUSTC=$BIN/ccc-rustc
if [ "$CCGEN" = clang ]; then
  SFX=.clang
  RCC_B=$BIN/rcc-by-clang
else
  SFX=
  RCC_B=$BIN/rcc-by-ccc
fi
CCC_C=$BIN/ccc-by-rcc$SFX
RCC_D=$BIN/rcc-by-ccc2$SFX
CCC_D=$BIN/ccc-by-rcc2$SFX
V=$B/chain-$CCGEN          # per-variant work directory
mkdir -p "$V"

# Human-readable names (window titles, tables)
NAME_RUSTC="ccc (built by rustc)"
if [ "$CCGEN" = clang ]; then
  NAME_C="ccc (built by rcc; C compiled by clang)"
else
  NAME_C="ccc (built by rcc, which was built by ccc)"
fi

now() { $PERL -MTime::HiRes=time -e 'printf "%.2f", time'; }
elapsed() { $PERL -e 'printf "%.1f", $ARGV[1] - $ARGV[0]' "$1" "$2"; }

# Run "$@" with a timeout of $1 seconds in its own process group; exit 124 on
# timeout (the whole group is killed), else the command's status.
with_timeout() {
  $PERL -e '
    my $t = shift @ARGV;
    my $pid = fork();
    die "fork: $!" unless defined $pid;
    if ($pid == 0) { setpgrp(0, 0); exec @ARGV or do { print STDERR "exec $ARGV[0]: $!\n"; exit 127 }; }
    local $SIG{ALRM} = sub { kill "KILL", -$pid; kill "KILL", $pid; waitpid($pid, 0); exit 124; };
    local $SIG{INT} = sub { kill "KILL", -$pid; waitpid($pid, 0); exit 130; };
    alarm $t;
    waitpid($pid, 0);
    my $s = $?;
    alarm 0;
    exit(($s & 127) ? 128 + ($s & 127) : $s >> 8);
  ' "$@"
}

# Content fingerprint of files/directories (dotfiles ignored).
fingerprint() {
  find "$@" -type f ! -name '.*' -print 2>/dev/null | LC_ALL=C sort | while IFS= read -r f; do
    cksum < "$f"
  done | cksum | tr -d ' \t'
}
filesum() { cksum < "$1" | tr -d ' \t'; }

# rcc arguments that compile ccc (binary crate + its library crate)
CCC_CRATE_ARGS="$CCC_SRC/src/main.rs --extern ccc=$CCC_SRC/src/lib.rs --env CARGO_MANIFEST_DIR=$CCC_SRC"

# ---- the ocean example ------------------------------------------------------------
# build/ocean/ocean.c is the snapshot's examples/vulkan-ocean/ocean.c; the
# SPIR-V shader includes are copied from ~/dev/ccc/examples/vulkan-ocean/build
# when they exist there and the shader sources match, else regenerated with
# the example's own rule (glslc -O -mfmt=c).  Same flags as the example.
OCEAN=$B/ocean
OCEAN_SRC=$CCC_SRC/examples/vulkan-ocean
OCEAN_PREFIX=/opt/homebrew
OCEAN_CPPFLAGS="-I$OCEAN_PREFIX/include -I$OCEAN/inc"
OCEAN_VKDEF="-DVULKAN_LIBRARY=\"$OCEAN_PREFIX/lib/libvulkan.1.dylib\""   # pass quoted, as one word
OCEAN_LDLIBS="-L$OCEAN_PREFIX/lib -Wl,-rpath,$OCEAN_PREFIX/lib -lSDL2 -lvulkan -lpthread -lm"
prepare_ocean() {
  mkdir -p "$OCEAN/inc" || return 1
  cp "$OCEAN_SRC/ocean.c" "$OCEAN/ocean.c" || return 1
  local s
  for s in ocean.vert ocean.frag sky.vert sky.frag; do
    [ -f "$OCEAN/inc/$s.inc" ] && [ "$OCEAN/inc/$s.inc" -nt "$OCEAN_SRC/shaders/$s" ] && continue
    if [ -f "$CCC_DIR/examples/vulkan-ocean/build/$s.inc" ] &&
       cmp -s "$CCC_DIR/examples/vulkan-ocean/shaders/$s" "$OCEAN_SRC/shaders/$s"; then
      cp "$CCC_DIR/examples/vulkan-ocean/build/$s.inc" "$OCEAN/inc/$s.inc" || return 1
    else
      glslc -O -mfmt=c -o "$OCEAN/inc/$s.inc" "$OCEAN_SRC/shaders/$s" || return 1
    fi
  done
}

# Status of a chain stage recorded by chain.sh: prints ok / FAILED / missing
stage_status() { cat "$V/status.$1" 2>/dev/null || echo missing; }

# The ccc that bench/difftest/selftest test for this variant; exits with a
# clear message if stage C did not produce a working one.
require_stage_c() {
  if [ "$(stage_status C)" != ok ] || [ ! -x "$CCC_C" ]; then
    echo "SKIPPED: stage C of the CCGEN=$CCGEN chain did not produce a working ccc ($(stage_status C))."
    echo "         See 'make chain CCGEN=$CCGEN' and $V/chain.log; 'make <target> CCGEN=clang' uses the clang-compiled chain."
    return 1
  fi
}
