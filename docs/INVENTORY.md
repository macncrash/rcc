# INVENTORY: what ccc actually uses

The spec for rcc's mini-std and type checker: the language and library subset
that `~/dev/ccc` (rustc 1.88, edition 2021, std only) uses in a **default-features
`--release` build with `cfg(test)` off**.

## 0. Method, scope, confidence

| Source | How it was made | Confidence |
|---|---|---|
| **MIR** (sections 1, 2, parts of 3/4/5) | `RUSTC_BOOTSTRAP=1 cargo rustc --lib --release -- -Zunpretty=mir -Zmir-opt-level=0 -Ztrim-diagnostic-paths=false -Zmir-include-spans=yes`, run on a copy of ccc (358 MB of MIR, 11,328 fn bodies plus 7,223 const bodies). Every call terminator and every fn-item constant was parsed, and receiver types were resolved by rustc. `-Zmir-opt-level=0` means no MIR inlining, so each call in the source, and each call produced by desugaring (for-loops, `?`, indexing, `format_args!`, derives, `vec!`), is still visible. | **compiler-verified** |
| **Stripped source** (sections 3 to 6) | `tools/inventory/strip_tests.py` walks the module tree from `src/lib.rs`, `src/main.rs` and `src/bin/*.rs` and blanks every item under `#[cfg(test)]`, `#[test]`, `#[cfg(feature=...)]` or `#[cfg(not(unix))]` (16,448 lines, keeping line numbers). **The stripped tree still passes `cargo check --release`.** Regex counts then run with comments removed and string contents blanked. | **grep-estimated** (each count marked) |
| **Experiments** (section 7) | Built the release ccc twice on a copy: once as-is and once with the Fx `SEED` changed. Compiled all 226 `tests/difftest/cases/*.c` with `-O2 -S` and `-O2 -c` (native arm64-darwin), and 646 objects with the builtin ELF assemblers (`ccc-x86/-riscv/-i686 -c` on preprocessed `.i`). | **measured** |

Counts called "refs" are MIR call sites, plus fn-item references that are not
calls. A generic function counts once, not once per instantiation. Macro
expansions are included: `format!` contributes `Arguments::new_v1` and so on.

Scripts are in `tools/inventory/` (`run.sh` regenerates everything into a
scratch dir). Per-site raw data: `hash_iter_sites.tsv` and `hash_order_review.tsv`.

Size of what must compile: **389 files and 225,611 lines in total. Non-test code is 209k lines, of which 160,508 are non-blank and non-comment.** All 389 files are reachable from the crate roots, and none sits behind `#[path]`. The 4 bins (`src/bin/ccc_*.rs`, `src/main.rs`) each contain only `ccc::compiler_main()`.

### cfg predicates

| predicate | where | arm64 macOS release | x86-64 Linux release |
|---|---|---|---|
| `test` | 70× `#[cfg(test)]` (all inline `mod tests {}` or fns) | false | false |
| `debug_assertions` | never named in a `cfg`. 15 `debug_assert!`/`debug_assert_eq!` calls (e.g. `src/passes/cfg_simplify.rs:1615`) are **still type-checked** but expand to `if cfg!(debug_assertions)`, which is false | false | false |
| `feature = "gcc_linker"` / `"gcc_assembler"` / `"gcc_m16"` | 4 / 9 / 5 `#[cfg(feature)]`, 2 `#[cfg(any(feature..., feature...))]`, 3 `cfg!(feature=...)` | false | false |
| `not(feature = "gcc_linker")` / `not(feature = "gcc_assembler")` | 25 / 3 | true | true |
| `unix` | 9× `#[cfg(unix)] { use std::os::unix::fs::PermissionsExt; set_permissions(.., from_mode(0o755)) }`, e.g. `src/backend/x86/linker/emit_exec.rs:1215` | true | true |
| `target_os = "macos"` | 1× `cfg!()` expression (both branches type-checked) | true | false |
| `all(target_os = "macos", target_arch = "aarch64")` | `src/driver/cli.rs:36` `cfg!()`: plain `ccc` defaults to the AArch64 target on Apple Silicon | true | false |
| `#[cfg_attr(feature..., allow(dead_code))]` | 15, lint-only | n/a | n/a |

Where cfg attributes sit: on items, on `use`, on statements (`#[cfg(feature="gcc_m16")] if ... {}` at `src/driver/pipeline.rs:577`), and **on block expressions in tail position**, where one function body has `#[cfg(feature=X)] { ... }` followed by `#[cfg(not(feature=X))] { ... }` as alternatives (`src/driver/external_tools.rs:128-136`, `src/driver/cli.rs:704`). None sit on match arms, struct fields or call arguments. On both hosts the predicates evaluate the same way except the two `cfg!()` expressions.

---

## 1. Std API surface, by type (compiler-verified, MIR)

Format: `method (refs)`. "via std traits" lists trait methods called with that
type as `Self`, including the implicit calls: `Deref` (autoderef and method
lookup through deref), `Index`/`IndexMut` (`a[i]`), `IntoIterator`/`Iterator::next`
(`for`), `Try`/`FromResidual` (`?`), and `PartialEq::eq` (`==`, plus `match` on
`&str` literals). Rows for compiler internals are listed but are not public API.
These are `core::fmt::rt::*`, `std::fmt::Arguments::new_*`, `std::hint::must_use`,
`alloc::alloc::exchange_malloc`, `std::rt::panic_fmt`, `core::panicking::*`,
`std::vec::from_elem`, `[T]::into_vec`, `Formatter::debug_*_finish`,
`LocalKey::new` and `EagerStorage`. They come from rustc's own lowering of
`format_args!`, `vec!`, `Box::new`, `panic!`, `assert*!`, `#[derive(Debug)]` and
`thread_local!`. rcc chooses its own lowering of those and only needs to keep
their behaviour.

Notes for the std author:
- `vec![x; n]` is called 283 times (`from_elem`, needs `Clone`). `vec![a, b, ..]` is called 247 times (`<[T]>::into_vec(Box<[T; N]>)`).
- Generic instantiations seen in local declarations: `Vec` 392 distinct, `Option` 1,043, `Result` 102, `Box` 98, `HashMap` 179 (**125 Fx, 54 RandomState**), `HashSet` 22 (**15 Fx, 7 RandomState**), `RefCell` 10, `Cell` 3 (`bool`, `usize`, `u32`), `Rc` 2 (`Rc<StructLayout>`, **`Rc<str>`**), `Cow` 1 (`Cow<'_, str>`), `BTreeMap` 2, `BTreeSet` 1, `VecDeque` 3, `BinaryHeap` 1 (`BinaryHeap<Reverse<(u32, usize)>>`), `Reverse` 4, `Discriminant` 5, `LazyLock` 1, `LocalKey` 3. Full list: `instances.py`.
- **The default `HashMap<K,V>` / `HashSet<T>` (RandomState, SipHash) is used too, not only Fx**: 51 files `use std::collections::HashMap`, mostly linkers, assemblers and `passes/inline.rs`. `HashMap::new()`/`HashSet::new()` therefore have to exist for `S = RandomState`. Only the default hasher state needs to exist; rcc may make it deterministic (see 7.1).
- HashMap key types (25): `String, u32, &str, BlockId, usize, (usize,usize), ExprId, Value, u64, ExprKey, (usize,u32), (u32,usize), u16, u8, StoreFwdKey, (String,String), PathBuf, (usize,BlockId), (String,u64), (usize,(u32,i64,IrType,Option<IrType>,AliasClass)), (u32,u32), &[u8], (String,bool,PathBuf), (u32,IrType), (usize,u32,u32)`. HashSet element types (17): `u32, String, usize, BlockId, &str, (usize,usize), ExprId, WarningKind, (usize,u32), u64, u8, (u32,u32), ExprKey, StoreFwdKey, PathBuf, (BlockId,usize), (u32,i64,i64)`. So `Hash`+`Eq` is needed for str/String/PathBuf/&[u8], all integer types, tuples up to arity 5, `Option`, and derived ccc types.
- `FromIterator` targets of `collect()`: `Vec<_>` 607, `HashMap<_,_,_>` 75, `HashSet<_,_>` 61, `String` 13 (from `slice::Iter<char>` (`&char`), `Chars`, `Take<Chars>`, `TakeWhile<Chars>`, `Map<..>` yielding `char`/`&str`), `Option<Vec<_>>` 5 (**collect of `Option` items into `Option<Vec>`**, e.g. `Map<slice::Iter<usize>, _>`), `VecDeque` 2. `sum::<usize|u64|f64>`, `product::<usize>`, `unzip` into `(Vec, Vec)`, `partition` into `Vec`. `Extend`: `Vec` (from slices, iterators, `Drain`, `Option`), `HashSet`.
- `?`: on `Result<_, String>` 1,153 sites with the same error type. **519 sites convert `&str` to `String` through `From`** (`Err("msg")?`, `.ok_or("msg")?`). Also `fmt::Error` 11 and `io::Error` 1. On `Option`, 746 sites. No other `From` conversions happen in `?`.
- `Index` arguments: `usize` 2,987 (index plus index_mut); `Range` 534, `RangeFrom` 444, `RangeTo` 249, `RangeFull` 64, `RangeInclusive` 5, `RangeToInclusive` 3 on Vec/[T]/str/String. `&K` on HashMap: `&u32`, `&String`, `&BlockId`, `&str`, tuples.

### 1.1 Per-type table

