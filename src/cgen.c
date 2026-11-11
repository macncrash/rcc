/*
 * cgen.c - emit portable C11 from MIR.
 *
 * Output layout (one translation unit for now; every section is produced
 * independently so the bodies can later be split across several files that
 * share the type/prototype prelude):
 *
 *     #include "rcc_rt.h"
 *     forward typedefs of all struct types, slice structs, fn pointer types
 *     struct definitions (dependency order)
 *     prototypes of all instances and drop glue functions
 *     globals, vtables
 *     drop glue bodies
 *     instance bodies (one C function per instance, a label per block)
 *     main
 *
 * Type mapping
 *     bool/ints/floats        bool, u8..i128, usize, isize, f32, f64
 *     char                    u32
 *     (), !, fn items         Unit
 *     tuple / struct / closure  struct { T f0; T f1; ... }
 *     enum                    struct { TAG tag; union { struct {..} v0; .. } u; }
 *     union                   struct { union { T f0; ... }; }
 *     [T; N]                  struct { T a[N]; }
 *     &T, *T                  T*
 *     &[T], &str, *[T]        struct { T *ptr; usize len; }   (str = [u8])
 *     &dyn Tr, *dyn Tr        Dyn { void *ptr; const void *const *vt; }
 *     Box<T>                  its struct: field f0 is the (thin or fat) pointer
 *     fn(A) -> R              typedef R (*FnN)(A)
 *
 * Function bodies are written to a temporary file first because emitting
 * them discovers the types that must be declared before them.
 */
#include "cgen.h"

#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct CTypeInfo {
    Type *ty;
    const char *name;
    uint8_t state; /* 0 declared, 1 being defined, 2 defined */
    bool is_struct, is_slice, is_fnptr;
} CTypeInfo;

typedef VEC(CTypeInfo *) CTypeList;

static Program *prog;
static FILE *out;      /* current body output */
static HMap ctype_tab; /* type id -> CTypeInfo* */
static CTypeList ctypes;
static StrBuf type_defs;
static const MirBody *cur_mir;
static const char *linkage = "static "; /* "" when split into several units */

static const char *ctype(Type *t);

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

static void sanitize(StrBuf *sb, const char *s, uint32_t max) {
    uint32_t n = 0;
    bool last_us = true;
    for (; *s && n < max; s++) {
        char c = *s;
        if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')) {
            sb_putc(sb, c);
            last_us = false;
            n++;
        } else if (!last_us) {
            sb_putc(sb, '_');
            last_us = true;
            n++;
        }
    }
}

static bool is_box(Type *t) { return t->kind == T_ADT && t->def == lang.box && lang.box; }

static bool variant_has_data(Type *t, uint32_t v);
static HMap zst_cache;      /* type id -> 1 no / 2 yes */
static TypeList zst_objects; /* types that need a `rcc_zst_N` constant */
static HMap zst_object_set;

/* Does the type carry no data (a zero-sized type in Rust)? Such values are
   never stored or copied by the generated C. */
static bool is_zst_like(Type *t) {
    switch ((TypeKind)t->kind) {
    case T_NEVER: case T_FNDEF:
        return true;
    case T_TUPLE: case T_ADT: case T_CLOSURE: case T_ARRAY:
        break;
    default:
        return false;
    }
    void *c = hmap_get(&zst_cache, t->id);
    if (c)
        return c == (void *)2;
    hmap_put(&zst_cache, t->id, (void *)1); /* recursive types are not ZSTs */
    bool r = true;
    if (t->kind == T_ARRAY) {
        r = t->len->cval == 0 || is_zst_like(t->elem);
    } else if (t->kind == T_ADT && (adt_def(t->def)->is_enum || adt_def(t->def)->is_union)) {
        r = false;
    } else {
        uint32_t n = ty_field_count(t, 0);
        for (uint32_t i = 0; i < n && r; i++)
            r = is_zst_like(ty_field_type(t, 0, i, (Span){0}));
    }
    hmap_put(&zst_cache, t->id, r ? (void *)2 : (void *)1);
    return r;
}

/* A struct that is exactly one byte in C (ccc miscompiles assignments of
   compound literals of such types; see tests/core/ccc_bugs/bool_eq_zero_branch.c). Returns the
   member path to store through ("tag", "f0", "f0.tag", ...), or NULL. */
static const char *single_byte_member(Type *t) {
    if (t->kind == T_BOOL || (t->kind == T_INT && ik_bits((IntKind)t->ik) == 8))
        return "";
    if (t->kind == T_ADT && adt_def(t->def)->is_enum) {
        AdtDef *adt = adt_def(t->def);
        for (uint32_t v = 0; v < adt->nvariants; v++)
            if (variant_has_data(t, v))
                return NULL;
        return ik_bits((IntKind)enum_tag_type(t)->ik) == 8 ? ".tag" : NULL;
    }
    if ((t->kind == T_TUPLE || t->kind == T_CLOSURE ||
         (t->kind == T_ADT && !adt_def(t->def)->is_union)) && ty_field_count(t, 0) == 1) {
        const char *inner = single_byte_member(ty_field_type(t, 0, 0, (Span){0}));
        return inner ? arena_printf(".f0%s", inner) : NULL;
    }
    return NULL;
}

static CTypeInfo *ctype_info(Type *t) {
    CTypeInfo *ci = hmap_get(&ctype_tab, t->id);
    if (ci)
        return ci;
    ci = NEW(CTypeInfo);
    ci->ty = t;
    hmap_put(&ctype_tab, t->id, ci);
    StrBuf sb = {0};
    switch ((TypeKind)t->kind) {
    case T_BOOL: ci->name = "bool"; break;
    case T_CHAR: ci->name = "u32"; break;
    case T_INT: ci->name = ik_name((IntKind)t->ik); break;
    case T_FLOAT: ci->name = t->ik == FK_F32 ? "f32" : "f64"; break;
    case T_NEVER: case T_FNDEF: ci->name = "Unit"; break;
    case T_TUPLE:
        if (t->substs->len == 0) {
            ci->name = "Unit";
            break;
        }
        /* fallthrough */
    case T_ADT: case T_CLOSURE: case T_ARRAY:
        sb_puts(&sb, t->kind == T_TUPLE ? "Tup_" : t->kind == T_ARRAY ? "Arr_"
                     : t->kind == T_CLOSURE ? "Closure_" : "");
        sanitize(&sb, ty_to_str(t), 40);
        sb_printf(&sb, "_%u", t->id);
        ci->name = arena_strdup(sb_str(&sb));
        ci->is_struct = true;
        vec_push(ctypes, ci);
        break;
    case T_REF: case T_PTR: {
        Type *p = t->elem;
        if (p->kind == T_DYN) {
            ci->name = "Dyn";
        } else if (p->kind == T_SLICE || p->kind == T_STR) {
            Type *elem = p->kind == T_STR ? ty_int(IK_U8) : p->elem;
            Type *key = ty_slice(elem);
            CTypeInfo *si = hmap_get(&ctype_tab, key->id);
            if (!si) {
                si = NEW(CTypeInfo);
                si->ty = key;
                hmap_put(&ctype_tab, key->id, si);
                sb_puts(&sb, "Slice_");
                sanitize(&sb, ty_to_str(elem), 40);
                sb_printf(&sb, "_%u", key->id);
                si->name = arena_strdup(sb_str(&sb));
                si->is_slice = true;
                vec_push(ctypes, si);
                (void)ctype(elem);
            }
            ci->name = si->name;
        } else {
            ci->name = arena_printf("%s*", ctype(p));
        }
        break;
    }
    case T_FNPTR:
        ci->name = arena_printf("Fn_%u", t->id);
        ci->is_fnptr = true;
        vec_push(ctypes, ci);
        for (uint32_t i = 0; i < t->substs->len; i++)
            (void)ctype(t->substs->t[i]);
        break;
    case T_SLICE: case T_STR: case T_DYN:
        fatal((Span){0}, "internal: unsized type `%s` used by value in generated C", ty_to_str(t));
    default:
        fatal((Span){0}, "internal: type `%s` reached code generation", ty_to_str(t));
    }
    return ci;
}

