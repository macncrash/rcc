/*
 * expand.c - the expansion driver: cfg evaluation and stripping, `mod foo;`
 * loading, the AST walk that finds and replaces macro invocations, macro
 * name resolution (textual scope, #[macro_export], builtins), #[derive]
 * dispatch, and the final cleanup/verification pass.
 *
 * Expansion order: one walk of the crate in source order expands every
 * invocation it can, depth first (outer call, then the calls in its
 * output). An invocation whose macro cannot be found yet (e.g. a
 * #[macro_export] macro used by path before its definition) is left in
 * place and the walk is repeated until nothing is left, or until a pass
 * makes no progress, after which a final pass reports the error.
 */
#include <stdio.h>
#include <stdlib.h>

#include "expand_int.h"

/* ------------------------------------------------------------------ */
/* Pointer map                                                         */
/* ------------------------------------------------------------------ */

static uint32_t ptr_hash(const void *p) {
    uint64_t x = (uint64_t)(uintptr_t)p;
    x ^= x >> 33;
    x *= 0xff51afd7ed558ccdULL;
    x ^= x >> 33;
    return (uint32_t)x;
}

void *ptrmap_get(const PtrMap *m, const void *key) {
    if (!m->cap)
        return NULL;
    uint32_t mask = m->cap - 1;
    for (uint32_t i = ptr_hash(key) & mask;; i = (i + 1) & mask) {
        if (m->keys[i] == key)
            return m->vals[i];
        if (!m->keys[i])
            return NULL;
    }
}

void ptrmap_put(PtrMap *m, const void *key, void *val) {
    if ((m->len + 1) * 2 > m->cap) {
        PtrMap n;
        n.cap = m->cap ? m->cap * 2 : 64;
        n.len = 0;
        n.keys = NEW_ARRAY(const void *, n.cap);
        n.vals = NEW_ARRAY(void *, n.cap);
        for (uint32_t i = 0; i < m->cap; i++)
            if (m->keys[i])
                ptrmap_put(&n, m->keys[i], m->vals[i]);
        *m = n;
    }
    uint32_t mask = m->cap - 1;
    for (uint32_t i = ptr_hash(key) & mask;; i = (i + 1) & mask) {
        if (m->keys[i] == key) {
            m->vals[i] = val;
            return;
        }
        if (!m->keys[i]) {
            m->keys[i] = key;
            m->vals[i] = val;
            m->len++;
            return;
        }
    }
}

/* ------------------------------------------------------------------ */
/* State                                                               */
/* ------------------------------------------------------------------ */

typedef struct MScope {
    MacroDef *def;
    struct MScope *prev;
} MScope;

typedef struct CfgEntry {
    Sym key;
    Sym value; /* 0 for a bare name */
} CfgEntry;

struct ExpandState {
    const ExpandOptions *opts;
    const char *fmt_root;
    const char *crate_name;
    VEC(CfgEntry) cfg;
    MScope *scope;
    VEC(MacroDef *) exported;
    PtrMap defs;     /* Item* (macro_rules) -> MacroDef* */
    PtrMap mod_dirs; /* Item* (mod) -> const char* directory for children */
    PtrMap frozen;   /* MacCall* -> 1 */
    VEC(const char *) modpath;
    const char *mod_dir;
    bool in_file_mod; /* current module body comes straight from a file */
    uint32_t depth;   /* macro expansion nesting */
    uint32_t site_depth;
    Span site;
    uint32_t unresolved;
    uint32_t expanded;
    bool final_pass;
    bool cleanup;
    uint8_t *seen_ids;
    uint32_t max_id;
    uint32_t fresh;
    Crate *crate;
};

static ExpandState S;
ExpandState *EX = &S;

const ExpandOptions *ex_opts(void) { return S.opts; }
const char *ex_fmt_root(void) { return S.fmt_root; }

Span ex_site(Span inner) { return S.site_depth ? S.site : inner; }

void ex_freeze(MacCall *m) { ptrmap_put(&S.frozen, m, (void *)1); }
bool ex_is_frozen(const MacCall *m) { return ptrmap_get(&S.frozen, m) != NULL; }

Sym ex_fresh_name(const char *stem) {
    return sym_intern_cstr(arena_printf("__rcc_%s%u", stem, S.fresh++));
}

const char *ex_getenv(const char *name) {
    const char *v = NULL;
    size_t n = strlen(name);
    if (S.opts)
        for (uint32_t i = 0; i < S.opts->nenvs; i++) {
            const char *e = S.opts->envs[i];
            if (strncmp(e, name, n) == 0 && e[n] == '=')
                v = e + n + 1;
        }
    if (v)
        return v;
    return getenv(name);
}

const char *ex_module_path(void) {
    CharVec cv = {0};
    const char *c = S.crate_name;
    for (; *c; c++)
        vec_push(cv, *c);
    for (uint32_t i = 0; i < S.modpath.len; i++) {
        vec_push(cv, ':');
        vec_push(cv, ':');
        for (c = S.modpath.data[i]; *c; c++)
            vec_push(cv, *c);
    }
    return arena_strndup(cv.data ? cv.data : "", cv.len);
}

static const char *dirname_of(const char *path) {
    const char *slash = strrchr(path, '/');
    if (!slash)
        return ".";
    if (slash == path)
        return "/";
    return arena_strndup(path, (size_t)(slash - path));
}

static const char *path_join(const char *a, const char *b) {
    if (b[0] == '/')
        return b;
    if (strcmp(a, ".") == 0)
        return arena_strdup(b);
    return arena_printf("%s/%s", a, b);
}

const char *ex_file_of_span(Span sp) {
    const SourceFile *f = source_get(sp.file);
    return f ? f->path : "<unknown>";
}

const char *ex_dir_of_span(Span sp) { return dirname_of(ex_file_of_span(sp)); }

/* ------------------------------------------------------------------ */
/* Token buffers                                                       */
/* ------------------------------------------------------------------ */

void tb_init(TokBuf *tb, Span sp) {
    memset(tb, 0, sizeof *tb);
    tb->sp = sp;
}

TokenList tb_list(const TokBuf *tb) {
    TokenList tl = {tb->v.data, tb->v.len};
    return tl;
}

void tb_tok(TokBuf *tb, Token t) { vec_push(tb->v, t); }

void tb_toks(TokBuf *tb, TokenList tl) {
    for (uint32_t i = 0; i < tl.len; i++)
        vec_push(tb->v, tl.toks[i]);
}

static PtrMap lex_cache;