| type / module | inherent / free (refs) | via std traits (refs) |
|---|---|---|
| `std::vec::Vec` | push (2855), len (1759), new (1092), extend_from_slice (759), is_empty (525), with_capacity (108), pop (66), retain (59), insert (58), remove (41), clear (38), splice (27), resize (18), drain (18), dedup (14), truncate (13), append (7), capacity (6), as_slice (4), split_off (3), reserve (2), retain_mut (1), swap_remove (1), resize_with (1) | Deref::deref (3969), Index::index (2362), IntoIterator::into_iter (1406), DerefMut::deref_mut (1080), IndexMut::index_mut (1033), Clone::clone (232), Extend::extend (151), Default::default (17), PartialEq::ne (3), PartialEq::eq (2), Hash::hash (2) |
| `str` | starts_with (1208), trim (812), contains (415), strip_prefix (292), is_empty (191), len (188), chars (158), split (154), parse (145), as_bytes (131), ends_with (128), find (126), to_lowercase (75), strip_suffix (52), trim_start_matches (50), split_once (42), splitn (31), trim_start (28), rfind (25), bytes (22), replace (21), split_whitespace (18), trim_end_matches (17), char_indices (15), lines (13), trim_end (10), trim_matches (10), rsplit_once (5), replacen (5), to_ascii_lowercase (4), rsplit (4), matches (3), eq_ignore_ascii_case (3), repeat (2), is_char_boundary (2), split_at (1), split_at_checked (1) | PartialEq::eq (7241), ToString::to_string (1464), Index::index (465), PartialEq::ne (83), Into::into (21), Clone::clone (1) |
| `std::string::String` | as_str (317), push_str (257), push (253), is_empty (199), new (113), len (65), as_bytes (57), with_capacity (33), clear (15), pop (8), from_utf8 (6), from_utf8_lossy (6), truncate (5), capacity (2), into_boxed_str (1), reserve (1), replace_range (1) | Deref::deref (3508), Clone::clone (1095), PartialEq::eq (376), Index::index (68), PartialEq::ne (62), Write::write_fmt (17), Into::into (13), ToString::to_string (4), Hash::hash (2), From::from (2) |
| `std::option::Option` | unwrap_or (551), ok_or (519), map (415), is_some (306), is_some_and (262), as_ref (239), copied (229), is_none (213), and_then (210), or_else (122), cloned (100), filter (91), map_or (90), unwrap (71), unwrap_or_else (69), expect (55), ok_or_else (48), take (45), unwrap_or_default (40), flatten (39), is_none_or (30), as_deref (26), or (14), get_or_insert_with (14), as_mut (6), Some (4), map_or_else (3), get_or_insert (3), iter (3), zip (3), and (1) | Try::branch (746), FromResidual::from_residual (746), Clone::clone (225), PartialEq::eq (195), PartialEq::ne (80), Default::default (25), IntoIterator::into_iter (5), Hash::hash (4), Ord::max (2), PartialOrd::ge (1) |
| `[T]` | iter (1938), len (1186), get (299), contains (287), copy_from_slice (272), into_vec (247), iter_mut (212), first (192), is_empty (96), last (80), to_vec (63), sort_by_key (42), last_mut (32), sort_unstable (30), sort (22), starts_with (20), join (20), get_mut (19), sort_by (13), sort_unstable_by_key (7), reverse (6), binary_search_by_key (5), binary_search (4), swap (4), windows (4), first_mut (3), chunks (3), fill (2), chunk_by (2), partition_point (2), rotate_left (2), chunks_exact (1), copy_within (1), split_last (1), sort_unstable_by (1), split_at (1) | IntoIterator::into_iter (315), Index::index (217), IndexMut::index_mut (74), TryInto::try_into (47), PartialEq::eq (35), PartialEq::ne (18) |
| `core::fmt::rt::Argument` | new_display (4183), new_debug (119), new_lower_hex (69), none (57), new_octal (1) |  |
| `std::result::Result` | map_err (88), unwrap_or (80), ok (70), unwrap (48), is_ok (38), is_ok_and (17), expect (13), unwrap_or_default (5), map (4), unwrap_or_else (3), is_err (2), or_else (1), and_then (1) | Try::branch (1684), FromResidual::from_residual (1684) |
| `std::slice::Iter` |  | Iterator::next (1455), Iterator::enumerate (528), Iterator::map (317), Iterator::any (244), Iterator::filter (105), Iterator::all (96), Iterator::flat_map (95), Iterator::find (90), Iterator::position (89), IntoIterator::into_iter (75), Iterator::filter_map (53), Iterator::rev (44), Iterator::cloned (30), Iterator::copied (23), Iterator::zip (19), Iterator::take_while (17), Iterator::find_map (16), Iterator::fold (12), Iterator::rposition (10), Iterator::collect (9), Iterator::chain (8), Iterator::flatten (8), Iterator::product (8), Iterator::skip (5), Iterator::min (4), Iterator::max (4), Iterator::take (2), Iterator::partition (1), Iterator::try_fold (1), Iterator::for_each (1) |
| `std::fmt::Arguments` | new_v1 (2860), new_const (203), new_v1_formatted (81) |  |
| `std::collections::HashMap` | get (1198), insert (745), contains_key (173), entry (156), new (117), get_mut (81), is_empty (66), remove (52), iter (44), keys (26), iter_mut (18), values (17), len (16), retain (13), clear (10), drain (1), reserve (1), into_values (1) | Default::default (258), Index::index (105), IntoIterator::into_iter (35), Clone::clone (9) |
| `std::collections::HashSet` | contains (465), insert (444), remove (40), new (39), is_empty (25), len (23), iter (19), clear (14), intersection (2), retain (2), difference (2) | Default::default (186), IntoIterator::into_iter (32), Extend::extend (15), Clone::clone (14) |
| `std::fmt::Formatter` | write_str (404), debug_tuple_field1_finish (213), write_fmt (169), debug_tuple_field2_finish (74), debug_struct_field1_finish (67), debug_struct_field3_finish (65), debug_struct_field2_finish (60), debug_struct_fields_finish (36), debug_tuple_field3_finish (29), debug_struct_field4_finish (25), debug_struct_field5_finish (16), debug_tuple_field4_finish (6), debug_struct (4), debug_tuple_field5_finish (3) |  |
| `std::fmt (free fns)` | format (1167) |  |
| `std::hint (free fns)` | must_use (1167) |  |
| `std::iter::Enumerate` |  | IntoIterator::into_iter (516), Iterator::next (516), Iterator::map (39), Iterator::filter (15), Iterator::find_map (7), Iterator::rev (4), Iterator::skip (3), Iterator::all (2), Iterator::take (2), Iterator::flat_map (1), Iterator::find (1), Iterator::any (1), Iterator::filter_map (1) |
| `std::ops::Range` | contains (24) | IntoIterator::into_iter (443), Iterator::next (443), Iterator::filter (35), Iterator::map (29), Iterator::rev (26), Iterator::find (23), Iterator::any (18), Iterator::collect (10), Clone::clone (10), Iterator::step_by (9), Iterator::all (3), Iterator::filter_map (2), Iterator::fold (2), Iterator::zip (1) |
| `std::boxed::Box` | new (247), leak (1) | AsRef::as_ref (165), Drop::drop (126), Clone::clone (61), PartialEq::eq (4), Hash::hash (4) |
| `std::iter::Map` |  | Iterator::collect (492), Iterator::sum (24), Iterator::filter (22), Iterator::max (20), Iterator::chain (9), Iterator::min (4), Iterator::unzip (2), Iterator::find (2), Clone::clone (2), Iterator::fold (1), Iterator::max_by_key (1) |
| `u32` | to_le_bytes (234), from_le_bytes (24), saturating_sub (9), wrapping_add (9), leading_zeros (8), count_ones (8), trailing_zeros (7), is_power_of_two (6), wrapping_neg (3), wrapping_sub (3), swap_bytes (2), div_ceil (2), saturating_add (2), wrapping_mul (2), from_str_radix (2), checked_mul (1), checked_add (1), checked_shl (1), wrapping_div (1), wrapping_rem (1), wrapping_shr (1) | Clone::clone (59), Ord::max (50), Shl::shl (9), Default::default (6), PartialEq::eq (5), Rem::rem (5), Hash::hash (4), Ord::min (3), PartialOrd::partial_cmp (2), Ord::cmp (2), TryFrom::try_from (2), PartialOrd::gt (1), AddAssign::add_assign (1) |
| `usize` | div_ceil (32), saturating_sub (28), is_multiple_of (18), next_power_of_two (8), checked_sub (3), trailing_zeros (3), next_multiple_of (2), wrapping_sub (2), checked_add (1), checked_mul (1), is_power_of_two (1) | Ord::max (182), Ord::min (68), PartialEq::eq (38), Clone::clone (38), Ord::cmp (14), From::from (5), Mul::mul (4), Sub::sub (4), Hash::hash (2), Div::div (2), Add::add (2), Ord::clamp (2), ToString::to_string (1), AddAssign::add_assign (1) |
| `std::slice::IterMut` |  | Iterator::next (224), IntoIterator::into_iter (107), Iterator::filter (46), Iterator::enumerate (28), Iterator::flat_map (12), Iterator::zip (6), Iterator::find (5), Iterator::for_each (3), Iterator::skip (2), Iterator::rev (2), Iterator::flatten (1) |
| `u64` | to_le_bytes (114), from_le_bytes (27), trailing_zeros (13), from_str_radix (11), leading_zeros (10), is_power_of_two (10), count_ones (8), wrapping_add (7), wrapping_sub (7), wrapping_shr (5), wrapping_mul (5), wrapping_div (4), wrapping_rem (4), saturating_mul (4), wrapping_neg (3), is_multiple_of (3), swap_bytes (2), saturating_add (2), rotate_left (1), from_ne_bytes (1), div_ceil (1), wrapping_shl (1), saturating_sub (1), trailing_ones (1) | Ord::max (81), Clone::clone (47), ToString::to_string (10), Ord::min (8), AddAssign::add_assign (7), Default::default (4), Ord::cmp (4), PartialEq::eq (4), Add::add (4), Shl::shl (2), Hash::hash (1), PartialOrd::partial_cmp (1), TryFrom::try_from (1), Sub::sub (1) |
| `std::iter::Filter` |  | Iterator::next (73), Iterator::count (72), IntoIterator::into_iter (69), Iterator::map (67), Iterator::collect (54), Iterator::cloned (8), Iterator::find (7), Iterator::filter_map (6), Iterator::filter (5), Iterator::all (4), Iterator::any (3), Iterator::max_by_key (2), Iterator::copied (2), Iterator::min_by_key (1), Iterator::max (1), Iterator::take (1), Iterator::flat_map (1), Iterator::nth (1), Iterator::max_by (1), Iterator::fold (1) |
| `std::vec (free fns)` | from_elem (283) |  |
| `std::rc::Rc` | new (9), make_mut (1) | Deref::deref (230), Clone::clone (18), AsRef::as_ref (11), ToString::to_string (8), PartialEq::eq (4), Hash::hash (2) |
| `std::ops::RangeInclusive` | new (134), contains (103) | IntoIterator::into_iter (13), Iterator::next (13), Iterator::rev (3), Iterator::all (2), Iterator::map (2), Iterator::find (1) |
| `std::vec::IntoIter` |  | Iterator::next (189), Iterator::enumerate (19), Iterator::map (18), Iterator::rev (12), Iterator::zip (6), Iterator::filter (4), Iterator::chain (3), Iterator::filter_map (2), Iterator::skip (1), Iterator::take (1), Iterator::find_map (1), Iterator::find (1), Iterator::partition (1), Iterator::flatten (1), Iterator::any (1), Iterator::peekable (1), Iterator::nth (1) |
| `alloc::alloc (free fns)` | exchange_malloc (247) |  |
| `bool` | then_some (44), then (24) | Clone::clone (89), Default::default (40), PartialEq::eq (25), Hash::hash (11), Not::not (2) |
| `std::path::Path` | new (80), exists (55), join (22), to_string_lossy (16), is_file (13), parent (13), display (9), file_name (6), file_stem (5), is_dir (4), to_str (2), is_absolute (2), to_path_buf (2), strip_prefix (1), components (1), canonicalize (1), with_extension (1) |  |
| `i64` | wrapping_add (36), wrapping_sub (23), to_le_bytes (13), wrapping_mul (10), wrapping_neg (8), from_str_radix (8), wrapping_shl (7), wrapping_shr (5), unsigned_abs (5), from_le_bytes (5), wrapping_div (4), wrapping_rem (4), trailing_zeros (4), checked_add (2), checked_sub (1), abs (1), count_ones (1), leading_zeros (1), saturating_add (1), checked_neg (1) | Clone::clone (47), Ord::max (10), ToString::to_string (8), PartialEq::eq (7), Hash::hash (3), TryFrom::try_from (2), Sub::sub (2), Add::add (2), PartialOrd::partial_cmp (1), Ord::cmp (1), Default::default (1), Ord::min (1), Ord::clamp (1) |
| `u8` | is_ascii_digit (65), is_ascii_alphanumeric (33), is_ascii_alphabetic (11), is_ascii_hexdigit (10), is_ascii_whitespace (8), saturating_add (2), wrapping_sub (2), wrapping_add (1) | PartialEq::eq (37), Clone::clone (23), BitAnd::bitand (5), Hash::hash (3), Default::default (3), From::from (3), ToString::to_string (2), Ord::max (1), Ord::min (1) |
| `std::iter::FlatMap` |  | IntoIterator::into_iter (74), Iterator::next (74), Iterator::filter_map (19), Iterator::any (11), Iterator::map (8), Iterator::filter (7), Iterator::flat_map (2), Iterator::collect (2), Iterator::take_while (1) |
| `std::cell::Ref` |  | Deref::deref (192) |
| `std::str::Chars` | as_str (3) | Iterator::next (65), Iterator::all (34), Iterator::count (32), IntoIterator::into_iter (24), Iterator::map (10), Iterator::any (8), Iterator::collect (8), Iterator::enumerate (2), Iterator::take_while (1), Iterator::take (1), Iterator::last (1) |
| `std::str::Split` |  | Iterator::map (66), Iterator::next (45), Iterator::collect (21), IntoIterator::into_iter (15), Iterator::any (12), Iterator::filter (8), Iterator::count (2), Iterator::all (1), Iterator::filter_map (1) |
| `[T; N]` | map (1) | IntoIterator::into_iter (84), Index::index (48), IndexMut::index_mut (19), Clone::clone (7), PartialEq::eq (3), Default::default (2), Hash::hash (1), PartialOrd::partial_cmp (1), Ord::cmp (1) |
| `std::iter::Rev` |  | IntoIterator::into_iter (68), Iterator::next (68), Iterator::take (8), Iterator::find (7), Iterator::find_map (2), Iterator::take_while (2), Iterator::map (2), Iterator::fold (2), Iterator::flat_map (1), Iterator::chain (1), Iterator::copied (1) |
| `std::io (free fns)` | _eprint (128), _print (23), stderr (1), stdout (1), stdin (1) |  |
| `std::collections::hash_map::Entry` | or_default (76), or_insert (57), or_insert_with (21) |  |
| `u16` | to_le_bytes (110), from_le_bytes (15), swap_bytes (2), from_be_bytes (1), wrapping_add (1), wrapping_sub (1) | Clone::clone (9), Default::default (2), Ord::cmp (1) |
| `std::rt (free fns)` | panic_fmt (131) |  |
| `char` | is_ascii_digit (56), is_ascii_alphanumeric (18), is_whitespace (12), len_utf8 (9), is_ascii_alphabetic (9), is_alphanumeric (4), encode_utf8 (3), from_u32 (2), to_ascii_lowercase (2), is_ascii_graphic (1), is_alphabetic (1), is_ascii_whitespace (1) | Clone::clone (2), ToString::to_string (2), PartialEq::eq (1) |
| `std::path::PathBuf` | new (2), as_path (2), push (1), pop (1) | Deref::deref (70), From::from (13), PartialEq::eq (5), Clone::clone (4) |
| `std::iter::FilterMap` |  | Iterator::collect (68), Iterator::filter (4), Iterator::map (4), Iterator::fold (3), Iterator::find_map (2), Iterator::sum (2), Iterator::flatten (2), Iterator::min (1), Iterator::any (1), Iterator::filter_map (1), Iterator::find (1), Iterator::peekable (1) |
| `i32` | to_le_bytes (44), from_le_bytes (9), wrapping_neg (3), wrapping_shr (2), saturating_mul (2), saturating_add (2), trailing_zeros (2), wrapping_add (2), wrapping_mul (1) | PartialEq::eq (10), Clone::clone (3), TryFrom::try_from (1), Ord::max (1), Hash::hash (1), PartialOrd::partial_cmp (1), Ord::cmp (1), Mul::mul (1) |
| `std::mem (free fns)` | take (45), discriminant (18), replace (9), needs_drop (5), swap (4), drop (2) |  |
| `core::fmt::rt::UnsafeArg` | new (81) |  |
| `std::cell::RefCell` | borrow_mut (39), borrow (14), get_mut (13), new (12), replace (1), into_inner (1) | Clone::clone (1) |
| `std::fmt::DebugStruct` | field (71), finish (4) |  |
| `f64` | to_bits (38), powi (4), from_bits (3), is_sign_negative (2), is_finite (1), total_cmp (1), min (1), round (1), to_le_bytes (1) | Clone::clone (13), PartialEq::eq (6), Neg::neg (1) |
| `std::collections::hash_map::Iter` |  | Iterator::next (25), Iterator::filter (24), Iterator::map (7), IntoIterator::into_iter (5), Iterator::collect (4), Iterator::find (2), Iterator::filter_map (1), Iterator::any (1) |
| `std::fs (free fns)` | read (28), write (17), set_permissions (9), canonicalize (7), read_to_string (2), read_dir (2), remove_file (1), create_dir_all (1) |  |
| `i128` | checked_mul (4), abs (4), wrapping_shl (2), wrapping_mul (2), wrapping_sub (2), wrapping_add (2), wrapping_div (2), wrapping_rem (2), wrapping_shr (2), wrapping_neg (2), to_le_bytes (2), leading_zeros (2), rem_euclid (1) | Ord::max (14), Ord::min (14), PartialEq::eq (2), Clone::clone (1), Hash::hash (1), PartialOrd::partial_cmp (1), Ord::cmp (1) |
| `std::borrow::Cow` | into_owned (12) | Deref::deref (39), ToString::to_string (11) |
| `std::env (free fns)` | var (53), current_dir (3), temp_dir (1), current_exe (1), args (1) |  |
| `std::time::Instant` | elapsed (26), now (25) |  |
| `std::array::IntoIter` |  | Iterator::next (44), Iterator::filter_map (2), Iterator::collect (2), Iterator::flatten (1), Iterator::find (1) |
| `std::str (free fns)` | from_utf8 (48) |  |
| `std::collections::hash_set::Iter` |  | Iterator::next (28), Iterator::copied (10), Iterator::filter_map (2), Iterator::all (2), Iterator::cloned (1), Iterator::flat_map (1), Iterator::map (1), Iterator::find (1), Iterator::filter (1) |
| `std::iter::Flatten` |  | IntoIterator::into_iter (19), Iterator::next (19), Iterator::filter (3), Iterator::collect (2), Iterator::any (1) |
| `std::iter::Zip` |  | IntoIterator::into_iter (10), Iterator::next (10), Iterator::map (10), Iterator::enumerate (4), Iterator::filter (4), Iterator::all (2), Iterator::take (2), Iterator::partition (1), Iterator::collect (1) |
| `u128` | leading_zeros (12), to_le_bytes (10), from_le_bytes (9), wrapping_add (3), wrapping_div (2), wrapping_rem (2), wrapping_shr (2), div_ceil (2), overflowing_add (1) |  |
| `std::iter (free fns)` | repeat_n (33), once (7), from_fn (1) |  |
| `std::iter::Chain` |  | IntoIterator::into_iter (10), Iterator::next (10), Iterator::collect (10), Iterator::chain (4), Iterator::map (2), Iterator::find (2), Iterator::filter (2), Iterator::copied (1) |
| `std::cell::RefMut` |  | DerefMut::deref_mut (39), Deref::deref (1) |
| `std::iter::Copied` |  | Iterator::collect (15), Iterator::filter (15), Iterator::flatten (5), Iterator::chain (2), Iterator::find (1), Iterator::take (1), Iterator::zip (1) |
| `core::panicking (free fns)` | panic (34), assert_failed (4) |  |
| `std::collections::hash_map::IterMut` |  | IntoIterator::into_iter (18), Iterator::next (18) |
| `std::iter::Take` |  | IntoIterator::into_iter (15), Iterator::next (15), Iterator::collect (2), Iterator::map (2), Iterator::enumerate (1), Iterator::filter_map (1) |
| `std::str::SplitN` |  | Iterator::collect (28), Iterator::next (2), Iterator::take (2) |
| `std::iter::Skip` |  | IntoIterator::into_iter (13), Iterator::next (13), Iterator::rev (2), Iterator::take (2), Iterator::position (1) |
| `std::collections::hash_map::Keys` |  | Iterator::map (9), Iterator::copied (5), IntoIterator::into_iter (4), Iterator::next (4), Iterator::cloned (3), Iterator::collect (3), Iterator::filter (2) |
| `i16` | to_le_bytes (20), from_le_bytes (3) | Clone::clone (1), PartialEq::eq (1), Hash::hash (1), PartialOrd::partial_cmp (1), Ord::cmp (1) |
| `std::cell::Cell` | get (9), new (8), set (8) | Clone::clone (3) |
| `f32` | to_bits (25), is_sign_negative (1), to_le_bytes (1) |  |
| `std::time::Duration` | as_secs_f64 (26) |  |
| `std::str::Bytes` |  | Iterator::all (9), Iterator::next (8), IntoIterator::into_iter (6), Iterator::collect (1), Iterator::any (1) |
| `std::iter::TakeWhile` |  | Iterator::count (14), Iterator::filter (2), IntoIterator::into_iter (2), Iterator::next (2), Iterator::collect (1), Iterator::find_map (1), Iterator::any (1) |
| `(tuple)` |  | PartialEq::eq (12), PartialEq::ne (5), Clone::clone (4), Ord::cmp (2) |
| `std::str::SplitWhitespace` |  | Iterator::next (9), Iterator::collect (6), IntoIterator::into_iter (3), Iterator::nth (2), Iterator::take (1) |
| `std::vec::Drain` |  | IntoIterator::into_iter (6), Iterator::next (6), Iterator::enumerate (5), Iterator::collect (2), Iterator::map (2) |
| `std::collections::hash_map::Values` |  | Iterator::any (6), Iterator::map (3), Iterator::copied (3), IntoIterator::into_iter (3), Iterator::next (3), Iterator::filter (1), Iterator::fold (1) |
| `std::str::CharIndices` |  | Iterator::skip (8), IntoIterator::into_iter (4), Iterator::next (4), Iterator::rev (2), Iterator::chain (1) |
| `std::str::Lines` |  | Iterator::next (6), IntoIterator::into_iter (5), Iterator::enumerate (2), Iterator::collect (2), Iterator::map (2), Iterator::flat_map (1), Iterator::filter (1) |
| `std::iter::StepBy` |  | IntoIterator::into_iter (9), Iterator::next (9) |
| `isize` |  | Hash::hash (15), PartialOrd::partial_cmp (1), Ord::cmp (1) |
| `std::thread::LocalKey` | with (11), new (5) |  |
| `std::iter::Cloned` |  | Iterator::collect (15) |
| `std::collections::hash_map::IntoIter` |  | Iterator::next (3), Iterator::filter (3), Iterator::map (3), Iterator::collect (2), Iterator::filter_map (2), Iterator::chain (2) |
| `std::ffi::OsStr` | to_string_lossy (8), to_str (5) |  |
| `std::process::Command` | args (5), new (3), output (2), arg (2) |  |
| `std::thread::local_impl::EagerStorage` | get (5), new (5) |  |
| `std::mem::Discriminant` |  | PartialEq::eq (9) |
| `std::fs::Permissions` |  | PermissionsExt::from_mode (9) |
| `std::cmp (free fns)` | min (6), max (1) |  |
| `std::collections::VecDeque` | pop_front (3), push_back (3), new (1) |  |
| `std::collections::BTreeSet` | insert (3), iter (2), new (1) | IntoIterator::into_iter (1) |
| `std::collections::BTreeMap` | new (1), is_empty (1), entry (1), insert (1), values (1) | IntoIterator::into_iter (1) |
| `std::option::IntoIter` |  | Iterator::flatten (3), Iterator::chain (1), Iterator::flat_map (1) |
| `std::collections::BinaryHeap` | push (2), new (1), peek (1), pop (1) |  |
| `i8` |  | Clone::clone (1), PartialEq::eq (1), Hash::hash (1), PartialOrd::partial_cmp (1), Ord::cmp (1) |
| `std::str::RSplit` |  | Iterator::next (4) |
| `std::slice (free fns)` | from_ref (4) |  |
| `std::collections::hash_set::IntoIter` |  | Iterator::next (3), Iterator::map (1) |
| `std::cmp::Ordering` | then (2), reverse (1), then_with (1) |  |
| `std::slice::Chunks` |  | Iterator::enumerate (2), IntoIterator::into_iter (1), Iterator::next (1) |
| `std::fs::File` | open (2) | Read::read (2) |
| `std::slice::ChunkBy` |  | IntoIterator::into_iter (2), Iterator::next (2) |
| `std::iter::Peekable` | peek (2) | Iterator::all (1), Iterator::next (1) |
| `std::slice::Windows` |  | Iterator::enumerate (1), Iterator::any (1), Iterator::all (1), Iterator::position (1) |
| `std::path::Display` |  | ToString::to_string (3) |
| `std::str::Matches` |  | Iterator::count (3) |
| `std::sync::LazyLock` | new (1) | Deref::deref (2) |
| `std::collections::btree_set::Iter` |  | Iterator::cloned (2), Iterator::next (1) |
| `std::option::Iter` |  | Iterator::flatten (2), Iterator::map (1) |
| `std::process (free fns)` | exit (2), id (1) |  |
| `std::slice::ChunksExact` | remainder (1) | IntoIterator::into_iter (1), Iterator::next (1) |
| `std::iter::FromFn` |  | Iterator::any (1), IntoIterator::into_iter (1), Iterator::next (1) |
| `std::thread::Builder` | new (1), stack_size (1), spawn (1) |  |
| `std::ptr (free fns)` | eq (2) |  |
| `std::collections::hash_set::Intersection` |  | Iterator::copied (2) |
| `std::iter::Once` |  | Iterator::chain (2) |
| `std::collections::hash_set::Difference` |  | Iterator::copied (2) |
| `std::collections::hash_map::VacantEntry` | insert (2) |  |
| `std::fs::ReadDir` |  | Iterator::flatten (2) |
| `std::fs::DirEntry` | file_name (2) |  |
| `std::ffi::OsString` |  | Deref::deref (2) |
| `std::process::ExitStatus` | success (2) |  |
| `dyn std::any::Any` | downcast_ref (2) |  |
| `std::sync::atomic::AtomicU64` | new (1), fetch_add (1) |  |
| `std::path::Components` |  | IntoIterator::into_iter (1), Iterator::next (1) |
| `std::collections::btree_map::Values` |  | IntoIterator::into_iter (1), Iterator::next (1) |
| `std::collections::hash_map::OccupiedEntry` | get (1), insert (1) |  |
| `std::io::StdoutLock` |  | Write::write_all (1), Write::flush (1) |
| `std::string::FromUtf8Error` | into_bytes (1) |  |
| `std::io::Stderr` |  | IsTerminal::is_terminal (1) |
| `std::ops::RangeFrom` |  | Iterator::zip (1) |
| `std::iter::Map<impl Iterator, {closure@src/ir/lowering/expr_calls.rs:990:20: 990:24}>` |  | Iterator::sum (1) |
| `std::collections::btree_map::Entry` | or_default (1) |  |
| `std::collections::btree_map::Iter` |  | Iterator::next (1) |
| `std::collections::hash_map::Drain` |  | Iterator::collect (1) |
| `std::collections::hash_map::IntoValues` |  | Iterator::filter (1) |
| `std::io::Stdout` | lock (1) |  |
| `std::io::Stdin` |  | Read::read_to_end (1) |
| `std::thread::JoinHandle` | join (1) |  |
| `std::env::Args` |  | Iterator::collect (1) |


