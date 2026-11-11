# The ocean test: ccc and rcc build each other

`ccc` (`~/dev/ccc`) is a C compiler written in Rust. `rcc` (this repository)
is a Rust compiler written in C. This directory shows that the two close the
loop: rcc compiles ccc's Rust source to C, ccc compiles rcc's C source, and
the compilers that come out of that loop are the same compilers. The result is
presented the way ccc's `examples/vulkan-ocean` presents ccc against clang:
the same ocean simulation compiled by each compiler, with the same checksum,
the time per frame, and three windows whose title bars name the compiler.

## What the chain proves

```
 A  ccc-rustc    = cargo/rustc  ( ccc's Rust )          the reference ccc
 B  rcc-by-ccc   = ccc-rustc    ( rcc's C )             rcc, compiled by ccc
 C  ccc-by-rcc   = rcc-by-ccc   ( ccc's Rust ) -> C -> ccc-rustc
 D  rcc-by-ccc2  = ccc-by-rcc   ( rcc's C )
    ccc-by-rcc2  = rcc-by-ccc2  ( ccc's Rust ) -> C -> ccc-by-rcc
```

- Stage C produces a ccc without rustc: rcc translated ccc's Rust source
  (388 files, 225k lines, plus rcc's own std) to C, and ccc compiled that C.
  No clang compiles any C on this path. On macOS ccc itself still runs the
  host `clang` as its assembler (`clang -arch arm64 -c file.s`, LLVM's
  integrated assembler) and as its link driver (ld64), and takes
  `stddef.h`/`stdarg.h` from clang's resource directory; that is ccc's
  normal macOS toolchain (`src/backend/arm/darwin.rs`), the same for the
  rustc-built ccc.
- Stage D runs the loop once more with only the stage-C products. If both
  compilers are correct, deterministic and complete, nothing changes: rcc
  compiled by ccc-by-rcc must emit byte-identical C for ccc, and the ccc of
  stage C and stage D must emit byte-identical assembly. That is the fixed
  point, and `make chain` checks it.
- `make bench` then shows that the ccc built through rcc is the same
  compiler in practice: it compiles the ocean demo to a binary that prints the
  same heights and checksum and runs at the same speed as the binaries made
  by the rustc-built ccc and by clang. Only ccc's own compile speed differs
  (it is a ccc binary made from machine-generated C instead of a rustc
  binary).
- `make difftest` runs ccc's differential suite (each case compiled by ccc
  and by clang, both binaries run, outputs compared) with both ccc builds.
- `make selftest` runs rcc's own test corpus (333 Rust programs with
  rustc-generated expected output) with rcc-by-ccc and ccc-by-rcc only.