void tb_text(TokBuf *tb, const char *text) {
    Sym key = sym_intern_cstr(text);
    TokenList *tl = ptrmap_get(&lex_cache, (void *)(uintptr_t)key);
    if (!tl) {
        /* Template fragments may have unbalanced delimiters, which the
           lexer rejects: balance them with extra delimiters around the
           text and drop those tokens again. */
        char stack[256];
        int sp = 0, pre = 0;
        char prefix[256];
        for (const char *c = text; *c; c++) {
            if (*c == '"') {
                for (c++; *c && *c != '"'; c++)
                    if (*c == '\\' && c[1])
                        c++;
                if (!*c)
                    break;
                continue;
            }
            if (*c == '(' || *c == '[' || *c == '{') {
                if (sp < 255)
                    stack[sp++] = *c;
            } else if (*c == ')' || *c == ']' || *c == '}') {
                if (sp > 0)
                    sp--;
                else if (pre < 255)
                    prefix[pre++] = *c == ')' ? '(' : *c == ']' ? '[' : '{';
            }
        }
        CharVec cv = {0};
        for (int i = pre - 1; i >= 0; i--) {
            vec_push(cv, prefix[i]);
            vec_push(cv, ' ');
        }
        for (const char *c = text; *c; c++)
            vec_push(cv, *c);
        for (int i = sp - 1; i >= 0; i--) {
            vec_push(cv, ' ');
            vec_push(cv, stack[i] == '(' ? ')' : stack[i] == '[' ? ']' : '}');
        }
        const char *copy = arena_strndup(cv.data, cv.len);
        uint32_t f = source_add("<expand>", copy, cv.len);
        TokenList all = lex_file(f);
        tl = NEW(TokenList);
        uint32_t n = all.len ? all.len - 1 : 0; /* drop EOF */
        tl->toks = all.toks + pre;
        tl->len = n - (uint32_t)pre - (uint32_t)sp;
        ptrmap_put(&lex_cache, (void *)(uintptr_t)key, tl);
    }
    for (uint32_t i = 0; i < tl->len; i++) {
        Token t = tl->toks[i];
        t.span = tb->sp;
        vec_push(tb->v, t);
    }
}

void tb_fmt(TokBuf *tb, const char *fmt, ...) {
    char buf[1024];
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    if (n < 0 || n >= (int)sizeof buf)
        fatal(tb->sp, "internal: tb_fmt text too long");
    tb_text(tb, buf);
}

void tb_ident(TokBuf *tb, Sym s) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = TK_IDENT;
    t.flags = TF_WS_BEFORE;
    t.v.sym = s;
    t.span = tb->sp;
    if (sym_is_strict_kw(s) && s != KW_SELF_VALUE && s != KW_SELF_TYPE &&
        s != KW_CRATE && s != KW_SUPER)
        t.flags |= TF_RAW;
    vec_push(tb->v, t);
}

Lit *mk_str_lit(const char *s, uint32_t len) {
    Lit *l = NEW(Lit);
    l->kind = LIT_STR;
    l->str = arena_strndup(s, len);
    l->str_len = len;
    return l;
}

Expr *mk_str_expr(const char *s, uint32_t len, Span sp) {
    Expr *e = ast_new_expr(EXPR_LIT, sp);
    e->lit = mk_str_lit(s, len);
    return e;
}

void tb_str(TokBuf *tb, const char *s, uint32_t len) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = TK_LITERAL;
    t.flags = TF_WS_BEFORE;
    t.v.lit = mk_str_lit(s, len);
    t.span = tb->sp;
    vec_push(tb->v, t);
}

void tb_char(TokBuf *tb, uint32_t c) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = TK_LITERAL;
    t.flags = TF_WS_BEFORE;
    Lit *l = NEW(Lit);
    l->kind = LIT_CHAR;
    l->char_val = c;
    t.v.lit = l;
    t.span = tb->sp;
    vec_push(tb->v, t);
}

void tb_uint(TokBuf *tb, uint64_t v, const char *suffix) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = TK_LITERAL;
    t.flags = TF_WS_BEFORE;
    Lit *l = NEW(Lit);
    l->kind = LIT_INT;
    l->int_lo = v;
    l->base = 10;
    if (suffix)
        l->suffix = sym_intern_cstr(suffix);
    t.v.lit = l;
    t.span = tb->sp;
    vec_push(tb->v, t);
}

void tb_bool(TokBuf *tb, bool v) { tb_ident(tb, v ? KW_TRUE : KW_FALSE); }

void tb_punct(TokBuf *tb, int kind) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = (uint8_t)kind;
    t.span = tb->sp;
    vec_push(tb->v, t);
}

void tb_nt(TokBuf *tb, NtKind k, void *node, Span sp) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = TK_INTERP;
    t.flags = TF_WS_BEFORE;
    t.nt_kind = (uint8_t)k;
    t.v.node = node;
    t.span = sp;
    vec_push(tb->v, t);
}

/* ------------------------------------------------------------------ */
/* cfg                                                                 */
/* ------------------------------------------------------------------ */

static void cfg_set(const char *key, const char *value, bool replace) {
    Sym k = sym_intern_cstr(key);
    Sym v = value ? sym_intern_cstr(value) : 0;
    if (replace) {
        uint32_t j = 0;
        for (uint32_t i = 0; i < S.cfg.len; i++)
            if (S.cfg.data[i].key != k)
                S.cfg.data[j++] = S.cfg.data[i];
        S.cfg.len = j;
    }
    CfgEntry e = {k, v};
    vec_push(S.cfg, e);
}

static void cfg_init(const ExpandOptions *o) {
    cfg_set("unix", NULL, false);
    cfg_set("target_family", "unix", false);
#if defined(__APPLE__)
    cfg_set("target_os", "macos", false);
    cfg_set("target_vendor", "apple", false);
    cfg_set("target_env", "", false);
#elif defined(__linux__)
    cfg_set("target_os", "linux", false);
    cfg_set("target_vendor", "unknown", false);
    cfg_set("target_env", "gnu", false);
#else
    cfg_set("target_os", "unknown", false);
#endif
#if defined(__aarch64__) || defined(__arm64__)
    cfg_set("target_arch", "aarch64", false);
#elif defined(__x86_64__)
    cfg_set("target_arch", "x86_64", false);
#elif defined(__riscv)
    cfg_set("target_arch", "riscv64", false);
#else
    cfg_set("target_arch", "unknown", false);
#endif
    cfg_set("target_pointer_width", sizeof(void *) == 8 ? "64" : "32", false);
    {
        uint16_t one = 1;
        cfg_set("target_endian", *(uint8_t *)&one ? "little" : "big", false);
    }
    cfg_set("panic", "abort", false);
    if (!o)
        return;
    for (uint32_t i = 0; i < o->ncfgs; i++) {
        const char *spec = o->cfgs[i];
        const char *eq = strchr(spec, '=');
        if (!eq) {
            cfg_set(spec, NULL, false);
            continue;
        }
        const char *key = arena_strndup(spec, (size_t)(eq - spec));
        const char *v = eq + 1;
        size_t vl = strlen(v);
        if (vl >= 2 && v[0] == '"' && v[vl - 1] == '"')
            v = arena_strndup(v + 1, vl - 2);
        cfg_set(key, v, strncmp(key, "target_", 7) == 0);
    }
}

static bool cfg_has(Sym key, Sym value) {
    for (uint32_t i = 0; i < S.cfg.len; i++)
        if (S.cfg.data[i].key == key && S.cfg.data[i].value == value)
            return true;
    return false;
}