#### std traits invoked on generic params / closures / dyn

| trait | method (refs) [self kinds] |
|---|---|
| `std::clone::Clone` | clone (2) [impl Trait] |
| `std::convert::AsRef` | as_ref (1) [projection] |
| `std::convert::Into` | into (5) [impl Trait] |
| `std::iter::IntoIterator` | into_iter (2) [impl Trait] |
| `std::iter::Iterator` | any (2) [impl Trait], next (2) [impl Trait/projection], map (1) [impl Trait], collect (1) [impl Trait] |
| `std::ops::Fn` | call (1094) [dyn for std::ops::Fn(&'a frontend::parser::ast::Expr) -> std::option::Option/dyn for std::ops::Fn(&'a ir::instruction::Operand) -> bool/dyn for std::ops::Fn(&'a str) -> std::option::Option/dyn for std::ops::Fn(&'a str) -> std::string::String/dyn std::ops::Fn() -> ir::constants::IrConst/dyn std::ops::Fn(ir::instruction::BlockId) -> ir::instruction::BlockId/dyn std::ops::Fn(ir::instruction::Value) -> ir::instruction::Value/dyn std::ops::Fn(u32, u32, u32) -> bool/dyn std::ops::Fn(u8) -> std::option::Option/impl Trait/{closure}] |
| `std::ops::FnMut` | call_mut (290) [T(param)/dyn for std::ops::FnMut(&'a str, &'b [u8]) -> std::result::Result/dyn std::ops::FnMut() -> ir::instruction::Value/impl Trait/{closure}] |
| `std::ops::FnOnce` | call_once (3) [impl Trait] |