static const char *ctype(Type *t) { return ctype_info(t)->name; }

static bool by_value_struct(Type *t) {
    return (t->kind == T_TUPLE && t->substs->len) || t->kind == T_ADT || t->kind == T_CLOSURE ||
           t->kind == T_ARRAY;
}

static void define_struct(CTypeInfo *ci);

static void need_complete(Type *t) {
    if (by_value_struct(t))
        define_struct(ctype_info(t));
}

static void emit_fields(StrBuf *sb, Type *t, uint32_t variant, const char *indent) {
    uint32_t n = ty_field_count(t, variant), emitted = 0;
    for (uint32_t i = 0; i < n; i++) {
        Type *ft = ty_field_type(t, variant, i, (Span){0});
        if (is_zst_like(ft))
            continue; /* zero-sized fields take no space */
        sb_printf(sb, "%s%s f%u;\n", indent, ctype(ft), i);
        emitted++;
    }
    if (emitted == 0)
        sb_printf(sb, "%su8 _e;\n", indent);
}

/* Does enum variant v have any field that occupies space? */
static bool variant_has_data(Type *t, uint32_t v) {
    uint32_t n = ty_field_count(t, v);
    for (uint32_t i = 0; i < n; i++)
        if (!is_zst_like(ty_field_type(t, v, i, (Span){0})))
            return true;
    return false;
}

static void define_struct(CTypeInfo *ci) {
    if (ci->state == 2)
        return;
    if (ci->state == 1)
        fatal((Span){0}, "internal: type `%s` contains itself by value", ty_to_str(ci->ty));
    ci->state = 1;
    Type *t = ci->ty;
    StrBuf sb = {0};
    sb_printf(&sb, "/* %s */\nstruct %s {\n", ty_to_str(t), ci->name);
    if (t->kind == T_ARRAY) {
        need_complete(t->elem);
        uint64_t n = t->len->cval;
        sb_printf(&sb, "    %s a[%llu];\n", ctype(t->elem), (unsigned long long)(n ? n : 1));
    } else if (t->kind == T_ADT && adt_def(t->def)->is_enum) {
        AdtDef *adt = adt_def(t->def);
        bool any = false;
        for (uint32_t v = 0; v < adt->nvariants; v++) {
            for (uint32_t f = 0; f < adt->variants[v].nfields; f++)
                need_complete(ty_field_type(t, v, f, (Span){0}));
            any |= variant_has_data(t, v);
        }
        sb_printf(&sb, "    %s tag;\n", ctype(enum_tag_type(t)));
        if (any) {
            sb_puts(&sb, "    union {\n");
            for (uint32_t v = 0; v < adt->nvariants; v++) {
                if (!variant_has_data(t, v))
                    continue;
                sb_printf(&sb, "        struct { /* %s */\n", sym_str(adt->variants[v].name));
                emit_fields(&sb, t, v, "            ");
                sb_printf(&sb, "        } v%u;\n", v);
            }
            sb_puts(&sb, "    } u;\n");
        }
    } else if (t->kind == T_ADT && adt_def(t->def)->is_union) {
        for (uint32_t f = 0; f < ty_field_count(t, 0); f++)
            need_complete(ty_field_type(t, 0, f, (Span){0}));
        sb_puts(&sb, "    union {\n");
        emit_fields(&sb, t, 0, "        ");
        sb_puts(&sb, "    };\n");
    } else {
        for (uint32_t f = 0; f < ty_field_count(t, 0); f++)
            need_complete(ty_field_type(t, 0, f, (Span){0}));
        emit_fields(&sb, t, 0, "    ");
    }
    sb_puts(&sb, "};\n");
    sb_puts(&type_defs, sb_str(&sb));
    ci->state = 2;
}

/* ------------------------------------------------------------------ */
/* Constants, places, operands                                         */
/* ------------------------------------------------------------------ */

static const char *c_int_const(Type *ty, uint64_t lo, uint64_t hi) {
    if (ty->kind == T_BOOL)
        return lo ? "1" : "0";
    if (ty->kind == T_CHAR)
        return arena_printf("((u32)%" PRIu64 "u)", lo);
    IntKind k = (IntKind)ty->ik;
    const char *tn = ik_name(k);
    if (ik_bits(k) == 128) {
        if (hi == 0)
            return arena_printf("((%s)%" PRIu64 "ull)", tn, lo);
        return arena_printf("((%s)(((u128)%" PRIu64 "ull << 64) | (u128)%" PRIu64 "ull))", tn, hi, lo);
    }
    if (ik_signed(k)) {
        int64_t v = (int64_t)lo;
        if (v == INT64_MIN)
            return arena_printf("((%s)(-9223372036854775807ll - 1))", tn);
        return arena_printf("((%s)%" PRId64 "ll)", tn, v);
    }
    return arena_printf("((%s)%" PRIu64 "ull)", tn, lo);
}

static const char *c_string_lit(const char *s, uint32_t len) {
    StrBuf sb = {0};
    sb_putc(&sb, '"');
    for (uint32_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c == '"' || c == '\\') {
            sb_putc(&sb, '\\');
            sb_putc(&sb, (char)c);
        } else if (c >= 0x20 && c < 0x7f && c != '?') {
            sb_putc(&sb, (char)c);
        } else {
            sb_printf(&sb, "\\%03o", c);
        }
    }
    sb_putc(&sb, '"');
    return sb_str(&sb);
}

/* The value of a zero-sized type: a named zero object (never a local,
   which is never written, and never a compound literal). */
static const char *c_zst_value(Type *ty) {
    if (!strcmp(ctype(ty), "Unit"))
        return "rcc_unit";
    if (!hmap_get(&zst_object_set, ty->id)) {
        hmap_put(&zst_object_set, ty->id, ty);
        vec_push(zst_objects, ty);
    }
    return arena_printf("rcc_zst_%u", ty->id);
}

static const char *c_const(const MirConst *c) {
    switch ((ConstKind)c->kind) {
    case MC_INT:
        return c_int_const(c->ty, c->lo, c->hi);
    case MC_FLOAT: {
        const char *tn = ctype(c->ty);
        double v = c->fval;
        if (v != v)
            return arena_printf("((%s)(0.0 / 0.0))", tn);
        if (v == INFINITY)
            return arena_printf("((%s)(1.0 / 0.0))", tn);
        if (v == -INFINITY)
            return arena_printf("((%s)(-1.0 / 0.0))", tn);
        char buf[64];
        snprintf(buf, sizeof buf, "%.17g", v);
        bool has_dot = strchr(buf, '.') || strchr(buf, 'e') || strchr(buf, 'n') || strchr(buf, 'i');
        return arena_printf("((%s)%s%s)", tn, buf, has_dot ? "" : ".0");
    }
    case MC_STR:
        return arena_printf("((%s){(u8 *)%s, %u})", ctype(c->ty), c_string_lit(c->str, c->len), c->len);
    case MC_BYTESTR:
        return arena_printf("((%s)%s)", ctype(c->ty), c_string_lit(c->str, c->len));
    case MC_ZST:
        return c_zst_value(c->ty);
    case MC_FN:
        return arena_printf("((%s)%s)", ctype(c->ty), c->inst->name);
    }
    return "?";
}

static const char *local_name(LocalId l) { return arena_printf("_%u", l); }

typedef struct CPlace {
    const char *expr; /* lvalue (sized) or fat pointer value (unsized) */
    bool fat;
    Type *ty;
} CPlace;

