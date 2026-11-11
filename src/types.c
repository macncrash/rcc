/* types.c - HMap, StrBuf, the Def table, and interned types. See types.h. */
#include "types.h"

#include <stdio.h>

/* ------------------------------------------------------------------ */
/* HMap                                                                */
/* ------------------------------------------------------------------ */

static uint64_t mix64(uint64_t a, uint64_t b) {
    uint64_t h = a * 0x9E3779B97F4A7C15ull;
    h ^= h >> 29;
    h += b * 0xC2B2AE3D27D4EB4Full;
    h ^= h >> 32;
    h *= 0xD6E8FEB86659FD93ull;
    h ^= h >> 31;
    return h;
}

void *hmap_get2(const HMap *m, uint64_t a, uint64_t b) {
    if (m->cap == 0)
        return NULL;
    uint32_t mask = m->cap - 1;
    uint32_t i = (uint32_t)mix64(a, b) & mask;
    for (;;) {
        const HMapEntry *e = &m->ents[i];
        if (e->a == 0)
            return NULL;
        if (e->a == a && e->b == b)
            return e->val;
        i = (i + 1) & mask;
    }
}

static void hmap_insert_raw(HMap *m, uint64_t a, uint64_t b, void *val) {
    uint32_t mask = m->cap - 1;
    uint32_t i = (uint32_t)mix64(a, b) & mask;
    for (;;) {
        HMapEntry *e = &m->ents[i];
        if (e->a == 0) {
            e->a = a;
            e->b = b;
            e->val = val;
            m->len++;
            return;
        }
        if (e->a == a && e->b == b) {
            e->val = val;
            return;
        }
        i = (i + 1) & mask;
    }
}

void hmap_put2(HMap *m, uint64_t a, uint64_t b, void *val) {
    if (a == 0)
        fatal((Span){0}, "internal: hmap key 0");
    if ((m->len + 1) * 4 > m->cap * 3) {
        HMap n = {0};
        n.cap = m->cap ? m->cap * 2 : 16;
        n.ents = NEW_ARRAY(HMapEntry, n.cap);
        for (uint32_t i = 0; i < m->cap; i++)
            if (m->ents[i].a)
                hmap_insert_raw(&n, m->ents[i].a, m->ents[i].b, m->ents[i].val);
        *m = n;
    }
    hmap_insert_raw(m, a, b, val);
}

/* ------------------------------------------------------------------ */
/* StrBuf                                                              */
/* ------------------------------------------------------------------ */

static void sb_reserve(StrBuf *sb, uint32_t extra) {
    if (sb->len + extra + 1 <= sb->cap)
        return;
    uint32_t nc = sb->cap ? sb->cap * 2 : 64;
    while (nc < sb->len + extra + 1)
        nc *= 2;
    char *n = arena_alloc(nc);
    if (sb->len)
        memcpy(n, sb->data, sb->len);
    sb->data = n;
    sb->cap = nc;
}

void sb_putc(StrBuf *sb, char c) {
    sb_reserve(sb, 1);
    sb->data[sb->len++] = c;
}

void sb_puts(StrBuf *sb, const char *s) {
    uint32_t n = (uint32_t)strlen(s);
    sb_reserve(sb, n);
    memcpy(sb->data + sb->len, s, n);
    sb->len += n;
}

void sb_printf(StrBuf *sb, const char *fmt, ...) {
    va_list ap;
    char buf[512];
    va_start(ap, fmt);
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    if (n < 0)
        return;
    if ((size_t)n < sizeof buf) {
        sb_puts(sb, buf);
        return;
    }
    sb_reserve(sb, (uint32_t)n);
    va_start(ap, fmt);
    vsnprintf(sb->data + sb->len, (size_t)n + 1, fmt, ap);
    va_end(ap);
    sb->len += (uint32_t)n;
}

const char *sb_str(StrBuf *sb) {
    sb_reserve(sb, 0);
    sb->data[sb->len] = 0;
    return sb->data;
}