#### std trait methods invoked on ccc-defined types (impls ccc must provide, derived or by hand)

| trait | method (refs) | #distinct self types | sample self types |
|---|---|---|---|
| `std::clone::Clone` | clone (601) | 87 | backend::arm::assembler::encoder::RelocType, backend::arm::assembler::encoder::Relocation, backend::arm::assembler::parser::AsmDirective, backend::arm::assembler::parser::AsmStatement, backend::arm::assembler::parser::DataValue, backend::arm::assembler::parser::Operand |
| `std::cmp::PartialEq` | eq (1221) | 40 | backend::Target, backend::arm::codegen::peephole::Access, backend::arm::codegen::peephole::LineKind, backend::arm::darwin::SectionKind, backend::elf_writer_common::ExprToken, backend::i686::codegen::peephole::LineKind |
| `std::cmp::PartialEq` | ne (200) | 26 | backend::Target, backend::arm::codegen::peephole::Access, backend::arm::codegen::peephole::LineKind, backend::common::GlobalSection, backend::i686::codegen::peephole::LineKind, backend::regalloc::PhysReg |
| `std::cmp::PartialOrd` | gt (1) | 1 | ir::constants::ConstHashKey |
| `std::default::Default` | default (35) | 20 | backend::arm::codegen::intrinsics::VecSpillPlan, backend::arm::darwin::SectionKind, backend::arm::darwin::State, backend::linker_common::args::LinkerArgs, backend::state::RegCache, common::types::AddressSpace |
| `std::hash::Hash` | hash (30) | 13 | common::types::AccessQuals, common::types::AddressSpace, common::types::CType, common::types::EnumType, common::types::IrType, ir::constants::ConstHashKey |
| `std::iter::IntoIterator` | into_iter (1) | 1 | backend::x86::codegen::peephole::passes::store_forwarding::SmallVecIter |
| `std::iter::Iterator` | next (1) | 1 | backend::x86::codegen::peephole::passes::store_forwarding::SmallVecIter |
| `std::ops::Deref` | deref (70) | 2 | ir::lowering::definitions::GlobalInfo, ir::lowering::definitions::LocalInfo |
| `std::ops::DerefMut` | deref_mut (2) | 1 | ir::lowering::definitions::LocalInfo |
| `std::string::ToString` | to_string (4) | 1 | ir::instruction::BlockId |


#### fn items referenced other than by a returning call (fn-item values such as `.map(str::trim)`, or diverging calls such as `panic_fmt`)

| path | uses | example |
|---|---|---|
| `str::trim` | 58 | src/common/error.rs:843 |
| `std::rt::panic_fmt` | 22 | src/common/const_arith.rs:205 |
| `core::panicking::panic` | 19 | src/common/long_double.rs:182 |
| `<str as std::string::ToString>::to_string` | 4 | src/common/error.rs:866 |
| `char::is_whitespace` | 4 | src/frontend/preprocessor/pipeline.rs:946 |
| `std::option::Option::Some` | 4 | src/passes/block_place.rs:58 |
| `core::panicking::assert_failed` | 4 | src/backend/elf/object_writer.rs:347 |
| `std::vec::Vec::new` | 2 | src/common/error.rs:358 |
| `<std::string::String as std::convert::From>::from` | 2 | src/backend/arm/codegen/peephole.rs:412 |
| `std::process::exit` | 2 | src/lib.rs:34 |
| `std::vec::Vec::len` | 1 | src/common/encoding.rs:38 |
| `std::option::Option::is_some` | 1 | src/common/const_eval.rs:84 |
| `<std::collections::HashSet as std::default::Default>::default` | 1 | src/frontend/preprocessor/pipeline.rs:139 |
| `u64::wrapping_div` | 1 | src/common/const_arith.rs:62 |
| `u64::wrapping_rem` | 1 | src/common/const_arith.rs:70 |
| `u64::saturating_add` | 1 | src/passes/block_place.rs:41 |

### Associated constants (grep-estimated, source)
`u32::MAX` 101, `usize::MAX` 64, `i32::MAX` 61, `i32::MIN` 54, `u64::MAX` 26, `i64::MAX` 17, `f64::INFINITY` 15, `i64::MIN` 14, `f64::NAN` 7, `u16::MAX` 6, `f64::NEG_INFINITY` 5, `i16::MIN/MAX`, `i8::MIN/MAX`, `u8::MAX`, `u128::MAX`, `i128::MAX/MIN`, `f32::INFINITY`, `f32::NAN`. `std::f64::consts` is not used.

### Other std items seen only as types or paths (MIR local types)
`std::num::ParseIntError`, `ParseFloatError`, `TryFromIntError`, `std::array::TryFromSliceError` (`<[u8;N]>::try_from(&[u8])` via `TryInto` 47), `std::str::Utf8Error`, `std::string::FromUtf8Error`, `std::env::VarError`, `std::io::Error`, `std::fmt::Error`, `std::path::Component`/`Components`/`StripPrefixError`, `std::process::Output` (fields `.status`, `.stdout`, `.stderr`), `std::marker::PhantomData` (1), `std::convert::Infallible`, `std::ops::ControlFlow` (from `?` and `try_fold`), `std::sync::atomic::Ordering::Relaxed`, `std::cmp::Ordering`.

### Not used (std author can skip)
No `unsafe` blocks, `extern`, `static mut`, `Arc`, `Mutex`, channels, `catch_unwind`, panic hooks, `std::fs::File::create`, `BufReader`/`BufWriter`, `OsString` building, `char::from_digit`, `f64::consts`, `Box<dyn ...>` (zero!), `async`, `include_str!`, `todo!`/`unimplemented!`, or `#[repr]`. The `Once` uses sit only in feature-gated code (stripped).

---

## 2. Iterator usage (compiler-verified, MIR)

**Adapters and consumers called on `Iterator`** (refs, all iterator types together):
`next` 3,423 (mostly `for`), `collect` 763, `map` 606, `enumerate` 592, `any` 313, `filter` 310, `all` 157, `find` 145, `count` 123, `flat_map` 115, `rev` 95, `filter_map` 93, `position` 91, `copied` 49, `cloned` 44, `zip` 34, `chain` 33, `find_map` 29, `sum` 27, `flatten` 25, `max` 25, `take` 23, `fold` 22, `take_while` 21, `skip` 19, `rposition` 10, `min` 9, `step_by` 9, `product` 8, `nth` 4, `for_each` 4, `partition` 3, `max_by_key` 3, `peekable` 2 (+ `Peekable::peek` 2), `unzip` 2, `min_by_key` 1, `max_by` 1, `last` 1, `try_fold` 1.

Not used: `windows`/`chunks` exist as **slice** methods only (`windows` 4, `chunks` 3, `chunks_exact` 1 with `remainder`, `chunk_by` 2). Also unused: `scan`, `inspect`, `skip_while`, `map_while`, `cycle`, `last` (except 1), `sum` of floats except one `f64` site, `sort` on iterators, `DoubleEndedIterator::next_back` (direct), `ExactSizeIterator::len` (direct).

**Iterator types that appear** (each needs `Iterator` and, where `rev`/`rposition` is called, `DoubleEndedIterator`; `rev` on `Enumerate` and `Skip` also needs `ExactSizeIterator`):
`slice::Iter`, `slice::IterMut`, `vec::IntoIter`, `vec::Drain`, `array::IntoIter`, `option::IntoIter`/`option::Iter`, `ops::Range`/`RangeInclusive`/`RangeFrom` (`zip` only), `str::Chars`, `CharIndices`, `Bytes`, `Split<char|&str|closure>`, `SplitN`, `RSplit`, `SplitWhitespace`, `Lines`, `Matches`, `hash_map::{Iter,IterMut,Keys,Values,IntoIter,IntoValues,Drain}`, `hash_set::{Iter,IntoIter,Intersection,Difference}`, `btree_map::{Iter,Values}`, `btree_set::Iter`, `slice::{Windows,Chunks,ChunksExact,ChunkBy}`, `path::Components`, `fs::ReadDir` (`flatten` over `Result<DirEntry>`), `env::Args`, `iter::{Map,Filter,FilterMap,FlatMap,Flatten,Enumerate,Zip,Chain,Rev,Take,Skip,TakeWhile,StepBy,Copied,Cloned,Peekable,Once,RepeatN,FromFn}`, and one ccc-defined `impl Iterator for SmallVecIter<'a>` (`src/backend/x86/codegen/peephole/passes/store_forwarding.rs:90`).
Free functions: `iter::repeat_n` 33, `iter::once` 7, `iter::from_fn` 1.

