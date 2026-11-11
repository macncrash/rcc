# rcc core architecture

For the people (and agents) extending name resolution, type checking,
monomorphization, MIR, C generation, the runtime and std. Read `DESIGN.md`
first for the project-level contract.

```
expand (src/expand*.c, not core)      one expanded AST per crate
  -> resolve      src/resolve.c       Defs, module tree, imports, scope lookup
  -> typeck       src/typeck_*.c      item signatures, traits, inference -> THIR
  -> mono + MIR   src/mono.c          instance worklist from `main`
                  src/mir_build.c     THIR + substs -> MIR (per instance)
                  src/mir_drop.c      drop elaboration (drop flags)
  -> cgen         src/cgen.c          MIR -> one C file, or a header + N units
                                      for big programs (+ rt/rcc_rt.h, rt/rt.c)
  -> $RCC_CC                          clang or ccc (units compiled in parallel)
driver: src/driver.c (`rcc ROOT.rs -o OUT`), called from src/main.c
```

Everything is lazy and demand driven from `main`: only bodies reachable from
`main` are type-checked (`typeck_body`), lowered and emitted. A broken or
unsupported function that is never called costs nothing. `--check-all` /
`--check-std` type-check every body of the user crates / of std; the bodies
are checked in a forked child that is restarted after each failing body, so
that all failures are listed (ccc: 7336 bodies in about 2 s). Note that impl headers
of ALL impls are lowered up front (`typeck_init` builds the impl tables), so
a type missing from std that appears in an impl header stops everything.

## Files and data structures

| file | owns |
|---|---|
| `types.h/.c` | `HMap` (int-pair keyed hash map), `StrBuf`, the global `Def` table (`DefId`), interned `Type`/`Substs`, type folding and printing |
| `resolve.h/.c` | def collection for all crates, `ModData` name tables, lazy import/glob resolution, `Scope` chains, `resolve_path`, traits in scope, lang items |
| `typeck.h` | public typeck API: generics, predicates, signatures, ADT/trait/impl defs, type lowering, `InferCtxt`, trait selection, closed-type queries, `typeck_body`, `const_eval` |
| `typeck_fn.h` | internal: `FnCtxt` (one per body) and the `check_*` conventions |
| `typeck_collect.c` | `generics_of`, `predicates_of`, `param_env`, `type_of`, `fn_sig`, `adt_def`, `trait_def`, `impl_def`, `lower_ty`, impl tables, `LangItems lang` |
| `typeck_infer.c` | inference variables, unification, snapshots |
| `typeck_traits.c` | candidate assembly, selection, projection, pending obligations, `norm_closed`/`select_closed`/`ty_is_copy` for mono |
| `typeck_expr.c` | expressions, blocks, coercion, operators, calls, `for`/`?` desugaring |
| `typeck_method.c` | value paths, associated items, method probing |
| `typeck_pat.c` | patterns, default binding modes |
| `typeck_closure.c` | closures, capture analysis, writeback, `typeck_body` |
| `typeck_const.c` | scalar constant evaluation over THIR |
| `typeck_dump.c` | `--dump-types` |
| `thir.h` | the typed IR (`Body`, `TExpr`, `TPat`, `TStmt`, `Local`, `ClosureInfo`) |
| `mir.h` | `Instance`, `Global`, `Vtable`, `Program`, MIR (`Place`, `Operand`, `Rvalue`, `MirStmt`, `Terminator`) |
| `mono.c` | instance table + worklist, `mono_resolve_callee`, vtables, globals and their init order, `ty_needs_drop`, drop-glue discovery, symbol names |
| `mir_build.c` | THIR -> MIR: scopes/drops, temporaries, patterns, calls, intrinsics, promotion |
| `mir_drop.c` | move paths, drop flags, open drops |
| `mir_dump.c` | `--dump-mir` |
| `cgen.h/.c` | C types, places, rvalues, calls, intrinsics, drop glue, vtables, `main` |
| `driver.h/.c` | command line, pipeline, invoking the C compiler |
| `rt/rcc_rt.h` | prelude of every generated C file (no system headers) |
| `rt/rt.c` | libc-facing runtime (`rcc_*` functions std calls through `extern "C"`) |
| `lib/std/src` | the standard library, in Rust |