static CPlace c_place(const Place *p) {
    CPlace cp;
    cp.expr = local_name(p->local);
    cp.fat = false;
    cp.ty = cur_mir->locals[p->local].ty;
    int downcast = -1;
    /* When the current expression is `(*ptr)`, `dptr` is `ptr`, so that
       member access is written `ptr->m` (plainer C; also avoids a ccc
       miscompilation of `&(*p).a.b` kept live across a call). */
    const char *dptr = NULL;
#define MEMBER_BASE() (dptr ? arena_printf("%s->", dptr) : arena_printf("%s.", cp.expr))
    for (uint32_t i = 0; i < p->nprojs; i++) {
        const Proj *pj = &p->projs[i];
        const char *was_dptr = dptr;
        if (pj->kind != PJ_DOWNCAST)
            dptr = NULL;
        switch ((ProjKind)pj->kind) {
        case PJ_DEREF: {
            const char *ptr = is_box(cp.ty) ? arena_printf("%s.f0", cp.expr) : cp.expr;
            Type *pointee = ty_pointee(cp.ty);
            if (!pointee)
                fatal((Span){0}, "internal: deref of non-pointer `%s` in cgen", ty_to_str(cp.ty));
            if (ty_is_unsized(pointee)) {
                cp.expr = ptr;
                cp.fat = true;
            } else {
                cp.expr = arena_printf("(*%s)", ptr);
                dptr = ptr;
            }
            break;
        }
        case PJ_FIELD:
            dptr = was_dptr;
            if (downcast >= 0)
                cp.expr = arena_printf("%su.v%d.f%u", MEMBER_BASE(), downcast, pj->index);
            else
                cp.expr = arena_printf("%sf%u", MEMBER_BASE(), pj->index);
            dptr = NULL;
            downcast = -1;
            break;
        case PJ_DOWNCAST:
            downcast = (int)pj->index;
            break;
        case PJ_INDEX:
            if (cp.fat)
                cp.expr = arena_printf("%s.ptr[_%u]", cp.expr, pj->local);
            else {
                dptr = was_dptr;
                cp.expr = arena_printf("%sa[_%u]", MEMBER_BASE(), pj->local);
                dptr = NULL;
            }
            cp.fat = false;
            break;
        case PJ_CONST_INDEX:
            if (cp.fat) {
                cp.expr = pj->from_end ? arena_printf("%s.ptr[%s.len - %u]", cp.expr, cp.expr, pj->index)
                                       : arena_printf("%s.ptr[%u]", cp.expr, pj->index);
            } else {
                uint64_t n = cp.ty->len->cval;
                dptr = was_dptr;
                cp.expr = arena_printf("%sa[%llu]", MEMBER_BASE(),
                                       (unsigned long long)(pj->from_end ? n - pj->index : pj->index));
                dptr = NULL;
            }
            cp.fat = false;
            break;
        case PJ_SUBSLICE:
            if (cp.fat) {
                const char *st = ctype(ty_ref(pj->ty, false));
                cp.expr = arena_printf("((%s){%s.ptr + %u, %s.len - %u})", st, cp.expr, pj->index,
                                       cp.expr, pj->index + pj->to);
            } else {
                cp.expr = arena_printf("(*(%s *)&%s.a[%u])", ctype(pj->ty), cp.expr, pj->index);
            }
            break;
        }
        cp.ty = pj->ty;
    }
#undef MEMBER_BASE
    return cp;
}

static const char *c_lvalue(const Place *p) {
    CPlace cp = c_place(p);
    if (cp.fat)
        fatal((Span){0}, "internal: unsized place `%s` used as a value", mir_place_str(cur_mir, p));
    return cp.expr;
}

static const char *c_operand(const Operand *o) {
    if (o->kind == OP_CONST)
        return c_const(&o->c);
    if (is_zst_like(o->place.ty))
        return c_zst_value(o->place.ty);
    return c_lvalue(&o->place);
}

static Type *operand_ty(const Operand *o) { return o->kind == OP_CONST ? o->c.ty : o->place.ty; }

/* ------------------------------------------------------------------ */
/* Rvalues                                                             */
/* ------------------------------------------------------------------ */

static const char *unsigned_name(IntKind k) {
    switch (k) {
    case IK_I8: case IK_U8: return "u8";
    case IK_I16: case IK_U16: return "u16";
    case IK_I32: case IK_U32: return "u32";
    case IK_I64: case IK_U64: return "u64";
    case IK_I128: case IK_U128: return "u128";
    default: return "usize";
    }
}

/* The unsigned type in which wrapping arithmetic for k is carried out. */
static const char *wide_name(IntKind k) {
    uint32_t bits = ik_bits(k);
    if (bits < 32)
        return "u32";
    return unsigned_name(k);
}

static const char *c_binary(const Rvalue *rv) {
    Type *ta = operand_ty(&rv->a);
    const char *a = c_operand(&rv->a), *b = c_operand(&rv->b);
    BinOp op = (BinOp)rv->op;
    static const char *sym[18] = {"+", "-", "*", "/", "%", "&&", "||", "^", "&", "|", "<<", ">>",
                                  "==", "<", "<=", "!=", ">=", ">"};
    if (ta->kind == T_BOOL && (op == BIN_EQ || op == BIN_NE)) {
        /* bool == bool as xor: ccc drops the `cset` that produced an
           operand of `cmp wA, wB, uxtb` (tests/core/ccc_bugs/
           bool_field_compare.c, cmp_of_cmp_result.c) */
        if (op == BIN_NE)
            return arena_printf("((bool)(%s ^ %s))", a, b);
        return arena_printf("((bool)((%s ^ %s) ^ 1))", a, b);
    }
    if (op >= BIN_EQ)
        return arena_printf("(%s %s %s)", a, sym[op], b);
    if (ta->kind == T_FLOAT) {
        if (op == BIN_REM)
            return arena_printf("%s(%s, %s)", ta->ik == FK_F32 ? "fmodf" : "fmod", a, b);
        return arena_printf("(%s %s %s)", a, sym[op], b);
    }
    if (ta->kind == T_BOOL)
        return arena_printf("((bool)(%s %s %s))", a, sym[op], b);
    if (ta->kind != T_INT)
        fatal((Span){0}, "internal: binary operator on `%s` in cgen", ty_to_str(ta));
    IntKind k = (IntKind)ta->ik;
    const char *tn = ik_name(k), *un = unsigned_name(k), *wn = wide_name(k);
    uint32_t bits = ik_bits(k);
    switch (op) {
    case BIN_ADD: case BIN_SUB: case BIN_MUL:
        return arena_printf("((%s)(%s)((%s)%s %s (%s)%s))", tn, un, wn, a, sym[op], wn, b);
    case BIN_DIV:
        return arena_printf("rcc_div_%s(%s, %s)", tn, a, b);
    case BIN_REM:
        return arena_printf("rcc_rem_%s(%s, %s)", tn, a, b);
    case BIN_BITXOR: case BIN_BITAND: case BIN_BITOR:
        return arena_printf("((%s)(%s %s %s))", tn, a, sym[op], b);
    case BIN_SHL:
        return arena_printf("((%s)(%s)((%s)%s << ((u32)%s & %uu)))", tn, un, wn, a, b, bits - 1);
    case BIN_SHR:
        return arena_printf("((%s)(%s >> ((u32)%s & %uu)))", tn, a, b, bits - 1);
    default:
        break;
    }
    fatal((Span){0}, "internal: bad binary operator in cgen");
}

static const char *c_unary(const Rvalue *rv) {
    Type *ta = operand_ty(&rv->a);
    const char *a = c_operand(&rv->a);
    if (rv->op == UN_NEG) {
        if (ta->kind == T_FLOAT)
            return arena_printf("(-%s)", a);
        IntKind k = (IntKind)ta->ik;
        return arena_printf("((%s)(%s)((%s)0 - (%s)%s))", ik_name(k), unsigned_name(k), wide_name(k),
                            wide_name(k), a);
    }
    if (ta->kind == T_BOOL)
        return arena_printf("(!%s)", a);
    return arena_printf("((%s)(~%s))", ik_name((IntKind)ta->ik), a);
}

/* The raw pointer inside a pointer-like value: Box/Rc/Arc keep it in f0. */
static const char *box_ptr(Type *t, const char *e) {
    return t->kind == T_ADT ? arena_printf("%s.f0", e) : e;
}