**`for` loops / `IntoIterator::into_iter` receivers**: `&Vec<T>` 1,033, `Range<integer>` 443, `&[T]` 309, `Vec<T>` (owned) 262, `&mut Vec<T>` 111, `[T; N]` owned 50 (**edition-2021 by-value array iteration**) and `&[T; N]` 34, `&HashSet` 28, `&HashMap` 20, `HashMap` owned 15, `RangeInclusive` 13, `&mut [T]` 6, `Option` 5, `HashSet` owned 4, `&BTreeSet`/`&BTreeMap` 1 each, `impl IntoIterator<Item=u8>` 2, and every iterator type above (identity `into_iter`).

**Generic iterator parameters**: `impl Iterator<Item = T>` in argument position (5), `impl IntoIterator<Item = u8>` (`src/passes/profile.rs:29`), **`impl Iterator<Item: AsRef<str>>`, which uses associated-type-bound syntax** (`src/backend/common.rs:137`), and `-> impl Iterator<Item = ..> + '_` return position (2, e.g. `src/frontend/preprocessor/macro_defs.rs:167`).

---

## 3. Traits (source, grep-estimated; impls cross-checked against MIR)

### 3.1 Traits ccc defines (8)

| trait | location | required / default methods | notes |
|---|---|---|---|
| `ArchCodegen` | `src/backend/traits.rs:104` | 121 / 70 | used as **`&mut dyn ArchCodegen`** (`src/backend/generation.rs:489`, 21 unsize coercions). Default methods call required ones through `self`. 4 impls (X86, I686, Arm, Riscv Codegen). |
| `F128SoftFloat` | `src/backend/f128_softfloat.rs:42` | 47 / 1 | used only through generic fns `fn f<T: F128SoftFloat + ?Sized>(cg: &mut T, ..)` (12 fns, **`?Sized` bound**). 2 impls (Arm, Riscv). |
| `InlineAsmEmitter` | `src/backend/inline_asm.rs:113` | 9 / 4 | `&mut dyn InlineAsmEmitter` (`src/backend/inline_asm.rs:404`). 4 impls. |
| `X86Arch` | `src/backend/elf_writer_common.rs:37` | 8 / 7 | **associated functions without `self`** (`fn elf_machine() -> u16;`, default `fn elf_flags() -> u32 { 0 }`), called as `A::reloc_abs(..)` from the generic `struct ElfWriterCore<A: X86Arch>` / `impl<A: X86Arch> ElfWriterCore<A>` (`:275`, `:303`). 2 impls on unit structs. |
| `GlobalSymbolOps: Clone` | `src/backend/linker_common/symbols.rs:35` | 11 / 0 | **supertrait**. Static constructors return `Self` (`fn new_defined(..) -> Self`). Used through `G: GlobalSymbolOps` generics with `HashMap<String, G>` (13 generic fns in `linker_common/`). 2 impls. |
| `StructLayoutProvider` | `src/common/types.rs:130` | 1 / 0 | **implemented for a std type**: `impl StructLayoutProvider for FxHashMap<String, Rc<StructLayout>>` (`:135`). Used as `&dyn StructLayoutProvider` (152 coercions, one of them from `&HashMap<..>`). |
| `TypeConvertContext` | `src/common/type_builder.rs:23` | 5 / 1 | `&dyn TypeConvertContext`. 2 impls (SemanticAnalyzer, Lowerer). |
| `OperandExt` | `src/passes/lftr.rs:110` | 1 / 0 | extension trait on `ir::Operand`, local to one module. |

No trait has generic parameters, associated types, associated consts, or generic methods. Object safety: `ArchCodegen`, `InlineAsmEmitter`, `StructLayoutProvider` and `TypeConvertContext` are used as `dyn`, so their vtables need every method; `ArchCodegen` has 191 entries.

### 3.2 Trait impls written by hand (52; `#[derive]`s are separate)

| trait | count | sites |
|---|---|---|
| `Default` | 13 | `SemaResult`, `SemanticAnalyzer`, `MacroTable`, `ConditionalStack`, `IrModule`, `Preprocessor`, `Driver`, `X86Codegen`, `ArmCodegen`, `RiscvCodegen`, `WarningConfig`, `DiagnosticEngine`, `SymbolTable` |
| `std::fmt::Display` | 7 | `TokenKind` (`src/frontend/lexer/token.rs:199`), `BlockId`, `x86 Register`, `SourceLocation`, `TempFile`, `CType` (`src/common/types.rs:1509`), `Severity` |
| `std::fmt::Debug` | 4 | `ParsedDeclAttrs`, `FunctionAttributes`, `Declaration`, `DeclAttributes` (`src/frontend/parser/ast.rs:114`); they use `f.debug_struct(..).field(..).finish()` |
| `std::ops::Deref` + `DerefMut` | 2+2 | `LocalInfo`, `GlobalInfo` (`src/ir/lowering/definitions.rs:102-129`), with `type Target = VarInfo` |
| `PartialEq`, `Eq`, `std::hash::Hash` | 1 each | `AccessQuals` (`src/common/types.rs:281-293`); `fn hash<H: Hasher>` |
| `std::hash::Hasher` | 1 | `FxHasher` (`src/common/fx_hash.rs:32`): `write`, `write_u8/u16/u32/u64/usize`, `finish` (`BuildHasherDefault<FxHasher>` needs `FxHasher: Default`, which is derived) |
| `Iterator` | 1 | `SmallVecIter<'a>` (`type Item = ...`) |
| `Drop` | 1 | **`TempFile`** (`src/common/temp_files.rs:81`): deletes the temp file unless `keep` is set. Observable. |
| local traits | 16 | `ArchCodegen` ×4, `InlineAsmEmitter` ×4, `X86Arch` ×2, `GlobalSymbolOps` ×2, `F128SoftFloat` ×2, `TypeConvertContext` ×2, `StructLayoutProvider` ×1 (for `FxHashMap<..>`), `OperandExt` ×1 |

None of these are implemented by hand: `From`/`Into`, `PartialOrd`/`Ord`, `Add`/`Sub`/other arithmetic operators, `Index`, `FromStr`, `Error`, `AsRef`, `Borrow`, `IntoIterator`, `Clone`, `Copy`. **There is no operator overloading apart from `Deref`/`DerefMut`.**

### 3.3 Derives (249 `#[derive]` attributes)
Traits derived: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Default` (with **`#[default]` on enum variants**, 4×, `src/passes/mod_ref.rs:36`), `PartialOrd`, `Ord`. `PartialOrd`/`Ord` are derived twice: `ir::constants::ConstHashKey` (an enum with payloads; `ConstHashKey > ..` is used) and `ir::instruction::Value` (a newtype). Most common: `Debug, Clone` 83, `Debug, Clone, Copy, PartialEq, Eq` 26, `Clone, Copy` 21. MIR shows std trait methods called on ccc types: `PartialEq::eq` 1,221 (40 types), `Clone::clone` 601 (87 types), `Default::default` 35 (20 types), `Hash::hash` 30 (13 types, from derived `Hash` on nested fields), `PartialEq::ne` 200.
Derived `Hash` on enums hashes the discriminant as `isize` (`Hash for isize` shows 15 refs). Derived `Debug` on payload-carrying enums and structs is used with `{:?}` (section 4).

### 3.4 `dyn Trait` (75 source occurrences: 73 `&dyn`/`&mut dyn`, 0 `Box<dyn>`)
`&dyn StructLayoutProvider` 22, `&mut dyn ArchCodegen` 18, `&mut dyn FnMut() -> Value` 8, `&mut dyn InlineAsmEmitter` 7, `&dyn Fn(&str) -> Option<usize>` 5, `&dyn TypeConvertContext` 3, `&dyn Fn(&Operand) -> bool` 2, `&dyn Fn(u32,u32,u32) -> bool` 2, and one each of `&dyn Fn() -> IrConst`, `Fn(Value) -> Value`, `Fn(BlockId) -> BlockId`, `Fn(&str) -> String`, `&mut dyn FnMut(&str, &[u8]) -> Result<..>`, `Fn(u8) -> Option<&'l str>`, `Fn(&Expr) -> Option<IrConst>`, plus `dyn Any + Send` (thread-join payload, `src/lib.rs:38`). Several are HRTB (`for<'a> Fn(&'a str) -> ..`). The MIR's 1,686 `dyn std::fmt::Debug` uses come from derived Debug internals only.

### 3.5 Generics with bounds (34 generic fns that are not lifetime-only; 41 lifetime-only)
Distinct bounds: `F: FnMut(&str)`, `F: FnMut(&mut IrFunction) -> usize` (a **`where` clause**, `src/ir/module.rs:277`; only 2 `where` clauses exist), `F: FnMut(...)` (`src/passes/mod.rs:75`), `F: Fn(&str) -> String`, `T: Copy`, `T` / `K` unbounded, `G: GlobalSymbolOps`, `T: F128SoftFloat + ?Sized`, `A: X86Arch` (on a struct and its impl), `H: std::hash::Hasher`, and **const generics `fn collect_non_nop_indices<const N: usize>(.., out: &mut [usize; N])`** (`src/backend/i686/codegen/peephole.rs:1188`, `src/backend/x86/codegen/peephole/types.rs:701`).
`impl Trait` in argument position (60): `impl Into<String>` 10, `impl FnMut(..)` / `impl Fn(..)` / `impl FnOnce(&mut Self) -> T` (~40, e.g. `src/frontend/parser/parse.rs:402`), `impl Iterator<Item=..>` 5, `impl IntoIterator<Item=u8>` 1. Return position: 2 (`-> impl Iterator`).
Generic structs: 15, of which 14 have only a lifetime parameter and 1 is `ElfWriterCore<A: X86Arch>`. 1 lifetime-only enum (`PreparsedFloat<'a>`). Lifetime-generic impls: 5, plus `impl Finder<'_>`.

### 3.6 Closures and fn pointers (MIR)
- **4,165 closure bodies**: 2,228 called through `&mut self` (FnMut), 1,493 by value (FnOnce), 444 through `&self` (Fn). There are 1,094 `Fn::call` and 290 `FnMut::call_mut` dispatches, about 1,021 of them direct calls of local closure variables (`let f = |x| ..; f(a)`).
- Only **2 `move` closures** (`src/passes/gep_reassoc.rs:123`). All other captures are inferred, using edition-2021 disjoint-field capture.
- Closures are stored in struct fields as `&dyn Fn` (e.g. `src/ir/lowering/global_init.rs:1501`). Together with fn-pointer fields there are 16 such fields. No closure is boxed or returned.
- Closures and fn items coerce to **fn pointers**: `fn(u8) -> &str` (`xreg_name`/`wreg_name`, `src/backend/arm/codegen/peephole.rs:1130`), `fn(u64,u64) -> u64` (`src/common/const_arith.rs:29`), `for<'a> fn(&'a str) -> Result<i64,String>`, an 11-argument `fn(&mut Vec<u8>, u32, ...)` (`write_elf64_shdr`, called 39× through a pointer, `src/backend/x86/linker/emit_exec.rs:1093`), `resolve_lib*` (`src/backend/arm/linker/link.rs:85`), and a `fn(&'a (Operand, Operand, BlockId)) -> &'a Operand` (`src/passes/phi_cmp.rs:66`). There are 11 fn-pointer types in source and 58 indirect fn-pointer calls.
- Fn items used as values: `str::trim` (`.map(str::trim)`), `char::is_whitespace`, `Option::Some`, `Vec::new`, `String::from`, `ToString::to_string`, `Option::is_some`, `Vec::len`, `HashSet::default`, `u64::wrapping_div/rem`, `u64::saturating_add`.

---

## 4. Formatting (source, grep-estimated; types from MIR)