/* Parse one predicate from t[*i..n). */
static bool cfg_pred(const Token *t, uint32_t *i, uint32_t n, Span sp) {
    if (*i >= n || t[*i].kind != TK_IDENT)
        fatal(*i < n ? t[*i].span : sp, "malformed cfg predicate");
    Sym name = t[*i].v.sym;
    Span nsp = t[*i].span;
    (*i)++;
    if (*i < n && t[*i].kind == TK_EQ) {
        (*i)++;
        if (*i >= n || t[*i].kind != TK_LITERAL || t[*i].v.lit->kind != LIT_STR)
            fatal(nsp, "cfg: expected string after `=`");
        Lit *l = t[*i].v.lit;
        (*i)++;
        return cfg_has(name, sym_intern(l->str, l->str_len));
    }
    if (*i < n && t[*i].kind == TK_LPAREN) {
        const char *s = sym_str(name);
        uint32_t close = tt_end(t, *i, n);
        uint32_t j = *i + 1;
        bool is_all = strcmp(s, "all") == 0, is_any = strcmp(s, "any") == 0,
             is_not = strcmp(s, "not") == 0;
        if (!is_all && !is_any && !is_not)
            fatal(nsp, "cfg: unknown predicate `%s(...)`", s);
        bool acc = is_all;
        int count = 0;
        while (j < close) {
            bool v = cfg_pred(t, &j, close, sp);
            count++;
            if (is_all)
                acc = acc && v;
            else if (is_any)
                acc = acc || v;
            else
                acc = !v;
            if (j < close) {
                if (t[j].kind != TK_COMMA)
                    fatal(t[j].span, "cfg: expected `,`");
                j++;
            }
        }
        if (is_not && count != 1)
            fatal(nsp, "cfg: not() takes one predicate");
        *i = close + 1;
        return acc;
    }
    return cfg_has(name, 0);
}

bool ex_cfg_eval(TokenList tl, Span sp) {
    uint32_t i = 0;
    bool v = cfg_pred(tl.toks, &i, tl.len, sp);
    if (i < tl.len && tl.toks[i].kind == TK_COMMA)
        i++;
    if (i != tl.len)
        fatal(sp, "cfg: trailing tokens in predicate");
    return v;
}

/* ------------------------------------------------------------------ */
/* Attributes                                                          */
/* ------------------------------------------------------------------ */

static bool attr_is(const Attr *a, const char *name) {
    return a->path && a->path->segs.len == 1 &&
           strcmp(sym_str(a->path->segs.data[0]->name), name) == 0;
}

static Attr *find_attr(AttrList l, const char *name) {
    for (uint32_t i = 0; i < l.len; i++)
        if (attr_is(l.data[i], name))
            return l.data[i];
    return NULL;
}

static void remove_attrs(AttrList *l, const char *name) {
    uint32_t j = 0;
    for (uint32_t i = 0; i < l->len; i++)
        if (!attr_is(l->data[i], name))
            l->data[j++] = l->data[i];
    l->len = j;
}

/* Expand cfg_attr in place, evaluate and remove cfg. Returns false if the
   node is configured out. */
static bool attrs_keep(AttrList *l) {
    if (!l->len)
        return true;
    bool keep = true;
    AttrList out = {0};
    for (uint32_t i = 0; i < l->len; i++) {
        Attr *a = l->data[i];
        if (attr_is(a, "cfg")) {
            if (a->args_kind != ATTR_ARGS_DELIMITED)
                fatal(a->span, "malformed #[cfg]");
            if (!ex_cfg_eval(a->tokens, a->span))
                keep = false;
            continue;
        }
        if (attr_is(a, "cfg_attr")) {
            if (a->args_kind != ATTR_ARGS_DELIMITED)
                fatal(a->span, "malformed #[cfg_attr]");
            const Token *t = a->tokens.toks;
            uint32_t n = a->tokens.len, j = 0;
            bool v = cfg_pred(t, &j, n, a->span);
            if (j < n && t[j].kind == TK_COMMA)
                j++;
            else if (j < n)
                fatal(a->span, "cfg_attr: expected `,`");
            if (!v)
                continue;
            AttrList produced = {0};
            Parser *p = parser_new((TokenList){(Token *)t + j, n - j}, a->span);
            while (!parser_at_eof(p)) {
                Attr *na = parser_meta(p);
                na->inner = a->inner;
                vec_push(produced, na);
                if (parser_peek(p, 0)->kind == TK_COMMA)
                    parser_set_pos(p, parser_pos(p) + 1);
                else if (!parser_at_eof(p))
                    fatal(parser_peek(p, 0)->span, "cfg_attr: expected `,`");
            }
            if (!attrs_keep(&produced))
                keep = false;
            for (uint32_t k = 0; k < produced.len; k++)
                vec_push(out, produced.data[k]);
            continue;
        }
        vec_push(out, a);
    }
    *l = out;
    return keep;
}

bool ex_attrs_keep(AttrList *l) { return attrs_keep(l); }

/* ------------------------------------------------------------------ */
/* Macro scopes and resolution                                         */
/* ------------------------------------------------------------------ */

static MacroDef *def_of(Item *mr) {
    MacroDef *d = ptrmap_get(&S.defs, mr);
    if (!d) {
        d = mbe_compile(mr);
        ptrmap_put(&S.defs, mr, d);
    }
    return d;
}

static void define_macro(Item *mr) {
    MacroDef *d = def_of(mr);
    MScope *sc = NEW(MScope);
    sc->def = d;
    sc->prev = S.scope;
    S.scope = sc;
    if (find_attr(mr->attrs, "macro_export")) {
        for (uint32_t i = 0; i < S.exported.len; i++)
            if (S.exported.data[i] == d)
                return;
        /* a later definition with the same name replaces the earlier */
        for (uint32_t i = 0; i < S.exported.len; i++)
            if (mbe_name(S.exported.data[i]) == mbe_name(d)) {
                S.exported.data[i] = d;
                return;
            }
        vec_push(S.exported, d);
    }
}

static MacroDef *lookup_textual(Sym name) {
    for (MScope *s = S.scope; s; s = s->prev)
        if (mbe_name(s->def) == name)
            return s->def;
    return NULL;
}

static MacroDef *lookup_exported(Sym name) {
    for (uint32_t i = 0; i < S.exported.len; i++)
        if (mbe_name(S.exported.data[i]) == name)
            return S.exported.data[i];
    return NULL;
}

static bool is_std_crate_name(Sym s) {
    const char *n = sym_str(s);
    return strcmp(n, "std") == 0 || strcmp(n, "core") == 0 || strcmp(n, "alloc") == 0;
}

static MacOut invoke(MacCall *m, MacPos pos) {
    MacOut out;
    memset(&out, 0, sizeof out);
    Path *p = m->path;
    Sym name = vec_last(p->segs)->name;
    MacroDef *d = NULL;
    bool std_path = false;
    if (p->segs.len == 1 && !p->global) {
        d = lookup_textual(name);
        if (!d)
            d = lookup_exported(name);
    } else {
        Sym first = p->segs.data[0]->name;
        if (is_std_crate_name(first))
            std_path = true;
        else
            d = lookup_exported(name);
    }
    (void)std_path;
    if (d) {
        if (S.depth > 1024)
            fatal(m->span, "recursion limit reached while expanding `%s!`", sym_str(name));
        out.kind = MO_TOKENS;
        out.toks = mbe_expand(d, m, pos);
        S.expanded++;
        return out;
    }
    if (builtin_exists(name)) {
        out = builtin_expand(name, m, pos);
        if (out.kind != MO_DEFER)
            S.expanded++;
        return out;
    }
    if (S.final_pass)
        fatal(m->span, "cannot find macro `%s!` in this scope", sym_str(name));
    out.kind = MO_DEFER;
    return out;
}

