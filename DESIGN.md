# rcc — a Rust compiler written in C

## Goal

One acceptance test: `rcc` compiles the C compiler at `~/dev/ccc` (225k lines of
Rust, std-only, no external crates) into a working `ccc`, and `rcc` itself is C
that `ccc` can compile. That closes the loop: neither tool needs rustc or gcc.

`rcc` is NOT a general Rust compiler. It only has to accept known-good code, so:

- No borrow checker, no lifetime checking (lifetimes are parsed and ignored).
- No diagnostics beyond "what went wrong and where" for our own debugging.
- Only the language features and std APIs that ccc uses (see `docs/INVENTORY.md`).
- Reference semantics are rustc 1.88, edition 2021, `--release` (wrapping
  integer arithmetic, `debug_assertions` off, `cfg(test)` off).

## Hard rules for the C code

- Portable C11, no compiler extensions that `ccc` lacks. Must build warning-free
  with `clang -std=c11 -Wall -Wextra` AND build with `~/dev/ccc/target/release/ccc`.
- Only libc. No third-party libraries, no code generators, no C++.
- Batch compiler: allocate from arenas, never free. No global constructors.
- Every internal error goes through `fatal(span, fmt, ...)` which prints
  `file:line:col` of the Rust source being compiled and exits 1. Never silently
  miscompile: an unsupported construct must `fatal` with a message naming it.
- One binary: `build/rcc`. Built by `make` (clang) or `make CC=...ccc`.

## Pipeline

```
source .rs
  -> lex        (src/lex.c)        tokens with spans
  -> parse      (src/parse.c)      AST (src/ast.h); macro calls kept as token trees
  -> expand     (src/expand*.c)    cfg stripping, `mod foo;` file loading,
                                   macro_rules!, builtin macros, #[derive]
  -> resolve    (src/resolve.c)    module tree, DefIds, use/glob/re-export, locals
  -> typeck     (src/typeck*.c)    item signatures, impl table, trait selection,
                                   inference, method probing, coercions, closures
                                   => typed IR with all adjustments explicit
  -> mono       (src/mono.c)       collect (DefId, substs) instances from `main`
  -> mir        (src/mir*.c)       CFG of basic blocks, places, rvalues, drops
  -> cgen       (src/cgen*.c)      C source, one label per block, goto
  -> cc         ccc or clang compiles the C + runtime (rt/rt.c) to a binary
```

## Decisions already made (do not relitigate)

- **Backend emits C.** No machine code. The C is compiled by `ccc` (or clang
  while developing).
- **Standard library is ours**, written in Rust under `lib/std/`, compiled by
  `rcc` together with the user crate. `core` and `alloc` are aliases of `std`.
  It is NOT the real libstd and uses no unstable features beyond a few
  `#[lang = "..."]` / `#[rcc_intrinsic]` attributes we define. It calls into
  libc and `rt/rt.c` through `extern "C"`. Public API matches real std for
  exactly the subset in the inventory.
- **Full monomorphization.** Generics are instantiated per concrete type.
  `dyn Trait` uses fat pointers with vtables.
- **Layout:** no niche optimisation. Enums are `struct { tag; union { ... } }`.
  `&[T]`/`&str` are `{ptr,len}`; `&dyn T`/`Box<dyn T>` are `{data,vtable}`.
  `u128/i128` map to `__int128`.
- **Panics abort** (print message to stderr, exit 101). No unwinding.
- **Drops:** MIR has explicit drop statements and the design must allow correct
  drop elaboration, but the first milestone may leak memory. Things with
  observable drop behaviour (file flush/close, `impl Drop`, `RefCell` borrow
  guards, `Rc` counts) must be correct from the start.
- **Integer semantics:** wrapping on overflow (emit unsigned arithmetic in C so
  there is no UB); bounds checks and division-by-zero panic as in Rust.
- **Hash maps:** ccc uses an Fx hasher via `BuildHasherDefault`; our `HashMap`
  must honour the supplied hasher. Iteration order need not match hashbrown.

## Testing

- Oracle is real `rustc` (1.88 is installed). `tests/run/*.rs` are small
  programs; expected stdout/exit code is whatever the rustc-built binary prints.
  `tests/run.sh` builds each with `rcc`, runs it, and diffs.
- Parser oracle: `rcc --parse` must accept every `.rs` file in `~/dev/ccc/src`.
- End goal: `rcc` builds ccc; that ccc passes ccc's own `tests/difftest`.

## Working in this repo

