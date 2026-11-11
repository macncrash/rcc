/*
 * rcc_rt.h - prelude of every C file emitted by rcc.
 *
 * Deliberately includes no system headers: only LP64 fixed-width typedefs,
 * the fat-pointer / unit types, checked-arithmetic helpers, and the
 * prototypes of the runtime (rt.c). Everything here must be accepted by
 * both clang and ccc.
 */
#ifndef RCC_RT_H
#define RCC_RT_H

typedef _Bool bool;
typedef unsigned char u8;
typedef signed char i8;
typedef unsigned short u16;
typedef short i16;
typedef unsigned int u32;
typedef int i32;
typedef unsigned long long u64;
typedef long long i64;
typedef unsigned long usize;
typedef long isize;
typedef unsigned __int128 u128;
typedef __int128 i128;
typedef float f32;
typedef double f64;

_Static_assert(sizeof(usize) == 8 && sizeof(void *) == 8, "rcc targets LP64");
_Static_assert(sizeof(u32) == 4 && sizeof(u64) == 8 && sizeof(u16) == 2, "integer sizes");

/* The value of (), !, fn items and other zero-sized types (size 1 in C). */
typedef struct Unit {
    u8 _e;
} Unit;
static const Unit rcc_unit;

/* &dyn Trait / Box<dyn Trait>: data pointer + vtable.
   vtable layout: [0] drop glue (or 0), [1] size, [2] align, [3..] methods. */
typedef struct Dyn {
    void *ptr;
    const void *const *vt;
} Dyn;

#define RCC_NORETURN _Noreturn

/* See cgen.c (emit_function): works around a ccc code generation bug. The
   driver defines RCC_CC_IS_CCC when $RCC_CC is ccc. */
#ifdef RCC_CC_IS_CCC
#define RCC_ADDR volatile
#else
#define RCC_ADDR
#endif

void *memcpy(void *dst, const void *src, unsigned long n);
void *memmove(void *dst, const void *src, unsigned long n);
void *memset(void *dst, int c, unsigned long n);
int memcmp(const void *a, const void *b, unsigned long n);
double fmod(double a, double b);
float fmodf(float a, float b);

/* ---- runtime (rt.c) ---- */
void rcc_rt_init(int argc, char **argv);
RCC_NORETURN void rcc_panic_bounds(usize index, usize len);
RCC_NORETURN void rcc_panic_cstr(const char *msg);
RCC_NORETURN void rcc_unreachable(void);
RCC_NORETURN void rcc_abort(void);
void rcc_box_free(void *p);

/* ---- integer division (panics like Rust) ---- */
#define RCC_DEF_UDIV(T)                                                        \
    static inline T rcc_div_##T(T a, T b) {                                    \
        if (b == 0)                                                            \
            rcc_panic_cstr("attempt to divide by zero");                       \
        return a / b;                                                          \
    }                                                                          \
    static inline T rcc_rem_##T(T a, T b) {                                    \
        if (b == 0)                                                            \
            rcc_panic_cstr(                                                    \
                "attempt to calculate the remainder with a divisor of zero"); \
        return a % b;                                                          \
    }
#define RCC_DEF_SDIV(T, U, MIN)                                                \
    static inline T rcc_div_##T(T a, T b) {                                    \
        if (b == 0)                                                            \
            rcc_panic_cstr("attempt to divide by zero");                       \
        if (b == -1 && a == (MIN))                                             \
            rcc_panic_cstr("attempt to divide with overflow");                 \
        return a / b;                                                          \
    }                                                                          \
    static inline T rcc_rem_##T(T a, T b) {                                    \
        if (b == 0)                                                            \
            rcc_panic_cstr(                                                    \
                "attempt to calculate the remainder with a divisor of zero"); \
        if (b == -1 && a == (MIN))                                             \
            rcc_panic_cstr(                                                    \
                "attempt to calculate the remainder with overflow");          \
        return a % b;                                                          \
    }
RCC_DEF_UDIV(u8) RCC_DEF_UDIV(u16) RCC_DEF_UDIV(u32) RCC_DEF_UDIV(u64)
RCC_DEF_UDIV(u128) RCC_DEF_UDIV(usize)
RCC_DEF_SDIV(i8, u8, (i8)(-128))
RCC_DEF_SDIV(i16, u16, (i16)(-32768))
RCC_DEF_SDIV(i32, u32, (i32)(-2147483647 - 1))
RCC_DEF_SDIV(i64, u64, (i64)(-9223372036854775807ll - 1))
RCC_DEF_SDIV(isize, usize, (isize)(-9223372036854775807l - 1))
RCC_DEF_SDIV(i128, u128, (i128)((u128)1 << 127))

/* ---- float -> int `as` casts: saturating, NaN -> 0 ---- */
#define RCC_DEF_F2I(F, T, LO, HI)                                              \
    static inline T rcc_f2i_##F##_##T(F x) {                                   \
        if (x != x)                                                            \
            return 0;                                                          \
        if (x <= (F)(LO))                                                      \
            return (LO);                                                       \
        if (x >= (F)(HI))                                                      \
            return (HI);                                                       \
        return (T)x;                                                           \
    }
