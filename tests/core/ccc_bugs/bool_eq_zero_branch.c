#include <stdio.h>
typedef _Bool bool; typedef unsigned char u8; typedef unsigned int u32; typedef int i32; typedef unsigned long usize;
typedef struct Unit { u8 _e; } Unit;
typedef struct { u8 *ptr; usize len; } Slice_u8_202;
typedef struct { u8 tag; union { struct { Slice_u8_202 f0; } v0; struct { void *a; void *b; long c[6]; } v1; } u; } Piece_281;
typedef struct { Piece_281 a[1]; } Arr_Piece_1__361;
typedef struct { Piece_281 *ptr; usize len; } Slice_Piece_282;
typedef struct { Slice_Piece_282 f0; } Arguments_263;
static Unit std__io___print(Arguments_263 a) { Unit u; fwrite(a.f0.ptr[0].u.v0.f0.ptr, 1, a.f0.ptr[0].u.v0.f0.len, stdout); u._e = 0; return u; }
Unit pb(u32 _1) {
    Unit _0;
    i32 _2; /* bit */
    u32 _6; /* b */
    u32 _7;
    Unit _8;
    bool _9;
    bool _10;
    Unit _11;
    Unit _12;
    bool _14;
    bool _15;
    Arguments_263 _16;
    Slice_Piece_282 _17;
    Arr_Piece_1__361* _18;
    Arr_Piece_1__361 _19;
    Piece_281 _20;
    bool _27;
    bool _28;
    _2 = ((i32)31ll);
bb1:;
    _7 = ((u32)(_1 >> ((u32)_2 & 31u)));
    _6 = ((u32)(_7 & ((u32)1ull)));
    _9 = (_6 == ((u32)1ull));
    _10 = _9;
    if (_10 == 0) goto bb4;
    goto bb3;
bb2:;
    goto bb17;
bb3:;
    _11 = ((Unit){0});
bb4:;
    _8 = ((Unit){0});
    _14 = (_6 == ((u32)1ull));
    _15 = _14;
    if (_15 == 0) goto bb10;
    goto bb9;
bb8:;
    _27 = (_2 == ((i32)0ll));
    _28 = _27;
    if (_28 == 0) goto bb13;
    goto bb12;
bb9:;
    _20 = ((Piece_281){.tag = ((u8)0ull), .u = {.v0 = {.f0 = ((Slice_u8_202){(u8 *)"1", 1})}}});
    _19 = ((Arr_Piece_1__361){{_20}});
    _18 = (&_19);
    _17 = ((Slice_Piece_282){(Piece_281 *)_18, 1ull});
    _16 = ((Arguments_263){.f0 = _17});
    _12 = std__io___print(_16);
bb10:;
    goto bb8;
bb12:;
    goto bb2;
bb13:;
    _2 = ((i32)(u32)((u32)_2 - (u32)((i32)1ll)));
    goto bb1;
bb17:;
    _0 = ((Unit){0});
    return _0;
}
int main(void) { pb(10); pb(255); return 0; }
/* Compiled correctly with CCC_PEEPHOLE_SKIP=drop_mask_of_narrow_value (ccc's
   arm64 peephole pass of that name). */