/* Wrap an unsized (fat) pointer into the destination smart pointer struct,
   copying the remaining fields from the source value. */
static const char *wrap_unsized(Type *src, Type *dst, const char *a, const char *fat) {
    if (dst->kind != T_ADT)
        return fat;
    StrBuf sb = {0};
    sb_printf(&sb, "((%s){.f0 = %s", ctype(dst), fat);
    uint32_t n = ty_field_count(dst, 0);
    for (uint32_t i = 1; i < n; i++)
        sb_printf(&sb, ", .f%u = %s.f%u", i, a, i);
    sb_puts(&sb, "})");
    (void)src;
    return sb_str(&sb);
}

static const char *c_cast(const Rvalue *rv) {
    Type *src = operand_ty(&rv->a), *dst = rv->ty;
    const char *a = c_operand(&rv->a);
    const char *dn = ctype(dst);
    switch ((CastKind)rv->cast) {
    case CAST_INT_INT:
        if (dst->kind == T_BOOL)
            return arena_printf("((bool)(%s != 0))", a);
        return arena_printf("((%s)%s)", dn, a);
    case CAST_INT_FLOAT: case CAST_FLOAT_FLOAT:
        return arena_printf("((%s)%s)", dn, a);
    case CAST_FLOAT_INT:
        if (dst->kind == T_INT)
            return arena_printf("rcc_f2i_%s_%s(%s)", ctype(src), dn, a);
        return arena_printf("((%s)rcc_f2i_%s_u32(%s))", dn, ctype(src), a);
    case CAST_PTR_PTR: {
        bool sf = ty_is_unsized(src->elem), df = ty_is_unsized(dst->elem);
        if (!sf && !df)
            return arena_printf("((%s)%s)", dn, a);
        if (sf && !df)
            return arena_printf("((%s)%s.ptr)", dn, a);
        if (sf && df) {
            if (dst->elem->kind == T_DYN)
                return a;
            Type *elem = dst->elem->kind == T_STR ? ty_int(IK_U8) : dst->elem->elem;
            return arena_printf("((%s){(%s *)%s.ptr, %s.len})", dn, ctype(elem), a, a);
        }
        fatal((Span){0}, "internal: thin-to-fat pointer cast in cgen");
    }
    case CAST_PTR_INT:
        if (ty_is_unsized(src->elem))
            return arena_printf("((%s)(usize)%s.ptr)", dn, a);
        return arena_printf("((%s)(usize)%s)", dn, a);
    case CAST_INT_PTR: case CAST_FNPTR_INT:
        return arena_printf("((%s)(usize)%s)", dn, a);
    case CAST_UNSIZE_ARRAY: {
        Type *dp = dst->kind == T_ADT ? dst->substs->t[0] : dst->elem;
        const char *fat_ty = ctype(ty_ref(dp, false));
        const char *fat = arena_printf("((%s){(%s *)%s, %lluull})", fat_ty, ctype(dp->elem),
                                       box_ptr(src, a), (unsigned long long)rv->count);
        return wrap_unsized(src, dst, a, fat);
    }
    case CAST_UNSIZE_DYN: {
        const char *fat = arena_printf("((Dyn){(void *)%s, %s})", box_ptr(src, a), rv->vtable->name);
        return wrap_unsized(src, dst, a, fat);
    }
    case CAST_TRANSMUTE:
        break;
    }
    fatal((Span){0}, "internal: bad cast in cgen");
}

static const char *c_aggregate(const Rvalue *rv) {
    Type *t = rv->ty;
    const char *tn = ctype(t);
    StrBuf sb = {0};
    uint32_t emitted = 0;
    sb_printf(&sb, "((%s){", tn);
    switch ((AggKind)rv->agg) {
    case AGG_TUPLE: case AGG_CLOSURE:
        for (uint32_t i = 0; i < rv->nops; i++) {
            if (is_zst_like(operand_ty(&rv->ops[i])))
                continue;
            sb_printf(&sb, "%s.f%u = %s", emitted++ ? ", " : "", i, c_operand(&rv->ops[i]));
        }
        if (!emitted)
            sb_puts(&sb, "0");
        break;
    case AGG_ARRAY:
        sb_puts(&sb, "{");
        if (rv->nops == 0)
            sb_puts(&sb, "0");
        for (uint32_t i = 0; i < rv->nops; i++)
            sb_printf(&sb, "%s%s", i ? ", " : "", c_operand(&rv->ops[i]));
        sb_puts(&sb, "}");
        break;
    case AGG_ADT: {
        AdtDef *adt = adt_def(t->def);
        if (adt->is_enum) {
            adt_compute_discs(adt);
            int64_t d = adt->variants[rv->variant].disc;
            sb_printf(&sb, ".tag = %s",
                      c_int_const(enum_tag_type(t), (uint64_t)d, d < 0 ? ~0ull : 0));
            if (variant_has_data(t, rv->variant)) {
                sb_printf(&sb, ", .u = {.v%u = {", rv->variant);
                for (uint32_t i = 0; i < rv->nops; i++) {
                    if (is_zst_like(operand_ty(&rv->ops[i])))
                        continue;
                    sb_printf(&sb, "%s.f%u = %s", emitted++ ? ", " : "",
                              rv->field_idx ? rv->field_idx[i] : i, c_operand(&rv->ops[i]));
                }
                sb_puts(&sb, "}}");
            }
        } else {
            for (uint32_t i = 0; i < rv->nops; i++) {
                if (is_zst_like(operand_ty(&rv->ops[i])))
                    continue;
                sb_printf(&sb, "%s.f%u = %s", emitted++ ? ", " : "",
                          rv->field_idx ? rv->field_idx[i] : i, c_operand(&rv->ops[i]));
            }
            if (!emitted)
                sb_puts(&sb, "0");
        }
        break;
    }
    }
    sb_puts(&sb, "})");
    return sb_str(&sb);
}

/* sizeof as Rust sees it: zero for zero-sized types. */
static const char *c_sizeof(Type *t) {
    return is_zst_like(t) ? "((usize)0)" : arena_printf("sizeof(%s)", ctype(t));
}