/* Track the outermost call site (line!(), panic locations). */
static bool span_within(Span in, Span outer) {
    return in.file == outer.file && in.lo >= outer.lo && in.hi <= outer.hi;
}

static Span site_enter(MacCall *m) {
    Span saved = S.site;
    /* A call written by the user inside another call's arguments is its
       own site; a call produced by a macro body inherits the outer site. */
    if (S.site_depth == 0 || span_within(m->span, S.site))
        S.site = m->span;
    S.site_depth++;
    S.depth++;
    return saved;
}
static void site_leave(Span saved) {
    S.site_depth--;
    S.depth--;
    S.site = saved;
}

/* ------------------------------------------------------------------ */
/* The walk                                                            */
/* ------------------------------------------------------------------ */

static void w_items(ItemList *items, MacPos pos);
static void w_item(Item *it, ItemList *out, MacPos pos);
static void w_block(Block *b);
static void w_stmts(StmtList *sl);
static void w_expr(Expr **pe);
static void w_pat(Pat **pp);
static void w_ty(Ty **pt);
static void w_path(Path *p);
static void w_generics(Generics *g);
static void w_bounds(BoundList *bl);
static void w_attrs(AttrList *l);

static void mark(NodeId id, Span sp) {
    if (!S.cleanup)
        return;
    if (id == 0 || id >= S.max_id)
        fatal(sp, "internal: bad NodeId %u after expansion", id);
    if (S.seen_ids[id >> 3] & (1u << (id & 7)))
        fatal(sp, "internal: duplicate NodeId %u after expansion", id);
    S.seen_ids[id >> 3] |= (uint8_t)(1u << (id & 7));
}

static void check_mac_left(MacCall *m) {
    if (S.cleanup && !ex_is_frozen(m))
        fatal(m->span, "internal: macro call `%s!` left after expansion",
              sym_str(vec_last(m->path->segs)->name));
    if (S.cleanup) {
        mark(m->id, m->span);
        w_path(m->path);
        /* interpolated nodes inside frozen calls */
        for (uint32_t i = 0; i < m->tokens.len; i++) {
            Token *t = &m->tokens.toks[i];
            if (t->kind != TK_INTERP)
                continue;
            switch (t->nt_kind) {
            case NT_EXPR:
            case NT_LIT: w_expr((Expr **)&t->v.node); break;
            case NT_TY: w_ty((Ty **)&t->v.node); break;
            case NT_BLOCK: w_block((Block *)t->v.node); break;
            default: break;
            }
        }
    }
}

static void w_attrs(AttrList *l) {
    if (!S.cleanup)
        return;
    for (uint32_t i = 0; i < l->len; i++) {
        Attr *a = l->data[i];
        mark(a->id, a->span);
        w_path(a->path);
        if (a->value)
            w_expr(&a->value);
    }
}

static GenericArgs *w_gargs(GenericArgs *ga) {
    if (!ga)
        return NULL;
    for (uint32_t i = 0; i < ga->args.len; i++) {
        GenericArg *a = ga->args.data[i];
        if (a->ty)
            w_ty(&a->ty);
        if (a->expr)
            w_expr(&a->expr);
        if (a->name_args)
            w_gargs(a->name_args);
        w_bounds(&a->bounds);
    }
    for (uint32_t i = 0; i < ga->inputs.len; i++)
        w_ty(&ga->inputs.data[i]);
    if (ga->output)
        w_ty(&ga->output);
    return ga;
}

static void w_path(Path *p) {
    if (!p)
        return;
    mark(p->id, p->span);
    for (uint32_t i = 0; i < p->segs.len; i++) {
        PathSegment *s = p->segs.data[i];
        mark(s->id, s->span);
        if (s->args)
            w_gargs(s->args);
    }
}

static void w_qself(QSelf *q) {
    if (q)
        w_ty(&q->ty);
}

static void w_gparams(GenericParamList *l) {
    uint32_t j = 0;
    for (uint32_t i = 0; i < l->len; i++) {
        GenericParam *g = l->data[i];
        if (!attrs_keep(&g->attrs))
            continue;
        mark(g->id, g->span);
        w_attrs(&g->attrs);
        w_bounds(&g->bounds);
        if (g->default_ty)
            w_ty(&g->default_ty);
        if (g->const_ty)
            w_ty(&g->const_ty);
        if (g->const_default)
            w_expr(&g->const_default);
        l->data[j++] = g;
    }
    l->len = j;
}

static void w_bounds(BoundList *bl) {
    for (uint32_t i = 0; i < bl->len; i++) {
        GenericBound *b = bl->data[i];
        if (b->path)
            w_path(b->path);
        w_gparams(&b->for_lifetimes);
    }
}

static void w_generics(Generics *g) {
    w_gparams(&g->params);
    for (uint32_t i = 0; i < g->where.len; i++) {
        WherePredicate *w = g->where.data[i];
        w_gparams(&w->for_lifetimes);
        if (w->ty)
            w_ty(&w->ty);
        w_bounds(&w->bounds);
    }
}

static void w_ty(Ty **pt) {
    Ty *t = *pt;
    if (!t)
        return;
    switch (t->kind) {
    case TY_MAC: {
        MacCall *m = t->mac;
        if (ex_is_frozen(m)) {
            check_mac_left(m);
            break;
        }
        if (S.cleanup)
            check_mac_left(m);
        Span saved = site_enter(m);
        MacOut o = invoke(m, POS_TY);
        if (o.kind == MO_DEFER) {
            S.unresolved++;
            site_leave(saved);
            return;
        }
        if (o.kind != MO_TOKENS)
            fatal(m->span, "macro does not expand to a type");
        *pt = parse_ty_from_tokens(o.toks, m->span);
        w_ty(pt);
        site_leave(saved);
        return;
    }
    case TY_PATH:
        w_qself(t->path.qself);
        w_path(t->path.path);
        break;
    case TY_REF: w_ty(&t->ref.elem); break;
    case TY_PTR: w_ty(&t->ptr.elem); break;
    case TY_SLICE: w_ty(&t->slice.elem); break;
    case TY_ARRAY:
        w_ty(&t->array.elem);
        w_expr(&t->array.len);
        break;
    case TY_TUPLE:
        for (uint32_t i = 0; i < t->tuple.elems.len; i++)
            w_ty(&t->tuple.elems.data[i]);
        break;
    case TY_FN:
        w_gparams(&t->fn_.for_lifetimes);
        for (uint32_t i = 0; i < t->fn_.params.len; i++)
            w_ty(&t->fn_.params.data[i]->ty);
        if (t->fn_.ret)
            w_ty(&t->fn_.ret);
        break;
    case TY_DYN:
    case TY_IMPL: w_bounds(&t->bounds); break;
    case TY_NEVER:
    case TY_INFER: break;
    }
    mark(t->id, t->span);
}

static void w_pats(PatList *l) {
    for (uint32_t i = 0; i < l->len; i++)
        w_pat(&l->data[i]);
}