/* ------------------------------------------------------------------ */
/* Defs                                                                */
/* ------------------------------------------------------------------ */

static VEC(Def *) defs;

Def *def_get(DefId id) {
    if (id == 0 || id > defs.len)
        fatal((Span){0}, "internal: bad DefId %u", id);
    return defs.data[id - 1];
}

uint32_t def_count(void) { return defs.len; }

DefId def_new(DefKind kind, Sym name, DefId parent, Span sp) {
    Def *d = NEW(Def);
    d->kind = kind;
    d->name = name;
    d->parent = parent;
    d->span = sp;
    if (parent) {
        Def *p = def_get(parent);
        d->crate_ = p->crate_;
        d->module = p->module;
    }
    vec_push(defs, d);
    return defs.len;
}

const char *def_kind_str(DefKind k) {
    switch (k) {
    case DEF_NONE: return "none";
    case DEF_MOD: return "mod";
    case DEF_STRUCT: return "struct";
    case DEF_UNION: return "union";
    case DEF_ENUM: return "enum";
    case DEF_VARIANT: return "variant";
    case DEF_TRAIT: return "trait";
    case DEF_TYPE_ALIAS: return "type";
    case DEF_ASSOC_TY: return "assoc type";
    case DEF_FN: return "fn";
    case DEF_CONST: return "const";
    case DEF_ASSOC_CONST: return "assoc const";
    case DEF_STATIC: return "static";
    case DEF_IMPL: return "impl";
    case DEF_TY_PARAM: return "type param";
    case DEF_CONST_PARAM: return "const param";
    case DEF_CLOSURE: return "closure";
    }
    return "?";
}

static void def_path_rec(StrBuf *sb, DefId id) {
    Def *d = def_get(id);
    if (d->parent) {
        def_path_rec(sb, d->parent);
        sb_puts(sb, "::");
    }
    if (d->kind == DEF_IMPL)
        sb_printf(sb, "{impl#%u}", id);
    else if (d->kind == DEF_CLOSURE)
        sb_printf(sb, "{closure#%u}", id);
    else if (d->name == 0)
        sb_printf(sb, "{block#%u}", id);
    else
        sb_puts(sb, sym_str(d->name));
}

const char *def_path_str(DefId id) {
    if (!id)
        return "<none>";
    StrBuf sb = {0};
    def_path_rec(&sb, id);
    return sb_str(&sb);
}

static const Attr *attr_find(const AttrList *attrs, const char *name) {
    for (uint32_t i = 0; i < attrs->len; i++) {
        const Attr *a = attrs->data[i];
        if (a->path && a->path->segs.len == 1 &&
            strcmp(sym_str(a->path->segs.data[0]->name), name) == 0)
            return a;
    }
    return NULL;
}

bool attrs_have(const AttrList *attrs, const char *name) {
    return attr_find(attrs, name) != NULL;
}

const char *attrs_str_value(const AttrList *attrs, const char *name) {
    const Attr *a = attr_find(attrs, name);
    if (!a || a->args_kind != ATTR_ARGS_EQ || !a->value ||
        a->value->kind != EXPR_LIT || a->value->lit->kind != LIT_STR)
        return NULL;
    return a->value->lit->str;
}

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

uint32_t ik_bits(IntKind k) {
    switch (k) {
    case IK_I8: case IK_U8: return 8;
    case IK_I16: case IK_U16: return 16;
    case IK_I32: case IK_U32: return 32;
    case IK_I128: case IK_U128: return 128;
    default: return 64;
    }
}

const char *ik_name(IntKind k) {
    static const char *names[] = {"i8", "i16", "i32", "i64", "i128", "isize",
                                  "u8", "u16", "u32", "u64", "u128", "usize"};
    return names[k];
}

static uint32_t next_type_id = 1;
static uint32_t next_substs_id = 1;

/* Interning tables: buckets chained through a side array. */
typedef struct TyBucket {
    Type *ty;
    struct TyBucket *next;
} TyBucket;
typedef struct SubBucket {
    Substs *s;
    struct SubBucket *next;
} SubBucket;