static void emit_assign(const MirStmt *s) {
    const Rvalue *rv = &s->rv;
    /* Values of zero-sized types carry no information: never store them.
       (This also sidesteps a ccc miscompilation of 1-byte struct stores,
       see tests/core/ccc_bugs/bool_eq_zero_branch.c.) */
    if (is_zst_like(s->place.ty))
        return;
    const char *dest = c_lvalue(&s->place);
    const char *val = NULL;
    switch ((RvalueKind)rv->kind) {
    case RV_USE:
        val = c_operand(&rv->a);
        break;
    case RV_REF: {
        if (is_zst_like(rv->place.ty)) {
            /* zero-sized places have no storage of their own */
            val = arena_printf("((%s *)&%s)", ctype(rv->place.ty), c_zst_value(rv->place.ty));
            break;
        }
        CPlace cp = c_place(&rv->place);
        val = cp.fat ? cp.expr : arena_printf("(&%s)", cp.expr);
        /* &T as a raw pointer type or vice versa is the same C type */
        break;
    }
    case RV_BINARY:
        val = c_binary(rv);
        break;
    case RV_UNARY:
        val = c_unary(rv);
        break;
    case RV_CAST:
        if (rv->cast == CAST_TRANSMUTE && is_zst_like(operand_ty(&rv->a))) {
            fprintf(out, "    memset(&%s, 0, sizeof(%s));\n", dest, ctype(rv->ty));
            return;
        }
        if (rv->cast == CAST_TRANSMUTE) {
            fprintf(out, "    { %s rcc_tmp = %s; memcpy(&%s, &rcc_tmp, sizeof(%s)); }\n",
                    ctype(operand_ty(&rv->a)), c_operand(&rv->a), dest, ctype(rv->ty));
            return;
        }
        val = c_cast(rv);
        break;
    case RV_AGGREGATE: {
        const char *member = single_byte_member(rv->ty);
        if (member && *member) {
            /* store through the scalar member instead of assigning a
               one-byte compound literal */
            if (rv->ty->kind == T_ADT && adt_def(rv->ty->def)->is_enum) {
                AdtDef *adt = adt_def(rv->ty->def);
                adt_compute_discs(adt);
                int64_t d = adt->variants[rv->variant].disc;
                fprintf(out, "    %s.tag = %s;\n", dest,
                        c_int_const(enum_tag_type(rv->ty), (uint64_t)d, d < 0 ? ~0ull : 0));
            } else {
                fprintf(out, "    %s.f0 = %s;\n", dest, c_operand(&rv->ops[0]));
            }
            return;
        }
        val = c_aggregate(rv);
        break;
    }
    case RV_REPEAT:
        if (rv->count == 0)
            return;
        fprintf(out, "    { usize rcc_i; for (rcc_i = 0; rcc_i < %lluull; rcc_i++) %s.a[rcc_i] = %s; }\n",
                (unsigned long long)rv->count, dest, c_operand(&rv->a));
        return;
    case RV_LEN: {
        if (rv->place.ty->kind == T_ARRAY) {
            val = arena_printf("((usize)%lluull)", (unsigned long long)rv->place.ty->len->cval);
            break;
        }
        CPlace cp = c_place(&rv->place);
        if (cp.fat)
            val = arena_printf("%s.len", cp.expr);
        else
            val = arena_printf("((usize)%lluull)", (unsigned long long)cp.ty->len->cval);
        break;
    }
    case RV_DISCR:
        val = arena_printf("%s.tag", c_lvalue(&rv->place));
        break;
    case RV_GLOBAL_ADDR:
        val = arena_printf("(&%s)", rv->global->name);
        break;
    }
    fprintf(out, "    %s = %s;\n", dest, val);
}

/* ------------------------------------------------------------------ */
/* Calls and intrinsics                                                */
/* ------------------------------------------------------------------ */

static const char *drop_fn_name(Type *t) { return arena_printf("rcc_drop_%u", t->id); }

static bool sym_is(Sym s, const char *name) { return strcmp(sym_str(s), name) == 0; }

static void emit_intrinsic(const MirStmt *s) {
    Sym name = s->intrinsic;
    const char *dest = c_lvalue(&s->place);
    Type *t0 = s->substs && s->substs->len ? s->substs->t[0] : NULL;
#define A(i) c_operand(&s->args[i])
    if (sym_is(name, "size_of")) {
        fprintf(out, "    %s = (usize)%s;\n", dest, c_sizeof(t0));
    } else if (sym_is(name, "align_of")) {
        if (is_zst_like(t0))
            fprintf(out, "    %s = (usize)1;\n", dest);
        else
            fprintf(out, "    %s = (usize)_Alignof(%s);\n", dest, ctype(t0));
    } else if (sym_is(name, "ptr_diff")) {
        /* element distance between two pointers into the same allocation
           (well defined for zero-sized elements too: they are 1 byte in C) */
        fprintf(out, "    %s = (isize)(%s - %s);\n", dest, A(0), A(1));
    } else if (sym_is(name, "ptr_offset")) {
        fprintf(out, "    %s = (%s)(%s + %s);\n", dest, ctype(s->place.ty), A(0), A(1));
    } else if (sym_is(name, "copy_nonoverlapping")) {
        fprintf(out, "    memcpy(%s, %s, %s * %s);\n", A(1), A(0), A(2), c_sizeof(t0));
    } else if (sym_is(name, "copy")) {
        fprintf(out, "    memmove(%s, %s, %s * %s);\n", A(1), A(0), A(2), c_sizeof(t0));
    } else if (sym_is(name, "write_bytes")) {
        fprintf(out, "    memset(%s, %s, %s * %s);\n", A(0), A(1), A(2), c_sizeof(t0));
    } else if (sym_is(name, "abort")) {
        fprintf(out, "    rcc_abort();\n");
    } else if (sym_is(name, "slice_from_raw_parts")) {
        Type *elem = ty_pointee(s->place.ty)->kind == T_STR ? ty_int(IK_U8)
                                                            : ty_pointee(s->place.ty)->elem;
        fprintf(out, "    %s = (%s){(%s *)%s, %s};\n", dest, ctype(s->place.ty), ctype(elem), A(0), A(1));
    } else if (sym_is(name, "ctpop") || sym_is(name, "ctlz") || sym_is(name, "cttz")) {
        IntKind k = (IntKind)t0->ik;
        uint32_t bits = ik_bits(k);
        const char *fn = sym_str(name);
        if (bits == 128) {
            fprintf(out, "    %s = rcc_%s128((u128)%s);\n", dest, fn, A(0));
        } else if (sym_is(name, "ctlz")) {
            fprintf(out, "    %s = rcc_ctlz64((u64)(%s)%s) - %uu;\n", dest, unsigned_name(k), A(0),
                    64 - bits);
        } else if (sym_is(name, "cttz")) {
            fprintf(out, "    { u32 rcc_n = rcc_cttz64((u64)(%s)%s); %s = rcc_n > %uu ? %uu : rcc_n; }\n",
                    unsigned_name(k), A(0), dest, bits, bits);
        } else {
            fprintf(out, "    %s = rcc_ctpop64((u64)(%s)%s);\n", dest, unsigned_name(k), A(0));
        }
    } else if (sym_is(name, "bswap")) {
        IntKind k = (IntKind)t0->ik;
        fprintf(out, "    %s = (%s)rcc_bswap((u128)(%s)%s, %uu);\n", dest, ik_name(k),
                unsigned_name(k), A(0), ik_bits(k) / 8);
    } else if (sym_is(name, "rotate_left") || sym_is(name, "rotate_right")) {
        IntKind k = (IntKind)t0->ik;
        uint32_t bits = ik_bits(k);
        const char *un = unsigned_name(k);
        bool left = sym_is(name, "rotate_left");
        fprintf(out,
                "    { %s rcc_x = (%s)%s; u32 rcc_n = (u32)%s %% %uu; "
                "%s = (%s)(rcc_n ? (%s)((rcc_x %s rcc_n) | (rcc_x %s (%uu - rcc_n))) : rcc_x); }\n",
                un, un, A(0), A(1), bits, dest, ik_name(k), un, left ? "<<" : ">>", left ? ">>" : "<<",
                bits);
    } else if (sym_is(name, "add_with_overflow") || sym_is(name, "sub_with_overflow") ||
               sym_is(name, "mul_with_overflow")) {
        const char *op = sym_is(name, "add_with_overflow") ? "add"
                         : sym_is(name, "sub_with_overflow") ? "sub" : "mul";
        const char *tn = ctype(t0);
        fprintf(out, "    { %s rcc_r; bool rcc_o = rcc_%s_ovf_%s(%s, %s, &rcc_r); %s = (%s){rcc_r, rcc_o}; }\n",
                tn, op, tn, A(0), A(1), dest, ctype(s->place.ty));
    } else {
        fatal(s->span, "unknown compiler intrinsic `%s`", sym_str(name));
    }
#undef A
}

static bool ret_is_void(const Instance *inst) {
    return inst->kind == INST_EXTERN && (ty_is_unit(inst->ret) || inst->ret->kind == T_NEVER);
}