- Several agents work here at once. Stay inside the files you were assigned;
  if you need a change in someone else's file, say so in your final report.
- Do not `git commit`; the coordinator commits at checkpoints.
- Never weaken or delete a test to make it pass. Report what fails honestly.
- Keep `docs/STATUS.md` sections for your component current: what works, what
  is known broken, what is next.

## Crates and CLI

`rcc ROOT.rs -o OUT [--extern NAME=LIBROOT.rs]... [--cfg SPEC]... [--emit-c FILE] [--std DIR]`

Whole-program compilation from source: the std crate (`lib/std/src/lib.rs`,
also reachable as `core` and `alloc`), any `--extern` library crates (ccc is a
lib crate `ccc` at `src/lib.rs` plus thin bins), and the root binary crate are
all expanded, resolved, type-checked and monomorphized together into one C
program. The C compiler is `$RCC_CC` (default `cc`). There are no rlibs.

`cfg` values: `test` and `debug_assertions` are false; `unix` true; `target_os`,
`target_arch`, `target_pointer_width`, `target_endian` describe the host;
cargo features are off unless given by `--cfg 'feature="x"'`.

## Contract between the expander and std (builtin macro lowering)

Builtin macros are implemented natively in the expander (std exports no
`macro_rules!`). They lower to ordinary Rust AST that names these std items by
absolute path, so std MUST provide them with these exact shapes:

```rust
// std::fmt
pub struct Arguments<'a> { pub pieces: &'a [Piece<'a>] }
pub enum Piece<'a> { Lit(&'static str), Arg(Argument<'a>, Spec) }
pub enum Argument<'a> {
    Display(&'a dyn Display), Debug(&'a dyn Debug), LowerHex(&'a dyn LowerHex),
    UpperHex(&'a dyn UpperHex), Octal(&'a dyn Octal), Binary(&'a dyn Binary),
    LowerExp(&'a dyn LowerExp), UpperExp(&'a dyn UpperExp), Pointer(&'a dyn Pointer),
}
pub struct Spec { pub fill: char, pub align: Align, pub sign_plus: bool, pub alternate: bool,
                  pub zero_pad: bool, pub width: Option<usize>, pub precision: Option<usize> }
pub enum Align { Unknown, Left, Center, Right }
pub fn format(args: Arguments<'_>) -> String;
```

- `format_args!("a{}b{:>8.3?}", x, y)` =>
  `match (&x, &y) { (__a0, __a1) => ::std::fmt::Arguments { pieces: &[Piece::Lit("a"),
  Piece::Arg(Argument::Display(__a0), Spec{..}), Piece::Lit("b"), Piece::Arg(Argument::Debug(__a1), Spec{..})] } }`
  (all paths absolute). Each argument is evaluated once, in order; `{0}`, `{name}`,
  `{ident}` capture, `{:w$}`, `{:.*}`, `{:.p$}` are resolved by the expander
  (`width`/`precision` become `Some(*__aN)`). `{{`/`}}` are unescaped into `Lit`.
- `format!(..)` => `::std::fmt::format(format_args!(..))`
- `print!/println!/eprint!/eprintln!` => `::std::io::_print(args)` / `::std::io::_eprint(args)`
  (the `ln` forms append `"\n"` to the last literal piece)
- `write!(d, ..)` => `d.write_fmt(format_args!(..))`; `writeln!` likewise with newline
- `panic!(..)` => `::std::rt::panic_fmt(format_args!(..), "file.rs", line, col)` (type `!`);
  `unreachable!`, `todo!`, `unimplemented!` are panics with the standard messages
- `assert!(c, ..)` => `if !(c) { panic }`; `assert_eq!/assert_ne!(a, b, ..)` =>
  `match (&a, &b) { (l, r) => if !(*l == *r) { ::std::rt::assert_failed("==", l, r, ...) } }`
  (operands need `Debug`); `debug_assert*` expand to nothing
- `vec![a, b]` => `{ let mut v = ::std::vec::Vec::with_capacity(2); v.push(a); v.push(b); v }`;
  `vec![x; n]` => `::std::vec::from_elem(x, n)`; `vec![]` => `::std::vec::Vec::new()`
- `matches!(e, p if g)` => `match e { p if g => true, _ => false }`
- `thread_local! { static N: T = init; }` => `static N: ::std::thread::LocalKey<T> =
  ::std::thread::LocalKey::new(__init_fn);` plus `fn __init_fn() -> T { init }`