static void w_pat(Pat **pp) {
    Pat *p = *pp;
    if (!p)
        return;
    switch (p->kind) {
    case PAT_MAC: {
        MacCall *m = p->mac;
        if (ex_is_frozen(m)) {
            check_mac_left(m);
            break;
        }
        if (S.cleanup)
            check_mac_left(m);
        Span saved = site_enter(m);
        MacOut o = invoke(m, POS_PAT);
        if (o.kind == MO_DEFER) {
            S.unresolved++;
            site_leave(saved);
            return;
        }
        if (o.kind != MO_TOKENS)
            fatal(m->span, "macro does not expand to a pattern");
        *pp = parse_pat_from_tokens(o.toks, m->span);
        w_pat(pp);
        site_leave(saved);
        return;
    }
    case PAT_IDENT:
        if (p->ident.sub)
            w_pat(&p->ident.sub);
        break;
    case PAT_LIT: w_expr(&p->lit); break;
    case PAT_RANGE:
        if (p->range.lo)
            w_expr(&p->range.lo);
        if (p->range.hi)
            w_expr(&p->range.hi);
        break;
    case PAT_PATH:
        w_qself(p->path.qself);
        w_path(p->path.path);
        break;
    case PAT_TUPLE:
    case PAT_SLICE:
    case PAT_OR: w_pats(&p->elems); break;
    case PAT_TUPLE_STRUCT:
        w_qself(p->tuple_struct.qself);
        w_path(p->tuple_struct.path);
        w_pats(&p->tuple_struct.elems);
        break;
    case PAT_STRUCT: {
        w_qself(p->struct_.qself);
        w_path(p->struct_.path);
        PatFieldList *fl = &p->struct_.fields;
        uint32_t j = 0;
        for (uint32_t i = 0; i < fl->len; i++) {
            PatField *f = fl->data[i];
            if (!attrs_keep(&f->attrs))
                continue;
            if (ex_unmark_sym(f->name) != f->name) {
                f->name = ex_unmark_sym(f->name);
                f->shorthand = false;
            }
            mark(f->id, f->span);
            w_attrs(&f->attrs);
            w_pat(&f->pat);
            fl->data[j++] = f;
        }
        fl->len = j;
        break;
    }
    case PAT_REF: w_pat(&p->ref.pat); break;
    case PAT_WILD:
    case PAT_REST: break;
    }
    mark(p->id, p->span);
}

/* Filter a list of expressions by their #[cfg] attributes and walk them. */
static void w_expr_list(ExprList *l) {
    uint32_t j = 0;
    for (uint32_t i = 0; i < l->len; i++) {
        Expr *e = l->data[i];
        if (!attrs_keep(&e->attrs))
            continue;
        w_expr(&e);
        l->data[j++] = e;
    }
    l->len = j;
}

static void w_param(Param *p) {
    mark(p->id, p->span);
    w_attrs(&p->attrs);
    w_pat(&p->pat);
    if (p->ty)
        w_ty(&p->ty);
}

static void w_params(ParamList *l) {
    uint32_t j = 0;
    for (uint32_t i = 0; i < l->len; i++) {
        Param *p = l->data[i];
        if (!attrs_keep(&p->attrs))
            continue;
        w_param(p);
        l->data[j++] = p;
    }
    l->len = j;
}

/* Remove a trailing `;` from macro output used as an expression. */
static TokenList strip_trailing_semi(TokenList tl) {
    if (tl.len && tl.toks[tl.len - 1].kind == TK_SEMI)
        tl.len--;
    return tl;
}

static void w_expr(Expr **pe) {
    Expr *e = *pe;
    if (!e)
        return;
    if (e->attrs.len) {
        if (!attrs_keep(&e->attrs))
            fatal(e->span, "unsupported: #[cfg] removing an expression in this position");
        w_attrs(&e->attrs);
    }
    switch (e->kind) {
    case EXPR_MAC: {
        MacCall *m = e->mac;
        if (ex_is_frozen(m)) {
            check_mac_left(m);
            break;
        }
        if (S.cleanup)
            check_mac_left(m);
        Span saved = site_enter(m);
        MacOut o = invoke(m, POS_EXPR);
        Expr *r = NULL;
        switch (o.kind) {
        case MO_DEFER:
            S.unresolved++;
            site_leave(saved);
            return;
        case MO_TOKENS:
            r = parse_expr_from_tokens(strip_trailing_semi(o.toks), m->span);
            break;
        case MO_EXPR: r = o.expr; break;
        case MO_NONE: r = ast_new_expr(EXPR_TUPLE, m->span); break;
        default: fatal(m->span, "macro does not expand to an expression");
        }
        for (uint32_t i = 0; i < e->attrs.len; i++)
            vec_push(r->attrs, e->attrs.data[i]);
        *pe = r;
        w_expr(pe);
        site_leave(saved);
        return;
    }
    case EXPR_LIT: break;
    case EXPR_PATH:
        w_qself(e->path.qself);
        w_path(e->path.path);
        break;
    case EXPR_UNARY: w_expr(&e->unary.operand); break;
    case EXPR_ADDR_OF: w_expr(&e->addr_of.operand); break;
    case EXPR_BINARY:
        w_expr(&e->binary.lhs);
        w_expr(&e->binary.rhs);
        break;
    case EXPR_ASSIGN:
    case EXPR_ASSIGN_OP:
        w_expr(&e->assign.lhs);
        w_expr(&e->assign.rhs);
        break;
    case EXPR_CAST:
        w_expr(&e->cast.expr);
        w_ty(&e->cast.ty);
        break;
    case EXPR_CALL:
        w_expr(&e->call.callee);
        w_expr_list(&e->call.args);
        break;
    case EXPR_METHOD_CALL:
        w_expr(&e->method_call.receiver);
        mark(e->method_call.method->id, e->method_call.method->span);
        if (e->method_call.method->args)
            w_gargs(e->method_call.method->args);
        w_expr_list(&e->method_call.args);
        break;
    case EXPR_FIELD: w_expr(&e->field.base); break;
    case EXPR_INDEX:
        w_expr(&e->index.base);
        w_expr(&e->index.index);
        break;
    case EXPR_STRUCT: {
        w_qself(e->struct_.qself);
        w_path(e->struct_.path);
        ExprFieldList *fl = &e->struct_.fields;
        uint32_t j = 0;
        for (uint32_t i = 0; i < fl->len; i++) {
            ExprField *f = fl->data[i];
            if (!attrs_keep(&f->attrs))
                continue;
            if (ex_unmark_sym(f->name) != f->name) {
                f->name = ex_unmark_sym(f->name);
                f->shorthand = false;
            }
            mark(f->id, f->span);
            w_attrs(&f->attrs);
            w_expr(&f->expr);
            fl->data[j++] = f;
        }
        fl->len = j;
        if (e->struct_.base)
            w_expr(&e->struct_.base);
        break;
    }
    case EXPR_TUPLE:
    case EXPR_ARRAY: w_expr_list(&e->elems); break;
    case EXPR_REPEAT:
        w_expr(&e->repeat.elem);
        w_expr(&e->repeat.count);
        break;
    case EXPR_RANGE:
        if (e->range.lo)
            w_expr(&e->range.lo);
        if (e->range.hi)
            w_expr(&e->range.hi);
        break;
    case EXPR_CLOSURE:
        w_params(&e->closure.params);
        if (e->closure.ret)
            w_ty(&e->closure.ret);
        w_expr(&e->closure.body);
        break;
    case EXPR_BLOCK: w_block(e->block.block); break;
    case EXPR_IF:
        w_expr(&e->if_.cond);
        w_block(e->if_.then);
        if (e->if_.els)
            w_expr(&e->if_.els);
        break;
    case EXPR_LET:
        w_pat(&e->let.pat);
        w_expr(&e->let.scrutinee);
        break;
    case EXPR_MATCH: {
        w_expr(&e->match.scrutinee);
        ArmList *al = &e->match.arms;
        uint32_t j = 0;
        for (uint32_t i = 0; i < al->len; i++) {
            Arm *a = al->data[i];
            if (!attrs_keep(&a->attrs))
                continue;
            mark(a->id, a->span);
            w_attrs(&a->attrs);
            w_pat(&a->pat);
            if (a->guard)
                w_expr(&a->guard);
            w_expr(&a->body);
            al->data[j++] = a;
        }
        al->len = j;
        break;
    }
    case EXPR_WHILE:
        w_expr(&e->while_.cond);
        w_block(e->while_.body);
        break;
    case EXPR_FOR:
        w_pat(&e->for_.pat);
        w_expr(&e->for_.iter);
        w_block(e->for_.body);
        break;
    case EXPR_LOOP: w_block(e->loop.body); break;
    case EXPR_BREAK:
        if (e->break_.value)
            w_expr(&e->break_.value);
        break;
    case EXPR_CONTINUE: break;
    case EXPR_RETURN:
        if (e->ret)
            w_expr(&e->ret);
        break;
    case EXPR_TRY: w_expr(&e->try_); break;
    case EXPR_UNDERSCORE: break;
    }
    mark(e->id, e->span);
}