static void emit_call(const MirStmt *s) {
    if (s->callee == CALLEE_INTRINSIC) {
        emit_intrinsic(s);
        return;
    }
    StrBuf sb = {0};
    const char *dest = c_lvalue(&s->place);
    bool assign = true;
    if (s->callee == CALLEE_DIRECT) {
        if (ret_is_void(s->inst))
            assign = false;
        sb_puts(&sb, s->inst->name);
    } else if (s->callee == CALLEE_INDIRECT) {
        sb_printf(&sb, "(%s)", c_operand(&s->rv.a));
    } else {
        /* virtual: cast the vtable slot to the method's C type */
        const char *self = c_operand(&s->args[0]);
        if (is_box(operand_ty(&s->args[0])))
            self = arena_printf("%s.f0", self); /* self: Box<dyn Trait> */
        sb_printf(&sb, "((%s (*)(void *", ctype(s->place.ty));
        for (uint32_t i = 1; i < s->nargs; i++)
            sb_printf(&sb, ", %s", ctype(operand_ty(&s->args[i])));
        sb_printf(&sb, "))%s.vt[%u])", self, 3 + s->slot);
    }
    sb_puts(&sb, "(");
    for (uint32_t i = 0; i < s->nargs; i++) {
        if (i)
            sb_puts(&sb, ", ");
        if (i == 0 && s->callee == CALLEE_VIRTUAL)
            sb_printf(&sb, "%s%s.ptr", c_operand(&s->args[0]),
                      is_box(operand_ty(&s->args[0])) ? ".f0" : "");
        else
            sb_puts(&sb, c_operand(&s->args[i]));
    }
    sb_puts(&sb, ")");
    if (assign && !s->diverges && !is_zst_like(s->place.ty))
        fprintf(out, "    %s = %s;\n", dest, sb_str(&sb));
    else
        fprintf(out, "    %s;\n", sb_str(&sb));
}

static void emit_drop(const MirStmt *s) {
    Type *t = s->place.ty;
    if (is_zst_like(t)) {
        if (ty_needs_drop(t))
            fprintf(out, "    %s((%s *)&%s);\n", drop_fn_name(t), ctype(t), c_zst_value(t));
        return;
    }
    CPlace cp = c_place(&s->place);
    if (s->shallow) {
        Type *pointee = t->substs->t[0];
        fprintf(out, "    rcc_box_free((void *)%s.f0%s);\n", cp.expr, ty_is_unsized(pointee) ? ".ptr" : "");
        return;
    }
    if (cp.fat) {
        if (t->kind == T_DYN) {
            fprintf(out, "    if (%s.vt[0]) ((void (*)(void *))%s.vt[0])(%s.ptr);\n", cp.expr, cp.expr,
                    cp.expr);
        } else if (t->kind == T_SLICE && ty_needs_drop(t->elem)) {
            fprintf(out, "    { usize rcc_i; for (rcc_i = 0; rcc_i < %s.len; rcc_i++) %s(&%s.ptr[rcc_i]); }\n",
                    cp.expr, drop_fn_name(t->elem), cp.expr);
        }
        return;
    }
    if (ty_needs_drop(t))
        fprintf(out, "    %s(&%s);\n", drop_fn_name(t), cp.expr);
}

/* ------------------------------------------------------------------ */
/* Functions                                                           */
/* ------------------------------------------------------------------ */

/* Small functions are emitted `static inline` (in the shared header when
   the program is split into units) so that the C compiler can inline
   them everywhere: clang -O1 only inlines functions marked `inline`.
   The limit is a number of MIR statements (RCC_INLINE_MAX, 0 = never). */
static uint32_t inline_max = 24;

static bool is_small(const Instance *inst) {
    const MirBody *m = inst->mir;
    if (!m || !inline_max || inst->kind == INST_GLOBAL_INIT || inst == prog->user_main ||
        inst == prog->start || m->nblocks > inline_max)
        return false;
    uint32_t n = 0;
    for (uint32_t b = 0; b < m->nblocks; b++)
        n += m->blocks[b].nstmts;
    return n <= inline_max;
}

static void emit_proto(FILE *f, const Instance *inst) {
    bool is_extern = inst->kind == INST_EXTERN;
    fprintf(f, "%s%s %s(", is_extern ? "extern " : is_small(inst) ? "static inline " : linkage,
            ret_is_void(inst) ? (inst->ret->kind == T_NEVER ? "RCC_NORETURN void" : "void")
                              : ctype(inst->ret),
            inst->name);
    for (uint32_t i = 0; i < inst->nparams; i++)
        fprintf(f, "%s%s _%u", i ? ", " : "", ctype(inst->params[i]), i + 1);
    if (inst->variadic)
        fprintf(f, "%s...", inst->nparams ? ", " : "");
    else if (inst->nparams == 0)
        fprintf(f, "void");
    fprintf(f, ")");
}

static uint32_t term_nsucc(const Terminator *t) {
    if (t->kind == TM_GOTO)
        return 1;
    if (t->kind == TM_SWITCH)
        return t->ntargets + 1;
    return 0;
}

static BlockId term_succ(const Terminator *t, uint32_t i) {
    if (t->kind == TM_GOTO)
        return t->target;
    /* the "otherwise" edge first: for an `if` that is the then-branch */
    return i == 0 ? t->target : t->targets[i - 1];
}

/*
 * The reachable blocks in reverse postorder: every block comes after its
 * predecessors except along loop back edges. Emitting in this order keeps
 * the C close to structured code (definitions textually before uses),
 * which C compilers with simple linear liveness analyses rely on.
 */
static BlockId *block_order(const MirBody *mir, uint32_t *count) {
    uint32_t n = mir->nblocks;
    bool *seen = NEW_ARRAY(bool, n);
    BlockId *post = NEW_ARRAY(BlockId, n);
    BlockId *stack = NEW_ARRAY(BlockId, n + 1);
    uint32_t *next = NEW_ARRAY(uint32_t, n + 1);
    uint32_t sp = 0, np = 0;
    stack[sp] = 0;
    next[sp++] = 0;
    seen[0] = true;
    while (sp) {
        BlockId b = stack[sp - 1];
        const Terminator *t = &mir->blocks[b].term;
        uint32_t ns = term_nsucc(t);
        if (next[sp - 1] < ns) {
            /* visit successors last-to-first so the first ends up first in
               the reversed order */
            BlockId c = term_succ(t, ns - 1 - next[sp - 1]);
            next[sp - 1]++;
            if (!seen[c]) {
                seen[c] = true;
                stack[sp] = c;
                next[sp++] = 0;
            }
        } else {
            post[np++] = b;
            sp--;
        }
    }
    BlockId *order = NEW_ARRAY(BlockId, np ? np : 1);
    for (uint32_t i = 0; i < np; i++)
        order[i] = post[np - 1 - i];
    *count = np;
    return order;
}