static TyBucket **ty_tab;
static uint32_t ty_tab_cap, ty_tab_len;
static SubBucket **sub_tab;
static uint32_t sub_tab_cap, sub_tab_len;

static uint64_t substs_hash(Type **t, uint32_t n) {
    uint64_t h = 0x1234567 + n;
    for (uint32_t i = 0; i < n; i++)
        h = mix64(h, t[i]->id);
    return h;
}

Substs *substs_new(Type **t, uint32_t n) {
    if (sub_tab_len * 2 >= sub_tab_cap) {
        uint32_t nc = sub_tab_cap ? sub_tab_cap * 2 : 1024;
        SubBucket **nt = NEW_ARRAY(SubBucket *, nc);
        for (uint32_t i = 0; i < sub_tab_cap; i++) {
            SubBucket *b = sub_tab[i];
            while (b) {
                SubBucket *nx = b->next;
                uint32_t h = (uint32_t)substs_hash(b->s->t, b->s->len) & (nc - 1);
                b->next = nt[h];
                nt[h] = b;
                b = nx;
            }
        }
        sub_tab = nt;
        sub_tab_cap = nc;
    }
    uint32_t h = (uint32_t)substs_hash(t, n) & (sub_tab_cap - 1);
    for (SubBucket *b = sub_tab[h]; b; b = b->next) {
        if (b->s->len != n)
            continue;
        bool eq = true;
        for (uint32_t i = 0; i < n && eq; i++)
            eq = b->s->t[i] == t[i];
        if (eq)
            return b->s;
    }
    Substs *s = arena_alloc(sizeof(Substs) + sizeof(Type *) * n);
    s->len = n;
    s->id = next_substs_id++;
    for (uint32_t i = 0; i < n; i++) {
        s->t[i] = t[i];
        s->flags |= t[i]->flags;
    }
    SubBucket *b = NEW(SubBucket);
    b->s = s;
    b->next = sub_tab[h];
    sub_tab[h] = b;
    sub_tab_len++;
    return s;
}

Substs *substs_empty(void) { return substs_new(NULL, 0); }
Substs *substs_1(Type *a) { return substs_new(&a, 1); }
Substs *substs_2(Type *a, Type *b) {
    Type *t[2] = {a, b};
    return substs_new(t, 2);
}

Substs *substs_concat(Substs *a, Substs *b) {
    if (b->len == 0)
        return a;
    if (a->len == 0)
        return b;
    Type **t = NEW_ARRAY(Type *, a->len + b->len);
    memcpy(t, a->t, sizeof(Type *) * a->len);
    memcpy(t + a->len, b->t, sizeof(Type *) * b->len);
    return substs_new(t, a->len + b->len);
}

Substs *substs_slice(Substs *s, uint32_t from, uint32_t to) {
    if (from == 0 && to == s->len)
        return s;
    return substs_new(s->t + from, to - from);
}

static uint64_t type_hash(const Type *t) {
    uint64_t h = mix64(t->kind + 1, ((uint64_t)t->ik << 8) | t->is_mut);
    h = mix64(h, ((uint64_t)t->def << 32) | t->index);
    h = mix64(h, t->name);
    h = mix64(h, t->elem ? t->elem->id : 0);
    h = mix64(h, t->len ? t->len->id : 0);
    h = mix64(h, t->substs ? t->substs->id : 0);
    h = mix64(h, t->cval);
    for (uint32_t i = 0; i < t->nbinds; i++)
        h = mix64(h, ((uint64_t)t->binds[i].assoc << 32) | t->binds[i].ty->id);
    return h;
}

static bool type_eq(const Type *a, const Type *b) {
    if (a->kind != b->kind || a->ik != b->ik || a->is_mut != b->is_mut ||
        a->def != b->def || a->index != b->index || a->name != b->name ||
        a->elem != b->elem || a->len != b->len || a->substs != b->substs ||
        a->cval != b->cval || a->nbinds != b->nbinds)
        return false;
    for (uint32_t i = 0; i < a->nbinds; i++)
        if (a->binds[i].assoc != b->binds[i].assoc || a->binds[i].ty != b->binds[i].ty)
            return false;
    return true;
}

