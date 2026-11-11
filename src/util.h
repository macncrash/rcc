/*
 * util.h - shared infrastructure for rcc.
 *
 *   - Arena allocation (never freed; batch compiler).
 *   - Growable arrays (VEC macros), backed by the arena.
 *   - String interning (Sym).
 *   - Source file table and Span.
 *   - fatal(span, fmt, ...).
 */
#ifndef RCC_UTIL_H
#define RCC_UTIL_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/* ------------------------------------------------------------------ */
/* Arena                                                               */
/* ------------------------------------------------------------------ */

/* Returns zero-filled memory aligned to 16 bytes. Never fails (fatal on OOM). */
void *arena_alloc(size_t size);
/* Copy `n` bytes of `s` into the arena and NUL-terminate. */
char *arena_strndup(const char *s, size_t n);
char *arena_strdup(const char *s);
/* printf into a fresh arena string. */
char *arena_printf(const char *fmt, ...);

/* Allocate one zeroed T. */
#define NEW(T) ((T *)arena_alloc(sizeof(T)))
/* Allocate n zeroed T. */
#define NEW_ARRAY(T, n) ((T *)arena_alloc(sizeof(T) * (size_t)(n)))

/* ------------------------------------------------------------------ */
/* Growable arrays                                                     */
/* ------------------------------------------------------------------ */

/*
 * VEC(T) is an anonymous struct type; because two anonymous structs are
 * never compatible in C, always typedef a named list type once:
 *     typedef VEC(Expr *) ExprList;
 * A zero-initialised VEC is an empty list. Storage lives in the arena.
 */
#define VEC(T)                                                                 \
    struct {                                                                   \
        T *data;                                                               \
        uint32_t len, cap;                                                     \
    }

/* Internal: returns a new buffer of at least double capacity. */
void *vec_grow_(void *data, uint32_t len, uint32_t *cap, size_t elem_size);

#define vec_push(v, x)                                                         \
    ((void)((v).len >= (v).cap                                                 \
                ? (void)((v).data = vec_grow_((v).data, (v).len, &(v).cap,     \
                                              sizeof *(v).data))               \
                : (void)0),                                                    \
     (v).data[(v).len++] = (x))
#define vec_last(v) ((v).data[(v).len - 1])
#define vec_pop(v) ((v).data[--(v).len])

typedef VEC(char) CharVec;

/* ------------------------------------------------------------------ */
/* String interning                                                    */
/* ------------------------------------------------------------------ */

/*
 * A Sym is a small integer identifying an interned string. Sym 0 means
 * "no symbol". Equal strings intern to equal Syms, so compare Syms with ==.
 * The first symbols interned (by lex_init) are the Rust keywords; see
 * the KW_* enum in lex.h, whose values are the Syms of those keywords.
 */
typedef uint32_t Sym;

Sym sym_intern(const char *s, size_t len);
Sym sym_intern_cstr(const char *s);
const char *sym_str(Sym s); /* NUL-terminated; "" for Sym 0 */
uint32_t sym_len(Sym s);

/* ------------------------------------------------------------------ */
/* Source files and spans                                              */
/* ------------------------------------------------------------------ */

typedef struct SourceFile {
    const char *path;
    const char *text; /* NUL-terminated contents */
    uint32_t len;
} SourceFile;

/*
 * A Span is a byte range [lo, hi) in a source file plus the 1-based line and
 * column of `lo` (column counted in bytes). file == 0 means "unknown".
 */
typedef struct Span {
    uint32_t file;
    uint32_t lo, hi;
    uint32_t line, col;
} Span;

/* Load a file into the table; fatal on I/O error. Returns its file id (>=1). */
uint32_t source_load(const char *path);
/* Register in-memory text (e.g. for tests or generated code). */
uint32_t source_add(const char *path, const char *text, uint32_t len);
const SourceFile *source_get(uint32_t file);

/* Span covering a through b (b must not precede a in the same file). */
Span span_join(Span a, Span b);

/* ------------------------------------------------------------------ */
/* Errors                                                              */
/* ------------------------------------------------------------------ */

/* Print "file:line:col: error: message" to stderr and exit(1). */
_Noreturn void fatal(Span sp, const char *fmt, ...);
_Noreturn void vfatal(Span sp, const char *fmt, va_list ap);
/* Print "file:line:col: warning: message" to stderr. */
void warn(Span sp, const char *fmt, ...);

#endif