`make chain CCGEN=clang` is the same chain with clang compiling the C (rcc
in stage B, ccc's generated C in stages C and D). It is much faster to build
and gives a ccc whose own code was optimized by clang, which shows how much of
the compile-speed gap is ccc's code generation versus the C that rcc emits.

## Running it

Requirements: those of the ocean example (`brew install sdl2 vulkan-loader
vulkan-headers molten-vk shaderc`), cargo, and a checkout of ccc in
`~/dev/ccc` (`CCC_DIR=...` to change).

| target | what it does |
|---|---|
| `make chain` | stages A to D with ccc compiling all C, each with its wall time, then the fixed-point checks |
| `make chain CCGEN=clang` | the same with clang compiling the C |
| `make bench` | `ocean.c` compiled by ccc-rustc, ccc-by-rcc and clang (`-O2`, the example's flags; best of 3 compile times), each run with `--bench 300 --threads 1`; one table and a PASS/FAIL line (exit 1 on mismatch) |
| `make before-after` | the three ocean windows side by side, one thread each, title bars `ccc (built by rustc)`, `ccc (built by rcc, which was built by ccc)`, `clang`; `DURATION=10` closes them after 10 s |
| `make difftest` | ccc's `tests/difftest` (`cases` suite, 216 programs; `DIFFTEST_SUITES=all` for c-testsuite and gcc-torture too) with ccc-rustc and with ccc-by-rcc, both summaries and every case whose outcome differs |
| `make selftest` | rcc's `tests/run.sh` with `--rcc rcc-by-ccc --cc ccc-by-rcc` |
| `make all` | all of the above except the windows, for both `CCGEN=ccc` and `CCGEN=clang`, then a summary block (also in `build/summary.md`) |
| `make summary` | print that block again |
| `make clean` | remove `build/` |

Every target accepts `CCGEN=clang`. Other knobs: `BENCH_FRAMES=N`,
`COMPILE_RUNS=N`, `FORCE=1` (rebuild every stage), `CCC_REV=<rev>` or
`CCC_REV=worktree`, `SNAPSHOT=0`.

How it stays out of everyone's way:

- Everything is built under `demo/build/` (gitignored). Nothing is written
  into `~/dev/ccc` or anywhere else in this repository.
- Both compilers are built from snapshots taken at the start of each `make`
  run: ccc via `git archive HEAD` (read-only; `CCC_REV=worktree` copies the
  working tree instead), rcc by copying `Makefile src lib rt tests` into
  `build/src/rcc`. Concurrent edits cannot change a build half-way.
- ccc-rustc is built with `cargo build --release --target-dir build/cargo`
  from that snapshot. (`~/dev/ccc/target/release/ccc` is used instead only
  when it is current: snapshot of HEAD, clean source tree, binary newer than
  the commit.)
- The ocean sources and the SPIR-V shader includes are copied into
  `build/ocean/` (the `.inc` files from the example's `build/` when present
  and its shaders match, otherwise regenerated with the example's own
  `glslc -O -mfmt=c` rule).
- difftest runs in a copy of `tests/difftest` under `build/difftest/`;
  selftest runs the snapshot's `tests/run.sh` with `RCC_TEST_WORK` and
  `RCC_TEST_LOGS` under `build/`.
- Each stage is skipped when its inputs did not change; a failing stage
  prints the end of its log and a command that reproduces it.
- The Makefile and scripts set `PATH=/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin`
  (bash 3.2, system perl and python3).

## Results

`make all` on 2026-11-11, ccc at `f5e0da05` (HEAD), rcc as of 07:01 that day,
Apple M3 Max. The machine was heavily shared during the run (load average
40 to 140 from other jobs), so the wall times of the long stages are
inflated and noisy; the ms/frame numbers come from a single thread and were
stable between two complete runs (within 0.03 ms).

```
== CCGEN=ccc: every C file compiled by ccc (no clang)
   A ccc-rustc          ccc built by cargo/rustc                                       40.6 s  ok
   B rcc-by-ccc         rcc's C compiled by ccc-rustc                                   2.0 s  ok
   C ccc-by-rcc         ccc's Rust compiled by rcc-by-ccc, C by ccc-rustc             190.9 s  ok
   D rcc-by-ccc2        rcc's C compiled by ccc-by-rcc                                 35.1 s  ok
   D ccc-by-rcc2        ccc's Rust compiled by rcc-by-ccc2, C by ccc-by-rcc          1083.4 s  ok
   check generated C of ccc: stage C vs stage D                                  identical (32 files, 75936524 bytes)
   check -O2 assembly of ocean.c and rcc src/*.c: stage C ccc vs stage D ccc     identical for all 29 files
   check -O2 assembly: ccc-by-rcc (C by ccc) vs ccc-by-rcc.clang (C by clang)    identical for all 29 files
   info  -O2 assembly: ccc-rustc vs stage C ccc                                  9 of 29 files byte-identical, 20 differ

== CCGEN=clang: rcc and the generated C compiled by clang
   A ccc-rustc          ccc built by cargo/rustc                                       40.6 s  ok
   B rcc-by-clang       rcc's C compiled by clang                                       0.9 s  ok
   C ccc-by-rcc.clang   ccc's Rust compiled by rcc-by-clang, C by clang                18.6 s  ok
   D rcc-by-ccc2.clang  rcc's C compiled by ccc-by-rcc.clang                            3.2 s  ok
   D ccc-by-rcc2.clang  ccc's Rust compiled by rcc-by-ccc2.clang, C by clang           18.5 s  ok
   (same three checks: identical, identical, identical; 9 of 29 vs ccc-rustc)
```

`make bench` (ocean.c, `--bench 300 --threads 1`):

```
binary                 compiler chain                                compile  ms/frame      best heights           checksum
                                                                     ocean.c       avg
ocean-ccc-rustc        ccc (built by rustc)                            0.6 s     5.171     5.127 -4.714 .. 4.320   0.9752
ocean-ccc-by-rcc       ccc (built by rcc, which was built by ccc)      4.1 s     5.169     5.033 -4.714 .. 4.320   0.9752
ocean-clang            clang                                           0.2 s     4.444     4.414 -4.714 .. 4.320   0.9752

PASS: heights and checksum identical across all 3 binaries (300 frames, 1 thread)
```

and with `CCGEN=clang`:

```
ocean-ccc-rustc        ccc (built by rustc)                            0.6 s     5.162     5.061 -4.714 .. 4.320   0.9752
ocean-ccc-by-rcc.clang ccc (built by rcc; C compiled by clang)         0.9 s     5.163     5.095 -4.714 .. 4.320   0.9752
ocean-clang            clang                                           0.2 s     4.451     4.349 -4.714 .. 4.320   0.9752

PASS: heights and checksum identical across all 3 binaries (300 frames, 1 thread)
```

`make difftest` (`cases` suite): ccc-rustc 216/216 pass, ccc-by-rcc 216/216,
ccc-by-rcc.clang 216/216; no case differs between the builds. (The flaky
case filter was exercised by marking `abi_variadic` as failed in the cached
ccc-rustc results: it was rerun twice with each build and reported as
`flaky`, not as a difference.)

`make selftest`: 333/333 with rcc-by-ccc + ccc-by-rcc (611.6 s, the slow
part being ccc-by-rcc compiling each test's C), 333/333 with
rcc-by-ccc2.clang + ccc-by-rcc.clang (102.3 s).

`make before-after`: the three windows open side by side and render; each
process printed `ocean: Apple M3 Max, 384x384 grid, 24 waves, 1 threads,
compiled with <chain>` after creating its window and Vulkan swapchain.

### Reading the numbers

- **Same compiler.** All binaries print the same heights and checksum, and
  the two ccc builds produce code of the same speed (5.17 ms/frame for both
  at average). The stage-C and stage-D ccc produce byte-identical assembly,
  and so do the stage-C ccc whose C was compiled by ccc and the one whose C
  was compiled by clang. rcc compiled by ccc and rcc compiled by clang emit
  byte-identical C for ccc (75.9 MB in 32 translation units).
- **Fixed point reached.** rcc-by-ccc2, whose C was compiled by a ccc that
  rcc built, emits exactly the C that rcc-by-ccc emits, and the ccc built
  from it compiles everything tested to the same output.
- **ccc vs clang on ocean.c.** At `f5e0da05` ccc's ocean binary takes
  5.17 ms/frame against clang's 4.45. The example's README reports parity
  (about 4.4 ms) for the working tree, which has an uncommitted
  vectorization pass (`src/passes/reduce_vectorize.rs`); this demo builds
  HEAD (`CCC_REV=worktree` builds the working tree instead).
- **Compile speed is where the builds differ.** ccc-rustc compiles ocean.c in
  0.6 s; the ccc built through rcc takes 0.9 s when clang compiled its C and
  4.1 s when ccc did. The second gap is ccc's code generation on rcc's
  output (and rcc currently disables inlining when ccc compiles its C, as a
  workaround for a ccc bug, see docs/STATUS.md "ccc issues met"); the first
  is rcc's C output versus rustc. The same ratio shows in stage D: rcc's
  28 C files take 3.2 s with ccc-by-rcc.clang and 35.1 s with ccc-by-rcc.
- **Why ccc-rustc's assembly differs from the rcc-built ccc's in 20 of 29
  files.** The differences are register and stack-slot assignment and
  instruction order, a few lines more or less per file (for example
  `main.c`: two independent `mov x10, #0` / `mov x11, #0` swapped).
  `build/chain-ccc/asm/` keeps all three versions of each file. ccc's register allocator iterates hash maps in places (e.g.
  `src/backend/regalloc.rs`, `reuse_points.values()`); Rust leaves the
  iteration order unspecified, rustc's std (hashbrown) and rcc's std (an
  insertion-ordered table, `lib/std/src/collections/hash_map.rs`) iterate
  differently, so the allocator makes different, equally valid choices.
  Each build is deterministic on its own. The behavioural checks above
  (checksums, difftest, the selftest) are what show the outputs equivalent.

### History of this run (what failed on the way)

- With the rcc sources of 06:40 the CCGEN=ccc chain built a ccc (stage C)
  that hung on any C function containing a loop, even at `-O0`. Cause: ccc
  miscompiles rcc's C for `ccc::passes::licm::build_value_to_base_alloca`.
  In its fixpoint loop `if map.insert(dest.0, base_alloca) !=
  Some(base_alloca) { changed = true; }`, the inlined `PartialEq::ne`
  (`_3 = eq(a, b); _0 = (!_3); return _0;`) loses its negation in the
  GetElementPtr arm: ccc emits `bl ..eq_7018; tst x0, #0xff; tst x0, #0xff;
  csinc x20, x20, xzr, eq` (the `cset x0, eq` is missing), while the Copy arm
  right below is correct. `changed` is therefore set exactly when nothing
  changed, and LICM never terminates. rcc's maintainers found the same bug in
  parallel and worked around it (rcc no longer lets ccc inline its output);
  with the rcc sources of 07:01 every stage passes. ccc itself still has the
  bug (ccc-rustc at `f5e0da05` and the older `~/dev/ccc/target/release/ccc`
  both emit the broken sequence for that function). A compile-only
  reproducer from that run is kept until `make clean`: the function on its
  own plus the generated header it includes,
  `build/bin/ccc-rustc -O1 -DRCC_CC_IS_CCC -w -I build/repro-licm -S
  build/repro-licm/licm_repro.c -o /tmp/licm.s`, then look at the two calls
  of `PartialEq_for_Option__eq_7018` in
  `_ccc__passes__licm__build_value_to_base_alloca`.
- In the first full `make all`, the CCGEN=ccc selftest had two failures:
  `t11_env_var` (caused by this harness: it exported `RCC_TEST_*` variables,
  which that test lists; fixed) and `t12_elf_writer`, a COMPILE_FAIL where
  ccc-by-rcc exited with status 1 and no message. It did not recur in
  9 later compiles of the same C (one alone, 8 in parallel) nor in the second
  `make all`; the machine was under heavy memory and CPU pressure at the
  time.
