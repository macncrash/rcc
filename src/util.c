/* util.c - arena, vectors, interning, source table, errors. */
#include "util.h"

#include <stdio.h>
#include <stdlib.h>

/* ------------------------------------------------------------------ */
/* Arena                                                               */
/* ------------------------------------------------------------------ */

#define ARENA_CHUNK (4u << 20)

static char *arena_cur;
static size_t arena_left;

void *arena_alloc(size_t size) {
    size = (size + 15) & ~(size_t)15;
    if (size == 0)
        size = 16;
    if (size > arena_left) {
        size_t chunk = size > ARENA_CHUNK / 4 ? size : ARENA_CHUNK;
        char *p = calloc(1, chunk);
        if (!p) {
            fprintf(stderr, "rcc: out of memory\n");
            exit(1);
        }
        if (chunk != ARENA_CHUNK)
            return p; /* big allocation gets its own chunk */
        arena_cur = p;
        arena_left = chunk;
    }
    void *r = arena_cur;
    arena_cur += size;
    arena_left -= size;
    return r;
}

char *arena_strndup(const char *s, size_t n) {
    char *p = arena_alloc(n + 1);
    memcpy(p, s, n);
    p[n] = 0;
    return p;
}

char *arena_strdup(const char *s) { return arena_strndup(s, strlen(s)); }

char *arena_printf(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    char buf[256];
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    if (n < 0)
        n = 0;
    if ((size_t)n < sizeof buf)
        return arena_strndup(buf, (size_t)n);
    char *p = arena_alloc((size_t)n + 1);
    va_start(ap, fmt);
    vsnprintf(p, (size_t)n + 1, fmt, ap);
    va_end(ap);
    return p;
}

void *vec_grow_(void *data, uint32_t len, uint32_t *cap, size_t elem_size) {
    uint32_t nc = *cap ? *cap * 2 : 4;
    void *n = arena_alloc(nc * elem_size);
    if (data && len)
        memcpy(n, data, len * elem_size);
    *cap = nc;
    return n;
}

/* ------------------------------------------------------------------ */
/* Interning                                                           */
/* ------------------------------------------------------------------ */

typedef struct SymEntry {
    const char *str;
    uint32_t len;
    uint32_t hash;
} SymEntry;

static SymEntry *sym_entries; /* index = Sym */
static uint32_t sym_count, sym_cap;
static uint32_t *sym_table; /* open addressing: holds Sym, 0 = empty */
static uint32_t sym_table_cap;

static uint32_t hash_bytes(const char *s, size_t n) {
    uint32_t h = 2166136261u;
    for (size_t i = 0; i < n; i++) {
        h ^= (unsigned char)s[i];
        h *= 16777619u;
    }
    return h;
}

static void sym_rehash(void) {
    uint32_t ncap = sym_table_cap ? sym_table_cap * 2 : 4096;
    uint32_t *nt = calloc(ncap, sizeof(uint32_t));
    if (!nt) {
        fprintf(stderr, "rcc: out of memory\n");
        exit(1);
    }
    for (uint32_t i = 1; i < sym_count; i++) {
        uint32_t j = sym_entries[i].hash & (ncap - 1);
        while (nt[j])
            j = (j + 1) & (ncap - 1);
        nt[j] = i;
    }
    free(sym_table);
    sym_table = nt;
    sym_table_cap = ncap;
}

Sym sym_intern(const char *s, size_t len) {
    if (sym_count == 0) {
        sym_cap = 1024;
        sym_entries = calloc(sym_cap, sizeof(SymEntry));
        sym_entries[0].str = "";
        sym_count = 1;
    }
    if ((sym_count + 1) * 2 > sym_table_cap)
        sym_rehash();
    uint32_t h = hash_bytes(s, len);
    uint32_t j = h & (sym_table_cap - 1);
    while (sym_table[j]) {
        SymEntry *e = &sym_entries[sym_table[j]];
        if (e->hash == h && e->len == len && memcmp(e->str, s, len) == 0)
            return sym_table[j];
        j = (j + 1) & (sym_table_cap - 1);
    }
    if (sym_count == sym_cap) {
        sym_cap *= 2;
        sym_entries = realloc(sym_entries, sym_cap * sizeof(SymEntry));
        if (!sym_entries) {
            fprintf(stderr, "rcc: out of memory\n");
            exit(1);
        }
    }
    Sym id = sym_count++;
    sym_entries[id].str = arena_strndup(s, len);
    sym_entries[id].len = (uint32_t)len;
    sym_entries[id].hash = h;
    sym_table[j] = id;
    return id;
}

Sym sym_intern_cstr(const char *s) { return sym_intern(s, strlen(s)); }

const char *sym_str(Sym s) {
    if (s == 0 || s >= sym_count)
        return "";
    return sym_entries[s].str;
}

uint32_t sym_len(Sym s) {
    if (s == 0 || s >= sym_count)
        return 0;
    return sym_entries[s].len;
}

/* ------------------------------------------------------------------ */
/* Source files                                                        */
/* ------------------------------------------------------------------ */

static VEC(SourceFile) files;

uint32_t source_add(const char *path, const char *text, uint32_t len) {
    if (files.len == 0) {
        SourceFile dummy = {"<unknown>", "", 0};
        vec_push(files, dummy);
    }
    SourceFile f = {arena_strdup(path), text, len};
    vec_push(files, f);
    return files.len - 1;
}

uint32_t source_load(const char *path) {
    FILE *fp = fopen(path, "rb");
    if (!fp) {
        fprintf(stderr, "rcc: cannot open %s\n", path);
        exit(1);
    }
    CharVec buf = {0};
    char tmp[65536];
    size_t n;
    while ((n = fread(tmp, 1, sizeof tmp, fp)) > 0)
        for (size_t i = 0; i < n; i++)
            vec_push(buf, tmp[i]);
    fclose(fp);
    uint32_t len = buf.len;
    vec_push(buf, 0);
    return source_add(path, buf.data, len);
}

const SourceFile *source_get(uint32_t file) {
    if (file == 0 || file >= files.len)
        return NULL;
    return &files.data[file];
}

Span span_join(Span a, Span b) {
    if (a.file != b.file || b.hi < a.lo)
        return a;
    a.hi = b.hi > a.hi ? b.hi : a.hi;
    return a;
}

/* ------------------------------------------------------------------ */
/* Errors                                                              */
/* ------------------------------------------------------------------ */

static void print_loc(Span sp) {
    const SourceFile *f = source_get(sp.file);
    if (f)
        fprintf(stderr, "%s:%u:%u: ", f->path, sp.line, sp.col);
    else
        fprintf(stderr, "rcc: ");
}

_Noreturn void vfatal(Span sp, const char *fmt, va_list ap) {
    print_loc(sp);
    fprintf(stderr, "error: ");
    vfprintf(stderr, fmt, ap);
    fprintf(stderr, "\n");
    exit(1);
}

_Noreturn void fatal(Span sp, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    vfatal(sp, fmt, ap);
}

void warn(Span sp, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    print_loc(sp);
    fprintf(stderr, "warning: ");
    vfprintf(stderr, fmt, ap);
    fprintf(stderr, "\n");
    va_end(ap);
}