#define RCC_DEF_F2I_ALL(F)                                                     \
    RCC_DEF_F2I(F, u8, (u8)0, (u8)255)                                         \
    RCC_DEF_F2I(F, u16, (u16)0, (u16)65535)                                    \
    RCC_DEF_F2I(F, u32, (u32)0, (u32)4294967295u)                              \
    RCC_DEF_F2I(F, u64, (u64)0, (u64)18446744073709551615ull)                  \
    RCC_DEF_F2I(F, usize, (usize)0, (usize)18446744073709551615ull)            \
    RCC_DEF_F2I(F, u128, (u128)0, ~(u128)0)                                    \
    RCC_DEF_F2I(F, i8, (i8)(-128), (i8)127)                                    \
    RCC_DEF_F2I(F, i16, (i16)(-32768), (i16)32767)                             \
    RCC_DEF_F2I(F, i32, (i32)(-2147483647 - 1), (i32)2147483647)               \
    RCC_DEF_F2I(F, i64, (i64)(-9223372036854775807ll - 1),                     \
                (i64)9223372036854775807ll)                                    \
    RCC_DEF_F2I(F, isize, (isize)(-9223372036854775807l - 1),                  \
                (isize)9223372036854775807l)                                   \
    RCC_DEF_F2I(F, i128, (i128)((u128)1 << 127), (i128)(~(u128)0 >> 1))
RCC_DEF_F2I_ALL(f32)
RCC_DEF_F2I_ALL(f64)

/* ---- overflow-reporting arithmetic: return 1 on overflow ---- */
#define RCC_DEF_UOVF(T, W)                                                     \
    static inline bool rcc_add_ovf_##T(T a, T b, T *r) {                       \
        *r = (T)((W)a + (W)b);                                                 \
        return *r < a;                                                         \
    }                                                                          \
    static inline bool rcc_sub_ovf_##T(T a, T b, T *r) {                       \
        *r = (T)((W)a - (W)b);                                                 \
        return a < b;                                                          \
    }                                                                          \
    static inline bool rcc_mul_ovf_##T(T a, T b, T *r) {                       \
        *r = (T)((W)a * (W)b);                                                 \
        return a != 0 && (T)(*r / a) != b;                                     \
    }
#define RCC_DEF_SOVF(T, U, W)                                                  \
    static inline bool rcc_add_ovf_##T(T a, T b, T *r) {                       \
        *r = (T)(U)((W)(U)a + (W)(U)b);                                        \
        return (b >= 0) ? (*r < a) : (*r > a);                                 \
    }                                                                          \
    static inline bool rcc_sub_ovf_##T(T a, T b, T *r) {                       \
        *r = (T)(U)((W)(U)a - (W)(U)b);                                        \
        return (b >= 0) ? (*r > a) : (*r < a);                                 \
    }                                                                          \
    static inline bool rcc_mul_ovf_##T(T a, T b, T *r) {                       \
        *r = (T)(U)((W)(U)a * (W)(U)b);                                        \
        if (a == 0 || b == 0)                                                  \
            return 0;                                                          \
        if ((a == -1 && b == (T)((U)1 << (sizeof(T) * 8 - 1))) ||              \
            (b == -1 && a == (T)((U)1 << (sizeof(T) * 8 - 1))))                \
            return 1;                                                          \
        return (T)(*r / b) != a;                                               \
    }
RCC_DEF_UOVF(u8, u32) RCC_DEF_UOVF(u16, u32) RCC_DEF_UOVF(u32, u32)
RCC_DEF_UOVF(u64, u64) RCC_DEF_UOVF(usize, usize) RCC_DEF_UOVF(u128, u128)
RCC_DEF_SOVF(i8, u8, u32) RCC_DEF_SOVF(i16, u16, u32) RCC_DEF_SOVF(i32, u32, u32)
RCC_DEF_SOVF(i64, u64, u64) RCC_DEF_SOVF(isize, usize, usize)
RCC_DEF_SOVF(i128, u128, u128)

/* ---- bit counting (portable) ---- */
static inline u32 rcc_ctpop64(u64 x) {
    u32 n = 0;
    while (x) {
        x &= x - 1;
        n++;
    }
    return n;
}
static inline u32 rcc_ctlz64(u64 x) {
    u32 n = 0;
    if (x == 0)
        return 64;
    while (!(x >> 63)) {
        x <<= 1;
        n++;
    }
    return n;
}
static inline u32 rcc_cttz64(u64 x) {
    u32 n = 0;
    if (x == 0)
        return 64;
    while (!(x & 1)) {
        x >>= 1;
        n++;
    }
    return n;
}
static inline u32 rcc_ctpop128(u128 x) {
    return rcc_ctpop64((u64)x) + rcc_ctpop64((u64)(x >> 64));
}
static inline u32 rcc_ctlz128(u128 x) {
    u64 hi = (u64)(x >> 64);
    return hi ? rcc_ctlz64(hi) : 64 + rcc_ctlz64((u64)x);
}
static inline u32 rcc_cttz128(u128 x) {
    u64 lo = (u64)x;
    return lo ? rcc_cttz64(lo) : 64 + rcc_cttz64((u64)(x >> 64));
}
static inline u128 rcc_bswap(u128 x, u32 bytes) {
    u128 r = 0;
    u32 i;
    for (i = 0; i < bytes; i++) {
        r = (r << 8) | (x & 0xff);
        x >>= 8;
    }
    return r;
}

#endif
