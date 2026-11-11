# The final acceptance demo: ccc (a C compiler written in Rust) and rcc (a
# Rust compiler written in C) build each other, and the result is shown the
# way ccc's examples/vulkan-ocean shows ccc against clang. See README.md.
#
#   make chain          bootstrap chain A..D, each step with its wall time
#   make chain CCGEN=clang   the same with clang compiling all C (faster)
#   make bench          ocean.c by ccc-rustc, ccc-by-rcc and clang: --bench N
#   make before-after   the three ocean windows side by side (needs a display)
#   make difftest       ccc's tests/difftest with ccc-rustc and ccc-by-rcc
#   make selftest       rcc's test corpus with rcc-by-ccc + ccc-by-rcc only
#   make all            chain, bench, difftest, selftest for both CCGEN=ccc and
#                       CCGEN=clang, then a summary block
#   make summary        print the summary of the last runs again
#   make clean
#
# Variables: CCGEN=ccc|clang (default ccc), BENCH_FRAMES=300, COMPILE_RUNS=3,
# DIFFTEST_SUITES=cases|all, CCC_REV=HEAD|<rev>|worktree, CCC_DIR=~/dev/ccc,
# FORCE=1 (rebuild every stage), SNAPSHOT=0 (keep the source snapshot),
# DURATION=N (before-after: close the windows after N seconds).
#
# Everything is built under build/. Nothing is written into ~/dev/ccc or
# elsewhere in this repository.

# /usr/local/bin is avoided on purpose (its bash/perl/python3 hang here).
export PATH := /usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin
SHELL := /bin/bash

CCGEN ?= ccc
export CCGEN BENCH_FRAMES COMPILE_RUNS DIFFTEST_SUITES CCC_REV CCC_DIR FORCE DURATION SELFTEST_ARGS SNAPSHOT

all:
	@./snapshot.sh
	-@SNAPSHOT=0 CCGEN=ccc ./chain.sh
	-@SNAPSHOT=0 CCGEN=clang ./chain.sh
	-@CCGEN=ccc ./bench.sh
	-@CCGEN=clang ./bench.sh
	-@CCGEN=ccc ./difftest.sh
	-@CCGEN=clang ./difftest.sh
	-@CCGEN=ccc ./selftest.sh
	-@CCGEN=clang ./selftest.sh
	@echo
	@./summary.sh | tee build/summary.md

snapshot:
	@./snapshot.sh

chain: snapshot
	@SNAPSHOT=0 ./chain.sh

bench: chain
	@./bench.sh

before-after: chain
	@./before-after.sh

difftest: chain
	@./difftest.sh

selftest: chain
	@./selftest.sh

summary:
	@./summary.sh

clean:
	rm -rf build

.PHONY: all snapshot chain bench before-after difftest selftest summary clean