/* Intern `proto` (a stack-allocated template). */
static Type *intern(const Type *proto) {
    if (ty_tab_len * 2 >= ty_tab_cap) {
        uint32_t nc = ty_tab_cap ? ty_tab_cap * 2 : 1024;
        TyBucket **nt = NEW_ARRAY(TyBucket *, nc);
        for (uint32_t i = 0; i < ty_tab_cap; i++) {
            TyBucket *b = ty_tab[i];
            while (b) {
                TyBucket *nx = b->next;
                uint32_t h = (uint32_t)type_hash(b->ty) & (nc - 1);
                b->next = nt[h];
                nt[h] = b;
                b = nx;
            }
        }
        ty_tab = nt;
        ty_tab_cap = nc;
    }
    uint32_t h = (uint32_t)type_hash(proto) & (ty_tab_cap - 1);
    for (TyBucket *b = ty_tab[h]; b; b = b->next)
        if (type_eq(b->ty, proto))
            return b->ty;
    Type *t = NEW(Type);
    *t = *proto;
    t->id = next_type_id++;
    uint8_t fl = 0;
    if (t->kind == T_PARAM)
        fl |= TYF_HAS_PARAM;
    if (t->kind == T_PROJ || t->kind == T_OPAQUE)
        fl |= TYF_HAS_PROJ;
    if (t->elem)
        fl |= t->elem->flags;
    if (t->len)
        fl |= t->len->flags;
    if (t->substs)
        fl |= t->substs->flags;
    if (t->nbinds) {
        DynBinding *nb = NEW_ARRAY(DynBinding, t->nbinds);
        memcpy(nb, proto->binds, sizeof(DynBinding) * t->nbinds);
        t->binds = nb;
        for (uint32_t i = 0; i < t->nbinds; i++)
            fl |= nb[i].ty->flags;
    }
    t->flags = fl;
    TyBucket *b = NEW(TyBucket);
    b->ty = t;
    b->next = ty_tab[h];
    ty_tab[h] = b;
    ty_tab_len++;
    return t;
}

static Type *prim_bool, *prim_char, *prim_str, *prim_never, *prim_unit, *prim_error;
static Type *prim_int[IK_COUNT], *prim_float[2];

static Type *mk_simple(TypeKind k, uint8_t ik) {
    Type p = {0};
    p.kind = k;
    p.ik = ik;
    return intern(&p);
}

void types_init(void) {
    if (prim_bool)
        return;
    prim_bool = mk_simple(T_BOOL, 0);
    prim_char = mk_simple(T_CHAR, 0);
    prim_str = mk_simple(T_STR, 0);
    prim_never = mk_simple(T_NEVER, 0);
    prim_error = mk_simple(T_ERROR, 0);
    for (int i = 0; i < IK_COUNT; i++)
        prim_int[i] = mk_simple(T_INT, (uint8_t)i);
    prim_float[0] = mk_simple(T_FLOAT, FK_F32);
    prim_float[1] = mk_simple(T_FLOAT, FK_F64);
    prim_unit = ty_tuple(substs_empty());
}

Type *ty_bool(void) { return prim_bool; }
Type *ty_char(void) { return prim_char; }
Type *ty_str(void) { return prim_str; }
Type *ty_never(void) { return prim_never; }
Type *ty_unit(void) { return prim_unit; }
Type *ty_error(void) { return prim_error; }
Type *ty_int(IntKind k) { return prim_int[k]; }
Type *ty_float(FloatKind k) { return prim_float[k]; }

