/*
 * typeck_pat.c - patterns.
 *
 * Default binding modes (match ergonomics): when a non-reference pattern
 * (tuple, struct, variant, slice, literal, range, path) meets a value of
 * reference type, the reference is looked through (an explicit TP_DEREF
 * node is produced) and the default binding mode becomes `ref` (or
 * `ref mut` for `&mut` when not already `ref`). Identifier patterns then
 * bind with that mode; an explicit `&pat` resets it to by-value.
 */
#include "typeck_fn.h"

typedef struct PatCtx {
    FnCtxt *fcx;
    HMap names; /* Sym -> Local* */
    VEC(Local *) order;
} PatCtx;

static TPat *tp_new(TPatKind k, Span sp, Type *ty) {
    TPat *p = NEW(TPat);
    p->kind = (uint8_t)k;
    p->span = sp;
    p->ty = ty;
    return p;
}

static TPat *check_pat(PatCtx *pc, Pat *p, Type *expected, BindMode bm);

/* Look through references on `*expected`, updating the binding mode.
   Returns the number of layers; layer types are stored in `layers`. */
static int peel_refs(PatCtx *pc, Type **expected, BindMode *bm, Type **layers, int max) {
    int n = 0;
    for (;;) {
        Type *t = structurally_resolve(pc->fcx, *expected);
        if (t->kind != T_REF || n == max)
            break;
        layers[n++] = t;
        if (!t->is_mut)
            *bm = BIND_REF;
        else if (*bm == BIND_VALUE)
            *bm = BIND_REF_MUT;
        *expected = t->elem;
    }
    return n;
}

static TPat *wrap_derefs(TPat *p, Type **layers, int n) {
    for (int i = n - 1; i >= 0; i--) {
        TPat *d = tp_new(TP_DEREF, p->span, layers[i]);
        d->sub = p;
        p = d;
    }
    return p;
}

static TPat *check_binding(PatCtx *pc, Pat *p, Type *expected, BindMode bm) {
    FnCtxt *fcx = pc->fcx;
    BindMode mode = bm;
    if (p->ident.by_ref)
        mode = p->ident.is_mut ? BIND_REF_MUT : BIND_REF;
    else if (p->ident.is_mut)
        mode = BIND_VALUE; /* `mut x` resets the default binding mode */
    Type *lty = mode == BIND_VALUE ? expected : ty_ref(expected, mode == BIND_REF_MUT);
    Local *l = hmap_get(&pc->names, p->ident.name);
    if (l) {
        unify_or_fatal(fcx, l->ty, lty, p->span);
    } else {
        l = new_local(fcx, p->ident.name, lty, !p->ident.by_ref && p->ident.is_mut, p->span);
        hmap_put(&pc->names, p->ident.name, l);
        vec_push(pc->order, l);
    }
    TPat *b = tp_new(TP_BIND, p->span, expected);
    b->local = l;
    b->bind_mode = (uint8_t)mode;
    if (p->ident.sub)
        b->sub = check_pat(pc, p->ident.sub, expected, bm);
    return b;
}

/* A path in pattern position that names a unit struct/variant or a const. */
static TPat *check_path_pat(PatCtx *pc, QSelf *qself, Path *path, Type *expected, BindMode bm,
                            Span sp) {
    FnCtxt *fcx = pc->fcx;
    TExpr *v = instantiate_value_path(fcx, qself, path, sp);
    Type *layers[8];
    int n = 0;
    Type *vt = infer_shallow(fcx->icx, v->ty);
    if (vt->kind != T_REF)
        n = peel_refs(pc, &expected, &bm, layers, 8);
    unify_or_fatal(fcx, expected, v->ty, sp);
    TPat *r;
    if (v->kind == TE_ADT) {
        AdtDef *adt = adt_def(vt->def);
        r = tp_new(adt->is_enum ? TP_VARIANT : TP_TUPLE, sp, expected);
        r->adt = vt->def;
        r->variant = v->variant;
    } else if (v->kind == TE_CONST) {
        r = tp_new(TP_CONST, sp, expected);
        r->value = v;
    } else {
        fatal(sp, "expected a unit struct, unit variant or constant in pattern");
    }
    return wrap_derefs(r, layers, n);
}

/* Sub-patterns of a tuple-like pattern with an optional `..`. `nfields` is
   the arity of the matched value. Fills subs/field_idx. */
