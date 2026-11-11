/* ccc (arm64, every optimization level including -O0): when the bool-returning
 * eq() below is inlined into q(), the `cset` of its comparison is dropped and
 * the caller tests a stale x0. Compile-only repro, reduced automatically from
 * rcc output (tests/core/const_pat_aggregate.rs; t12_linear_scan failed the
 * same way):
 *
 *     ccc -O0 -S bool_fn_inlined_cset_dropped.c -o - | grep -B3 -A2 'tst w0'
 *
 * shows, in _const_pat_aggregate__q:
 *
 *     ldrb w9, [sp, #0]
 *     ldrb w0, [x0]
 *     cmp w9, w0
 *     tst w0, #0xff        <- tests the loaded byte, not the comparison
 *     b.eq ...
 *
 * See also bool_field_compare.c (a runnable member of the same family).
 * Workaround in rcc: the driver skips the pass (src/driver.c).
 */
typedef _Bool bool;
typedef unsigned char u8;
typedef unsigned long usize;
typedef struct Unit {
} Unit;
typedef struct Dyn {
} Dyn;
typedef struct Slice_u8_203 Slice_u8_203;
typedef struct Quals_536 Quals_536;
typedef Unit (*Fn_539)(void);
struct Slice_u8_203 { u8 *ptr; usize len; };
struct Argument_545 {
    union {
        struct {
        } v10;
    } u;
};
struct Piece_542 {
    union {
        struct {
        } v1;
    } u;
};
struct Quals_536 {
    u8 f0;
};
struct Kind_538 {
    union {
        struct {
        } v1;
    } u;
};
struct Tup_Reg_Quals__744 {
    union {
        struct {
        } v1;
    } u;
};
struct LocalKey_RefCell_Option_Thread__3429 {
};
struct AtomicU64_445 {
};
static const void *const vtable_0[] = {
};
static Unit const_pat_aggregate__main(void) {
    Slice_u8_203 _10;
    Quals_536 _11;
    _10 = const_pat_aggregate__q(_11);
}
static Slice_u8_203 const_pat_aggregate__q(Quals_536 _1) {
    Slice_u8_203 _0;
    Quals_536* _2;
    Quals_536* volatile _3;
    bool _5;
    Quals_536* _6;
    Quals_536* volatile _7;
    bool _9;
    _5 = const_pat_aggregate__PartialEq_for_Quals__eq(_2, _3);
    if (_5) goto bb3;
    goto bb2;
bb3:;
    _0 = ((Slice_u8_203){(u8 *)"none", 4});
    goto bb1;
bb2:;
    _9 = const_pat_aggregate__PartialEq_for_Quals__eq(_6, _7);
    if (_9) goto bb5;
    goto bb4;
bb5:;
    goto bb1;
bb4:;
    _0 = ((Slice_u8_203){(u8 *)"other", 5});
bb1:;
    return _0;
}
static bool const_pat_aggregate__PartialEq_for_Quals__eq(Quals_536* _1, Quals_536* _2) {
    bool _0;
    u8 _3;
    _3 = _1->f0;
    _0 = (_3 == _2->f0);
    return _0;
}
int main(int argc, char **argv) {
    return (int)std__rt__lang_start_1((Fn_539)const_pat_aggregate__main);
}
/* Compiled correctly with CCC_PEEPHOLE_SKIP=drop_mask_of_narrow_value (ccc's
   arm64 peephole pass of that name). */