### Defs (types.h)

One `Def` per module, item, enum variant, associated item, generic parameter
and closure, for all crates together. `Def.parent` is the lexical parent
(module, trait, impl, enum; closures: the owning fn), `Def.module` the
innermost module. Items declared inside a function body live in an anonymous
*block module* (`ModData.is_block`, `Def.name == 0`) whose parent is the
enclosing module. All lazily computed typeck facts are cached in the Def
(`generics`, `preds`, `type_of`, `sig`, `adt`, `trait`, `impl`, `body`,
`closure`, `opaques`).

### Types (types.h)

`Type` and `Substs` are hash-consed: pointer equality is type equality, and
`id` is a dense deterministic number (used for hashing and C names). Flags
(`TYF_HAS_INFER/PARAM/PROJ`) let folds skip closed subtrees. Notable kinds:

- `T_ADT(def, substs)`, `T_TUPLE(substs)`, `T_REF/T_PTR(elem, is_mut)`,
  `T_SLICE`, `T_STR`, `T_ARRAY(elem, len)` where `len` is a `T_CONST`, a
  const `T_PARAM` or an inference variable (const generics are just type
  arguments of kind `T_CONST`).
- `T_FNDEF(def, substs)`: the zero-sized type of a fn item or tuple
  constructor. `T_FNPTR(substs = params ++ [ret])`.
- `T_CLOSURE(def, substs)`: `substs` are the enclosing item's substs; the
  signature/captures are in `Def.closure` (`ClosureInfo`).
- `T_DYN(trait, args-without-Self, assoc bindings)`: one principal trait;
  auto traits (`Send`/`Sync`) are dropped.
- `T_PARAM(index, name)`: index into the substs of the item in scope.
  Substs are parent-first: `[impl/trait params.., own params..]`; a trait's
  param 0 is `Self`. Lifetimes are erased everywhere.
- `T_PROJ(assoc_def, trait_substs)`: `<S[0] as Trait<S[1..]>>::Assoc`.
- `T_OPAQUE(fn, ordinal, substs)`: return-position `impl Trait`. Callers see
  only its bounds; mono reveals the hidden type (`opaque_concrete`).
- `T_INFER(var, kind)`: general / integer / float inference variable; the
  only non-interned kind.

### Resolve

`resolve_add_crate` walks each expanded crate once and creates Defs
(including items in fn bodies). `use` items become `Import` records that are
resolved on first lookup and memoized; globs are searched at lookup time with
a cycle guard. Visibility is ignored except that globs only see non-private
names when the importer is outside the glob's target module.

There is no separate "resolve bodies" pass: typeck walks a body with a
`Scope` chain (`SCOPE_LOCALS` ribs it pushes itself, `SCOPE_GENERICS`,
`SCOPE_SELF`, `SCOPE_MODULE`) and calls `resolve_path` / `resolve_ident`.
`resolve_path` consumes as many segments as modules and enums allow and
returns the rest ("`Vec` + `::new`") for typeck to resolve as associated
items. Lookup order for the first segment: local ribs, generic params,
`Self`, block modules outwards, the module (items, imports, globs), extern
crates (`std`/`core`/`alloc` are one crate), the std prelude
(`std::prelude::v1`), primitive types.

### Typeck

Item-level queries (`typeck_collect.c`) are memoized per Def. Predicates are
`PRED_TRAIT(trait_ref)` or `PRED_PROJ(<trait_ref>::Assoc == ty)`;
`param_env(def)` is `predicates_of` plus all supertrait predicates.

Bodies: one `FnCtxt` per body. `check_expr(fcx, e, expected)` returns a
`TExpr` whose type may contain inference variables; `expected` is only a
hint, the caller coerces (`coerce`) where a type is required. Obligations
(`icx->pending`) are processed by `infer_select_pending` whenever a type
must be known (`structurally_resolve`) and at the end of the body.
`fcx_finish` applies integer/float fallback (`i32`/`f64`), resolves every
type in the THIR (writeback), decides deferred operators (builtin vs. trait
call), and runs closure capture analysis.