Macro calls in non-test code: `format!` 1,168, `format_args!` 1,128 (passed to `emit_fmt(fmt::Arguments)`, e.g. `src/backend/generation.rs:621`), `emit!` 383 (expands to `format_args!`), `write!` 184, `unreachable!` 131 (30 without a message), `eprintln!` 116, `panic!` 21, `println!` 20, `debug_assert!` 12, `assert!` 5, `print!` 3, `debug_assert_eq!` 3, `writeln!` 1, `assert_eq!` 1. Writers: `String` (via `std::fmt::Write`, 17 `write_fmt`) and `Formatter` (169 `write_fmt`, 404 `write_str`). No `io::Write` writer gets `write!`.

**Placeholder arguments:** implicit `{}` 4,104; named or inline-captured `{name}` 285 (both captured identifiers such as `format!("{prefix}{k}")`, `src/backend/arm/codegen/returns.rs:59`, and explicit named args such as `format_args!("    sub {dst}, {src}, t5", dst=dst_reg, src=src_reg)`, `src/backend/riscv/codegen/intrinsics.rs:356`); positional `{0}` 21 (`src/backend/x86/codegen/emit.rs:348`). Every string needs `{{`/`}}` escaping.

**Every distinct spec:**

| spec | count | example |
|---|---|---|
| `{}` | 4,202 | everywhere |
| `{:?}` | 119 | `src/ir/lowering/complex.rs:741` |
| `{:x}` | 63 | `src/backend/i686/linker/reloc.rs:176` |
| `{:.3}` | 10 | timing, `src/driver/pipeline.rs:955` |
| `{:.4}` | 4 | timing, `src/passes/mod.rs:159` |
| `{:016x}` | 2 | `src/passes/profile.rs:44` (profile file name) |
| `{:02x}` | 2 | `src/backend/elf_writer_common.rs:261` |
| `{:#x}` | 2 | `src/backend/arm/codegen/peephole.rs:3352` |
| `{:>4}` `{:>6}` `{:<5}` `{:5}` | 2/1/1/1 | regalloc debug dump, `src/backend/regalloc.rs:534` |
| `{:03o}` | 1 | `src/backend/common.rs:1736` (octal escape in `.ascii` strings) |

Not used: `{:#?}`, `{:+}`, `{:e}`, `{:b}`, `{:X}`, width or precision taken from arguments (`{:1$}`, `{:.*}`), fill characters other than `0` and space. Center alignment is not used.

**Types formatted (MIR `Argument::new_*`):**
- Display: `&str` 1,771, `String` 1,215, `i64` 306, `usize` 280, `u32` 247, `i32` 92, `u8` 45, `u64` 44, `Cow<str>` 39, `io::Error` 37, `f64` 26, `u16` 19, `TokenKind` 13, `char` 12, `CType` 11 + `Box<CType>` 4, `bool` 6, `path::Display` 6, `ParseIntError` 3, `i16`/`i8` 2 each, `Severity` 1.
- LowerHex: `u64` 63, `i64`/`u32` 2 each, `u8`/`usize` 1 each. Octal: `u8` 1.
- Debug: `IrType` 22, `IntrinsicOp` 15, `IrBinOp` 10, `IrCmpOp` 9, assembler `Operand`s and slices of them (~30), `BinOp` 5, `Option<..>`, `Vec<..>`, `[&str]`, `Target`, `PhysReg`, `IrConst`, `Instruction`, `ConstArg`, and **`Discriminant<Instruction>`**.
- **f64 is only formatted with `{:.3}`/`{:.4}` in `CCC_TIME_*` timing output.** No float reaches assembly text through `Display`, so shortest round-trip float printing is **not** needed. `{:.N}` is enough.

**Does `Debug` output reach real output?** Every MIR `Argument::new_debug` site (~110) was inspected:
- Almost all are error strings (`Err(format!(..))`), `panic!`, `unreachable!`, or `eprintln!` behind env-var debug flags.
- `src/passes/inline.rs:1320` builds a **cache key** from `format!("|{:?}", ConstArg)`. This is internal; derived Debug only has to be injective.
- `src/passes/inline.rs:2436-2499` is the IR dumper for `CCC_INLINE_DUMP_IR`, a debug path.
- **`src/passes/profile.rs:53`** hashes `format!("{:?}", std::mem::discriminant(inst))` into a function "shape hash", which is written to profile files on disk (`-fprofile-*` style feature). rustc prints `Discriminant(N)`. rcc should print the same if profile files must interoperate. It does not affect assembly.
- Display impls (`CType`, `TokenKind`) reach **stderr diagnostics**, not assembly.

---

## 5. Language features checklist (grep-estimated unless marked MIR)

| feature | count | example / note |
|---|---|---|
| fn items | 6,108 | |
| generic fns: type/const params / lifetime-only | 34 / 41 | §3.5 |
| const generics | 2 fns | `src/backend/i686/codegen/peephole.rs:1188` |
| generic structs / enums / impls | 15 / 1 / 6 | lifetimes only, except `ElfWriterCore<A: X86Arch>` |
| structs with lifetime params (refs in fields) | 14 | `SemaConstEval<'a>` `src/frontend/sema/const_eval.rs:56` |
| structs / tuple structs / unit structs / enums | 277 / 7 / 2 / 122 | |
| type aliases | 40 | `FxHashMap<K,V>` is a **generic type alias** with a defaulted-param-filling RHS |
| associated types | 3 defs | `type Target` ×2, `type Item` ×1, all on the impl side; plus 1 fn-local `type` alias (`src/passes/cfg_simplify.rs:633`) |
| associated consts in impls | ~100 (grep: 113 indented `const X:` items, some fn-local) | `impl X { const FOO: u32 = ..; }` (e.g. `AccessQuals::NONE`) |
| `const` items | 701 (360 module/assoc-level consts in MIR) | 62 inside fn bodies (`src/lib.rs:15`) |
| const initialisers that call const fns (MIR) | `Cell::new`, `RefCell::new`, `Vec::new`, `AtomicU64::new`, `LazyLock::new(closure)`, `RangeInclusive::new` (promoted) | e.g. `static BUILTIN_MAP: LazyLock<FxHashMap<&'static str, BuiltinInfo>> = LazyLock::new(\|\| {...})` `src/frontend/sema/builtins.rs:43` |
| const exprs in array lengths/consts | `1 << 30`, `N * 2`, const names in `[T; N]` (532 array types, 63 `[x; n]` repeat exprs) | `src/backend/arm/codegen/peephole.rs:2646` |
| `const fn` (defined) | 0 | |
| statics | 7 (`AtomicU64`, `LazyLock`, `Once` (gated)) + 2 `thread_local!` blocks (5 keys) | `thread_local! { static X: Cell<bool> = const { Cell::new(false) }; }` **uses the `const { }` initializer form**, `src/common/types.rs:13`, `src/backend/arm/codegen/peephole.rs:7760` |
| `static mut`, `unsafe`, `extern` | 0, 0, 0 | |
| raw pointers | 17 type mentions | only `x as *const T as usize` for **identity keys** (15 casts, MIR `PointerExposeProvenance`), e.g. `ExprId::of` `src/frontend/parser/ast.rs:731`, plus `std::ptr::eq` 2 |
| trait objects | §3.4 | |
| `impl Trait` arg / return | 60 / 2 | |
| closures | 4,165 (MIR) | §3.6 |
| `?` | 2,461 source; MIR: Result 1,684 + Option 746 | §1 notes; `&str` to `String` via From in 519 |
| `if let` / `while let` | 4,418 / 85 | |
| **`let ... else`** | 2,258 | `let Some(x) = .. else { continue };` `src/passes/loop_unroll.rs:184` |
| let chains (`if let .. && let ..`) | 0 | edition 2021, unavailable |
| `match` | 3,095 | plus 1,458 `matches!` |
| `match` on `&str`/`String` with string-literal patterns | ~7,241 MIR `<str as PartialEq>::eq` calls | e.g. `src/common/const_eval.rs:75` |
| slice patterns `[a, b, ..]`, `[first, rest @ ..]` | ~77 | `src/frontend/preprocessor/macro_defs.rs:1221` |
| `..` rest in tuple/struct/slice patterns | ~226 | |
| `@` bindings | 45 | `src/ir/lowering/stmt_init.rs:649` |
| range patterns `a..=b` (int and char) | 119 (13 char ranges) | `b'0'..=b'9'`, `'a'..='z'` `src/frontend/lexer/scan.rs:856` |
| exclusive range patterns | 0 | |
| negative literal patterns | 1 | `src/backend/arm/codegen/peephole.rs:1498` |
| or-patterns (incl. nested `Some(A \| B)`) | very common | `IrType::I16 \| IrType::U16 => ..` |
| `ref` / `ref mut` patterns | 486 / 11 | `src/frontend/sema/const_eval.rs:162` |
| box patterns | 0 | |
| match ergonomics (default binding modes) | pervasive | `match &x { Foo(a) => }`, `for (k, v) in &map`, `\|&(a, b)\|` closure params |
| labelled loops / breaks | 3 labels (`'again`, `'outer`, `'ft_scan`), 4 labelled break/continue | `src/backend/mod.rs:188` |
| `loop` with `break value` | 10 `let x = loop {}`; 48 `break expr` | `src/passes/loop_unroll.rs:233` |
| `as` casts (numeric) | 5,167 source; MIR below | |
| u128 / i128 | 164 / 113 mentions | §5.1 |
| f32 / f64 | 47 / 172 mentions | §5.2 |
| tuple field access `.0` | 3,538 | also on tuple structs |
| struct update `..base` | 2 | `src/ir/lowering/expr_types.rs:627` |
| `Self` | 961 | |
| turbofish `::<>` | 129 | `collect::<Vec<_>>()`, `parse::<u32>()`, `sum::<usize>()` |
| literals | hex 4,514, binary 1,745, octal 9, suffixed ints 1,049 (`0u64`, `1i128`), float suffix 6 (`2.0_f64`), byte chars `b'x'` common, char 3,189, **no raw strings, no byte strings** | |
| `impl` blocks for one type spread across files | 15 types; `Lowerer` across **38 files**, each `*Codegen` across 16, `Parser` 6, `Preprocessor` 6, `Driver` 4 | |
| items nested in fn bodies | fn 14, struct 5, enum 3, `macro_rules!` 3, const ~60 | `src/ir/analysis.rs:196` (nested fn) |
| `use` inside fn bodies | ~398 | including `use super::external_tools::GccM16Mode;` |
| glob imports `use x::*` | 124 | `super::*` 25, `super::super::types::*` 12, `super::elf::*` 12; no enum-variant globs |
| `pub use` re-exports | 88 | `src/frontend/lexer/mod.rs:4`, `src/ir/reexports.rs` |
| `pub(crate)` / `pub(super)` / `pub(in ..)` | 832 / 1,815 / 0 | |
| `#[macro_export]` macros imported by path | 2 (`emit!`, `delegate_to_impl!`) | `use crate::emit;` in 15+ files |
| attributes with semantics | `#[derive]` 249, `#[default]` 4, `#[inline]` 244 + `#[inline(always)]` 6 (ignorable), `#[allow]` 71 (ignorable), `#[cfg_attr(.., allow(..))]` 15 (ignorable), `#![recursion_limit = "512"]` (`src/lib.rs:1`), `#![allow(dead_code)]` 6 | no `#[repr]`, `#[must_use]`, `#[path]`, `#[macro_use]`, `#[non_exhaustive]` |
| recursion in types | via `Box` (`Box<Expr>` 1,885 decl uses, `Box<CType>`, `Box<Stmt>`, `Box<TypeSpecifier>`, `Box<FunctionType>`, `Box<Initializer>`) and `Vec`, `Rc<StructLayout>` | `src/frontend/parser/ast.rs` |
| shadowed std names | local types `Operand`, `Symbol`, `Target`, `Value`, `Range` (`src/passes/simplify.rs:468`, shadows `std::ops::Range` in that module), `SmallVec`; local fns `default`, `len`, `write`, `format` (`src/frontend/preprocessor/macro_defs.rs:1476`), `min`, `max`, `replace`, `drop` (in `impl Drop`), `eq`, `hash` | no local `Option`/`Result`/`Vec`/`String`/`Box`. **Local enums with a variant named `None`**: `Root::None` (`src/passes/mod_ref.rs:37`, `#[default]`), `Displacement::None` (`src/backend/x86/assembler/parser.rs:221`), `ExtKind::None` (`src/backend/x86/codegen/peephole/types.rs:88`), `Conflict::None` (`src/backend/arm/codegen/peephole.rs:7576`). They are always written qualified and never glob-imported, so a bare `None` is always `Option::None` |
| `env!` / `concat!` / `stringify!` / `cfg!` | 1 / 1 / 1 / 5 | **`env!("CARGO_MANIFEST_DIR")`** at `src/frontend/preprocessor/predefined_macros.rs:252` (rcc must supply the path of the ccc checkout or the equivalent); `concat!` of string literals `src/frontend/preprocessor/includes.rs:813`; `stringify!($name)` inside `pass!` |

