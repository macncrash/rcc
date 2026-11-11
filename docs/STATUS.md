# rcc status

Each component owner keeps their own section current.

## Front end (lexer, AST, parser, AST dump)

Files: `src/util.{h,c}`, `src/lex.{h,c}`, `src/ast.h`, `src/parse.{h,c}`,
`src/astdump.c`, `src/main.c`, `Makefile`, `tests/parse.sh`, `tests/parse/`.

### What works

- `make` builds `build/rcc` with clang (`-std=c11 -Wall -Wextra -O1 -g`) with no
  warnings. `make CC=~/dev/ccc/target/release/ccc` builds a working binary.
  Switching CC rebuilds everything automatically through a stamp file.
- `rcc --tokens F`, `rcc --parse F`, `rcc --dump-ast F`, and
  `rcc --check-macros F` (re-parses std macro arguments through the
  token-stream API).
- `tests/parse.sh [RCC] [SRCDIR]`: runs `--parse` on all 389 `.rs` files
  under `~/dev/ccc/src` plus `tests/parse/*.rs`, then diffs the golden dumps
  `tests/parse/*.ast`.
  - clang-built rcc: 395/395 files parse, 6/6 golden dumps match.
  - ccc-built rcc: 395/395 files parse, 6/6 golden dumps match. Its
    `--dump-ast` and `--tokens` output is byte-identical to the clang build on
    all 395 files.
- Checks that the parse is not vacuous, run over all of ccc/src:
  - The number of `fn NAME` token pairs equals the number of `(fn` nodes in
    the dump, except for 5 files. In those files the missing fns are inside
    macro calls or `macro_rules!` bodies (`delegate_to_impl!`), which are not
    expanded yet.
  - Keyword tokens and dump nodes match exactly for match (3183), loop, while,
    enum, impl, and trait. `let` has 29723 tokens and 29718 nodes; the 5
    missing are inside macro_rules bodies.
  - `--check-macros` re-parses the arguments of 7524 calls to vec!, format!,
    assert_eq!, matches! and similar macros with `parser_expr`/`parser_pat`.
    None fail.
- I checked the golden dumps by hand: operator precedence and associativity
  (`tests/parse/precedence.rs`), splitting `>>`, `>=` and `>>=`, turbofish,
  qualified paths, `dyn`/`impl` bounds, struct literals in conditions,
  closures, `let else`, labels, block-like statements, every pattern form,
  every item kind, use trees, visibilities, every literal form and escape,
  lifetimes vs chars, and raw identifiers.
- A scratch C test checked that an interpolated `NT_EXPR` token behaves like a
  parenthesised expression (`$a * $b` with a=`1+2` gives `(* (+ 1 2) (+ 3 4))`).
  It also exercised `parse_assoc_items_from_tokens` and `parse_stmts_from_tokens`.

### Deliberately unsupported (each one fatals with a message naming it; none occurs in ccc)

async/await, `try` blocks, inline `const { }` blocks, `static` closures,
`box` patterns, `~const` bounds, trait aliases, bare trait objects without `dyn`
(`(Trait) + Send`, `for<'a> Trait`), `#[unsafe(...)]` attributes, `default fn`
(specialization), macros 2.0. Doc comments are dropped by the lexer, so there
are no `#[doc]` attributes.

### Known gaps / unverified

- Only `NT_EXPR` interpolation has been tested. The `NT_TY`, `NT_PAT`, `NT_STMT`,
  `NT_ITEM`, `NT_BLOCK`, `NT_PATH`, `NT_VIS` and `NT_LIT` paths are written but
  not exercised. `parser_meta`, `parser_vis` and `parser_stmt` are also not
  exercised.
- Error recovery: none. The first error is fatal.
- Spans: every node has start..end. Line and column are only exact for the
  start. Spans of split tokens (`>>` split into `>` `>`) are adjusted by one byte.
- Rust-2024-only syntax (`unsafe extern`, `safe fn`) is accepted, but only
  lightly.

### ccc bugs hit

None. All front-end C code compiles and runs identically under ccc.

### Next

- Front-end support for the expander as it needs it, for example more
  incremental-parser entry points or keeping doc comments as attributes.

## Expander (cfg, modules, macro_rules!, builtin macros, derives)

Files: `src/expand.h` (API), `src/expand_int.h` (internal), `src/expand.c`
(driver: cfg, `mod foo;` loading, the walk, macro resolution, cleanup and
NodeId check), `src/expand_mbe.c` (macro_rules! matcher/transcriber,
local-variable hygiene), `src/expand_builtin.c` (builtin macros, format
strings), `src/expand_derive.c`, `src/expand_clone.c` (deep copies with fresh
NodeIds), `src/astprint.c` (AST -> Rust source), `--expand` in `src/main.c`,
`tests/expand.sh`, `tests/expand_ccc.sh`, `tests/expand/`.

API: `Crate *expand_crate(root_path, crate_name, &ExpandOptions)`. The result
has no macro calls, no `macro_rules!` items, no `#[cfg]`/`#[cfg_attr]`/
`#[derive]`/`#[test]`/`#[path]`/`#[macro_use]`/`#[macro_export]`/`#[default]`
attributes; every `mod foo;` is an inline module whose items keep spans into
their own file (the file's `#![...]` attrs are appended to the module item
with `inner = true`). `use crate::m;` imports of `#[macro_export]` macros are
removed. Every node has a unique NodeId (checked by a final pass). Nodes made
by builtins get the macro call's span; derived impls get the derive
attribute's span; macro_rules output keeps the spans of the macro body tokens.

CLI: `rcc --expand ROOT.rs [--expand-compat] [--fmt-root PATH]
[--crate-name N] [--cfg SPEC]... [--env K=V]...` prints the expanded crate.
`--cfg 'target_os="linux"'` *replaces* the host value of a `target_*` key.
`--expand-compat` leaves the format/panic/assert families and
`thread_local!` as macro calls (arguments expanded) and keeps
`#[derive(PartialEq)]` (rustc only allows constants in patterns for the
built-in derive), so that the output builds with real rustc. `--fmt-root P`
makes the formatting-contract paths `P::fmt::...` etc. (used with the std
mock).

### What works (all verified against rustc 1.88)

- cfg: `all/any/not`, names, `key = "value"`; `cfg_attr` (nested, multiple
  attrs); on items, impl/trait items, fields, variants, tuple fields, fn
  params, statements, tail-position blocks, array/tuple/call elements,
  struct-literal fields, match arms, generic params, `use`, modules (before
  loading the file), inner `#![cfg]`; `cfg!()`. Host values: `unix`,
  `target_family="unix"`, `target_os`, `target_arch`, `target_vendor`,
  `target_env`, `target_pointer_width`, `target_endian`, `panic="abort"`;
  `test`/`debug_assertions` false. `#[test]`/`#[bench]` fns dropped.
- Modules: `foo.rs`, `foo/mod.rs`, nested directories for non-mod.rs files,
  inline modules containing file modules, `#[path]`, missing-file error.