Trait selection (`typeck_traits.c: assemble`) mirrors rustc: caller bounds
and associated-type/opaque bounds first, then object candidates, builtin
`Fn*`/`Copy`/`Clone`/`Sized`/auto-trait candidates, then impls (headers
unified with fresh variables; impls whose where-clauses definitely fail are
discarded). One candidate -> confirmed; several -> ambiguous (kept
pending). Projections are normalized eagerly (`infer_normalize`); an
ambiguous projection becomes a fresh variable plus a pending `PRED_PROJ`.
Builtin arithmetic on integer/float *variables* is typed without the
operator impls (`project_inner`).

Where each language feature is handled:

| feature | where |
|---|---|
| literals, int/float defaulting | `check_lit`, `fcx_finish` |
| paths to items, `Type::assoc`, `Trait::method`, `<T as Tr>::x`, enum variants through `Self`/aliases | `typeck_method.c: instantiate_value_path`, `resolve_assoc_value`, `resolve_variant_path` |
| method calls: autoderef, autoref, inherent before trait, array->slice | `lookup_method`, `probe_step` |
| operators (builtin and overloaded), `&prim` operands | `check_binop`, `check_unary`, `finalize_deferred_op` |
| field access / indexing with autoderef, `Index`/`IndexMut` | `check_field`, `check_index` |
| place mutability (`Deref` -> `DerefMut`, `Index` -> `IndexMut`) | `fix_place_mut` |
| coercions (deref, `&mut`->`&`, unsizing, never, fn item/closure -> fn ptr) | `coerce_inner`, `try_unsize`, `CoerceMany` (`cm_add`) |
| calls, `Fn*` overloaded calls, expected-type propagation into args | `check_call`, `check_args`, `expected_inputs` |
| `for` | `check_for` (desugars to `match into_iter(..) { mut it => loop { match next(&mut it) {..} } }`) |
| `?` | `check_try` (match on `Option`/`Result` lang enums; `From::from` on the error) |
| `if let`, `while let`, `let else`, match | `check_let_cond`, `check_if`, `check_while`, `check_block`, `check_match` |
| divergence (`!`), blocks without tail | `fcx->diverges`, `check_block` |
| patterns, default binding modes, or-patterns, slice patterns | `typeck_pat.c` |
| closures: signature from `Fn*` bounds, capture modes, kind | `typeck_closure.c: deduce_sig`, `analyze_closure` |
| return-position `impl Trait` | `lower_ty` (IT_RET), `opaque_fold`, `opaque_concrete` |
| argument-position `impl Trait` | synthetic generic params in `generics_of` |
| const generics, array lengths | `lower_const_expr`, `typeck_anon_const`, `const_eval` |