static void emit_function(const Instance *inst) {
    const MirBody *mir = inst->mir;
    cur_mir = mir;
    const char *desc = "";
    if (inst->kind == INST_FN)
        desc = arena_printf("%s%s", def_path_str(inst->def), substs_to_str(inst->substs));
    else if (inst->ty)
        desc = ty_to_str(inst->ty);
    fprintf(out, "\n/* %s */\n", desc);
    emit_proto(out, inst);
    fprintf(out, " {\n");
    /* Locals that receive the address of a place reached through another
       pointer (`_6 = &_2->f1`). ccc folds such an address into its later
       use without keeping the base pointer alive, so its register can be
       reused in between (tests/core/ccc_bugs/field_addr_live_across_call.c).
       RCC_ADDR is `volatile` when compiling with ccc, forcing the address
       to be materialized; it is empty for other compilers. */
    bool *addr_tmp = NEW_ARRAY(bool, mir->nlocals);
    for (BlockId b = 0; b < mir->nblocks; b++) {
        const MirBlock *blk = &mir->blocks[b];
        for (uint32_t i = 0; i < blk->nstmts; i++) {
            const MirStmt *s = &blk->stmts[i];
            if (s->kind != ST_ASSIGN || s->rv.kind != RV_REF || s->place.nprojs)
                continue;
            for (uint32_t k = 0; k < s->rv.place.nprojs; k++)
                if (s->rv.place.projs[k].kind == PJ_DEREF)
                    addr_tmp[s->place.local] = true;
        }
    }
    fprintf(out, "    %s _0;\n", ctype(mir->locals[0].ty));
    for (uint32_t i = mir->nargs + 1; i < mir->nlocals; i++) {
        const MirLocal *l = &mir->locals[i];
        const char *q = addr_tmp[i] && (l->ty->kind == T_REF || l->ty->kind == T_PTR) &&
                        !ty_is_unsized(l->ty->elem) ? " RCC_ADDR" : "";
        if (l->name)
            fprintf(out, "    %s%s _%u; /* %s */\n", ctype(l->ty), q, i, sym_str(l->name));
        else
            fprintf(out, "    %s%s _%u;\n", ctype(l->ty), q, i);
    }
    uint32_t norder;
    BlockId *order = block_order(mir, &norder);
    for (uint32_t oi = 0; oi < norder; oi++) {
        BlockId b = order[oi];
        const MirBlock *blk = &mir->blocks[b];
        fprintf(out, "bb%u:;\n", b);
        for (uint32_t i = 0; i < blk->nstmts; i++) {
            const MirStmt *s = &blk->stmts[i];
            switch ((StmtKindM)s->kind) {
            case ST_ASSIGN: emit_assign(s); break;
            case ST_CALL: emit_call(s); break;
            case ST_DROP: emit_drop(s); break;
            case ST_BOUNDS_CHECK:
                fprintf(out, "    if (%s >= %s) rcc_panic_bounds(%s, %s);\n", c_operand(&s->rv.a),
                        c_operand(&s->rv.b), c_operand(&s->rv.a), c_operand(&s->rv.b));
                break;
            case ST_FORGET: case ST_NOP: break;
            }
        }
        const Terminator *t = &blk->term;
        switch ((TermKind)t->kind) {
        case TM_GOTO:
            if (oi + 1 >= norder || order[oi + 1] != t->target)
                fprintf(out, "    goto bb%u;\n", t->target);
            break;
        case TM_SWITCH: {
            Type *dt = operand_ty(&t->discr);
            const char *d = c_operand(&t->discr);
            bool wide = dt->kind == T_INT && ik_bits((IntKind)dt->ik) == 128;
            if (dt->kind == T_BOOL && t->ntargets == 1) {
                /* Always test the bool positively: `if (b) goto T; goto F;`.
                   ccc drops the `cset` of a comparison that reaches
                   `if (b == 0)` or `if (!b)` through copies and then tests a
                   stale register (see
                   tests/core/ccc_bugs/bool_eq_zero_branch.c and
                   the note on `bool` in rt/rcc_rt.h). */
                BlockId yes = t->values[0].lo ? t->targets[0] : t->target;
                BlockId no = t->values[0].lo ? t->target : t->targets[0];
                fprintf(out, "    if (%s) goto bb%u;\n", d, yes);
                if (oi + 1 >= norder || order[oi + 1] != no)
                    fprintf(out, "    goto bb%u;\n", no);
            } else if (t->ntargets == 1 || wide) {
                for (uint32_t i = 0; i < t->ntargets; i++)
                    fprintf(out, "    if (%s == %s) goto bb%u;\n", d,
                            c_int_const(dt, t->values[i].lo, t->values[i].hi), t->targets[i]);
                fprintf(out, "    goto bb%u;\n", t->target);
            } else {
                fprintf(out, "    switch (%s) {\n", d);
                for (uint32_t i = 0; i < t->ntargets; i++)
                    fprintf(out, "    case %s: goto bb%u;\n",
                            c_int_const(dt, t->values[i].lo, t->values[i].hi), t->targets[i]);
                fprintf(out, "    default: goto bb%u;\n    }\n", t->target);
            }
            break;
        }
        case TM_RETURN:
            if (ret_is_void(inst))
                fprintf(out, "    return;\n");
            else if (is_zst_like(mir->locals[0].ty))
                fprintf(out, "    return %s;\n", c_zst_value(mir->locals[0].ty));
            else
                fprintf(out, "    return _0;\n");
            break;
        case TM_UNREACHABLE: case TM_NONE:
            fprintf(out, "    rcc_unreachable();\n");
            break;
        }
    }
    fprintf(out, "}\n");
}

/* ------------------------------------------------------------------ */
/* Drop glue                                                           */
/* ------------------------------------------------------------------ */

static void glue_field(Type *ft, const char *lv) {
    if (!ty_needs_drop(ft))
        return;
    if (is_zst_like(ft))
        fprintf(out, "    %s((%s *)&%s);\n", drop_fn_name(ft), ctype(ft), c_zst_value(ft));
    else
        fprintf(out, "    %s(&%s);\n", drop_fn_name(ft), lv);
}

static void emit_drop_glue(Type *t) {
    if (t->kind == T_SLICE || t->kind == T_DYN || t->kind == T_STR)
        return;
    Span sp = {0};
    fprintf(out, "\n/* drop glue for %s */\n%svoid %s(%s *p) {\n", ty_to_str(t), linkage, drop_fn_name(t),
            ctype(t));
    switch ((TypeKind)t->kind) {
    case T_TUPLE:
        for (uint32_t i = 0; i < t->substs->len; i++)
            glue_field(t->substs->t[i], arena_printf("p->f%u", i));
        break;
    case T_ARRAY:
        if (t->len->cval)
            fprintf(out, "    { usize i; for (i = 0; i < %lluull; i++) %s(&p->a[i]); }\n",
                    (unsigned long long)t->len->cval, drop_fn_name(t->elem));
        break;
    case T_CLOSURE: {
        ClosureInfo *ci = def_get(t->def)->closure;
        for (uint32_t i = 0; i < ci->ncaptures; i++)
            if (ci->captures[i].mode == CAP_VALUE)
                glue_field(ty_field_type(t, 0, i, sp), arena_printf("p->f%u", i));
        break;
    }
    case T_ADT: {
        if (is_box(t)) {
            Type *pointee = t->substs->t[0];
            if (pointee->kind == T_DYN) {
                fprintf(out, "    if (p->f0.vt[0]) ((void (*)(void *))p->f0.vt[0])(p->f0.ptr);\n");
                fprintf(out, "    rcc_box_free(p->f0.ptr);\n");
            } else if (pointee->kind == T_SLICE || pointee->kind == T_STR) {
                if (pointee->kind == T_SLICE && ty_needs_drop(pointee->elem))
                    fprintf(out, "    { usize i; for (i = 0; i < p->f0.len; i++) %s(&p->f0.ptr[i]); }\n",
                            drop_fn_name(pointee->elem));
                fprintf(out, "    rcc_box_free((void *)p->f0.ptr);\n");
            } else {
                if (ty_needs_drop(pointee))
                    fprintf(out, "    %s(p->f0);\n", drop_fn_name(pointee));
                fprintf(out, "    rcc_box_free((void *)p->f0);\n");
            }
            break;
        }
        Instance *di = mono_drop_impl(t, sp);
        if (di)
            fprintf(out, "    %s(p);\n", di->name);
        AdtDef *adt = adt_def(t->def);
        if (adt->is_union)
            break;
        if (!adt->is_enum) {
            for (uint32_t f = 0; f < adt->variants[0].nfields; f++)
                glue_field(ty_field_type(t, 0, f, sp), arena_printf("p->f%u", f));
            break;
        }
        adt_compute_discs(adt);
        fprintf(out, "    switch (p->tag) {\n");
        for (uint32_t v = 0; v < adt->nvariants; v++) {
            bool any = false;
            for (uint32_t f = 0; f < adt->variants[v].nfields; f++)
                any |= ty_needs_drop(ty_field_type(t, v, f, sp));
            if (!any)
                continue;
            int64_t d = adt->variants[v].disc;
            fprintf(out, "    case %s:\n", c_int_const(enum_tag_type(t), (uint64_t)d, d < 0 ? ~0ull : 0));
            for (uint32_t f = 0; f < adt->variants[v].nfields; f++)
                glue_field(ty_field_type(t, v, f, sp), arena_printf("p->u.v%u.f%u", v, f));
            fprintf(out, "        break;\n");
        }
        fprintf(out, "    default: break;\n    }\n");
        break;
    }
    default:
        break;
    }
    fprintf(out, "}\n");
}