bool ex_expand_expr_now(Expr **e) {
    uint32_t before = S.unresolved;
    w_expr(e);
    return S.unresolved == before;
}

static void w_block(Block *b) {
    if (!b)
        return;
    MScope *saved = S.scope;
    mark(b->id, b->span);
    w_attrs(&b->attrs);
    w_stmts(&b->stmts);
    S.scope = saved;
}

static Stmt *item_stmt(Item *it) {
    Stmt *s = ast_new_stmt(STMT_ITEM, it->span);
    s->item = it;
    return s;
}

static bool stmt_attrs_keep(Stmt *s) {
    if (!attrs_keep(&s->attrs))
        return false;
    if (s->kind == STMT_EXPR && s->expr.expr->attrs.len &&
        !attrs_keep(&s->expr.expr->attrs))
        return false;
    return true;
}

static void w_stmts(StmtList *sl) {
    StmtList out = {0};
    for (uint32_t i = 0; i < sl->len; i++) {
        Stmt *s = sl->data[i];
        if (s->kind == STMT_ITEM) {
            ItemList items = {0};
            w_item(s->item, &items, POS_ITEM);
            for (uint32_t k = 0; k < items.len; k++) {
                if (items.data[k] == s->item) {
                    mark(s->id, s->span);
                    vec_push(out, s);
                } else {
                    Stmt *ns = item_stmt(items.data[k]);
                    mark(ns->id, ns->span);
                    vec_push(out, ns);
                }
            }
            continue;
        }
        if (!stmt_attrs_keep(s))
            continue;
        mark(s->id, s->span);
        w_attrs(&s->attrs);
        switch (s->kind) {
        case STMT_LET:
            w_pat(&s->let.pat);
            if (s->let.ty)
                w_ty(&s->let.ty);
            if (s->let.init)
                w_expr(&s->let.init);
            if (s->let.els)
                w_block(s->let.els);
            vec_push(out, s);
            break;
        case STMT_EXPR:
            w_expr(&s->expr.expr);
            vec_push(out, s);
            break;
        case STMT_EMPTY:
            vec_push(out, s);
            break;
        case STMT_ITEM:
            break;
        case STMT_MAC: {
            MacCall *m = s->mac.mac;
            if (ex_is_frozen(m)) {
                check_mac_left(m);
                vec_push(out, s);
                break;
            }
            if (S.cleanup)
                check_mac_left(m);
            Span saved = site_enter(m);
            MacOut o = invoke(m, POS_STMT);
            StmtList produced = {0};
            switch (o.kind) {
            case MO_DEFER:
                S.unresolved++;
                vec_push(out, s);
                site_leave(saved);
                continue;
            case MO_TOKENS:
                produced = parse_stmts_from_tokens(o.toks, m->span);
                break;
            case MO_STMTS: produced = o.stmts; break;
            case MO_EXPR: {
                Stmt *ns = ast_new_stmt(STMT_EXPR, m->span);
                ns->expr.expr = o.expr;
                vec_push(produced, ns);
                break;
            }
            case MO_ITEMS:
                for (uint32_t k = 0; k < o.items.len; k++)
                    vec_push(produced, item_stmt(o.items.data[k]));
                break;
            case MO_NONE: break;
            }
            /* The statement's own attributes go on the first produced stmt. */
            if (produced.len && s->attrs.len) {
                Stmt *f = produced.data[0];
                AttrList *dst = f->kind == STMT_ITEM ? &f->item->attrs : &f->attrs;
                for (uint32_t k = 0; k < s->attrs.len; k++)
                    vec_push(*dst, s->attrs.data[k]);
            }
            if (produced.len && s->mac.semi) {
                Stmt *l = vec_last(produced);
                if (l->kind == STMT_EXPR)
                    l->expr.semi = true;
            }
            if (produced.len && !s->mac.semi && i + 1 < sl->len) {
                /* `m! { .. }` in the middle of a block is a statement */
                Stmt *l = vec_last(produced);
                if (l->kind == STMT_EXPR && !l->expr.semi && l->expr.expr->kind != EXPR_BLOCK &&
                    l->expr.expr->kind != EXPR_IF && l->expr.expr->kind != EXPR_MATCH &&
                    l->expr.expr->kind != EXPR_LOOP && l->expr.expr->kind != EXPR_WHILE &&
                    l->expr.expr->kind != EXPR_FOR)
                    l->expr.semi = true;
            }
            /* Unmark: the produced statements get walked (and marked) now. */
            if (S.cleanup)
                fatal(m->span, "internal: expansion during cleanup");
            w_stmts(&produced);
            for (uint32_t k = 0; k < produced.len; k++)
                vec_push(out, produced.data[k]);
            site_leave(saved);
            break;
        }
        }
    }
    *sl = out;
}

/* ---- items ---- */

static bool has_test_attr(AttrList l) {
    return find_attr(l, "test") || find_attr(l, "bench");
}

static void w_vdata(VariantData *d) {
    uint32_t j = 0;
    for (uint32_t i = 0; i < d->fields.len; i++) {
        FieldDef *f = d->fields.data[i];
        if (!attrs_keep(&f->attrs))
            continue;
        mark(f->id, f->span);
        w_attrs(&f->attrs);
        if (f->vis.path)
            w_path(f->vis.path);
        w_ty(&f->ty);
        d->fields.data[j++] = f;
    }
    d->fields.len = j;
}