Type *ty_tuple(Substs *elems) {
    Type p = {0};
    p.kind = T_TUPLE;
    p.substs = elems;
    return intern(&p);
}
Type *ty_tuple2(Type *a, Type *b) { return ty_tuple(substs_2(a, b)); }
Type *ty_adt(DefId def, Substs *substs) {
    Type p = {0};
    p.kind = T_ADT;
    p.def = def;
    p.substs = substs;
    return intern(&p);
}
Type *ty_ref(Type *elem, bool is_mut) {
    Type p = {0};
    p.kind = T_REF;
    p.elem = elem;
    p.is_mut = is_mut;
    return intern(&p);
}
Type *ty_ptr(Type *elem, bool is_mut) {
    Type p = {0};
    p.kind = T_PTR;
    p.elem = elem;
    p.is_mut = is_mut;
    return intern(&p);
}
Type *ty_slice(Type *elem) {
    Type p = {0};
    p.kind = T_SLICE;
    p.elem = elem;
    return intern(&p);
}
Type *ty_array(Type *elem, Type *len) {
    Type p = {0};
    p.kind = T_ARRAY;
    p.elem = elem;
    p.len = len;
    return intern(&p);
}
Type *ty_fnptr(Substs *params_and_ret) {
    Type p = {0};
    p.kind = T_FNPTR;
    p.substs = params_and_ret;
    return intern(&p);
}
Type *ty_fndef(DefId def, Substs *substs) {
    Type p = {0};
    p.kind = T_FNDEF;
    p.def = def;
    p.substs = substs;
    return intern(&p);
}
Type *ty_closure(DefId def, Substs *substs) {
    Type p = {0};
    p.kind = T_CLOSURE;
    p.def = def;
    p.substs = substs;
    return intern(&p);
}
Type *ty_dyn(DefId trait, Substs *args, DynBinding *binds, uint32_t nbinds) {
    Type p = {0};
    p.kind = T_DYN;
    p.def = trait;
    p.substs = args;
    p.binds = binds;
    p.nbinds = nbinds;
    return intern(&p);
}
Type *ty_param(uint32_t index, Sym name) {
    Type p = {0};
    p.kind = T_PARAM;
    p.index = index;
    p.name = name;
    return intern(&p);
}
Type *ty_proj(DefId assoc, Substs *trait_substs) {
    Type p = {0};
    p.kind = T_PROJ;
    p.def = assoc;
    p.substs = trait_substs;
    return intern(&p);
}
Type *ty_opaque(DefId fn, uint32_t ordinal, Substs *substs) {
    Type p = {0};
    p.kind = T_OPAQUE;
    p.def = fn;
    p.index = ordinal;
    p.substs = substs;
    return intern(&p);
}
Type *ty_const(uint64_t v) {
    Type p = {0};
    p.kind = T_CONST;
    p.cval = v;
    return intern(&p);
}
Type *ty_infer_node(uint32_t var, InferKind k) {
    Type *t = NEW(Type);
    t->kind = T_INFER;
    t->ik = (uint8_t)k;
    t->index = var;
    t->flags = TYF_HAS_INFER;
    t->id = next_type_id++;
    return t;
}

/* ------------------------------------------------------------------ */
/* Folding                                                             */
/* ------------------------------------------------------------------ */

Type *ty_fold(TypeFolder *f, Type *t) {
    if (!(t->flags & f->mask))
        return t;
    return f->fold(f, t);
}

Substs *substs_fold(TypeFolder *f, Substs *s) {
    if (!(s->flags & f->mask))
        return s;
    Type *small[8];
    Type **n = s->len <= 8 ? small : NEW_ARRAY(Type *, s->len);
    bool changed = false;
    for (uint32_t i = 0; i < s->len; i++) {
        n[i] = ty_fold(f, s->t[i]);
        changed |= n[i] != s->t[i];
    }
    return changed ? substs_new(n, s->len) : s;
}