- `cfg!`, `env!`/`option_env!` (from the process environment or `--env K=V`),
  `concat!`, `stringify!`, `line!`, `column!`, `file!`, `module_path!`,
  `include_str!`, `compile_error!` are evaluated in the expander.
- `#[derive(..)]` for Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
  PartialOrd, Ord generates ordinary impl items naming `::std::...` traits.

## Amendments from the inventory (docs/INVENTORY.md) — these override earlier text

- **Acceptance is behavioural, not byte-identical.** ccc's own output depends on
  hash iteration order (its ELF output differs run to run), so an rcc-built ccc
  is judged by ccc's `tests/difftest` and by the programs it compiles behaving
  correctly. Our default `HashMap::new()` hasher should be deterministic.
- **Panics in a spawned thread must be catchable by `join()`.** ccc runs its
  whole body in a `thread::Builder::new().stack_size(1 GB).spawn(..)` thread,
  joins it, downcasts the panic payload (`Box<dyn Any + Send>`) to `&str` /
  `String`, prints `ccc: internal error: ...` and exits 1 (ccc `src/lib.rs`).
  Implement without unwinding: `panic` in a spawned thread stores the message
  as the payload and `longjmp`s (or `pthread_exit`s) to the thread entry, running
  no destructors; `join()` then returns `Err(payload)`. A panic on the main
  thread prints the message and exits 101. The spawned thread needs a real big
  stack (pthread attr) because ccc recurses deeply.
- **Pointer identity is observable.** ccc uses `expr as *const Expr as usize`
  and instruction addresses as map keys. Places must be accessed in place:
  never copy a value to a temporary where Rust would take its address
  (`&x.f`, `&v[i]`, match scrutinee places, method receivers by reference).
- **`?` converts errors with `From`** at ~500 sites (`&str` -> `String`).
- **Float/int edges:** `f64/f32 as int` saturates (NaN -> 0) and must not be C
  UB; `str::parse::<f64/f32>` must be correctly rounded (use `strtod`/`strtof`
  in `rt/`). Primitive operators on references (`x += &y`, `&a + &b`) occur.
- Syntax seen inside macro arguments that the parser was never shown directly:
  `thread_local! { static X: T = const { .. }; }`; also
  `impl Iterator<Item: AsRef<str>>` (associated-type bounds).

## Expander contract as implemented (authoritative; supersedes the sketch above)

The exact generated shapes are in `docs/STATUS.md`, "Expander / Lowering
shapes". Differences from the original sketch that std and typeck must follow:

- `format_args!` has two forms: inline `Arguments { pieces: &[.. Argument::Display(&(arg)) ..] }`
  when every argument is used once in order; otherwise
  `match (&a0, ..) { (__rcc_a0, ..) => CONSUMER(Arguments{..}) }` with the
  consuming call (`format`, `_print`, `panic_fmt`, `write_fmt`, ...) inside the arm.
- `Argument` also has `DebugLowerHex(&dyn Debug)` and `DebugUpperHex(&dyn Debug)`.
- `Spec` also has `sign_minus: bool`.
- `vec![a, b]` => `::std::vec::from_array([a, b])` (needs
  `pub fn from_array<T, const N: usize>(a: [T; N]) -> Vec<T>`).
- `panic_fmt(Arguments<'_>, &'static str, u32, u32) -> !`;
  `assert_failed(op: &'static str, left: &dyn Debug, right: &dyn Debug,
  args: Option<Arguments<'_>>, file: &'static str, line: u32, col: u32) -> !`.
- `LocalKey::new(__rcc_tls_init_NAME)` must be usable in a static initializer.
- Reserved identifiers: `__rcc_*`; derives use rustc's names (`__self_0`, ...).

## Final acceptance demo (requested by the user): the ocean test

The end result must be shown the way `~/dev/ccc/examples/vulkan-ocean` shows
ccc against clang: a visible, side-by-side run with a checksum, not only a
test-suite number. Deliverable: `demo/` with a Makefile such that

- `make bench` builds ccc twice (once with rustc/cargo, once with `rcc` whose
  own C was compiled by ccc), compiles `ocean.c` with each and with clang, runs
  all three with `--bench N`, and shows identical heights and checksum, plus
  ms/frame and how long each ccc took to compile `ocean.c`.
- `make before-after` opens the windows side by side with the title bar naming
  the compiler chain that built each binary (e.g. `ccc (built by rcc)`).
- The same Makefile runs ccc's `tests/difftest` with the rcc-built ccc.

Expectation to verify: the rcc-built ccc generates code of the same quality
(same ms/frame) since it is the same compiler; only its own compile speed differs.