static void check_positional(PatCtx *pc, PatList *elems, uint32_t nfields, Type *(*field_ty)(void *, uint32_t),
                             void *ctx, BindMode bm, TPat *out, Span sp) {
    int rest = -1;
    for (uint32_t i = 0; i < elems->len; i++)
        if (elems->data[i]->kind == PAT_REST) {
            if (rest >= 0)
                fatal(sp, "`..` can only be used once per pattern");
            rest = (int)i;
        }
    uint32_t npats = elems->len - (rest >= 0 ? 1 : 0);
    if (rest < 0 ? npats != nfields : npats > nfields)
        fatal(sp, "this pattern has %u field(s), but the value has %u", npats, nfields);
    out->subs = NEW_ARRAY(TPat *, npats ? npats : 1);
    out->field_idx = NEW_ARRAY(uint32_t, npats ? npats : 1);
    out->nsubs = npats;
    uint32_t k = 0;
    for (uint32_t i = 0; i < elems->len; i++) {
        if ((int)i == rest)
            continue;
        uint32_t idx = (rest >= 0 && (int)i > rest) ? nfields - (elems->len - i) : i;
        out->field_idx[k] = idx;
        out->subs[k] = check_pat(pc, elems->data[i], field_ty(ctx, idx), bm);
        k++;
    }
}

typedef struct VariantFieldCtx {
    FnCtxt *fcx;
    VariantDef *v;
    Substs *substs;
    Span sp;
} VariantFieldCtx;

static Type *variant_field_ty(void *ctx, uint32_t i) {
    VariantFieldCtx *c = ctx;
    return fcx_normalize(c->fcx, ty_subst(c->v->fields[i].ty, c->substs), c->sp);
}

static Type *tuple_field_ty(void *ctx, uint32_t i) { return ((Type *)ctx)->substs->t[i]; }