- macro_rules!: all fragment kinds (expr ident ty tt pat pat_param literal
  block stmt item path lifetime vis meta), `* + ?` with separators and
  nesting, multiple rules, recursion (tt munchers; limit 1024), all
  positions (expr, stmt, items, impl/trait items, pattern, type), macros
  defining macros, `$crate`, textual scoping incl. shadowing and fn-local
  macros, `#[macro_export]` (crate-wide, by path `crate::m!`, also before
  the definition), `#[macro_use] mod`. Def-site hygiene for local variables
  bound in a macro body (renamed `__rcc_h<N>_<name>`); other identifiers are
  unhygienic, as in rustc's mixed-site hygiene for items/fields/methods.
- Builtins: format_args format print println eprint eprintln write writeln
  panic unreachable todo unimplemented assert assert_eq assert_ne
  debug_assert* (dropped, argument not evaluated) vec matches thread_local
  (incl. `const { }` init) cfg env option_env concat stringify line column
  file module_path include_str include_bytes compile_error dbg; also via
  `std::`/`core::` paths. Format strings: the whole std::fmt grammar;
  format strings from `concat!` or forwarded through macro_rules (`$fmt:expr`,
  `$($arg:tt)*`, literal fragments).
- Derives: Debug Clone Copy PartialEq Eq Hash Default (incl. `#[default]`)
  PartialOrd Ord, on unit/tuple/named structs, enums of all variant kinds
  (explicit discriminants and `#[repr(int)]` respected), unions (Clone/Copy),
  generics with lifetimes, bounds, const params and where clauses. `{:?}`,
  `{:#?}` and DefaultHasher values are identical to rustc's derives.

### Tests

`tests/expand.sh [--rcc PATH] [unit|run|ccc]` (see its header). Results with
both the clang-built and the ccc-built rcc (identical; the ccc-built binary's
expansion of all of ccc is byte-identical to the clang build's):
- unit (`tests/expand/`, 20 programs incl. a multi-file module tree):
  expand+reparse 20/20, compat round trip 20/20, std-mock round trip 18/18
  (2 programs pass std::fmt::Arguments to user code and skip the mock).
- run (`tests/run/*.rs`, 333): expand+reparse 333/333, compat round trip vs
  `.expected` 333/333, std-mock round trip (the full lowering, run against a
  mock of the std contract) 331/331 (2 programs pass std::fmt::Arguments to
  user code and are skipped).
- ccc: lib, main and a bin expand with zero errors and zero remaining macro
  calls under the host cfg and under `target_os="linux"
  target_arch="x86_64"`, full and compat, and re-parse.
- `tests/expand_ccc.sh`: the compat expansion of ccc (lib + 5 bins, one
  file each, ~170k lines) builds with `cargo build --release` under
  /private/tmp, and the rebuilt ccc compiles (`-O2`) and runs all 217
  difftest cases (`tests/difftest/cases/*.c`, with `.helper.c` files)
  identically to the original ccc, except `abi_variadic`, whose output
  differs between two runs of the original ccc too.

### Lowering shapes (for resolve/typeck/std; R = `::std`)

Changes to the DESIGN.md contract are marked CHANGED.
- `format_args!(..)` => `R::fmt::Arguments { pieces: &[P, ..] }` with
  `P = R::fmt::Piece::Lit("text")` (adjacent text merged, `{{`/`}}`
  unescaped, empty text dropped) or
  `R::fmt::Piece::Arg(R::fmt::Argument::KIND(REF), R::fmt::Spec { fill: 'c',
  align: R::fmt::Align::{Unknown|Left|Center|Right}, sign_plus: b, alternate:
  b, sign_minus: b, zero_pad: b, width: W, precision: W })` (CHANGED:
  `sign_minus` added for the `{:-}` flag, observable through
  `Formatter::sign_minus()`; tests/run/t08_display_impl needs it), `W` = `::std::option::Option::
  None` | `::std::option::Option::Some(5usize)` | `...Some(*__rcc_aN)` |
  `...Some(ident)` (captured count). KIND is Display Debug LowerHex UpperHex
  Octal Binary LowerExp UpperExp Pointer, and CHANGED: `DebugLowerHex` /
  `DebugUpperHex` for `{:x?}` / `{:X?}` (both `&'a dyn Debug`).
  CHANGED (two forms): if every explicit argument is used exactly once, as a
  value, in argument order, REF is `&(arg)` inline and there is no match
  (captured `{x}` are `&x`). Otherwise
  `match (&a0, &a1, .., &cap0, ..) { (__rcc_a0, __rcc_a1, ..,) => CONSUMER(ARGS) }`
  with REF = `__rcc_aN`. The consumer (format/_print/panic_fmt/write_fmt/
  assert_failed call) is INSIDE the arm, so the `&[..]` temporary never
  outlives the arm. Only a bare `format_args!` in match form returns
  Arguments out of the arm (the temporary must then live to the end of the
  enclosing statement).
- `format!` => `R::fmt::format(ARGS)`; `print!/println!` => `R::io::_print(ARGS)`;
  `eprint!/eprintln!` => `R::io::_eprint(ARGS)`; `ln` adds `"\n"` to the last
  literal piece (or a new one).
- `write!(d, ..)` => `d.write_fmt(ARGS)` (CHANGED: in match form `d` is
  evaluated after the arguments); `writeln!(d)` writes `"\n"`.
- `panic!(..)` => `R::rt::panic_fmt(ARGS, "file.rs", LINEu32, COLu32)`, type `!`,
  with `fn panic_fmt(Arguments<'_>, &'static str, u32, u32) -> !`.
  `panic!()` = "explicit panic"; `unreachable!()` = "internal error: entered
  unreachable code" (`: msg` appended with args); `todo!` = "not yet
  implemented"; `unimplemented!` = "not implemented". The location is the
  outermost user-written macro call.
- `assert!(c)` => `if !(c) { R::rt::panic_fmt(..."assertion failed: c"...); }`
  (or the given message); type `()`.
- `assert_eq!(a, b[, fmt..])` => `match (&a, &b) { (__rcc_l, __rcc_r) => { if
  !(*__rcc_l == *__rcc_r) { R::rt::assert_failed("==", __rcc_l, __rcc_r,
  ::std::option::Option::None /* or Some(ARGS) */, "file.rs", Lu32, Cu32); } } }`;
  `assert_ne!` uses `if *__rcc_l == *__rcc_r` and `"!="`. Signature:
  `fn assert_failed(op: &'static str, left: &dyn Debug, right: &dyn Debug,
  args: Option<Arguments<'_>>, file: &'static str, line: u32, col: u32) -> !`.
- `vec![]` => `::std::vec::Vec::new()`; `vec![x; n]` => `::std::vec::from_elem(x, n)`;
  CHANGED: `vec![a, b]` => `R::vec::from_array([a, b])` with
  `pub fn from_array<T, const N: usize>(a: [T; N]) -> Vec<T>`. The push
  sequence of the original contract loses coercions to the expected element
  type (`let v: Vec<Box<dyn Fn()>> = vec![Box::new(..), ..]`, 8 programs in
  tests/run); with an array literal the expected type reaches each element.
  `#[cfg]` on elements is honoured.
- `matches!(e, p if g)` => `match e { p if g => true, _ => false }`.
- `thread_local! { [attrs] [vis] static N: T = [const] init; }` =>
  `[attrs] [vis] static N: R::thread::LocalKey<T> = R::thread::LocalKey::new(__rcc_tls_init_N);
  fn __rcc_tls_init_N() -> T { init }` (`LocalKey::new` must be a `const fn`).
- `dbg!(e)` => `match e { __rcc_tmp => { ::std::eprintln!(...); __rcc_tmp } }`.
- `option_env!` => `::std::option::Option::Some("v")` or
  `::std::option::Option::None::<&'static str>`. `line!/column!` are `u32`
  literals (`12u32`), the others string literals (`include_bytes!` a byte
  string).
- Derives (`impl<params + T: ::std::TRAIT> ::std::TRAIT for Name<args> where ..`):
  Clone `fn clone(&self) -> Self` (fieldwise `::std::clone::Clone::clone(&self.f)`,
  or `*self` when Copy is derived too and there are no type params, or for
  unions); Copy/Eq empty impls; Debug `fn fmt(&self, f: &mut
  ::std::fmt::Formatter<'_>) -> ::std::fmt::Result` using
  `f.debug_struct("N").field("a", &self.a).finish()`, `f.debug_tuple(..)
  .field(&self.0).finish()`, `f.write_str("N")`; PartialEq `fn eq(&self,
  other: &Self) -> bool` (`self.a == other.a && ..`; enums `match (self,
  other) { (E::V(__self_0), E::V(__arg1_0)) => *__self_0 == *__arg1_0, ..,
  _ => false }`); Hash `fn hash<__H: ::std::hash::Hasher>(&self, state: &mut
  __H)` (enums with >1 variant first hash `let __self_discr = match self {
  E::A => 0isize, E::B(..) => 1isize, .. }` — discriminant values, typed by
  `#[repr]`); Default `fn default() -> Self`; PartialOrd/Ord: lexicographic
  `match ::std::cmp::PartialOrd::partial_cmp(&self.a, &other.a) {
  ::std::option::Option::Some(::std::cmp::Ordering::Equal) => .., cmp => cmp }`,
  enums compare `__self_discr`/`__arg1_discr` when variants differ.
- Reserved identifiers (all start with `__rcc_`): `__rcc_a<N>`, `__rcc_l`,
  `__rcc_r`, `__rcc_tmp`, `__rcc_tls_init_<NAME>`, `__rcc_h<N>_<name>`
  (hygienic macro locals). Derives use rustc's own names (`__self_<i>`,
  `__arg1_<i>`, `__self_discr`, `__arg1_discr`, `__H`, `f`, `other`,
  `state`, `cmp`).