Type *ty_super_fold(TypeFolder *f, Type *t) {
    switch ((TypeKind)t->kind) {
    case T_BOOL: case T_CHAR: case T_INT: case T_FLOAT: case T_STR:
    case T_NEVER: case T_PARAM: case T_INFER: case T_CONST: case T_ERROR:
        return t;
    case T_TUPLE: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_tuple(s);
    }
    case T_ADT: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_adt(t->def, s);
    }
    case T_REF: {
        Type *e = ty_fold(f, t->elem);
        return e == t->elem ? t : ty_ref(e, t->is_mut);
    }
    case T_PTR: {
        Type *e = ty_fold(f, t->elem);
        return e == t->elem ? t : ty_ptr(e, t->is_mut);
    }
    case T_SLICE: {
        Type *e = ty_fold(f, t->elem);
        return e == t->elem ? t : ty_slice(e);
    }
    case T_ARRAY: {
        Type *e = ty_fold(f, t->elem);
        Type *l = ty_fold(f, t->len);
        return e == t->elem && l == t->len ? t : ty_array(e, l);
    }
    case T_FNPTR: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_fnptr(s);
    }
    case T_FNDEF: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_fndef(t->def, s);
    }
    case T_CLOSURE: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_closure(t->def, s);
    }
    case T_PROJ: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_proj(t->def, s);
    }
    case T_OPAQUE: {
        Substs *s = substs_fold(f, t->substs);
        return s == t->substs ? t : ty_opaque(t->def, t->index, s);
    }
    case T_DYN: {
        Substs *s = substs_fold(f, t->substs);
        DynBinding *nb = NULL;
        bool changed = s != t->substs;
        if (t->nbinds) {
            nb = NEW_ARRAY(DynBinding, t->nbinds);
            for (uint32_t i = 0; i < t->nbinds; i++) {
                nb[i].assoc = t->binds[i].assoc;
                nb[i].ty = ty_fold(f, t->binds[i].ty);
                changed |= nb[i].ty != t->binds[i].ty;
            }
        }
        return changed ? ty_dyn(t->def, s, nb, t->nbinds) : t;
    }
    }
    return t;
}

static Type *subst_fold(TypeFolder *f, Type *t) {
    if (t->kind == T_PARAM) {
        if (t->index >= f->substs->len)
            fatal((Span){0}, "internal: type parameter %s (#%u) out of range for substs %s",
                  sym_str(t->name), t->index, substs_to_str(f->substs));
        return f->substs->t[t->index];
    }
    return ty_super_fold(f, t);
}

Type *ty_subst(Type *t, Substs *s) {
    if (!(t->flags & TYF_HAS_PARAM))
        return t;
    TypeFolder f = {subst_fold, TYF_HAS_PARAM, NULL, s};
    return ty_fold(&f, t);
}

Substs *substs_subst(Substs *x, Substs *s) {
    if (!(x->flags & TYF_HAS_PARAM))
        return x;
    TypeFolder f = {subst_fold, TYF_HAS_PARAM, NULL, s};
    return substs_fold(&f, x);
}

/* ------------------------------------------------------------------ */
/* Printing                                                            */
/* ------------------------------------------------------------------ */

static void print_substs(StrBuf *sb, Substs *s, uint32_t from) {
    if (!s || s->len <= from)
        return;
    sb_putc(sb, '<');
    for (uint32_t i = from; i < s->len; i++) {
        if (i > from)
            sb_puts(sb, ", ");
        ty_print(sb, s->t[i]);
    }
    sb_putc(sb, '>');
}