/* ------------------------------------------------------------------ */
/* Top level                                                           */
/* ------------------------------------------------------------------ */

/* Fn pointer typedefs may mention other fn pointer types: emit those first. */
static void emit_fnptr_typedef(FILE *f, CTypeInfo *ci) {
    if (ci->state)
        return;
    ci->state = 2;
    Type *t = ci->ty;
    for (uint32_t k = 0; k < t->substs->len; k++) {
        Type *c = t->substs->t[k];
        while (c->kind == T_REF || c->kind == T_PTR)
            c = c->elem;
        if (c->kind == T_FNPTR)
            emit_fnptr_typedef(f, ctype_info(c));
    }
    fprintf(f, "typedef %s (*%s)(", ctype(fnptr_ret(t)), ci->name);
    for (uint32_t k = 0; k + 1 < t->substs->len; k++)
        fprintf(f, "%s%s", k ? ", " : "", ctype(t->substs->t[k]));
    fprintf(f, "%s);\n", t->substs->len == 1 ? "void" : "");
}

static void copy_file(FILE *dst, FILE *src) {
    char buf[65536];
    size_t n;
    rewind(src);
    while ((n = fread(buf, 1, sizeof buf, src)) > 0)
        fwrite(buf, 1, n, dst);
}

/*
 * Emit the program. With nunits == 1 everything goes to units[0] as one
 * translation unit (functions are `static`). Otherwise `header` receives
 * the shared prelude (types, prototypes, extern globals) and every unit
 * gets `#include "<header_name>"` plus its share of the function bodies;
 * unit 0 also defines the globals, the vtables and `main`.
 */
static void cgen_emit(Program *p, FILE *header, const char *header_name, FILE **units, uint32_t nunits) {
    prog = p;
    bool split = nunits > 1;
    linkage = split ? "" : "static ";
    const char *im = getenv("RCC_INLINE_MAX");
    if (im && *im)
        inline_max = (uint32_t)atoi(im);
    FILE **bodies = NEW_ARRAY(FILE *, nunits);
    for (uint32_t u = 0; u < nunits; u++)
        if (!(bodies[u] = tmpfile()))
            fatal((Span){0}, "cannot create temporary files for code generation");
    FILE *decls = tmpfile(), *defs = tmpfile();
    if (!decls || !defs)
        fatal((Span){0}, "cannot create temporary files for code generation");

    /* Bodies first: they discover types. */
    for (uint32_t i = 0; i < p->drop_glue.len; i++) {
        out = bodies[i % nunits];
        emit_drop_glue(p->drop_glue.data[i]);
    }
    FILE *inl = tmpfile();
    if (!inl)
        fatal((Span){0}, "cannot create temporary files for code generation");
    uint32_t nb = 0;
    for (uint32_t i = 0; i < p->instances.len; i++)
        if (p->instances.data[i]->mir) {
            if (split && is_small(p->instances.data[i]))
                out = inl;
            else
                out = bodies[nb++ % nunits];
            emit_function(p->instances.data[i]);
        }
    out = bodies[0];
    fprintf(out, "\nint main(int argc, char **argv) {\n    rcc_rt_init(argc, argv);\n");
    for (uint32_t i = 0; i < p->init_order.len; i++)
        fprintf(out, "    %s = %s();\n", p->init_order.data[i]->name, p->init_order.data[i]->init->name);
    if (p->start) {
        fprintf(out, "    return (int)%s((%s)%s);\n}\n", p->start->name, ctype(p->start->params[0]),
                p->user_main->name);
    } else {
        fprintf(out, "    (void)%s();\n    return 0;\n}\n", p->user_main->name);
    }

    /* Prototypes and declarations shared by all units (may still discover
       types); definitions of globals and vtables go to `defs`. */
    out = decls;
    for (uint32_t i = 0; i < p->drop_glue.len; i++) {
        Type *t = p->drop_glue.data[i];
        if (t->kind == T_SLICE || t->kind == T_DYN || t->kind == T_STR)
            continue;
        fprintf(out, "%svoid %s(%s *p);\n", linkage, drop_fn_name(t), ctype(t));
    }
    for (uint32_t i = 0; i < p->instances.len; i++) {
        emit_proto(out, p->instances.data[i]);
        fprintf(out, ";\n");
    }
    fprintf(out, "\n");
    for (uint32_t i = 0; i < p->globals.len; i++) {
        Global *g = p->globals.data[i];
        const char *what = g->def ? def_path_str(g->def) : "promoted";
        if (g->is_foreign || split)
            fprintf(decls, "extern %s %s; /* %s */\n", ctype(g->ty), g->name, what);
        if (!g->is_foreign)
            fprintf(defs, "%s%s %s; /* %s */\n", linkage, ctype(g->ty), g->name, what);
    }
    fprintf(out, "\n");
    for (uint32_t i = 0; i < p->vtables.len; i++) {
        Vtable *vt = p->vtables.data[i];
        if (split)
            fprintf(decls, "extern const void *const %s[];\n", vt->name);
        fprintf(defs, "/* vtable: %s as %s */\n%sconst void *const %s[] = {\n", ty_to_str(vt->concrete),
                ty_to_str(vt->dyn), linkage, vt->name);
        if (ty_needs_drop(vt->concrete))
            fprintf(defs, "    (void *)%s,\n", drop_fn_name(vt->concrete));
        else
            fprintf(defs, "    0,\n");
        fprintf(defs, "    (void *)sizeof(%s), (void *)_Alignof(%s),\n", ctype(vt->concrete),
                ctype(vt->concrete));
        for (uint32_t k = 0; k < vt->nslots; k++) {
            if (vt->slots[k])
                fprintf(defs, "    (void *)%s,\n", vt->slots[k]->name);
            else
                fprintf(defs, "    0,\n");
        }
        fprintf(defs, "};\n");
    }
    /* zero-sized constants: one private copy per unit is fine */
    for (uint32_t i = 0; i < zst_objects.len; i++)
        fprintf(decls, "static const %s rcc_zst_%u;\n", ctype(zst_objects.data[i]), zst_objects.data[i]->id);

    /* Types (complete everything that was mentioned). */
    for (uint32_t i = 0; i < ctypes.len; i++)
        if (ctypes.data[i]->is_struct)
            define_struct(ctypes.data[i]);
    FILE *final = split ? header : units[0];
    fprintf(final, "/* Generated by rcc. Do not edit. */\n#include \"rcc_rt.h\"\n\n");
    for (uint32_t i = 0; i < ctypes.len; i++) {
        CTypeInfo *ci = ctypes.data[i];
        if (ci->is_struct || ci->is_slice)
            fprintf(final, "typedef struct %s %s;\n", ci->name, ci->name);
    }
    for (uint32_t i = 0; i < ctypes.len; i++)
        if (ctypes.data[i]->is_fnptr)
            emit_fnptr_typedef(final, ctypes.data[i]);
    for (uint32_t i = 0; i < ctypes.len; i++) {
        CTypeInfo *ci = ctypes.data[i];
        if (ci->is_slice)
            fprintf(final, "struct %s { %s *ptr; usize len; };\n", ci->name, ctype(ci->ty->elem));
    }

    fprintf(final, "\n%s\n", sb_str(&type_defs));
    copy_file(final, decls);
    copy_file(final, inl);
    fclose(inl);
    for (uint32_t u = 0; u < nunits; u++) {
        if (split)
            fprintf(units[u], "/* Generated by rcc. Do not edit. */\n#include \"%s\"\n\n", header_name);
        if (u == 0)
            copy_file(units[u], defs);
        copy_file(units[u], bodies[u]);
        fclose(bodies[u]);
    }
    fclose(decls);
    fclose(defs);
}

void cgen_program(Program *p, FILE *final) { cgen_emit(p, NULL, NULL, &final, 1); }

void cgen_program_split(Program *p, FILE *header, const char *header_name, FILE **units, uint32_t nunits) {
    cgen_emit(p, header, header_name, units, nunits);
}
