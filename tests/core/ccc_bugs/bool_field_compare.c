/* ccc (arm64, -O0/-O1/-O2) miscompiles fx_eq(): comparing two `_Bool` fields
 * loaded from memory. The load of the first field is normalized with
 * `cmp w12, #0` but the `cset` is dropped, and the second field is then
 * compared with the stale w0 (the pointer argument):
 *
 *     ldrb w12, [x0]
 *     cmp w12, #0
 *     ldrb w13, [x1]
 *     cmp w13, w0, uxtb
 *
 * Expected (clang, gcc):  eq(a,b)=1 ne(a,b)=0 eq(a,c)=0 ne(a,c)=1
 * ccc:                    eq(a,b)=0 ne(a,b)=1 eq(a,c)=0 ne(a,c)=1
 *
 * Found because a ccc-compiled ccc (built by rcc) generated different code
 * than the clang-compiled one: the derived PartialEq of
 * passes::mod_ref::FnEffects never returned true, so ModRef::compute never
 * reached its fixed point. With `typedef unsigned char bool;` the code is
 * correct too, but that only hides this instance.
 */
#include <stdio.h>
typedef _Bool bool;
typedef unsigned long long u64;
typedef struct { bool f0; u64 f1; u64 f2; bool f3; } Fx;

__attribute__((noinline)) static inline bool fx_eq(Fx *_1, Fx *_2) {
    bool _0;
    bool _3;
    bool _4;
    bool _5;
    bool _6;
    u64 _7;
    u64 _8;
    bool _9;
bb0:;
    _6 = _1->f0;
    _5 = (_6 == _2->f0);
    if (_5) goto bb7;
    goto bb8;
bb7:;
    _7 = _1->f1;
    _4 = (_7 == _2->f1);
    goto bb9;
bb8:;
    _4 = 0;
bb9:;
    if (_4) goto bb4;
    goto bb5;
bb4:;
    _8 = _1->f2;
    _3 = (_8 == _2->f2);
    goto bb6;
bb5:;
    _3 = 0;
bb6:;
    if (_3) goto bb1;
    goto bb2;
bb1:;
    _9 = _1->f3;
    _0 = (_9 == _2->f3);
    goto bb3;
bb2:;
    _0 = 0;
bb3:;
    return _0;
}

__attribute__((noinline)) static inline bool fx_ne(Fx *_1, Fx *_2) {
    bool _0;
    bool _3;
bb0:;
    _3 = fx_eq(_1, _2);
    _0 = (!_3);
    return _0;
}

int main(void) {
    Fx a = {0, 0, 0, 0}, b = {0, 0, 0, 0}, c = {1, 0, 1, 0};
    printf("eq(a,b)=%d ne(a,b)=%d eq(a,c)=%d ne(a,c)=%d   (expect 1 0 0 1)\n",
           fx_eq(&a, &b), fx_ne(&a, &b), fx_eq(&a, &c), fx_ne(&a, &c));
    return 0;
}
/* Compiled correctly with CCC_PEEPHOLE_SKIP=drop_mask_of_narrow_value (ccc's
   arm64 peephole pass of that name). */