void ty_print(StrBuf *sb, Type *t) {
    if (!t) {
        sb_puts(sb, "<null>");
        return;
    }
    switch ((TypeKind)t->kind) {
    case T_BOOL: sb_puts(sb, "bool"); break;
    case T_CHAR: sb_puts(sb, "char"); break;
    case T_INT: sb_puts(sb, ik_name((IntKind)t->ik)); break;
    case T_FLOAT: sb_puts(sb, t->ik == FK_F32 ? "f32" : "f64"); break;
    case T_STR: sb_puts(sb, "str"); break;
    case T_NEVER: sb_puts(sb, "!"); break;
    case T_ERROR: sb_puts(sb, "{error}"); break;
    case T_TUPLE:
        sb_putc(sb, '(');
        for (uint32_t i = 0; i < t->substs->len; i++) {
            if (i)
                sb_puts(sb, ", ");
            ty_print(sb, t->substs->t[i]);
        }
        if (t->substs->len == 1)
            sb_putc(sb, ',');
        sb_putc(sb, ')');
        break;
    case T_ADT:
        sb_puts(sb, sym_str(def_get(t->def)->name));
        print_substs(sb, t->substs, 0);
        break;
    case T_REF:
        sb_puts(sb, t->is_mut ? "&mut " : "&");
        ty_print(sb, t->elem);
        break;
    case T_PTR:
        sb_puts(sb, t->is_mut ? "*mut " : "*const ");
        ty_print(sb, t->elem);
        break;
    case T_SLICE:
        sb_putc(sb, '[');
        ty_print(sb, t->elem);
        sb_putc(sb, ']');
        break;
    case T_ARRAY:
        sb_putc(sb, '[');
        ty_print(sb, t->elem);
        sb_puts(sb, "; ");
        ty_print(sb, t->len);
        sb_putc(sb, ']');
        break;
    case T_FNPTR:
        sb_puts(sb, "fn(");
        for (uint32_t i = 0; i + 1 < t->substs->len; i++) {
            if (i)
                sb_puts(sb, ", ");
            ty_print(sb, t->substs->t[i]);
        }
        sb_puts(sb, ")");
        if (!ty_is_unit(fnptr_ret(t))) {
            sb_puts(sb, " -> ");
            ty_print(sb, fnptr_ret(t));
        }
        break;
    case T_FNDEF:
        sb_puts(sb, "fn{");
        sb_puts(sb, def_path_str(t->def));
        print_substs(sb, t->substs, 0);
        sb_putc(sb, '}');
        break;
    case T_CLOSURE:
        sb_printf(sb, "{closure#%u", t->def);
        print_substs(sb, t->substs, 0);
        sb_putc(sb, '}');
        break;
    case T_DYN:
        sb_puts(sb, "dyn ");
        sb_puts(sb, sym_str(def_get(t->def)->name));
        if (t->substs->len || t->nbinds) {
            sb_putc(sb, '<');
            for (uint32_t i = 0; i < t->substs->len; i++) {
                if (i)
                    sb_puts(sb, ", ");
                ty_print(sb, t->substs->t[i]);
            }
            for (uint32_t i = 0; i < t->nbinds; i++) {
                if (i || t->substs->len)
                    sb_puts(sb, ", ");
                sb_puts(sb, sym_str(def_get(t->binds[i].assoc)->name));
                sb_puts(sb, " = ");
                ty_print(sb, t->binds[i].ty);
            }
            sb_putc(sb, '>');
        }
        break;
    case T_PARAM: sb_puts(sb, sym_str(t->name)); break;
    case T_INFER:
        sb_printf(sb, t->ik == INFER_INT ? "?%ui" : t->ik == INFER_FLOAT ? "?%uf" : "?%u",
                  t->index);
        break;
    case T_PROJ: {
        Def *a = def_get(t->def);
        sb_putc(sb, '<');
        ty_print(sb, t->substs->t[0]);
        sb_puts(sb, " as ");
        sb_puts(sb, sym_str(def_get(a->parent)->name));
        print_substs(sb, t->substs, 1);
        sb_puts(sb, ">::");
        sb_puts(sb, sym_str(a->name));
        break;
    }
    case T_OPAQUE:
        sb_printf(sb, "impl#%u@%s", t->index, def_path_str(t->def));
        print_substs(sb, t->substs, 0);
        break;
    case T_CONST: sb_printf(sb, "%llu", (unsigned long long)t->cval); break;
    }
}

const char *ty_to_str(Type *t) {
    StrBuf sb = {0};
    ty_print(&sb, t);
    return sb_str(&sb);
}

const char *substs_to_str(Substs *s) {
    StrBuf sb = {0};
    print_substs(&sb, s, 0);
    return sb_str(&sb);
}