THIR invariants (thir.h): every type resolved; every adjustment explicit;
trait method calls name the trait's method with `[Self, trait args, method
args]`; place expressions are `TE_LOCAL/STATIC/DEREF/FIELD/INDEX`.

### Mono and MIR

`mono_program` creates the `main` instance (and `#[lang = "start"]`
instantiated with main's return type) and runs the worklist: building MIR
for an instance requests the instances it calls. `mono_ty(t, substs)`
substitutes and fully normalizes (`norm_closed`), so MIR contains only
closed types. `mono_resolve_callee` maps a callee `(def, substs)` to: a
direct instance (impl method or the trait's default method), a virtual call
(vtable slot), a closure call, an untupled fn-item/fn-pointer call, a
constructor, a compiler intrinsic, or a bitwise clone.

Closure ABI: the body instance takes `(env, args_tuple)` where `env` is
`&C`, `&mut C` or `C` according to the closure's kind; `Fn::call` etc. on a
closure type call it directly; `call_once` on an `Fn`/`FnMut` closure goes
through `INST_CLOSURE_ONCE`; closure -> fn pointer through
`INST_FNPTR_SHIM`; fn items/pointers in vtables through
`INST_FN_TRAIT_SHIM`.

MIR (mir.h) is a CFG over locals: `_0` return place, `_1..` arguments.
Calls are statements (a diverging call is followed by `TM_UNREACHABLE`).
Statics and non-scalar consts are `Global`s reached through
`RV_GLOBAL_ADDR` + deref, so places are always accessed in place (pointer
identity is preserved). Scalar consts are inlined (`try_const`).
`&<constant expression>` is promoted to a `GLOBAL_PROMOTED`.

Scopes and drops (mir_build.c header comment): variables are scheduled in
their block scope, temporaries in the innermost statement scope (tail
expressions use the enclosing one; `let` extends `&temp`; `if`/`while`
conditions drop their temporaries before the body; `if let`/`while let`/
`match` scrutinee temporaries live to the end of the construct). Leaving a
scope emits `ST_DROP(local)` meaning "drop if still initialized".

Drop elaboration (mir_drop.c) first runs a forward dataflow over the CFG
(`flow_analyze`): for every local with a scheduled drop, is it *maybe*
and is it *definitely* initialized at each point (whole assignments and
call results initialize; whole moves, `ST_FORGET` and drops
de-initialize; arguments start initialized). A scheduled `ST_DROP(local)`
that can never see an initialized value is deleted (failure path of
`let .. else`, early returns, moved-out temporaries), one that always
does becomes an unconditional drop, and only locals with an undecided
drop, or with partial moves, get runtime flags: moves out of such a local
or its fields / variant fields / Box contents create *move paths*; each
path has a bool flag (set on assignment, cleared on move, tested at the
drop). Partly moved values get "open" drops (per field / per variant /
Box contents then shallow free). After this pass every `ST_DROP` is
unconditional. The builder's `MirLocal.maybe_uninit` is no longer used.

Pattern matching: arms are tested in order (`pat_test` branches to the next
arm), then bindings are assigned (`pat_bind`); with a guard, bindings are
first bit-copied, the guard runs, and the real moves happen afterwards.
Constant patterns of scalar type compare directly, `&str`/slices through
the `str_eq`/`slice_eq` lang fns, every other type through
`PartialEq::eq`. After a body is built, `merge_switch_chains` folds each
chain of single-value tests of the same place (`tag == A`, else `tag ==
B`, ..) into one multi-way `TM_SWITCH`, which cgen emits as a C `switch`.
Destructuring assignment is desugared in typeck
(`check_destructuring_assign`: `let (tmp0, tmp1) = rhs; a = tmp0; ..`).

### C generation

See the header comment of `cgen.c` for the type mapping. One C function per
instance, a label per reachable block, `goto`/`switch`. Arithmetic is done
in unsigned types (wrapping, no UB); shifts mask the amount; `/` and `%`
call `rcc_div_T`/`rcc_rem_T` (panic on 0 and MIN/-1); float->int casts call
saturating helpers; indexing emits `rcc_panic_bounds` checks. Zero-sized
types (`is_zst_like`): fields of such types are omitted from structs, values
are never stored or loaded (operands become the `rcc_unit` / `rcc_zst_N`
zero objects), and `size_of` is 0; a local or array element of a ZST still
occupies one byte in C. Enums are `{tag; union}` with the discriminant value
as tag, no niches. Blocks are emitted in reverse postorder; bool branches
are written positively (`if (b) goto T; goto F;`) and `bool == bool` as xor; member access through a pointer is `p->m`. Globals are zero-initialized C statics filled in by `main` in
dependency order before the user's `main` runs.

Large programs (>= 3000 instances, or `RCC_UNITS=N`) are split:
`cgen_program_split` writes one header with all types, prototypes and
`extern` declarations (`OUT.rcc.c.h`) and N units (`OUT.rcc.c.K.c`) that
include it; bodies are dealt round-robin, unit 0 also defines the globals,
the vtables and `main`. Functions then have external linkage. The driver
compiles the units with up to `RCC_JOBS` (default: number of CPUs)
concurrent `$RCC_CC -c` processes and links them with `rt/rt.c`. ccc
itself: 48.6k instances, 2.5M lines / 75 MB of C in 32 units.

## std (lib/std)

Plain Rust compiled with every program. Rules:

- Only reach the OS through `extern "C"` functions named `rcc_*` defined in
  `rt/rt.c` (plus libm functions with their exact C prototypes). The
  generated C includes no system header.
- Compiler-known items carry `#[lang = "name"]` (list: `typeck_init` in
  `typeck_collect.c`, plus `start`, `str_eq`, `slice_eq`, `manually_drop`,
  `phantom_data`). Intrinsics are declared in `intrinsics.rs` inside
  `extern "rcc-intrinsic"`; MIR-level ones are implemented in
  `mir_build.c: lower_intrinsic`, the rest in `cgen.c: emit_intrinsic`.
- Primitive types get inherent methods with `impl u32 { .. }`,
  `impl<T> [T] { .. }`, `impl str { .. }`, `impl<T> *const T { .. }`.
- `Box<T>` must stay a one-field struct holding the raw pointer; `Vec<T>`
  with capacity 0 holds a null pointer.
- Keep one generic impl where real std has one (e.g. `Iterator for
  Range<A: Step>`): inference relies on a projection resolving while its
  input is still an inference variable.
- ccc workarounds: the driver skips ccc's `drop_mask_of_narrow_value` peephole pass (`CCC_PEEPHOLE_SKIP`), cgen/rt have `RCC_ADDR`; repros are in
  `tests/core/ccc_bugs/`. If ccc-compiled output misbehaves, diff against
  the clang-compiled output and reduce the generated C.
- Expander hygiene renames locals bound inside a `macro_rules!` body at the
  token level; names that follow `fn`/`struct`/`enum`/`trait`/`type`/`mod`,
  `.`, `::` or precede `::`/`!` are never renamed. A body that binds `x`
  and also *calls* a free function `x` is still wrong (see STATUS).

### How to add a std API

1. Find (or create) the module file under `lib/std/src/`; add `pub mod x;`
   to `lib.rs` if new. New modules (collections, rc, cell, fs, path, env,
   process, thread, sync, time, io) do not need to touch the core files.
2. Write it in plain Rust using what exists. Need the OS? add an `rcc_*`
   function to `rt/rt.c` and declare it `extern "C"`.
3. Test with a small program: `build/rcc t.rs -o t && ./t`; compare with
   rustc. `build/rcc any.rs --check-std` type-checks all of std (only
   non-generic-dependent errors show up before instantiation).

## Debugging a failing test

```
build/rcc t.rs -o t --emit-c t.c     # keep the generated C
build/rcc t.rs --dump-resolve        # module tree, imports and their targets
build/rcc t.rs --dump-types          # THIR of the root crate's reachable fns
build/rcc t.rs --dump-mir            # MIR of every instance
build/rcc --expand t.rs              # what the expander produced
RCC_CFLAGS='-O0 -g' build/rcc ...    # debuggable binary
RCC_TIME=1 build/rcc ...             # CPU time per phase, peak RSS, instance counts
RCC_UNITS=4 RCC_JOBS=8 build/rcc ..  # force N C units / limit parallel cc jobs
tests/core.sh [PATTERN]              # core regression tests (tests/core/*.rs)
```

- "mismatched types" / "trait bound not satisfied" / "type annotations
  needed": typeck. Reduce the program; `--dump-types` on a variant that
  compiles shows what was inferred. Many such errors are a missing std impl.
- C compiler errors: look at the function in `t.c`; its comment names the
  Rust instance; `--dump-mir` shows the MIR it came from.
- Wrong output or a crash at run time: check drops first (`--dump-mir`,
  look for `drop(...)`/`free(...)` and the bool flag locals), then the
  pattern/branch structure.
- rcc itself looping in trait selection prints "trait selection is taking
  long" with the predicate.

All internal errors are `fatal(span, ..)` with the Rust source position; an
unsupported construct must fatal by name, never miscompile.