### 5.1 Integer semantics (MIR, release)
- Binary ops on primitive types: `usize` dominates (`Lt` 5,632 bounds and loops, `Add` 4,410). `u32` uses many `Shl`/`BitOr` (1,978/1,940, instruction encoders). There are **u128 ops `Shl` 102, `Shr` 52, `BitAnd` 50, `Sub` 39, `Mul` 13, `Add` 11, `Div` 5, `Rem` 4** and **i128 `Sub` 24, `Shr` 20, `Shl` 17, `Div` 6, `Mul` 4, `Neg` 6**.
- **Operators on references to primitives** go through std's `impl Op<&T> for &T`-style trait impls: `Shl` 11, `AddAssign` 9, `Add` 8, `Sub` 7, `Rem` 5, `Mul` 5, `BitAnd` 5 (on `&u8`), `Not` 2 (`!&bool`), `Div` 2, `Neg` 1 (`-&f64`). Examples are `a + *b`-free forms such as `x += &y` and `&a << b`. rcc can auto-deref these instead of modelling the impls.
- Release semantics to reproduce:
  - `+ - *` wrap.
  - **Shifts mask the amount** (`x << n` behaves like `x << (n & (bits-1))`; C is UB).
  - Integer `/` and `%` **panic on zero** (~143 div and 117 rem checks) and on **`MIN / -1`** (59 checks).
  - Overflow checks fire only inside `const` items (57 shift-overflow and 6 sub-overflow asserts in const bodies).
  - **4,245 index bounds checks.**
- `as` casts (MIR, IntToInt 5,098): 96 distinct (from, to) pairs among all 12 integer types, plus `bool` to `u32/i64/i32/usize/u64`, `u8 as char` 88, `char` to `usize/u8/u32/u16/i64/i32`, and **fieldless enum to integer** (`ENUM as i64/u64/i128/char/u128/usize/u32`, ~270; `ENUM as char` 18).
  - IntToFloat 66: `u64/i64/u128/i128/i32/usize/i8/i16` to `f64`, and `u64/u128/i64/i128` to `f32`.
  - **FloatToInt 23: `f64` to `i64/u64/u8/u32/u16/i8/i32/i16/u128/i128`, `f32` to `i64`. These saturate in Rust (NaN goes to 0), and C would be UB here.**
  - FloatToFloat: `f64`/`f32` both ways (30).
- Helpers called (see §1 tables):
  - `wrapping_{add,sub,mul,neg,shl,shr,div,rem}` on u8..u128/i32..i128.
  - `checked_{add,sub,mul,shl,neg}`.
  - `saturating_{add,sub,mul}`.
  - `overflowing_add` (u128).
  - `leading_zeros`, `trailing_zeros`, `trailing_ones`, `count_ones`, `rotate_left`, `swap_bytes`, `is_power_of_two`, `next_power_of_two`, `div_ceil`, `next_multiple_of`, `is_multiple_of` (stabilised in 1.87), `unsigned_abs`, `abs`, `rem_euclid`.
  - `to_le_bytes`/`from_le_bytes` on all widths, `from_be_bytes` (u16), `from_ne_bytes` (u64).
  - `from_str_radix`.
  - `Ord::{max,min,clamp,cmp}` on integers, `std::cmp::{min,max}`.