static TPat *check_pat(PatCtx *pc, Pat *p, Type *expected, BindMode bm) {
    FnCtxt *fcx = pc->fcx;
    InferCtxt *icx = fcx->icx;
    Type *layers[8];
    switch (p->kind) {
    case PAT_WILD:
        return tp_new(TP_WILD, p->span, expected);
    case PAT_IDENT: {
        if (!p->ident.by_ref && !p->ident.is_mut && !p->ident.sub) {
            Res r = resolve_ident(fcx->scope, p->ident.name, NS_VALUE);
            if (r.kind == RES_DEF) {
                DefKind k = def_get(r.def)->kind;
                bool unit_ctor = false;
                if (k == DEF_STRUCT)
                    unit_ctor = adt_def(r.def)->variants[0].ctor == VD_UNIT;
                else if (k == DEF_VARIANT)
                    unit_ctor = adt_def(r.def)->variants[def_get(r.def)->index].ctor == VD_UNIT;
                if (unit_ctor || k == DEF_CONST || k == DEF_ASSOC_CONST)
                    return check_path_pat(pc, NULL, ast_path_from_sym(p->ident.name, p->span), expected,
                                          bm, p->span);
            }
        }
        return check_binding(pc, p, expected, bm);
    }
    case PAT_LIT: {
        Expr *le = p->lit;
        bool is_str = le->kind == EXPR_LIT && (le->lit->kind == LIT_STR || le->lit->kind == LIT_BYTE_STR);
        int n = 0;
        if (is_str) {
            /* a string literal has type &str: look through outer references */
            for (;;) {
                Type *t = structurally_resolve(fcx, expected);
                if (t->kind != T_REF || n == 8)
                    break;
                Type *inner = structurally_resolve(fcx, t->elem);
                if (inner->kind != T_REF)
                    break;
                layers[n++] = t;
                expected = t->elem;
            }
        } else {
            n = peel_refs(pc, &expected, &bm, layers, 8);
        }
        TExpr *v = check_expr(fcx, le, expected);
        Type *et = structurally_resolve(fcx, expected);
        Type *vt = infer_shallow(icx, v->ty);
        if (le->kind == EXPR_LIT && le->lit->kind == LIT_BYTE_STR && et->kind == T_REF &&
            structurally_resolve(fcx, et->elem)->kind == T_SLICE) {
            /* b"..." against &[u8] */
            TExpr *u = te_new(TE_UNSIZE, v->span, et);
            u->a = v;
            v = u;
        } else {
            unify_or_fatal(fcx, expected, vt, p->span);
        }
        TPat *r = tp_new(TP_CONST, p->span, expected);
        r->value = v;
        return wrap_derefs(r, layers, n);
    }
    case PAT_RANGE: {
        int n = peel_refs(pc, &expected, &bm, layers, 8);
        TPat *r = tp_new(TP_RANGE, p->span, expected);
        r->inclusive = p->range.end == RANGE_INCLUSIVE;
        if (p->range.lo)
            r->lo = coerce(fcx, check_expr(fcx, p->range.lo, expected), expected);
        if (p->range.hi)
            r->hi = coerce(fcx, check_expr(fcx, p->range.hi, expected), expected);
        return wrap_derefs(r, layers, n);
    }
    case PAT_PATH:
        return check_path_pat(pc, p->path.qself, p->path.path, expected, bm, p->span);
    case PAT_TUPLE: {
        int n = peel_refs(pc, &expected, &bm, layers, 8);
        Type *t = structurally_resolve(fcx, expected);
        if (t->kind == T_INFER) {
            uint32_t k = p->elems.len;
            Type **v = NEW_ARRAY(Type *, k ? k : 1);
            for (uint32_t i = 0; i < k; i++) {
                if (p->elems.data[i]->kind == PAT_REST)
                    fatal(p->span, "type annotations needed for tuple pattern with `..`");
                v[i] = infer_var(icx, INFER_TY, p->span);
            }
            Type *nt = ty_tuple(substs_new(v, k));
            unify_or_fatal(fcx, t, nt, p->span);
            t = nt;
        }
        if (t->kind != T_TUPLE)
            fatal(p->span, "mismatched types: expected `%s`, found a tuple pattern", fcx_ty_str(fcx, t));
        TPat *r = tp_new(TP_TUPLE, p->span, expected);
        check_positional(pc, &p->elems, t->substs->len, tuple_field_ty, t, bm, r, p->span);
        return wrap_derefs(r, layers, n);
    }
    case PAT_TUPLE_STRUCT:
    case PAT_STRUCT: {
        bool is_struct = p->kind == PAT_STRUCT;
        uint32_t variant;
        DefId ctor;
        Type *aty = resolve_variant_path(fcx, is_struct ? p->struct_.qself : p->tuple_struct.qself,
                                         is_struct ? p->struct_.path : p->tuple_struct.path,
                                         is_struct ? NS_TYPE : NS_VALUE, &variant, &ctor, p->span);
        int n = peel_refs(pc, &expected, &bm, layers, 8);
        unify_or_fatal(fcx, expected, aty, p->span);
        AdtDef *adt = adt_def(aty->def);
        VariantDef *v = &adt->variants[variant];
        TPat *r = tp_new(adt->is_enum ? TP_VARIANT : TP_TUPLE, p->span, expected);
        r->adt = aty->def;
        r->variant = variant;
        VariantFieldCtx vc = {fcx, v, aty->substs, p->span};
        if (!is_struct) {
            check_positional(pc, &p->tuple_struct.elems, v->nfields, variant_field_ty, &vc, bm, r,
                             p->span);
        } else {
            uint32_t k = p->struct_.fields.len;
            r->subs = NEW_ARRAY(TPat *, k ? k : 1);
            r->field_idx = NEW_ARRAY(uint32_t, k ? k : 1);
            r->nsubs = k;
            for (uint32_t i = 0; i < k; i++) {
                PatField *f = p->struct_.fields.data[i];
                int idx = variant_field_index(v, f->name);
                if (idx < 0)
                    fatal(f->span, "`%s` has no field named `%s`", sym_str(v->name), sym_str(f->name));
                r->field_idx[i] = (uint32_t)idx;
                r->subs[i] = check_pat(pc, f->pat, variant_field_ty(&vc, (uint32_t)idx), bm);
            }
        }
        return wrap_derefs(r, layers, n);
    }
    case PAT_SLICE: {
        int n = peel_refs(pc, &expected, &bm, layers, 8);
        Type *t = resolve_or_fatal(fcx, expected, p->span);
        if (t->kind != T_ARRAY && t->kind != T_SLICE)
            fatal(p->span, "mismatched types: expected `%s`, found a slice pattern", fcx_ty_str(fcx, t));
        int rest = -1;
        for (uint32_t i = 0; i < p->elems.len; i++) {
            Pat *el = p->elems.data[i];
            if (el->kind == PAT_REST ||
                (el->kind == PAT_IDENT && el->ident.sub && el->ident.sub->kind == PAT_REST)) {
                if (rest >= 0)
                    fatal(p->span, "`..` can only be used once per slice pattern");
                rest = (int)i;
            }
        }
        uint32_t npats = p->elems.len - (rest >= 0 ? 1 : 0);
        TPat *r = tp_new(TP_SLICE, p->span, expected);
        r->subs = NEW_ARRAY(TPat *, npats ? npats : 1);
        r->nsubs = npats;
        r->nprefix = rest >= 0 ? (uint32_t)rest : npats;
        r->has_rest = rest >= 0;
        uint32_t k = 0;
        for (uint32_t i = 0; i < p->elems.len; i++) {
            Pat *el = p->elems.data[i];
            if ((int)i != rest) {
                r->subs[k++] = check_pat(pc, el, t->elem, bm);
                continue;
            }
            if (el->kind != PAT_IDENT)
                continue;
            Type *rest_ty;
            if (t->kind == T_SLICE) {
                rest_ty = t;
            } else {
                Type *len = infer_shallow(icx, t->len);
                if (len->kind != T_CONST || len->cval < npats)
                    fatal(p->span, "array length must be known for a `..` binding");
                rest_ty = ty_array(t->elem, ty_const(len->cval - npats));
            }
            Pat tmp = *el;
            tmp.ident.sub = NULL;
            r->rest = check_binding(pc, &tmp, rest_ty, bm);
        }
        if (t->kind == T_ARRAY) {
            Type *len = infer_shallow(icx, t->len);
            if (len->kind == T_INFER && rest < 0)
                unify_or_fatal(fcx, len, ty_const(npats), p->span);
        }
        return wrap_derefs(r, layers, n);
    }
    case PAT_REST:
        fatal(p->span, "`..` is not allowed here");
    case PAT_REF: {
        Type *t = structurally_resolve(fcx, expected);
        if (t->kind == T_INFER) {
            Type *nt = ty_ref(infer_var(icx, INFER_TY, p->span), p->ref.is_mut);
            unify_or_fatal(fcx, t, nt, p->span);
            t = nt;
        }
        if (t->kind != T_REF)
            fatal(p->span, "mismatched types: expected `%s`, found a reference pattern",
                  fcx_ty_str(fcx, t));
        TPat *d = tp_new(TP_DEREF, p->span, expected);
        d->sub = check_pat(pc, p->ref.pat, t->elem, BIND_VALUE);
        return d;
    }
    case PAT_OR: {
        TPat *r = tp_new(TP_OR, p->span, expected);
        r->subs = NEW_ARRAY(TPat *, p->elems.len);
        r->nsubs = p->elems.len;
        for (uint32_t i = 0; i < p->elems.len; i++)
            r->subs[i] = check_pat(pc, p->elems.data[i], expected, bm);
        return r;
    }
    case PAT_MAC:
        fatal(p->span, "unexpanded macro in pattern position");
    }
    fatal(p->span, "internal: unknown pattern kind");
}

TPat *check_pat_top(FnCtxt *fcx, Pat *p, Type *expected) {
    PatCtx pc = {fcx, {0}, {0}};
    TPat *r = check_pat(&pc, p, expected, BIND_VALUE);
    if (fcx->scope->kind != SCOPE_LOCALS)
        fatal(p->span, "internal: pattern checked outside of a locals scope");
    for (uint32_t i = 0; i < pc.order.len; i++)
        scope_bind(fcx->scope, pc.order.data[i]->name, pc.order.data[i]);
    return r;
}

int pat_use_kind(FnCtxt *fcx, const TPat *p) {
    int u = 0;
    if (!p)
        return 0;
    if (p->kind == TP_BIND) {
        if (p->bind_mode == BIND_REF_MUT)
            u = 1;
        else if (p->bind_mode == BIND_VALUE && !fcx_is_copy(fcx, p->ty))
            u = 2;
    }
    int s = pat_use_kind(fcx, p->sub);
    if (s > u)
        u = s;
    s = pat_use_kind(fcx, p->rest);
    if (s > u)
        u = s;
    for (uint32_t i = 0; i < p->nsubs; i++) {
        s = pat_use_kind(fcx, p->subs[i]);
        if (s > u)
            u = s;
    }
    return u;
}
