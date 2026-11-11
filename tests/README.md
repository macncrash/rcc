# rcc runtime tests

`tests/run/` holds small self-contained Rust programs (`fn main`, std only,
deterministic). Their expected stdout and exit status come from the real
rustc 1.88 oracle, built with

    rustc --edition 2021 -C opt-level=1 -C debug-assertions=off -C overflow-checks=off

so the reference semantics are release semantics (wrapping arithmetic,
`debug_assertions` off, `cfg(test)` off).

## Running

    make                         # builds build/rcc
    tests/run.sh                 # all tests, parallel (one job per CPU)
    tests/run.sh -k t03_         # only tests whose name contains "t03_"
    tests/run.sh -j 4            # 4 parallel jobs
    tests/run.sh --cc clang      # sets RCC_CC=clang for rcc's C backend
    tests/run.sh --rcc PATH      # use another compiler binary (default build/rcc)
    tests/run.sh -v              # also list passing tests

For each `NAME.rs` the harness runs `rcc NAME.rs -o BIN` (120s limit,
`COMPILE_TIMEOUT`), then runs `BIN` with a 10s limit (`RUN_TIMEOUT`) in a fresh
empty working directory, and compares stdout and exit status. Each test is
classified as

| result        | meaning                                                      |
|---------------|--------------------------------------------------------------|
| PASS          | stdout and exit status match                                 |
| COMPILE_FAIL  | rcc exited nonzero, timed out, or produced no executable     |
| RUN_FAIL      | wrong stdout or wrong exit status                            |
| TIMEOUT       | the program did not finish within the run time limit         |

One line is printed per failing test (first compiler error line, or the first
differing stdout line), then a per-tier summary table and a total. Full logs
(compile output, stdout, stderr, diff) are in `build/test-logs/NAME.log`. The
script exits nonzero if any test fails. Stderr is never compared (panic
messages only need to go to stderr; the exit code 101 is what is checked).

Only bash 3.2, perl and stock macOS/Linux tools are needed (no GNU `timeout`).

## Files per test

| file            | required | purpose                                                     |
|-----------------|----------|-------------------------------------------------------------|
| `NAME.rs`       | yes      | the program                                                 |
| `NAME.expected` | yes      | exact expected stdout (generated, do not edit by hand)      |
| `NAME.exitcode` | no       | expected exit status when nonzero (e.g. 101 for a panic)    |
| `NAME.args`     | no       | command-line arguments, one per line                        |
| `NAME.stdin`    | no       | fed to stdin (otherwise stdin is `/dev/null`)               |
| `NAME.env`      | no       | `KEY=VALUE` lines added to the environment                  |

## Tiers

Names are `tNN_topic.rs`. Lower tiers never need features from higher ones, so
the compiler can be brought up tier by tier. In particular t00-t03 use no
`Vec`, `String`, traits, generics or format specs beyond `{}`.

| tier | topic |
|------|-------|
| t00  | `fn main`, `println!` with `{}`, integer types and arithmetic, `as` casts, `wrapping_*`, bool, bit ops, shifts, u128/i128, char, basic f64, control flow, functions, recursion |
| t01  | tuples, arrays, indexing, `for` over ranges, labelled break/continue, loop values, shadowing, block expressions, consts/statics, `&`/`&mut` to locals |
| t02  | structs, impl blocks, enums of all variant kinds, `match` with every pattern form, `if let`, `while let`, `let else`, `matches!` |
| t03  | Option/Result (`?`, combinators), Box and recursive types, slices, `&str` basics, slice patterns |
| t04  | generics, traits (default methods, associated types/consts, supertraits, bounds, where), `impl Trait`, operator overloading, Default/Clone/Copy/From/Into, derives |
| t05  | closures (Fn/FnMut/FnOnce, captures, returned/stored closures, fn pointers), trait objects |
| t06  | Vec and String methods, iterator adapters and consumers, hand-written `Iterator` |
| t07  | HashMap/HashSet (incl. Fx hasher via `BuildHasherDefault`), BTreeMap/BTreeSet, VecDeque, BinaryHeap, Rc/Weak, RefCell/Cell, Cow, `std::mem` |
| t08  | formatting: `{:?}`/`{:#?}`, width/fill/align/precision/sign/radix, named/positional/captured args, Display/Debug impls, `fmt::Write`, float and integer printing edge cases |
| t09  | numerics: checked/wrapping/saturating/overflowing ops, bit methods, byte conversions, float methods, every `as` cast edge case, 128-bit arithmetic |
| t10  | modules/`use`/visibility, `macro_rules!`, `#[cfg]`/`cfg!`, thread_local, LazyLock/OnceLock/Once, atomics, drop order |
| t11  | OS surface: args, env, fs, stdin/stdout/stderr, BufReader/BufWriter, `Command`, `process::exit`, panics, Path/PathBuf, threads with big stacks, Instant |
| t12  | integration programs in the style of ccc (lexer/parser/evaluator, VM, SSA IR passes, register allocation, preprocessor, Rc<RefCell> graphs, union-find, ELF byte encoder), some code lifted from `~/dev/ccc/src` |

## Adding a test

1. Write `tests/run/tNN_topic.rs` in the lowest tier whose features it needs.
   Keep it focused on one topic so a failure points at the missing feature,
   and print intermediate values (not just "ok") so a miscompile changes the
   output.
2. Rules: std only, edition 2021, compiles on rustc 1.88; output must not
   depend on addresses, hash iteration order (sort before printing), timing,
   randomness, absolute paths or the host environment; output must not differ
   between debug and release (except where a comment says it deliberately
   relies on `overflow-checks=off` wrapping). File-system tests use relative
   paths inside the scratch working directory and clean up after themselves.
3. Add `.args` / `.stdin` / `.env` files if needed.
4. Run `tests/gen_expected.sh tNN_topic` to create `NAME.expected` (and
   `NAME.exitcode` if nonzero). It compiles with the oracle flags in
   `build/oracle/`, runs the binary twice in fresh scratch directories and
   refuses to write the expectation if the two runs differ.
   `tests/gen_expected.sh` with no arguments regenerates everything.
5. Never hand-edit `.expected` files, and never weaken a test to make rcc pass.