### 5.2 Float semantics (MIR)
- `f64`: `to_bits` 38, `from_bits`, `powi`, `round`, `min`, `total_cmp`, `is_sign_negative`, `is_finite`, `to_le_bytes`. `f32`: `to_bits` 25, `is_sign_negative`, `to_le_bytes`. Constants `INFINITY`, `NEG_INFINITY`, `NAN`.
- Ops: `f64` Mul/Add/Sub/Div/**Rem** (2)/Neg/compare.
- **Correctly rounded `str::parse::<f64>` and `::<f32>`** are used for C float literals (`src/frontend/lexer/scan.rs:419`) and assembler `.double`/`.float` (`src/backend/arm/assembler/parser.rs:1703`).
- `long double` is soft-float on `u128`/`u64` (`src/common/long_double.rs`, 3,369 lines), so it depends on exact integer semantics, not host floats.

### 5.3 String and char methods (MIR; full list in §1 rows `str`, `String`, `char`, `u8`)
- `str`: `starts_with` 1,208, `trim` 812, `contains`, `strip_prefix`/`strip_suffix`, `split`/`splitn`/`rsplit`/`split_once`/`rsplit_once`/`split_whitespace`/`split_at`/`split_at_checked`, `find`/`rfind`, `trim_*_matches` (the pattern can be `char`, `&str`, `[char; N]`, or a closure), `parse::<u8|u32|u64|usize|i32|i64|f32|f64>`, `to_lowercase`, `to_ascii_lowercase`, `eq_ignore_ascii_case`, `replace`, `replacen`, `repeat`, `lines`, `chars`, `char_indices`, `bytes`, `as_bytes`, `is_char_boundary`, `matches`.
- `String`: `push`, `push_str`, `pop`, `truncate`, `clear`, `with_capacity`, `capacity`, `reserve`, `replace_range`, `into_boxed_str`, `from_utf8`, `from_utf8_lossy`. `std::str::from_utf8` 48.
- `char`: `is_ascii_*`, `is_whitespace`, `is_alphanumeric`, `is_alphabetic`, `len_utf8`, `encode_utf8`, `from_u32`, `to_ascii_lowercase`.
- `u8`: `is_ascii_digit` and related. Non-ASCII Unicode tables are needed only for `char::is_whitespace`, `is_alphanumeric`, `is_alphabetic` and `str::to_lowercase`.

---

## 6. The five `macro_rules!` (source, verbatim-summarised)

| macro | def site | scope | arms / pattern | expansion | uses |
|---|---|---|---|---|---|
| `emit!` | `src/backend/common.rs:1063` | `#[macro_export]`, imported as **`use crate::emit;`** | `($state:expr, $($arg:tt)*)` | `$state.out.emit_fmt(format_args!($($arg)*))`: forwards a tt-repetition into a builtin macro | 383 |
| `delegate_to_impl!` | `src/backend/traits.rs:37` | `#[macro_export]` | 5 arms, recursive tt-muncher. `()` => `{}`. `(fn $name:ident(&mut self $(, $pname:ident : $pty:ty)*) -> $ret:ty => $impl_name:ident; $($rest:tt)*)`, and the variants with `&self` and/or without `-> $ret:ty`. Each emits `fn $name(&mut self $(, $pname : $pty)*) -> $ret { self.$impl_name($($pname),*) }` and then recurses `delegate_to_impl!{ $($rest)* }` | method items | **0 invocations (dead)**. The definition must still parse. |
| `timed_pass!` | `src/passes/mod.rs:442` (**inside a fn body**) | local | `($name:expr, $body:expr)` | `{{ if time_passes { let t0 = Instant::now(); let n = $body; ...eprintln!(.., iter, $name, ..); n } else { $body } }}`. It **refers to locals `time_passes` and `iter` of the enclosing fn** (def-site hygiene for locals defined before the macro), and **`$body` is expanded twice** | 13 |
| `should_run!` | `src/passes/mod.rs:470` (inside a fn) | local | `($self_idx:expr, $($upstream:expr),*)` | `{{ prev_pass_changes[$self_idx] > 0 $(\|\| prev_pass_changes[$upstream] > 0)* }}`: a separator-less repetition with an operator inside, referring to local `prev_pass_changes` | 12 |
| `pass!` | `src/backend/arm/codegen/peephole.rs:614` (inside a fn) | local | `($name:ident($($arg:expr),*))` | `!skip.contains(&stringify!($name)) && $name($($arg),*)`: uses `stringify!` on an ident and refers to local `skip` | 62 |

Required macro_rules features: the fragment specifiers `expr`, `ident`, `ty`, `tt`; `$(...)*` with and without a `,` separator, nested inside parentheses; recursion; invocation with `()`, `{}` and `[]` delimiters; textual scoping for fn-local macros; path-based import of `#[macro_export]` macros (`crate::emit`); and mixed-site hygiene. Locals referenced by the macro body resolve at the definition site, which in all three cases is also visible at the call site, so plain unhygienic expansion gives the same result. Not used: `$crate`, `literal`/`pat`/`block`/`path`/`lifetime`/`vis`/`item` fragments, `?` repetition, and macros defining macros.

Builtin macros required: `format!`, `format_args!`, `write!`, `writeln!`, `print!`, `println!`, `eprintln!`, `panic!`, `unreachable!` (with and without a message), `assert!`, `assert_eq!`, `debug_assert!`, `debug_assert_eq!`, `vec!` (both forms), `matches!` (with `|` patterns and `if` guards), `thread_local!` (`const {}` form), `cfg!`, `env!`, `concat!`, `stringify!`.

---

## 7. Runtime behaviour dependencies

### 7.1 Hash iteration order: **output DOES depend on it** (measured)

| experiment | result |
|---|---|
| Same binary run twice, native arm64-darwin `-O2 -S` and `-c` (226 cases; clang assembles) | identical |
| Fx `SEED` changed (`src/common/fx_hash.rs`), native `-O2 -S` | **52 / 226 `.s` files differ** (e.g. register choice `add w14` versus `add w15` in `call_loop_sum.s`) |
| Same binary run twice, builtin ELF assemblers (`ccc-x86/-riscv/-i686 -c`) | **617 / 646 `.o` files differ between two runs of the same binary.** `.symtab` order of undefined symbols changes because of SipHash `RandomState` maps; the code bytes are identical |

A subagent classified all 212 HashMap/HashSet iteration sites
(`tools/inventory/hash_order_review.tsv`; the counts below are its).
143 are order-free, 42 sorted, 19 leak order into output, and 8 are unsure.

- **Fx order reaching codegen** (deterministic for one hashbrown implementation, but tied to it):
  - `src/passes/loop_analysis.rs:74`: loop list order feeds iv_widen, lftr, loop_vectorize, step_to_load, bound_hoist and iv_strength_reduce. licm and gep_reassoc sort by length only.
  - `src/ir/mem2reg/promote.rs:519`: phi insertion order and value numbering.
  - `src/passes/iv_strength_reduce.rs:523`, `:571`.
  - `src/passes/iv_widen.rs:325`: ties after a sort.
  - `src/backend/regalloc.rs:293`: partner order affects register choice.
- **SipHash order reaching output** (nondeterministic in real ccc today):
  - `src/backend/elf/symbol_table.rs:68`, `:99`, `:160`: symbol table of every builtin-assembled `.o`.
  - `src/backend/linker_common/merge.rs:129`: COMMON symbol layout.
  - `src/backend/{arm/linker/emit_dynamic.rs:46, x86/linker/emit_exec.rs:52}`: copy relocations.
  - `*/linker/emit_shared.rs:{42,168,148}`: `.dynsym` order.
  - `src/backend/riscv/linker/symbols.rs:150`: PLT order.
  - `src/backend/i686/linker/emit.rs:40`: DT_NEEDED order.
  - `src/backend/i686/linker/symbols.rs:282`: unsorted "undefined symbols" error text.

Consequences for rcc:
1. A ccc built by rcc **will not produce byte-identical assembly** to a rustc-built ccc unless rcc's `HashMap` reproduces hashbrown's exact probe and iteration order, including the SIMD group width. That width is 16 with SSE2 on x86-64 and 8 with NEON or generic code on aarch64, so the rustc-built ccc very likely already differs between those two hosts (inferred, not measured). Acceptance tests must compare **program behaviour** (difftest runs), not `.s`/`.o` bytes.
2. rcc may make `RandomState` deterministic, for example with a fixed seed. That is allowed, and it makes ccc's ELF output reproducible.
3. Correctness must not depend on order, and the review found no site where it does.

### 7.2 Threads, stack, panics, exit codes
- One thread: `std::thread::Builder::new().stack_size(1 << 30).spawn(closure)`, then `.join()` (`src/lib.rs:15-29`). Everything runs on that 1 GB stack. The parser caps nesting at `MAX_NESTING = 6000` (`src/frontend/parser/parse.rs:276`). Sema and lowering recurse over `Box<Expr>` trees just as deeply. rcc's C will have larger frames than rustc's, so keep the big thread (pthread with a 1 GB stack) or run on a main stack raised with `setrlimit`.
- Panics: ccc does **not** abort. `join()` returns `Err(Box<dyn Any + Send>)`, and ccc `downcast_ref::<&str>()` / `::<String>()`s it and prints `ccc: internal error: <msg>` and **exits 1** (`src/lib.rs:36-45`). rcc's "panics abort with 101" differs: the exit code is 101 rather than 1, and the message format differs. Either emulate it (print `ccc: internal error: ` + msg, exit 1) or accept the difference. No `catch_unwind`, panic hooks or `Mutex` poisoning are used.
- Exit codes: `std::process::exit(1)` on error (2 sites, `src/lib.rs:34/44`). Success returns from `main`, giving exit code 0.
- `std::sync::atomic::AtomicU64` is used for the temp-file counter (`fetch_add`, Relaxed). `LazyLock` (the builtins table) and `thread_local!` `Cell`/`RefCell` are accessed only from the worker thread.

### 7.3 Environment variables read
- `SDKROOT`: macOS SDK include dir (`src/backend/arm/darwin.rs:424`).
- Debug and tuning variables: `CCC_DISABLE_PASSES` (16 sites, comma list), `CCC_TIME_PHASES`, `CCC_TIME_PASSES`, `CCC_DUMP_IR`, `CCC_DUMP_REGALLOC`, `CCC_DEBUG_MEM2REG`, `CCC_ASM_DEBUG` (writes `/tmp/asm_debug_*.s`), `CCC_KEEP_ASM`, `CCC_INLINE_{DEBUG,DEBUG_DETAIL,DUMP_IR,DUMP_SPECIALIZED,SKIP,VALIDATE}`, `CCC_PEEPHOLE_SKIP`, `CCC_TMPVEC`, `LINKER_DEBUG`, `LINKER_DEBUG_LAYOUT`, `LINKER_DEBUG_TLS`.
- All are read with `std::env::var(..)` and tested via `is_ok()`, `is_ok_and(..)`, or the value.
- Also `std::env::args()`, `current_dir()`, `current_exe()` plus `canonicalize()` (to find `include/` next to the binary, `src/frontend/preprocessor/predefined_macros.rs:234`), and `temp_dir()` (`TMPDIR`).
- Compile time: `env!("CARGO_MANIFEST_DIR")`.

### 7.4 External processes (`std::process::Command`, only `new/arg/args/output`, `ExitStatus::success`)
On **macOS (arm64-darwin target)**, all in `src/backend/arm/darwin.rs`:
- `xcrun --show-sdk-path` (when `SDKROOT` is unset), `:426`.
- `clang -print-resource-dir`, `:435`.
- Assemble: `clang -arch arm64 -c -o <out> <tmp.s> [extra]`, `:469`.
- Link: `clang -arch arm64 -o <out> <objs...> <user args>` (`-static` is dropped, `-shared` becomes `-dynamiclib`), `:477`.

On **Linux (all ELF targets)**, nothing is spawned in a default build: assembler and linker are built in. The `gcc`, `i686-linux-gnu-gcc`, ... commands in `src/backend/mod.rs:247-293` sit only behind `gcc_*` features.

### 7.5 Filesystem
- Reads: `fs::read`, `read_to_string`, `File::open` + `Read::read` (magic sniffing, `src/driver/file_types.rs:62`), `read_dir` (library search), `canonicalize`, `Path::{exists,is_file,is_dir}`, stdin `read_to_end` (input `-`).
- Writes: `fs::write` (outputs, `.d` dep files, profile data), `create_dir_all`, `remove_file`, `set_permissions(from_mode(0o755))` (`cfg(unix)`), stdout via `stdout().lock()` + `write_all` + `flush` (for `-E`/`-S -o -`).
- Temp files (`src/common/temp_files.rs`): `temp_dir()/{prefix}_{pid}_{stem}.{counter}.{ext}` (needs `process::id()`). They are **deleted by `impl Drop for TempFile`** unless kept.

### 7.6 Timing, terminal
- `Instant::now()`/`elapsed().as_secs_f64()` only feeds `CCC_TIME_*` stderr output.
- `std::io::stderr().is_terminal()` selects coloured diagnostics in `ColorMode::Auto` (`src/common/error.rs:76`).

---

## 8. Top risks for a no-borrowck Rust-to-C compiler with a hand-written std

1. **Method resolution with autoderef and autoref across many receiver types.**
   - 8,083 `Deref` and 1,121 `DerefMut` calls (MIR).
   - Methods are found through `Vec` to `[T]`, `String` to `str`, `Box`, `Rc` to `T`, `Ref`/`RefMut`, `Cow` to `str`, `PathBuf` to `Path`, `OsString` to `OsStr`, and ccc's own `Deref for LocalInfo` (`src/ir/lowering/definitions.rs:102`).
   - Trait methods come into scope only when the trait is imported: `fmt::Write`, `io::{Read,Write,IsTerminal}`, `PermissionsExt`, `OperandExt`.
   - `impl Lowerer` is spread over 38 files.
   - Inherent methods on primitives and slices make up the bulk of §1.
2. **Type inference driven by the expected type.**
   - `collect()` into Vec, HashMap, HashSet, String, `Option<Vec<_>>` or VecDeque (763).
   - `parse()` and `.into()` resolved from a `let` annotation (`let value: f64 = text.parse().unwrap_or(0.0)`, `src/frontend/lexer/scan.rs:419`).
   - `Default::default()`, integer-literal defaulting, closure parameter types taken from the callee's `Fn` bound.
   - `?` inserting `From<&str> for String` (519 sites).
3. **Closures: 4,165 bodies.**
   - Fn/FnMut/FnOnce kind is inferred from the expected bound.
   - Edition-2021 disjoint-field capture decides what is borrowed or moved.
   - Coercion to `&dyn Fn`/`&mut dyn FnMut` and to fn pointers.
   - `impl FnOnce(&mut Self) -> T` with re-entrant `&mut self` (`src/frontend/parser/parse.rs:402`).
   - Locals are captured by mutable reference while the enclosing fn mutates other fields.
   - Without a borrow checker, getting by-ref versus by-move capture wrong silently changes semantics, for example when a moved `String` is used later.
4. **Patterns and match ergonomics.**
   - 3,095 `match`, 4,418 `if let`, 2,258 `let-else`, 1,458 `matches!`, and default binding modes everywhere (`for (k, v) in &map`, `|&(a, b)|`).
   - 486 `ref`, slice patterns with `rest @ ..`, 45 `@` bindings, 119 int/char/byte range patterns, nested or-patterns.
   - About 7,000 `&str` literal comparisons in matches.
   - Exhaustiveness can be trusted (known-good code), but binding modes must be exactly right.
5. **Trait objects with big vtables and supertrait/default-method dispatch.**
   - `&mut dyn ArchCodegen` (191 methods, 70 defaults that call other trait methods through `self`) in `src/backend/traits.rs:104` and `src/backend/generation.rs:489`.
   - `impl StructLayoutProvider for FxHashMap<String, Rc<StructLayout>>` coerced to `&dyn` (`src/common/types.rs:135`).
   - `X86Arch`'s self-less associated fns through `A: X86Arch`, and `T: F128SoftFloat + ?Sized`.
6. **Hash-order coupling** (§7.1). rcc's `HashMap` will reorder Fx iteration. Output is still valid, but bytes differ (52/226 `.s` changed under a reseed). Bugs that only show under some orders (ties in `src/passes/loop_analysis.rs:74`, `src/backend/regalloc.rs:293`) become reachable, so failures may be ccc's own latent order-dependent bugs, not rcc bugs. Use difftest behaviour as the oracle, and consider a mode in which rcc's HashMap replays insertion order.
7. **Exact numeric semantics in C.**
   - Masked shift amounts (2,383 `Shl`, 417 `Shr`).
   - Wrapping arithmetic on all 12 integer widths, plus u128/i128 `Div`/`Rem`/`Mul` (C needs `__int128`, and ccc itself must compile those for the bootstrap).
   - **Saturating float-to-int `as` casts** (23 sites, UB in C).
   - Panics on `/0` and `MIN/-1`, and 4,245 bounds checks.
   - Correctly rounded `str::parse::<f64/f32>` (C float literals, `src/frontend/lexer/scan.rs:419`).
   - `powi`, `round`, `total_cmp`, `to_bits`/`from_bits`.
8. **Drop and interior mutability that must behave correctly before full drop elaboration.**
   - `RefCell::borrow_mut` (39) and `borrow` (14) guards inside expressions (`src/frontend/sema/analysis.rs:160`, `src/ir/lowering/lower.rs:232-234`). A guard that is leaked rather than dropped makes the next `borrow_mut` panic ("already borrowed"), so temporary-scope drop timing must be right from day one.
   - `thread_local!` `LocalKey::with` on a `RefCell` (`src/backend/arm/codegen/peephole.rs:7760`).
   - `Rc::make_mut` depends on accurate strong counts (`src/ir/lowering/structs.rs:163`).
   - `impl Drop for TempFile` deletes files (`src/common/temp_files.rs:81`).
   - `mem::take`/`replace`/`swap` (58).
9. **Pointer identity as map keys.** `ExprId(expr as *const Expr as usize)` (`src/frontend/parser/ast.rs:731`) and `inst as *const Instruction as usize` (`src/backend/arm/codegen/prologue.rs:121`, `src/backend/arm/codegen/calls.rs:20`, `src/backend/arm/codegen/intrinsics.rs:316`), plus `std::ptr::eq` (`src/ir/lowering/global_init.rs:135`). rcc must never copy a value that Rust would access in place: `&expr` must point into the `Box`/`Vec` storage, not at a temporary copy. Otherwise lookups silently miss and codegen changes.
10. **Name resolution and expansion details at scale.**
    - 124 glob imports (`use super::*` alongside `use super::super::types::*`, `use super::elf::*`) and 88 `pub use` re-exports.
    - `#[macro_export]` macros imported by path (`use crate::emit;`).
    - Fn-local `macro_rules!` that use enclosing locals and expand `$body` twice (`src/passes/mod.rs:442`).
    - `#[cfg]` on tail-position block expressions (`src/driver/external_tools.rs:128-136`).
    - `const { }` thread-local initialisers, `env!("CARGO_MANIFEST_DIR")`, the generic type alias `FxHashMap<K,V>` expanding to a three-parameter `HashMap`, local shadowing of `Range`, and 398 fn-local `use` items.

Honourable mentions: the 1 GB thread stack and the panic-to-exit-1 path (§7.2); `Hash` and `Eq` for 25 key types including 5-tuples; `Formatter` and `fmt::Arguments` as first-class values (`emit_fmt(format_args!(..))`, 2,860 `Arguments::new_v1`); `[T; N]` by-value `IntoIterator`; const-generic `[usize; N]` parameters; and `Option<Vec<_>>: FromIterator<Option<_>>`.
