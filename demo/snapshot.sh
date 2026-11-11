#!/bin/bash
# Snapshot the two source trees the demo builds from into build/src:
#   build/src/ccc   ccc at git revision $CCC_REV (default HEAD; `git archive`,
#                   read-only for ~/dev/ccc), or its working tree with
#                   CCC_REV=worktree
#   build/src/rcc   this repository's Makefile, src/, lib/, rt/, tests/
# SNAPSHOT=0 keeps an existing snapshot (e.g. to rerun a stage on exactly the
# sources of the previous run).
. "$(dirname "$0")/common.sh"
CCC_REV=${CCC_REV:-HEAD}
mkdir -p "$B/src"

if [ "${SNAPSHOT:-1}" = 0 ] && [ -f "$CCC_SRC/.rev" ] && [ -d "$RCC_SRC/src" ]; then
  echo "snapshot: kept (SNAPSHOT=0): ccc $(cat "$CCC_SRC/.rev"), rcc of $(cat "$RCC_SRC/.time")"
  exit 0
fi

if [ "$CCC_REV" = worktree ]; then
  mkdir -p "$CCC_SRC"
  rsync -a --delete --exclude target --exclude .git --exclude 'examples/*/build' \
    --exclude 'tests/difftest/suites' "$CCC_DIR"/ "$CCC_SRC"/ || exit 1
  rev="worktree of $(git -C "$CCC_DIR" rev-parse --short HEAD)$(git -C "$CCC_DIR" diff --quiet HEAD -- src || echo '+changes') at $(date '+%H:%M')"
  echo "$rev" > "$CCC_SRC/.rev"
else
  full=$(git -C "$CCC_DIR" rev-parse "$CCC_REV") || exit 1
  short=$(git -C "$CCC_DIR" rev-parse --short "$CCC_REV")
  if [ "$(cat "$CCC_SRC/.rev" 2>/dev/null)" != "$short" ]; then
    rm -rf "$CCC_SRC"
    mkdir -p "$CCC_SRC"
    git -C "$CCC_DIR" archive "$full" | tar -x -C "$CCC_SRC" || exit 1
    echo "$short" > "$CCC_SRC/.rev"
  fi
fi

mkdir -p "$RCC_SRC"
rsync -a --delete --exclude .time "$RCC_REPO/Makefile" "$RCC_REPO/src" "$RCC_REPO/lib" "$RCC_REPO/rt" \
  "$RCC_REPO/tests" "$RCC_SRC"/ || exit 1
date '+%Y-%m-%d %H:%M:%S' > "$RCC_SRC/.time"
echo "snapshot: ccc $(cat "$CCC_SRC/.rev") (from $CCC_DIR), rcc sources of $(cat "$RCC_SRC/.time")"