### Known gaps / unverified

- Hygiene is only for local variables bound by a pattern inside one macro
  expansion (lowercase names), renamed at the token level, plus implicit
  captures in format strings written in the macro body. Labels are
  unhygienic. Wrong cases: a body that binds `x` and also refers to an
  outer `x` or calls a function `x` in the same expansion
  (`let len = len(v);`), or declares a struct field named like one of its
  locals. (Struct-literal and pattern field names, shorthand included, are
  handled.)
- Matching is greedy: a repetition never gives back iterations to let the
  rest of the rule match (rustc rejects such ambiguous macros anyway).
  Parse errors inside a started fragment are fatal, as in rustc.
- `stringify!` follows the source spacing (rustc normalises some spacing);
  only identifiers are stringified in ccc. Assertion-failure messages use
  astprint's rendering of the condition.
- Cross-crate macros (a bin using a lib's `#[macro_export]` macro) are not
  supported: each crate is expanded on its own. ccc does not need it.
- `format!("{}", *s)` with `s: &str` cannot coerce `&str` to `&dyn Display`
  under the contract (unsized); rustc accepts it. Not seen in ccc.
- No `#[derive]` of other traits; `derive(Debug/PartialEq/Hash/Default/
  PartialOrd/Ord)` on unions fatal.
- Not exercised: `include_bytes!` on non-UTF-8 data (works on the test file),
  `pub(in path)` visibility fragments.

### Next

- Coordinate the CHANGED items above with std/typeck.

## Core (resolve, typeck, mono, MIR, cgen, rt, std core)

Files: `src/types.*`, `src/resolve.*`, `src/typeck*.{c,h}`, `src/thir.h`,
`src/mir*.{c,h}`, `src/mono.c`, `src/cgen.*`, `src/driver.*`, `rt/`,
`lib/std/`, `tests/core/`. Architecture and how-tos: `docs/ARCHITECTURE.md`.

### What works

`build/rcc ROOT.rs -o OUT` runs the whole pipeline (expand -> resolve ->
typeck -> mono/MIR -> C -> `$RCC_CC`) for a root crate plus std (and
`--extern` crates, untested). Debug flags: `--emit-c F`, `--dump-resolve`,
`--dump-types`, `--dump-mir`, `--check-all`, `--check-std`, `--no-cc`,
`--no-std`.

`tests/run.sh` (333 programs), same numbers for all three configurations
(clang-built rcc + clang; clang-built rcc + ccc; ccc-built rcc + ccc) unless
noted:

| tier | pass | note |
|---|---|---|
| t00 | 27/27 | |
| t01 | 21/21 | |
| t02 | 25/25 | |
| t03 | 23/23 | |
| t04 | 33/33 | |
| t05 | 22/22 | |
| t06 | 39/39 | |
| t07 | 26/26 | |
| t08 | 22/22 | |
| t09 | 25/25 | |
| t10 | 33/33 | |
| t11 | 22/22 | |
| t12 | 15/15 | |
| total | 333/333 | all three configurations |

Language (all exercised by the corpus): items of every kind incl. nested
in fn bodies; modules, `use` with globs/renames/re-exports, std prelude;
structs/enums/unions(basic), generics, const generics (`[T; N]` impls,
`fn f<const N: usize>`), traits with default methods, associated
types/consts, supertraits, generic methods, blanket impls, where clauses,
`impl Trait` in argument and return position, `dyn Trait` (vtables,
`&dyn`/`Box<dyn>`/`Rc<dyn>` unsizing, `self: Box<Self>` methods, `Box<dyn
FnOnce>`), operator overloading, `Deref`/`DerefMut`, `Index`/`IndexMut`,
closures (Fn/FnMut/FnOnce inference, capture by ref/mut/value as rustc incl.
edition-2021 disjoint by-value captures, closures as fn pointers, returned
and stored closures), fn pointers, method probing (autoderef, autoref,
unsizing, inherent before trait, traits in scope), all coercions listed in
the task, integer/float literal inference with fallback, `as` casts,
patterns of every kind with default binding modes, `if let`/`while
let`/`let else`/`?` (with `From`), labeled breaks, loops with values, `for`
over any `IntoIterator`, u128/i128, statics/consts (scalar consts inlined,
others initialized before `main` in dependency order, constant-expression
promotion), drops: scope-based with drop flags, partial moves, temporaries
with rustc's lifetime rules (`impl Drop`, `Rc`, `RefCell` guards all behave).

std (lib/std): marker, ops (all operator traits, Deref, Index, Fn*, ranges,
ControlFlow/Try), clone, cmp, default, convert, option, result, mem, ptr,
alloc, boxed, num (all integer and float inherent methods the inventory
lists and most others; parse errors), char, iter (traits, all common
adapters and sources, Step-based ranges), slice (methods, sort, iterators,
SliceIndex, join/concat), str (methods, patterns, iterators, FromStr),
string, vec, array, fmt (full contract incl. `{:#?}`, padding, radix,
shortest round-trip floats, exponent forms), io (`_print`/`_eprint` only),
rt (panic_fmt, assert_failed, Termination, lang start), process (exit,
abort), hash (traits, `BuildHasherDefault`), borrow (Borrow, ToOwned, Cow),
any (TypeId, Any, downcasts), cell (Cell, RefCell, OnceCell), rc (Rc, Weak),
hint, f32/f64 constants.

### Not yet (by owner)

std modules still to write (they need no core changes): `collections`
(HashMap/HashSet with RandomState + DefaultHasher (SipHash-1-3), BTreeMap/
BTreeSet, VecDeque, BinaryHeap), `sync` (Arc, Mutex, Once/OnceLock/LazyLock,
atomics), `thread` (spawn/Builder/JoinHandle, `LocalKey` for
`thread_local!`, panic payload capture per DESIGN.md), `env`, `fs`, `io`
(Read/Write/BufRead, stdin/stdout/stderr, BufReader/BufWriter, Error),
`path`, `process::Command`, `time`, `error::Error`, `ffi::OsStr/OsString`.

Core gaps (each fatals with a message unless noted):
- Trait upcasting (`&dyn Sub` -> `&dyn Super`). Not used by ccc.
- Byte-string patterns against arrays. Not used by ccc.
- `union` beyond plain field access; `#[repr]` only affects enum tag types.
- `dyn Trait` with more than one non-auto trait; generic associated types.
- Unsized struct tails (hence `Rc` keeps counts in a second allocation, and
  std's `Path`/`OsStr` are sized views behind thin pointers).
- Zero-sized types have size 0 in struct layout and `size_of`, but a local
  or array element of a ZST still takes one byte in C (pointer arithmetic
  on `*const ZST` therefore advances by 1; nothing depends on it).
- `char` classification / case mapping tables are partial (ASCII, Latin-1,
  Greek, Cyrillic, common CJK ranges); `{:?}` escaping of non-printable
  chars covers the common ranges only.
- No visibility checking except private fields during field lookup; no
  borrow checking; lifetimes ignored; minimal diagnostics.
- Match lowering is still arm-by-arm; chains of single-value tests of the
  same place are folded into one `switch` afterwards
  (`mir_build.c: merge_switch_chains`), string arms stay sequential.
- Trait selection results are cached only for closed trait refs (not a
  bottleneck: all 7336 bodies of ccc type-check in under a second).

### ccc bring-up (phase 2)

Done since the first handover (regression tests in `tests/core/`, run with
`tests/core.sh`; `tests/core.sh --add F.rs` adds one with a rustc-generated
`.expected`):
- Constant patterns of aggregate type compare with `PartialEq::eq`
  (`const_pat_aggregate.rs`); destructuring assignment incl. `_`, `..`,
  nested tuples, tuple structs, struct and slice forms; `let (a, b);`
  (`destructuring_assign.rs`).
- Typeck fixes found by `--check-all` on ccc (`typeck_ccc_batch1.rs`): the
  trait of the enclosing `impl Trait for T` is in scope for method calls;
  `cond as T` no longer pushes the cast type into `if`/`match` operands
  (only literals take it as a hint); by-value methods skip unsized receivers
  (`slice[a..b].try_into()`); `&{integer}` operands of builtin arithmetic;
  `map[&mut key]`; coercion sites resolve pending obligations before
  unifying an unknown source type; LUB coercion of `if`/`match` arms
  (`if c { &string } else { "" }`).
- Drop elaboration now runs an initializedness dataflow (maybe / definitely
  initialized per droppable local, `mir_drop.c: flow_analyze`): drops that
  can never see an initialized value are removed (the `else` path of
  `let .. else`, early returns before a deferred initialization), drops of
  definitely initialized values are unconditional, and only the remaining
  locals (and partially moved ones) get runtime drop flags. This replaced
  the builder's `maybe_uninit` heuristic, which dropped uninitialized
  locals (`drop_flow_uninit.rs`, `let_else_drops_uninit_binding.rs`).
- A closure is `Copy`/`Clone` only if its captures are
  (`option_of_closure_is_copy.rs`).
- Expander: an identifier that follows `fn`/`struct`/`enum`/`trait`/`type`/
  `mod` in a macro body is never hygiene-renamed (`fn min` next to a local
  `min`; `tests/expand/mbe_hygiene_fn_name.rs`); the `lo`/`hi` workaround in
  `num/float.rs` is reverted.
- std (language level): `bool::then`/`then_some` (`bool.rs`), `Clone` for
  the remaining iterator adapters.
- Scale: `RCC_TIME=1` prints CPU time per phase, peak RSS and instance
  counts. Programs with >= 3000 instances are emitted as several C
  translation units sharing one header (`OUT.rcc.c.h`, `OUT.rcc.c.N.c`)
  and compiled in parallel (`RCC_UNITS`, `RCC_JOBS`; `cgen_program_split`).
  `--check-all` forks only after a failing body (2.5 s for ccc instead of
  60 s).

### ccc bring-up: results

Commands (std + ccc lib + ccc bin are compiled together):

    build/rcc ~/dev/ccc/src/lib.rs --crate-name ccc \
        --env CARGO_MANIFEST_DIR=$HOME/dev/ccc --check-all        # type-check only
    RCC_TIME=1 RCC_CC=clang build/rcc ~/dev/ccc/src/main.rs --crate-name ccc_bin \
        --extern ccc=$HOME/dev/ccc/src/lib.rs \
        --env CARGO_MANIFEST_DIR=$HOME/dev/ccc -o ccc             # build

- `--check-all`: **0 of 7336 bodies fail** (about 2 s).
- rcc on ccc (arm64 macOS, clang-built or ccc-built rcc, same numbers):
  expand 0.2 s, typeck + mono + MIR 1.3 s, cgen 0.8 s, peak RSS 3.5 GB;
  48,580 instances, 3000 globals, 240 vtables, 2694 drop glues. Generated
  C: one 33 MB header (926k lines, of which 39.7k small `static inline`
  functions) + 32 units, 44 MB / 1.68M lines. The C written by the
  clang-built rcc and by the ccc-built rcc is byte-identical.
- C compile + link, 16 cores: clang -O1 about 160-220 CPU-s, 16-25 s wall;
  ccc -O1 about 800 CPU-s, 1-3 min wall (machine was loaded). `rcc`
  itself builds with ccc (`make CC=.../ccc`) in 8 s.
- The rcc-built ccc works: hello world; ccc's `tests/difftest` (run from a
  scratch copy, `difftest.py --ccc BINARY`): 1927 tests, the same status
  for every test as the rustc-built ccc once load-induced timeouts are
  re-run: 1788 pass, 1 `wrong_output` (`gcc-torture/return-addr`, in both),
  138 `ref_fail`. (First full run: both binaries 1782 pass / 6 timeouts,
  12 different tests timed out, all 12 pass when re-run alone.)
- `-O2 -S` of the 217 difftest cases: rcc-built vs rustc-built ccc 142
  byte-identical, 75 differ in register choice / instruction order (hash
  iteration order, as INVENTORY 7.1 predicted); clang-compiled vs
  ccc-compiled rcc-built ccc: **217 of 217 byte-identical**.
- ccc's ocean example (`--bench`): identical heights and checksum and the
  same ms/frame for all three ccc builds. Compiling ocean.c (-O2) takes
  0.5 s with the rustc-built ccc, 0.9 s with the rcc-built one (clang -O1;
  it was 2.6 s before small functions were emitted `static inline`) and
  about 5 s with the ccc-compiled one.
- Loop closed: rcc built by ccc (`make CC=.../ccc`), ccc built by that rcc
  with ccc as `RCC_CC`. That ccc-compiled ccc writes byte-identical
  assembly to the clang-compiled one on all 217 cases (see above) and on
  the full difftest has the same status as the rustc-built ccc for every
  test except two `cases` whose *compile* hit the harness's 60 s limit on
  the loaded machine (`large_always_inline`, `neon_intrinsics`: 25-29 s
  CPU each, against 1.7 s for the rustc-built ccc; ccc -O1 code is about
  9x slower than clang -O1 code here). Stage 3: rcc compiled by the
  rcc-built ccc generates the same C for ccc again (md5 equal) and passes
  `tests/core.sh` with that ccc as `RCC_CC`.
- `tests/run.sh`: 333/333 with clang-built rcc + clang, clang-built rcc +
  ccc, and ccc-built rcc + ccc. Use `RCC_RUN_LOGS` / `RCC_RUN_WORK` to give
  a run private log and work directories (concurrent runs otherwise
  overwrite each other's results), and never reuse an output path for
  binaries you execute right after rebuilding (macOS ran a stale copy).
- Memory: MIR of all instances stays alive until cgen (3.5 GB peak; it was
  4.8 GB before `MirStmt` lost three embedded operands). Next steps if it
  matters: union `Place`/`MirConst` inside `Operand`, emit and free MIR
  per instance.

### ccc issues met (repros in `tests/core/ccc_bugs/`)

- **One ccc pass explains every `bool`/comparison miscompilation found so
  far**: the arm64 peephole pass `drop_mask_of_narrow_value`. With
  `CCC_PEEPHOLE_SKIP=drop_mask_of_narrow_value` (ccc's own bisecting knob)
  all of the repros below are compiled correctly, at every optimization
  level. Symptom in the assembly: a `cset xN, cc` that materializes a
  comparison is rewritten/removed and a later `tst w0, #0xff`, `cmp wA,
  w0, uxtb` or `csel` reads a stale x0. It fires at -O0 too.
  - `bool_field_compare.c` (runnable, 60 lines): `eq` of two structs with
    `_Bool` fields returns false for equal values (shape of a derived
    `PartialEq`).
  - `bool_fn_inlined_cset_dropped.c` (compile-only, reduced automatically
    from rcc output): an inlined `bool`-returning comparison function;
    `t12_linear_scan` and `tests/core/const_pat_aggregate.rs` failed this
    way with ccc as `RCC_CC`.
  - `bool_eq_zero_branch.c` (the first owner's repro): `b = (x == 1);
    c = b; if (c == 0) goto L;`.
  - Not reduced to files, seen in the ccc-compiled ccc: `t = (v != 0);
    u = (t != s);` in `passes::loop_unroll::unroll_one` (`cmp x11, #0;
    cmp w21, w0, uxtb`), and `if (a & m) == 0 || (b & m) != 0` in
    `backend::arm::codegen::peephole::fold_shift_into_uses` (`tst w11, w0;
    tst w1, w0; cset x11, ne; mov x2, #1; tst x0, #0xff; csel ..`).
  Effects observed: wrong output of small programs; a ccc-compiled ccc
  that hung, printed bogus semantic errors, or (most insidious) worked but
  generated different code than the clang-compiled ccc built from the
  same C, e.g. `passes::mod_ref::FnEffects ==` always false.
  How it was found: build ccc from the same generated C with clang and
  with ccc, diff `-O2 -S` of all 217 difftest cases, then link ccc- and
  clang-compiled units (and then single functions) in mixes until one
  function remains; finally `CCC_PEEPHOLE_SKIP=<pass>` over all pass names.
  Workaround: when `$RCC_CC` is ccc the driver adds
  `drop_mask_of_narrow_value` to `CCC_PEEPHOLE_SKIP` in the environment of
  the C compiler. cgen additionally writes bool branches positively
  (`if (b) goto T; goto F;`) and `bool == bool` as xor; both are harmless
  leftovers of earlier attempts and neither is sufficient alone.
  Attempts that did NOT help: `volatile` bool locals (ccc generates the
  same code), `unsigned char` instead of `_Bool`, `noinline` everywhere
  (each only removed some instances).
- `field_addr_live_across_call.c`: in a loop, `q = &cur->next; f(...);
  cur = *q;` crashes: ccc folds the address into the later load without
  keeping `cur` alive, and reuses its (caller-saved) register in between.
  Structured `for` loops are affected too, so this is a plain ccc bug
  (arm64). Workaround: locals that receive `&place-through-a-pointer` are
  declared `RCC_ADDR` (= `volatile` when the driver sees that `$RCC_CC` is
  ccc and passes `-DRCC_CC_IS_CCC`). The same pattern in rcc's own C source
  could be miscompiled by ccc; no instance has shown up in the tests.
- Building rcc itself with `ccc -O1 -c` per file (without `-g`) produced a
  binary whose expander failed on `lib/std/src/iter/adapters.rs`; the
  Makefile's flags (`-O1 -g`) give a working binary. Not investigated.

### Environment note

On this machine the Homebrew `bash`, `perl` and `python3` in
`/usr/local/bin` started hanging (uninterruptible) mid-session; running the
test scripts with `PATH=/usr/bin:/bin:/usr/sbin:/sbin` avoids them.

## std: OS modules (io, fs, env, path, process, time, ffi, error, os)

Files: `lib/std/src/{io/,fs.rs,env.rs,path.rs,process.rs,time.rs,error.rs,ffi/,os/}`,
the `rcc_os_*` block at the end of `rt/rt.c` (declared in `lib/std/src/os/sys.rs`),
`tests/std_os/`. Run the tests with
`RCC_RUN_DIR=tests/std_os tests/run.sh` (and `tests/gen_expected.sh`; both
scripts accept `RCC_RUN_DIR`).

### What works

- t11: 22/22 (clang and ccc as C compiler). `tests/std_os`: 14/14 with
  clang and with ccc (io errors/traits, large files (6 MB round trips,
  200k-line BufWriter/BufReader, copy, seek, set_len), metadata/permissions/
  symlinks/read_dir/error kinds, path edge cases, processes (1.5 MB on both
  stdout and stderr, exit codes, signals, kill, try_wait, stdin pipes, env,
  cwd, Debug), env, time, stdin sharing, exit/panic/main-returns-Err
  flushing, ExitCode).
- All of ccc (`src/main.rs --extern ccc=src/lib.rs --check-all`) type-checks.
- `io`: Read/Write/BufRead/Seek with real std's default methods and error
  texts, Error/ErrorKind (Display/Debug identical to real std, errno ->
  kind table in rt.c, strerror messages), Cursor, BufReader, BufWriter
  (flush on drop, into_inner), LineWriter, Lines/Split/Bytes/Chain/Take,
  copy/empty/sink/repeat/read_to_string, stdin/stdout/stderr + locks,
  IsTerminal. Stdout is C stdio with LineWriter semantics (complete lines
  flushed at once, a partial line stays buffered; flushed at exit,
  `process::exit`, panic, before fork); stderr is unbuffered and no longer
  flushes stdout (real std behaviour; `rt.rs` panics still flush first).
  One process-wide 8 KiB stdin buffer (in rt.c) shared by all handles.
- `fs`: File (+ `&File`), OpenOptions (std's validation of flag
  combinations), Metadata/FileType/Permissions (Debug like real std),
  read_dir/DirEntry, DirBuilder, read/read_to_string/write/copy/rename/
  remove_*/create_dir(_all)/canonicalize/read_link/hard_link/soft_link/
  set_permissions/metadata/symlink_metadata/exists. `os::unix::fs`:
  PermissionsExt, MetadataExt, OpenOptionsExt, DirBuilderExt, FileTypeExt,
  DirEntryExt, symlink. `struct stat`/`dirent` are decoded in rt.c.
- `path`: full unix Path/PathBuf API incl. Components (port of std's
  front/back state machine, so `parent`, `file_name`, `as_path` give the
  same sub-slices), Iter, Ancestors, Display, StripPrefixError, absolute.
- `ffi`: OsStr/OsString, minimal CString/CStr; `os::unix::ffi::{OsStrExt,
  OsStringExt}`.
- `env`: args/args_os, var/var_os/vars/vars_os/set_var/remove_var,
  VarError, current_dir/set_current_dir, current_exe, temp_dir, home_dir,
  split_paths/join_paths, consts.
- `process`: Command (args, env/env_remove/env_clear, current_dir, stdio
  inherit/null/piped/File, spawn/output/status), Child (wait, try_wait,
  kill, wait_with_output draining both pipes with poll), Output, ExitStatus
  (Display/Debug like std), ExitStatusExt, CommandExt::arg0, ExitCode
  (+ Termination), exit/abort/id. fork + execvp with a CLOEXEC error pipe
  (missing program = spawn error NotFound).
- `time`: Duration (Debug identical to std incl. precision/width/`+`),
  Instant, SystemTime/UNIX_EPOCH/SystemTimeError. `error::Error` with
  `Box<dyn Error>` From impls (`&str`, `String`, `Cow<str>`, any `E: Error`).

### Design: Path / OsStr without unsized tails

`OsStr`, `Path` (and `CStr`) are sized views `{ inner: &[u8] }`; `&Path` is
a thin pointer to a view. Borrowed views (`Path::new`, `AsRef<Path> for
str/String/OsStr`, `parent`, `file_name`, components, ...) are boxed and
leaked (16 bytes each; the bytes are never copied). `PathBuf`/`OsString`
own a `Vec<u8>` plus a `Cell` view of it that `Deref` refreshes, so
autoderef from the owned types allocates nothing. Comparison, hashing,
`starts_with`, `push`, `pop`, `set_extension` work on raw byte slices.
Consequence: no `Box<Path>`/`Rc<Path>`, no `Path::as_mut_os_str`, and a
small permanent leak per borrowed view (fine for a batch compiler).

### Known gaps

- SIGPIPE is not ignored at start-up (real std ignores it), so writing to
  a closed pipe kills the process instead of returning BrokenPipe.
- `io::Error::last_os_error` returns error 0 (rt.c reports errno through
  return values); no vectored I/O, no `File::set_times`/`FileTimes`, no
  `Metadata::created` on Linux, no `fs::File` path in Debug.
- `Stdin`/`Stdout` locks do not lock (each C call is atomic).
- Linux: written to need only the `#ifdef __APPLE__` parts of the rcc_os_*
  block (stat time fields, `_NSGetExecutablePath`, F_FULLFSYNC); untested.

## std: collections

Files: `lib/std/src/collections/{mod,hash_map,hash_set,btree_map,btree_set,vec_deque,binary_heap}.rs`,
the hashers at the end of `lib/std/src/hash.rs`, tests in `tests/std_collections/`
(`tests/std_collections/run.sh [--rcc PATH] [PATTERN]`; expected output from rustc 1.88).

### What works

- `HashMap<K, V, S = RandomState>` / `HashSet<T, S>`: insertion-ordered
  "index map": entries in a dense `Vec`, plus an open-addressing table of
  `u64` slots (`hash32 << 32 | index+1`, linear probing, max load 70%,
  backward-shift deletion, `swap_remove` of the entry). The supplied
  `BuildHasher` is honoured; the 64-bit hash is mixed (Fibonacci multiply,
  top bits) so weak hashers (Fx low bits, identity hashes, strided keys) do
  not cluster. Iteration order is insertion order until the first removal,
  always deterministic. API: new/with_capacity/default/with_hasher/
  with_capacity_and_hasher, insert/get/get_mut/get_key_value/contains_key/
  remove/remove_entry/try_insert (all with `Borrow<Q>` lookups), the entry
  API (Entry/OccupiedEntry/VacantEntry with or_insert, or_insert_with,
  or_insert_with_key, or_default, and_modify, key, get, get_mut, into_mut,
  insert, insert_entry, remove, remove_entry), len/is_empty/clear/reserve/
  shrink_to_fit/capacity/hasher, iter/iter_mut/keys/values/values_mut/
  into_iter (owned, &, &mut)/into_keys/into_values/drain/retain/extract_if,
  Extend (tuples and `(&K, &V)`), FromIterator, From<[_; N]>, Index<&Q>,
  PartialEq/Eq, Debug (`{:?}`/`{:#?}`), Clone, Default. Sets: insert,
  replace, contains, get, get_or_insert, take, remove, union/intersection/
  difference/symmetric_difference (lazy iterators), is_subset/is_superset/
  is_disjoint, iter/drain/retain/extend (T and &T)/into_iter, `| & ^ -` on
  `&HashSet`.
- Hashing: `DefaultHasher` is SipHash-1-3 with zero keys and hashes exactly
  like real std (tested for every primitive, str/String, slices, Vec,
  tuples, Option/Result, derived structs/enums, Box/Rc, BTreeMap/BTreeSet/
  VecDeque and raw writes of every length). `RandomState` is deterministic
  (zero keys); also exported as `std::hash::{DefaultHasher, RandomState}`.
- `BTreeMap` / `BTreeSet`: a two-level B+ tree (sorted `Vec` of sorted
  chunks of 1..=128 entries; split at 128, merge small neighbours), so
  inserts/removes move at most a chunk. Full API incl. range/range_mut with
  every bound form (incl. `(Bound, Bound)` tuples and `range::<str, _>`),
  entry API, first/last_key_value, pop_first/pop_last, first/last_entry,
  double-ended ordered iterators, retain, append, split_off, extract_if,
  FromIterator (stable sort + keep last duplicate, like std), Ord/Hash/
  Index; set ops are ordered merges.
- `VecDeque`: ring buffer (`buf, cap, head, len`); everything in the task
  list plus range, swap_remove_*, binary_search*, partition_point, rotate.
  `as_slices` is correct but its split point differs from std's growth
  policy (only the total length is observable in the corpus).
- `BinaryHeap`: std's exact sift algorithms (push/pop/rebuild/rebuild_tail/
  into_sorted_vec), so `{:?}` of the internal order matches rustc; peek_mut
  (`PeekMut` re-sifts only after `DerefMut`, `PeekMut::pop`), append,
  retain, drain, into_iter_sorted, From<Vec>/From<[T; N]>, with `Reverse`.
- Small core-std gaps fixed on the way (append-only except the derives):
  `Debug` for `Range`/`RangeFrom`/`RangeTo`/`RangeToInclusive`/`RangeFull`
  in std's `0..3` form (their derived `Debug` was removed), `Clone/Copy/
  PartialEq/Eq/Debug` for `Bound`, `DebugMap::key/value`,
  `FromIterator for Box<[T]>`, transparent `Debug/Display` for `Wrapping`.

Performance (1M-entry Fx map insert/get/remove, 200k String entry/lookup,
500k tuple set, 300k BTreeMap insert/range/remove): within 1.0-1.8x of
rustc `-C opt-level=1` with the C at `-O1`.

### Tests

`tests/std_collections/`: hashmap_api, hashmap_large (100k random ops vs a
model with SipHash, Fx, Fx on strided keys, identity hasher, identity on
strided keys; 50k String set), hashset_api, siphash (73 hash values vs
real std), btree_api (incl. 60k random ops and range queries vs a sorted
model), deque_heap_api. All pass with clang and with ccc as the C compiler.

End-to-end check (2026-11-11): all 7336 bodies of ccc type-check; rcc builds
ccc (`src/main.rs` + `--extern ccc=src/lib.rs`) in 13 s; that ccc compiles
rcc's own C, and the resulting rcc passes the suite like the clang-built
one. On identical C input the rcc-built ccc and the cargo-built ccc produce
binaries with identical output on the t07/t12 collection-heavy tests.

### Known gaps

- `HashMap` iteration order is not hashbrown's (allowed by DESIGN.md).
- `extract_if` is eager (std's is lazy; same result when fully consumed).
- No `raw_entry`, `get_many_mut`, cursors (`BTreeMap::lower_bound`),
  `LinkedList`, `try_reserve` errors (allocation aborts).

## std: sync and thread

Files: `lib/std/src/sync/{mod,atomic,arc,mutex,once}.rs`,
`lib/std/src/thread/{mod,local}.rs`, `lib/std/src/panic.rs`, the panic
entry in `lib/std/src/rt.rs`, the `rcc_thr_*` block of `rt/rt.c` (plus two
calls of `rcc_thr_c_panic` at the top of `rcc_panic_cstr` /
`rcc_panic_bounds`), `tests/std_sync/` (own runner:
`tests/std_sync/run.sh [--gen] [-k PAT]`, honours `RCC_CC`, `RCC`,
`RCC_FLAGS`).

### What works (all of tests/std_sync, 8/8, with clang and with ccc as RCC_CC; t10 33/33)

- `thread`: `spawn`, `Builder::{new,name,stack_size,spawn,spawn_scoped}`
  (returns `io::Result`), `JoinHandle::{join,thread,is_finished}` (dropping
  it detaches), `scope`/`Scope::spawn`/`ScopedJoinHandle` (scope waits for
  all its threads and panics with "a scoped thread panicked" if one
  panicked unjoined), `current()` (`name`, `id`; main is `Some("main")`,
  `ThreadId(1)`), `ThreadId`, `yield_now`, `sleep`, `sleep_ms`, `park` (=
  yield), `panicking()` (always false), `thread::Result`. Real pthreads; the
  requested stack size is honoured (rounded up to the page size; default
  8 MiB). tests/std_sync/ccc_driver recurses 1,000,000 frames in a 1 GB
  thread.
- Panic capture without unwinding (DESIGN.md amendment): on a spawned
  thread `panic_fmt` formats the message, prints `thread '<name>' panicked
  at file:line:col:\nmsg` (one write), stores the payload in the
  thread-local `PANIC_PAYLOAD` and calls `rcc_thr_unwind`, which
  `longjmp`s to the `setjmp` in the thread entry `rcc_thr_start`; that calls
  the monomorphized `thread_panicked::<F, T>`, which stores `Err(payload)` in
  the shared `Packet` for `join`. No destructor of the abandoned frames runs
  (their memory leaks; a held `MutexGuard` keeps the mutex locked).
  Payload: `&'static str` if the `Arguments` has no pieces or one `Lit`
  piece, else `String` (as real std's `Arguments::as_str()`). Runtime panics
  raised in C (`/` by zero: `&'static str`; index out of bounds: `String`)
  go through `rcc_thr_c_panic` -> the Rust hook `panic::c_panic_hook`.
  The main thread keeps the old behaviour (message, exit 101).
  `panic::panic_any` exists; no hooks / `catch_unwind` (ccc uses none).
- `thread_local!` / `LocalKey<T>`: `LocalKey::new(fn() -> T)` is a const
  fn (the key is the static's address); `with`, `try_with`; for
  `LocalKey<Cell<T>>` `set/get/take/replace`, for `LocalKey<RefCell<T>>`
  `with_borrow/with_borrow_mut/set/take/replace` (`set` does not run the
  initializer, as real std). Each thread has its own table in rt.c (a
  `_Thread_local` open-addressing map key address -> boxed value); values
  are created on first access per thread and dropped (LIFO) when a spawned
  thread exits; main-thread values are never dropped.
- `sync::atomic`: `AtomicBool`, `AtomicU8..U64`, `AtomicI8..I64`,
  `AtomicUsize`, `AtomicIsize`, `AtomicPtr`, `Ordering`, `fence`,
  `compiler_fence`; `new` (const), `load/store/swap/compare_exchange(_weak)/
  compare_and_swap/fetch_add/sub/and/or/xor/nand/max/min/fetch_update/
  fetch_not (bool)/get_mut/into_inner/as_ptr`, `Default`, `From`, `Debug`.
  One C function `rcc_thr_atomic(ptr, size, op, v, w)` with `__atomic`
  builtins, always SeqCst (ccc supports them; checked).
- `Arc`/`Weak` (`#[lang = "Arc"]`, same two-allocation layout as `Rc`, so
  `Arc<dyn Trait>` unsizing works): `new`, `new_cyclic`, `clone`,
  `strong_count`, `weak_count`, `ptr_eq`, `as_ptr`, `get_mut`, `make_mut`,
  `try_unwrap`, `into_inner`, `unwrap_or_clone`, `downgrade`/`upgrade`,
  `Arc<str>`/`Arc<[T]>` from `&str`/`String`/`&[T]`/`Vec`/`Box`, the usual
  trait impls.
- `Mutex`/`MutexGuard`, `RwLock` + guards, `Condvar` (`wait`,
  `wait_while`, `notify_*`), `PoisonError`, `TryLockError`, `LockResult`,
  `TryLockResult`: atomic words with spin/yield/sleep backoff (no OS mutex,
  so `const fn new` works in statics); never poisoned.
- `Once` (`call_once`, `call_once_force`, `is_completed`, `wait`),
  `OnceLock` (`new` const, `get`, `get_mut`, `get_or_init`, `set`, `wait`,
  `into_inner`, `take`, `From`, `Clone`, `Debug`), `LazyLock<T, F = fn() ->
  T>` (`new` const with fn pointer or closure, `Deref`, `DerefMut`, `force`).

### Known gaps

- No `mpsc`, `Barrier`, `Condvar::wait_timeout*`, `park_timeout`,
  `available_parallelism` (needs `NonZeroUsize`; `rcc_thr_ncpu` exists in
  rt.c), `catch_unwind`/`set_hook`/`resume_unwind` (unused by ccc).
- Send/Sync are not required by `spawn` (no auto-trait checks needed).
- Locks spin and then sleep 50us per retry: correct, but not efficient
  under long contention.
- Payload type differs from rustc when the format arguments are literals:
  rustc inlines literal arguments into the format string (`panic!("{}",
  "x")`, `panic!("n={}", 1)`) so `as_str()` is `Some` and the payload is a
  `&str`; rcc's expander keeps them as arguments, so the payload is a
  `String`. Fixing it is an expander change (inline `Lit` string/integer
  arguments with a plain `{}` spec). ccc downcasts both types, so it does
  not matter there.
- The driver links with `-lm` only; pthreads are in libc on macOS and on
  glibc >= 2.34. Older glibc would need `-pthread` in `src/driver.c`.

### Bugs found (not fixed here)

- `tests/core/pending/option_of_closure_is_copy.rs`: `Option<closure>` /
  tuples containing a closure that captures a non-Copy value are copied as
  operands (`copy _2`) but still dropped -> double free. Worked around in
  thread/mod.rs (the closure is kept in a `ManuallyDrop`).
- `tests/core/ccc_bugs/tlv_call_lost_arg.c` (ccc, arm64-darwin): an
  indirect call made after a `_Thread_local` access loses argument
  registers that were live across the TLV getter call. rt.c works around it
  by reaching its thread-local state only through a volatile function
  pointer.

## Demo (the ocean test)

Files: `demo/` (`Makefile`, `README.md`, `common.sh`, `snapshot.sh`,
`chain.sh`, `bench.sh`, `before-after.sh`, `difftest.sh`, `selftest.sh`,
`summary.sh`). Builds only under `demo/build/`, from snapshots of ccc
(`git archive HEAD`) and of rcc (`Makefile src lib rt tests`); writes nothing
into `~/dev/ccc` or the rest of this repository.

### What works (`make all`, ccc `f5e0da05`, rcc sources of 2026-11-11 07:01)

- `make chain`: A ccc-rustc (cargo), B rcc-by-ccc, C ccc-by-rcc (generated C
  compiled by ccc-rustc), D rcc-by-ccc2 + ccc-by-rcc2 (all C compiled by
  ccc-by-rcc). All stages pass with ccc compiling every C file. Stage C and
  D emit byte-identical C for ccc (32 units, 75.9 MB), and the stage C and D
  ccc emit byte-identical `-O2 -S` for ocean.c and rcc's 28 sources; so does
  the stage-C ccc built with clang (`CCGEN=clang`) from the same C. Versus
  ccc-rustc, 9 of 29 files are byte-identical and 20 differ only in
  register/stack-slot choice and instruction order (ccc iterates hash maps
  in its register allocator; rustc's and rcc's HashMap orders differ).
- `make bench`: ocean.c `--bench 300 --threads 1`, same heights
  (-4.714 .. 4.320) and checksum (0.9752) for ccc-rustc, ccc-by-rcc and
  clang; 5.17 / 5.17 / 4.44 ms/frame. Compile time of ocean.c: ccc-rustc
  0.6 s, ccc-by-rcc 4.1 s, ccc-by-rcc.clang 0.9 s, clang 0.2 s.
- `make difftest` (`cases`, 216): 216/216 for ccc-rustc, ccc-by-rcc and
  ccc-by-rcc.clang; no differing case.
- `make selftest`: tests/run 333/333 with rcc-by-ccc + ccc-by-rcc as
  `RCC_CC`, and 333/333 with rcc-by-ccc2.clang + ccc-by-rcc.clang.
- `make before-after`: three windows (SDL + Vulkan initialised, rendering);
  their placement uses a copy of ocean.c whose `SDL_CreateWindow` call takes
  `-DWINDOW_X/Y/W/H`.

### Known gaps

- Wall times of stages C and D are long when ccc compiles the generated C
  (190.9 s and 1083.4 s on a heavily loaded machine; 34 s and 603 s earlier
  under less load), against 18.6 s and 18.5 s with clang. ccc-by-rcc is
  about 7x slower than ccc-rustc on ocean.c (partly the "ccc never inlines
  rcc output" workaround).
- One intermittent selftest COMPILE_FAIL in the first full run
  (`t12_elf_writer`: ccc-by-rcc exited 1 with no message, under heavy memory
  and CPU pressure); not reproduced in 9 retries or in the second run.

### ccc bugs hit

- The LICM hang (`passes::licm::build_value_to_base_alloca`, see "ccc
  issues met" above) also hit the demo with the rcc sources of 06:40: the
  CCGEN=ccc stage-C ccc hung on any function with a loop, even at `-O0`.
  A compile-only reproducer (the function on its own plus the generated
  header) is in `demo/build/repro-licm/` until `make clean`:
  `ccc-rustc -O1 -DRCC_CC_IS_CCC -w -I demo/build/repro-licm -S
  demo/build/repro-licm/licm_repro.c` shows `tst x0, #0xff; tst x0, #0xff;
  csinc x20, x20, xzr, eq` after the first `eq_7018` call (the `cset x0, eq`
  of `!eq(..)` is gone) and the correct sequence after the second.