static const char *find_mod_file(const char *dir, const char *name, Span sp,
                                 const char **child_dir) {
    const char *a = arena_printf("%s/%s.rs", dir, name);
    const char *b = arena_printf("%s/%s/mod.rs", dir, name);
    FILE *f = fopen(a, "rb");
    if (f) {
        fclose(f);
        *child_dir = arena_printf("%s/%s", dir, name);
        return a;
    }
    f = fopen(b, "rb");
    if (f) {
        fclose(f);
        *child_dir = arena_printf("%s/%s", dir, name);
        return b;
    }
    fatal(sp, "file not found for module `%s` (looked for `%s` and `%s`)", name, a, b);
}

/* Replace `mod foo;` by the parsed file. Returns false if the file's inner
   #![cfg] removes it. */
static void load_mod(Item *it) {
    const char *file;
    const char *child_dir;
    Attr *pa = find_attr(it->attrs, "path");
    if (pa) {
        if (pa->args_kind != ATTR_ARGS_EQ || !pa->value || pa->value->kind != EXPR_LIT ||
            pa->value->lit->kind != LIT_STR)
            fatal(pa->span, "malformed #[path] attribute");
        const char *base = S.in_file_mod ? ex_dir_of_span(it->span) : S.mod_dir;
        file = path_join(base, pa->value->lit->str);
        child_dir = dirname_of(file);
    } else {
        file = find_mod_file(S.mod_dir, sym_str(it->name), it->span, &child_dir);
    }
    uint32_t fid = source_load(file);
    Crate *c = parse_crate(lex_file(fid), fid);
    it->mod.inline_ = true;
    it->mod.items = c->items;
    for (uint32_t i = 0; i < c->attrs.len; i++) {
        c->attrs.data[i]->inner = true;
        vec_push(it->attrs, c->attrs.data[i]);
    }
    ptrmap_put(&S.mod_dirs, it, (void *)child_dir);
    ptrmap_put(&S.mod_dirs, &it->mod, (void *)1); /* marks "file module" */
}

static void w_mod(Item *it) {
    if (!it->mod.inline_)
        load_mod(it);
    const char *dir = ptrmap_get(&S.mod_dirs, it);
    bool file_mod = ptrmap_get(&S.mod_dirs, &it->mod) != NULL;
    if (!dir) {
        dir = arena_printf("%s/%s", S.mod_dir, sym_str(it->name));
        ptrmap_put(&S.mod_dirs, it, (void *)dir);
    }
    const char *saved_dir = S.mod_dir;
    bool saved_file = S.in_file_mod;
    MScope *saved_scope = S.scope;
    S.mod_dir = dir;
    S.in_file_mod = file_mod;
    vec_push(S.modpath, sym_str(it->name));
    w_items(&it->mod.items, POS_ITEM);
    S.modpath.len--;
    S.mod_dir = saved_dir;
    S.in_file_mod = saved_file;
    if (!find_attr(it->attrs, "macro_use"))
        S.scope = saved_scope;
}

static void do_derives(Item *it, ItemList *out) {
    VEC(Sym) traits = {0};
    VEC(Span) spans = {0};
    for (uint32_t i = 0; i < it->attrs.len; i++) {
        Attr *a = it->attrs.data[i];
        if (!attr_is(a, "derive"))
            continue;
        if (a->args_kind != ATTR_ARGS_DELIMITED)
            fatal(a->span, "malformed #[derive]");
        const Token *t = a->tokens.toks;
        uint32_t n = a->tokens.len;
        for (uint32_t j = 0; j < n;) {
            /* path, take the last segment */
            Sym last = 0;
            while (j < n && t[j].kind != TK_COMMA) {
                if (t[j].kind == TK_IDENT)
                    last = t[j].v.sym;
                j++;
            }
            if (last) {
                vec_push(traits, last);
                vec_push(spans, a->span);
            }
            j++;
        }
    }
    remove_attrs(&it->attrs, "derive");
    if (!traits.len)
        return;
    bool has_copy = false;
    for (uint32_t i = 0; i < traits.len; i++)
        if (strcmp(sym_str(traits.data[i]), "Copy") == 0)
            has_copy = true;
    for (uint32_t i = 0; i < traits.len; i++) {
        if (S.opts && S.opts->compat && strcmp(sym_str(traits.data[i]), "PartialEq") == 0) {
            /* rustc only allows constants of a type in patterns if its
               PartialEq is the built-in derive (StructuralPartialEq, not
               nameable on stable), so compat output keeps the attribute. */
            Attr *a = NEW(Attr);
            a->id = ast_fresh_id();
            a->span = spans.data[i];
            a->path = ast_path_from_sym(sym_intern_cstr("derive"), a->span);
            a->args_kind = ATTR_ARGS_DELIMITED;
            a->delim = DELIM_PAREN;
            TokBuf tb;
            tb_init(&tb, a->span);
            tb_text(&tb, "PartialEq");
            a->tokens = tb_list(&tb);
            vec_push(it->attrs, a);
            continue;
        }
        ItemList gen = derive_generate(it, traits.data[i], spans.data[i], has_copy);
        for (uint32_t k = 0; k < gen.len; k++)
            vec_push(*out, gen.data[k]);
    }
}

static void strip_variant_helpers(Item *it) {
    if (it->kind != ITEM_ENUM)
        return;
    for (uint32_t i = 0; i < it->enum_.variants.len; i++)
        remove_attrs(&it->enum_.variants.data[i]->attrs, "default");
}

static void w_fn_sig(FnSig *sig) {
    if (sig->self_param) {
        SelfParam *sp = sig->self_param;
        if (!attrs_keep(&sp->attrs))
            sig->self_param = NULL;
        else {
            mark(sp->id, sp->span);
            w_attrs(&sp->attrs);
            if (sp->ty)
                w_ty(&sp->ty);
        }
    }
    w_params(&sig->params);
    if (sig->ret)
        w_ty(&sig->ret);
}

static void w_use_tree(UseTree *u) {
    mark(u->id, u->span);
    w_path(u->prefix);
    for (uint32_t i = 0; i < u->items.len; i++)
        w_use_tree(u->items.data[i]);
}

