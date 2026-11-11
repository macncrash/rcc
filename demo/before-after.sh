#!/bin/bash
# Three ocean windows side by side, each simulating on one thread, the title
# bar naming the compiler chain that built the binary (the example's
# COMPILER_NAME), the same way ccc's examples/vulkan-ocean `make before-after`
# shows ccc against clang.
#
# The only difference from the bench binaries: ocean.c opens its window
# centered at 1280x720, so the copy compiled here has that one call changed to
# take its position and size from -DWINDOW_X/Y/W/H, which places the three
# windows next to each other instead of on top of each other.
#
#   usage: CCGEN=ccc|clang [DURATION=seconds] before-after.sh
#   DURATION: close the windows after that many seconds (default: run until
#   you close them; Esc quits a window).
. "$(dirname "$0")/common.sh"
require_stage_c || exit 1
prepare_ocean || exit 1
W=$OCEAN/windows
mkdir -p "$W"
sed 's/SDL_CreateWindow("ocean", SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED, 1280, 720,/SDL_CreateWindow("ocean", WINDOW_X, WINDOW_Y, WINDOW_W, WINDOW_H,/' \
  "$OCEAN/ocean.c" > "$W/ocean.c"
grep -q 'WINDOW_X, WINDOW_Y' "$W/ocean.c" || { echo "could not find the SDL_CreateWindow call in ocean.c"; exit 1; }

# Screen width in points (Retina panels report pixels; halve them).
res=$(system_profiler SPDisplaysDataType 2>/dev/null | grep -m1 'Resolution:')
sw=$(echo "$res" | sed -n 's/.*Resolution: *\([0-9]*\) x.*/\1/p')
case "$res" in *Retina*) sw=$((sw / 2)) ;; esac
[ -n "$sw" ] && [ "$sw" -gt 600 ] || sw=1440
ww=$((sw / 3 - 8)); wh=$((ww * 9 / 16))

i=0
BINS=
for t in ccc-rustc "$(basename "$CCC_C")" clang; do
  case $t in
    ccc-rustc) cc=$CCC_RUSTC; title=$NAME_RUSTC ;;
    clang) cc="$CLANG -arch arm64"; title=clang ;;
    *) cc=$CCC_C; title=$NAME_C ;;
  esac
  x=$((i * (ww + 8) + 4))
  echo "compiling ocean-$t (\"$title\") at x=$x, ${ww}x${wh}"
  $cc -O2 $OCEAN_CPPFLAGS "$OCEAN_VKDEF" -DCOMPILER_NAME="\"$title\"" \
    -DWINDOW_X=$x -DWINDOW_Y=40 -DWINDOW_W=$ww -DWINDOW_H=$wh \
    -o "$W/ocean-$t" "$W/ocean.c" $OCEAN_LDLIBS || { echo "FAILED: $t could not compile ocean.c"; exit 1; }
  BINS="$BINS $W/ocean-$t"
  i=$((i + 1))
done

echo "launching:$BINS"
echo "(each with --threads 1 --novsync; the title bar shows sim ms/frame, fps and 'built with <chain>')"
pids=
for b in $BINS; do
  "$b" --threads 1 --novsync > "$b.out" 2>&1 &
  pids="$pids $!"
done
if [ -n "${SCREENSHOT:-}" ]; then
  # capture only the strip of the screen where the three windows are
  $PERL -e 'select(undef, undef, undef, 5)'
  screencapture -x -R "0,0,$sw,$((wh + 80))" "$SCREENSHOT" && echo "screenshot of the window strip: $SCREENSHOT"
fi
if [ -n "${DURATION:-}" ]; then
  $PERL -e 'select(undef, undef, undef, $ARGV[0])' "$DURATION"
  kill $pids 2> /dev/null
fi
wait
ok=0
for b in $BINS; do
  echo "$(basename "$b"): $(head -1 "$b.out")"
  grep -q '^ocean: .*compiled with' "$b.out" && ok=$((ok + 1))
done
echo "$ok of 3 windows initialized SDL + Vulkan and started rendering"
[ $ok = 3 ]