/* Walk one item; append the resulting items (0, 1 or more) to `out`. */
static void w_item(Item *it, ItemList *out, MacPos pos) {
    if (!attrs_keep(&it->attrs))
        return;
    if (has_test_attr(it->attrs))
        return;
    switch (it->kind) {
    case ITEM_MACRO_RULES:
        if (S.cleanup)
            return; /* dropped from the final tree */
        define_macro(it);
        vec_push(*out, it);
        return;
    case ITEM_MAC: {
        MacCall *m = it->mac;
        if (ex_is_frozen(m)) {
            check_mac_left(m);
            mark(it->id, it->span);
            w_attrs(&it->attrs);
            vec_push(*out, it);
            return;
        }
        if (S.cleanup)
            check_mac_left(m);
        Span saved = site_enter(m);
        MacOut o = invoke(m, pos);
        ItemList produced = {0};
        switch (o.kind) {
        case MO_DEFER:
            S.unresolved++;
            vec_push(*out, it);
            site_leave(saved);
            return;
        case MO_TOKENS:
            produced = pos == POS_ASSOC ? parse_assoc_items_from_tokens(o.toks, m->span)
                                        : parse_items_from_tokens(o.toks, m->span);
            break;
        case MO_ITEMS: produced = o.items; break;
        case MO_NONE: break;
        default: fatal(m->span, "macro does not expand to items");
        }
        w_items(&produced, pos);
        for (uint32_t k = 0; k < produced.len; k++)
            vec_push(*out, produced.data[k]);
        site_leave(saved);
        return;
    }
    case ITEM_MOD:
        w_mod(it);
        break;
    case ITEM_USE: w_use_tree(it->use_tree); break;
    case ITEM_EXTERN_CRATE: break;
    case ITEM_FN:
        w_generics(&it->fn_.generics);
        w_fn_sig(&it->fn_.sig);
        if (it->fn_.body)
            w_block(it->fn_.body);
        break;
    case ITEM_STRUCT:
    case ITEM_UNION:
        w_generics(&it->adt.generics);
        w_vdata(&it->adt.data);
        break;
    case ITEM_ENUM: {
        w_generics(&it->enum_.generics);
        VariantList *vl = &it->enum_.variants;
        uint32_t j = 0;
        for (uint32_t i = 0; i < vl->len; i++) {
            Variant *v = vl->data[i];
            if (!attrs_keep(&v->attrs))
                continue;
            mark(v->id, v->span);
            w_attrs(&v->attrs);
            w_vdata(&v->data);
            if (v->disc)
                w_expr(&v->disc);
            vl->data[j++] = v;
        }
        vl->len = j;
        break;
    }
    case ITEM_TRAIT:
        w_generics(&it->trait.generics);
        w_bounds(&it->trait.supertraits);
        w_items(&it->trait.items, POS_ASSOC);
        break;
    case ITEM_IMPL:
        w_generics(&it->impl.generics);
        if (it->impl.trait_path)
            w_path(it->impl.trait_path);
        w_ty(&it->impl.self_ty);
        w_items(&it->impl.items, POS_ASSOC);
        break;
    case ITEM_TYPE_ALIAS:
        w_generics(&it->type_alias.generics);
        w_bounds(&it->type_alias.bounds);
        if (it->type_alias.ty)
            w_ty(&it->type_alias.ty);
        break;
    case ITEM_CONST:
        w_generics(&it->const_.generics);
        if (it->const_.ty)
            w_ty(&it->const_.ty);
        if (it->const_.expr)
            w_expr(&it->const_.expr);
        break;
    case ITEM_STATIC:
        w_ty(&it->static_.ty);
        if (it->static_.expr)
            w_expr(&it->static_.expr);
        break;
    case ITEM_FOREIGN_MOD:
        w_items(&it->foreign_mod.items, POS_FOREIGN);
        break;
    }
    if (it->vis.path)
        w_path(it->vis.path);
    if (S.cleanup) {
        remove_attrs(&it->attrs, "macro_use");
        remove_attrs(&it->attrs, "path");
        strip_variant_helpers(it);
    }
    mark(it->id, it->span);
    w_attrs(&it->attrs);
    vec_push(*out, it);
    if (it->kind == ITEM_STRUCT || it->kind == ITEM_ENUM || it->kind == ITEM_UNION) {
        ItemList gen = {0};
        do_derives(it, &gen);
        strip_variant_helpers(it);
        if (gen.len) {
            w_items(&gen, POS_ITEM);
            for (uint32_t k = 0; k < gen.len; k++)
                vec_push(*out, gen.data[k]);
        }
    }
}

static void w_items(ItemList *items, MacPos pos) {
    ItemList out = {0};
    for (uint32_t i = 0; i < items->len; i++)
        w_item(items->data[i], &out, pos);
    *items = out;
}

/* ------------------------------------------------------------------ */
/* Cleanup: imports of macros                                           */
/* ------------------------------------------------------------------ */

static bool root_has_item(Sym name) {
    for (uint32_t i = 0; i < S.crate->items.len; i++) {
        Item *it = S.crate->items.data[i];
        if (it->name == name && it->kind != ITEM_MACRO_RULES)
            return true;
    }
    return false;
}

/* Is this use leaf `crate::NAME` (or `$crate::NAME`) naming an exported
   macro? Then it imports a macro that no longer exists. */
static bool is_macro_import(const Path *full) {
    if (full->segs.len != 2 || full->segs.data[0]->name != KW_CRATE)
        return false;
    Sym name = full->segs.data[1]->name;
    return lookup_exported(name) && !root_has_item(name);
}

static bool prune_use(UseTree *u, Path *prefix) {
    Path *full = ast_new_path(u->span);
    for (uint32_t i = 0; prefix && i < prefix->segs.len; i++)
        vec_push(full->segs, prefix->segs.data[i]);
    for (uint32_t i = 0; i < u->prefix->segs.len; i++)
        vec_push(full->segs, u->prefix->segs.data[i]);
    if (u->kind == USE_SIMPLE)
        return !is_macro_import(full);
    if (u->kind == USE_GLOB)
        return true;
    uint32_t j = 0;
    for (uint32_t i = 0; i < u->items.len; i++)
        if (prune_use(u->items.data[i], full))
            u->items.data[j++] = u->items.data[i];
    u->items.len = j;
    return j > 0;
}

static void prune_macro_imports(ItemList *items) {
    uint32_t j = 0;
    for (uint32_t i = 0; i < items->len; i++) {
        Item *it = items->data[i];
        if (it->kind == ITEM_USE && !prune_use(it->use_tree, NULL))
            continue;
        if (it->kind == ITEM_MOD)
            prune_macro_imports(&it->mod.items);
        items->data[j++] = it;
    }
    items->len = j;
}

/* ------------------------------------------------------------------ */
/* Entry point                                                         */
/* ------------------------------------------------------------------ */

Crate *expand_crate(const char *root_path, const char *crate_name,
                    const ExpandOptions *opts) {
    memset(&S, 0, sizeof S);
    lex_init();
    S.opts = opts;
    S.fmt_root = opts && opts->fmt_root ? opts->fmt_root : "::std";
    S.crate_name = crate_name;
    cfg_init(opts);
    uint32_t fid = source_load(root_path);
    Crate *c = parse_crate(lex_file(fid), fid);
    S.crate = c;
    if (!attrs_keep(&c->attrs))
        return c; /* #![cfg(false)] on the crate: empty crate */

    uint32_t prev_unres = UINT32_MAX, prev_exported = 0;
    for (int pass = 0;; pass++) {
        S.unresolved = 0;
        S.expanded = 0;
        S.scope = NULL;
        S.mod_dir = dirname_of(root_path);
        S.in_file_mod = true;
        S.modpath.len = 0;
        w_items(&c->items, POS_ITEM);
        if (S.unresolved == 0)
            break;
        if (S.final_pass)
            fatal(c->span, "internal: unresolved macros after final pass");
        if ((S.unresolved >= prev_unres && S.exported.len == prev_exported) || pass > 50)
            S.final_pass = true;
        prev_unres = S.unresolved;
        prev_exported = S.exported.len;
    }
    prune_macro_imports(&c->items);
    /* Cleanup and verification pass. */
    S.cleanup = true;
    S.max_id = ast_fresh_id() + 1;
    S.seen_ids = NEW_ARRAY(uint8_t, S.max_id / 8 + 1);
    S.scope = NULL;
    S.mod_dir = dirname_of(root_path);
    S.in_file_mod = true;
    mark(c->id, c->span);
    w_attrs(&c->attrs);
    w_items(&c->items, POS_ITEM);
    S.cleanup = false;
    return c;
}
