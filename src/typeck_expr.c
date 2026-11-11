/*
 * typeck_expr.c - type checking of expressions, statements and blocks;
 * coercions; calls; operators; the `for` and `?` desugarings.
 * See typeck_fn.h for the conventions.
 */
#include "typeck_fn.h"

#include <stdio.h>
#include <stdlib.h>

/* ------------------------------------------------------------------ */
/* Small helpers                                                       */
/* ------------------------------------------------------------------ */

TExpr *te_new(TExprKind kind, Span sp, Type *ty) {
    TExpr *e = NEW(TExpr);
    e->kind = (uint8_t)kind;
    e->span = sp;
    e->ty = ty;
    return e;
}

static TExpr *te1(TExprKind kind, Span sp, Type *ty, TExpr *a) {
    TExpr *e = te_new(kind, sp, ty);
    e->a = a;
    return e;
}

TExpr *make_ref(Span sp, TExpr *a, bool is_mut) {
    TExpr *e = te1(TE_REF, sp, ty_ref(a->ty, is_mut), a);
    e->is_mut = is_mut;
    return e;
}

TExpr *make_call(Span sp, DefId def, Substs *substs, Type *ty, TExpr **args, uint32_t nargs) {
    TExpr *callee = te_new(TE_FN, sp, ty_fndef(def, substs));
    callee->item.def = def;
    callee->item.substs = substs;
    TExpr *e = te1(TE_CALL, sp, ty, callee);
    e->args = args;
    e->nargs = nargs;
    return e;
}

static TExpr **args1(TExpr *a) {
    TExpr **v = NEW_ARRAY(TExpr *, 1);
    v[0] = a;
    return v;
}

static TExpr **args2(TExpr *a, TExpr *b) {
    TExpr **v = NEW_ARRAY(TExpr *, 2);
    v[0] = a;
    v[1] = b;
    return v;
}

const char *fcx_ty_str(FnCtxt *fcx, Type *t) { return ty_to_str(infer_resolve(fcx->icx, t)); }

Type *fcx_normalize(FnCtxt *fcx, Type *t, Span sp) { return infer_normalize(fcx->icx, t, sp); }

Type *structurally_resolve(FnCtxt *fcx, Type *t) {
    t = infer_shallow(fcx->icx, t);
    if (t->kind == T_INFER && t->ik == INFER_TY) {
        infer_select_pending(fcx->icx);
        t = infer_shallow(fcx->icx, t);
    }
    return t;
}

Type *resolve_or_fatal(FnCtxt *fcx, Type *t, Span sp) {
    t = structurally_resolve(fcx, t);
    if (t->kind == T_INFER && t->ik == INFER_TY)
        fatal(sp, "type annotations needed: the type of this expression must be known here");
    return t;
}

void unify_or_fatal(FnCtxt *fcx, Type *expected, Type *actual, Span sp) {
    if (!infer_unify(fcx->icx, expected, actual))
        fatal(sp, "mismatched types: expected `%s`, found `%s`", fcx_ty_str(fcx, expected),
              fcx_ty_str(fcx, actual));
}

Local *new_local(FnCtxt *fcx, Sym name, Type *ty, bool is_mut, Span sp) {
    Local *l = NEW(Local);
    l->id = fcx->locals.len;
    l->name = name;
    l->ty = ty;
    l->is_mut = is_mut;
    l->closure = fcx->closure;
    l->span = sp;
    vec_push(fcx->locals, l);
    return l;
}

Type *lower_body_ty(FnCtxt *fcx, Ty *ast) {
    fcx->lc.scope = fcx->scope;
    return fcx_normalize(fcx, lower_ty(&fcx->lc, ast), ast->span);
}

bool fcx_is_copy(FnCtxt *fcx, Type *t) {
    t = infer_resolve(fcx->icx, t);
    switch ((TypeKind)t->kind) {
    case T_BOOL: case T_CHAR: case T_INT: case T_FLOAT: case T_NEVER:
    case T_PTR: case T_FNPTR: case T_FNDEF: case T_INFER:
        return true;
    case T_REF:
        return !t->is_mut;
    case T_STR: case T_SLICE: case T_DYN:
        return false;
    case T_TUPLE:
        for (uint32_t i = 0; i < t->substs->len; i++)
            if (!fcx_is_copy(fcx, t->substs->t[i]))
                return false;
        return true;
    case T_ARRAY:
        return fcx_is_copy(fcx, t->elem);
    default:
        break;
    }
    if (!lang.copy)
        return false;
    Predicate p = {PRED_TRAIT, {lang.copy, substs_1(t)}, 0, NULL};
    return trait_eval(fcx->icx, &p) == EVAL_YES;
}

static DefId m_deref, m_deref_mut, m_index, m_index_mut;

static void init_method_defs(void) {
    if (m_deref || !lang.deref)
        return;
    m_deref = trait_item(lang.deref, sym_intern_cstr("deref"));
    if (lang.deref_mut)
        m_deref_mut = trait_item(lang.deref_mut, sym_intern_cstr("deref_mut"));
    if (lang.index)
        m_index = trait_item(lang.index, sym_intern_cstr("index"));
    if (lang.index_mut)
        m_index_mut = trait_item(lang.index_mut, sym_intern_cstr("index_mut"));
}

static bool is_box(Type *t) { return t->kind == T_ADT && t->def == lang.box && lang.box; }

Type *autoderef_ty(FnCtxt *fcx, Type *t, Span sp) {
    t = structurally_resolve(fcx, t);
    switch ((TypeKind)t->kind) {
    case T_REF:
        return t->elem;
    case T_ADT:
        if (is_box(t))
            return t->substs->t[0];
        break;
    case T_PARAM: case T_PROJ: case T_OPAQUE:
        break;
    default:
        return NULL;
    }
    return infer_overloaded_deref(fcx->icx, t, sp);
}

TExpr *deref_once(FnCtxt *fcx, TExpr *e) {
    Type *t = structurally_resolve(fcx, e->ty);
    if (t->kind == T_REF)
        return te1(TE_DEREF, e->span, t->elem, e);
    if (is_box(t))
        return te1(TE_DEREF, e->span, t->substs->t[0], e);
    if (t->kind != T_ADT && t->kind != T_PARAM && t->kind != T_PROJ && t->kind != T_OPAQUE)
        return NULL;
    Type *target = infer_overloaded_deref(fcx->icx, t, e->span);
    if (!target)
        return NULL;
    init_method_defs();
    TExpr *call = make_call(e->span, m_deref, substs_1(t), ty_ref(target, false),
                            args1(make_ref(e->span, e, false)), 1);
    return te1(TE_DEREF, e->span, target, call);
}

static bool is_call_of(const TExpr *e, DefId method) {
    return e->kind == TE_CALL && e->a->kind == TE_FN && e->a->item.def == method && method;
}

void fix_place_mut(FnCtxt *fcx, TExpr *e) {
    init_method_defs();
    switch ((TExprKind)e->kind) {
    case TE_FIELD:
    case TE_INDEX:
        fix_place_mut(fcx, e->a);
        break;
    case TE_DEREF: {
        TExpr *c = e->a;
        DefId to = 0;
        if (is_call_of(c, m_deref))
            to = m_deref_mut;
        else if (is_call_of(c, m_index))
            to = m_index_mut;
        if (to) {
            c->a->item.def = to;
            c->a->ty = ty_fndef(to, c->a->item.substs);
            c->ty = ty_ref(infer_shallow(fcx->icx, c->ty)->elem, true);
            TExpr *r = c->args[0];
            if (r->kind == TE_REF && !r->is_mut) {
                r->is_mut = true;
                r->ty = ty_ref(r->a->ty, true);
                fix_place_mut(fcx, r->a);
            }
        } else if (is_box(infer_shallow(fcx->icx, c->ty))) {
            fix_place_mut(fcx, c);
        }
        break;
    }
    default:
        break;
    }
}

/* ------------------------------------------------------------------ */
/* Coercion                                                            */
/* ------------------------------------------------------------------ */

/* Register `src: dyn-Trait` obligations for an unsizing coercion. */
static void register_dyn_obligations(FnCtxt *fcx, Type *src, Type *dyn, Span sp) {
    TraitRef tr = {dyn->def, substs_concat(substs_1(src), dyn->substs)};
    Predicate p = {PRED_TRAIT, tr, 0, NULL};
    infer_register(fcx->icx, p, sp);
    if (dyn->nbinds) {
        PredList sup = {0};
        supertraits_of(tr, &sup);
        for (uint32_t i = 0; i < dyn->nbinds; i++) {
            DefId owner = def_get(dyn->binds[i].assoc)->parent;
            for (uint32_t k = 0; k < sup.len; k++) {
                if (sup.data[k].kind != PRED_TRAIT || sup.data[k].tr.trait != owner)
                    continue;
                Predicate pp = {PRED_PROJ, sup.data[k].tr, dyn->binds[i].assoc, dyn->binds[i].ty};
                infer_register(fcx->icx, pp, sp);
                break;
            }
        }
    }
}

/* Can a pointer to `se` be unsized to a pointer to `te`? Performs the
   needed unifications / registrations on success. */
static bool try_unsize(FnCtxt *fcx, Type *se, Type *te, Span sp) {
    se = structurally_resolve(fcx, se);
    te = structurally_resolve(fcx, te);
    if (te->kind == T_SLICE && se->kind == T_ARRAY)
        return infer_unify(fcx->icx, se->elem, te->elem);
    /* An unresolved source is assumed to be a sized type that will satisfy
       the trait (as rustc does); the obligation is checked later. */
    if (te->kind == T_DYN && se->kind != T_DYN && se->kind != T_SLICE && se->kind != T_STR) {
        register_dyn_obligations(fcx, se, te, sp);
        return true;
    }
    return false;
}

static Type *fndef_as_fnptr(FnCtxt *fcx, Type *t, Span sp) {
    FnSigT *sig = fn_sig(t->def);
    uint32_t n = sig->inputs->len;
    Type **v = NEW_ARRAY(Type *, n + 1);
    for (uint32_t i = 0; i < n; i++)
        v[i] = ty_subst(sig->inputs->t[i], t->substs);
    v[n] = ty_subst(sig->output, t->substs);
    return fcx_normalize(fcx, ty_fnptr(substs_new(v, n + 1)), sp);
}

static Type *closure_as_fnptr(Type *t) {
    ClosureInfo *ci = def_get(t->def)->closure;
    uint32_t n = ci->inputs->len;
    Type **v = NEW_ARRAY(Type *, n + 1);
    for (uint32_t i = 0; i < n; i++)
        v[i] = ty_subst(ci->inputs->t[i], t->substs);
    v[n] = ty_subst(ci->output, t->substs);
    return ty_fnptr(substs_new(v, n + 1));
}

static TExpr *coerce_inner(FnCtxt *fcx, TExpr *e, Type *target, bool hard) {
    InferCtxt *icx = fcx->icx;
    Type *src = infer_shallow(icx, e->ty), *tgt = infer_shallow(icx, target);
    if (src == tgt)
        return e;
    if (src->kind == T_NEVER) {
        if (tgt->kind == T_INFER)
            icx->vars[tgt->index].diverging = true;
        return te1(TE_NEVER_TO_ANY, e->span, target, e);
    }
    if (src->kind == T_INFER && src->ik == INFER_TY && icx->npending) {
        infer_select_pending(icx);
        src = infer_shallow(icx, e->ty);
        if (src == tgt)
            return e;
    }
    if (src->kind == T_INFER || tgt->kind == T_INFER)
        goto plain;
    if (tgt->kind == T_REF && src->kind == T_REF && (src->is_mut || !tgt->is_mut)) {
        if (try_unsize(fcx, src->elem, tgt->elem, e->span))
            return te1(TE_UNSIZE, e->span, tgt, e);
        if (src->is_mut == tgt->is_mut && infer_unify(icx, src, tgt))
            return e;
        TExpr *cur = te1(TE_DEREF, e->span, src->elem, e);
        for (int steps = 0; steps < 16; steps++) {
            if (infer_unify(icx, cur->ty, tgt->elem)) {
                if (tgt->is_mut)
                    fix_place_mut(fcx, cur);
                return make_ref(e->span, cur, tgt->is_mut);
            }
            TExpr *next = deref_once(fcx, cur);
            if (!next)
                break;
            cur = next;
        }
        goto fail;
    }
    if (tgt->kind == T_PTR && src->kind == T_REF && (src->is_mut || !tgt->is_mut) &&
        infer_unify(icx, src->elem, tgt->elem))
        return te1(TE_CAST, e->span, tgt, e);
    if (tgt->kind == T_PTR && src->kind == T_PTR && src->is_mut && !tgt->is_mut &&
        infer_unify(icx, src->elem, tgt->elem))
        return te1(TE_CAST, e->span, tgt, e);
    if (tgt->kind == T_ADT && src->kind == T_ADT && tgt->def == src->def &&
        (is_box(tgt) || (tgt->def == lang.rc && lang.rc) || (tgt->def == lang.arc && lang.arc)) &&
        try_unsize(fcx, src->substs->t[0], tgt->substs->t[0], e->span))
        return te1(TE_UNSIZE, e->span, tgt, e);
    if (tgt->kind == T_FNPTR && src->kind == T_FNDEF) {
        if (infer_unify(icx, fndef_as_fnptr(fcx, src, e->span), tgt))
            return te1(TE_REIFY, e->span, tgt, e);
        goto fail;
    }
    if (tgt->kind == T_FNPTR && src->kind == T_CLOSURE) {
        if (infer_unify(icx, closure_as_fnptr(src), tgt))
            return te1(TE_CLOSURE_FNPTR, e->span, tgt, e);
        goto fail;
    }
plain:
    if (infer_unify(icx, src, tgt))
        return e;
fail:
    if (hard)
        fatal(e->span, "mismatched types: expected `%s`, found `%s`", fcx_ty_str(fcx, target),
              fcx_ty_str(fcx, e->ty));
    return NULL;
}

TExpr *coerce(FnCtxt *fcx, TExpr *e, Type *target) { return coerce_inner(fcx, e, target, true); }

TExpr *try_coerce(FnCtxt *fcx, TExpr *e, Type *target) {
    Snapshot s = infer_snapshot(fcx->icx);
    TExpr *r = coerce_inner(fcx, e, target, false);
    if (!r)
        infer_rollback(fcx->icx, s);
    return r;
}

TExpr *check_expr_coerce(FnCtxt *fcx, Expr *e, Type *target) {
    return coerce(fcx, check_expr(fcx, e, target), target);
}

/*
 * Merging the types of several branches (if/else, match arms, break values,
 * array elements): the first non-diverging branch fixes the type (the
 * expected type, if it is a concrete sized type); later branches are
 * coerced to it.
 */
typedef struct CoerceMany {
    Type *merged;
    Type *expected;
    TExprVec prev; /* branches added so far (already coerced to `merged`) */
} CoerceMany;

static bool is_unsized_kind(Type *t) {
    return t->kind == T_DYN || t->kind == T_SLICE || t->kind == T_STR;
}

/* Coerce an already-added branch to `target` in place (the node keeps its
   address: its old contents move to a fresh node under the adjustment). */
static bool cm_recoerce(FnCtxt *fcx, TExpr *prev, Type *target) {
    TExpr *inner = NEW(TExpr);
    *inner = *prev;
    TExpr *c = try_coerce(fcx, inner, target);
    if (!c)
        return false;
    if (c == inner)
        return true;
    for (uint32_t i = 0; i < fcx->deferred_ops.len; i++)
        if (fcx->deferred_ops.data[i] == prev)
            fcx->deferred_ops.data[i] = inner;
    *prev = *c;
    return true;
}

static TExpr *cm_add_inner(FnCtxt *fcx, CoerceMany *cm, TExpr *e);

static TExpr *cm_add(FnCtxt *fcx, CoerceMany *cm, TExpr *e) {
    TExpr *r = cm_add_inner(fcx, cm, e);
    if (infer_shallow(fcx->icx, r->ty)->kind != T_NEVER)
        vec_push(cm->prev, r);
    return r;
}

static TExpr *cm_add_inner(FnCtxt *fcx, CoerceMany *cm, TExpr *e) {
    Type *t = infer_shallow(fcx->icx, e->ty);
    if (t->kind == T_NEVER)
        return e; /* wrapped by cm_finish */
    if (!cm->merged) {
        Type *x = cm->expected ? infer_shallow(fcx->icx, cm->expected) : NULL;
        if (x && x->kind != T_INFER && !is_unsized_kind(x)) {
            cm->merged = x;
            return coerce(fcx, e, x);
        }
        if (x && x->kind == T_INFER) {
            cm->merged = x;
            return coerce(fcx, e, x);
        }
        cm->merged = e->ty;
        return e;
    }
    Type *m = infer_shallow(fcx->icx, cm->merged);
    if ((m->kind == T_CLOSURE || m->kind == T_FNDEF) && (t->kind == T_CLOSURE || t->kind == T_FNDEF) &&
        m != t) {
        /* distinct fn items / closures unify as a fn pointer */
        cm->merged = m->kind == T_CLOSURE ? closure_as_fnptr(m) : fndef_as_fnptr(fcx, m, e->span);
    }
    TExpr *c = try_coerce(fcx, e, cm->merged);
    if (c)
        return c;
    /* LUB: the earlier branches may coerce to this one's type instead
       (`if c { &string } else { "" }`) */
    if (t->kind != T_INFER && m->kind != T_INFER) {
        Snapshot s = infer_snapshot(fcx->icx);
        bool ok = true;
        for (uint32_t i = 0; i < cm->prev.len && ok; i++)
            ok = try_coerce(fcx, cm->prev.data[i], e->ty) != NULL;
        infer_rollback(fcx->icx, s);
        if (ok) {
            for (uint32_t i = 0; i < cm->prev.len; i++)
                (void)cm_recoerce(fcx, cm->prev.data[i], e->ty);
            cm->merged = e->ty;
            return e;
        }
    }
    return coerce(fcx, e, cm->merged);
}

static Type *cm_type(CoerceMany *cm) { return cm->merged ? cm->merged : ty_never(); }

static TExpr *cm_fix(FnCtxt *fcx, CoerceMany *cm, TExpr *e) {
    if (cm->merged && infer_shallow(fcx->icx, e->ty)->kind == T_NEVER &&
        infer_shallow(fcx->icx, cm->merged)->kind != T_NEVER)
        return te1(TE_NEVER_TO_ANY, e->span, cm->merged, e);
    if (cm->merged && infer_shallow(fcx->icx, cm->merged)->kind == T_FNPTR) {
        Type *t = infer_shallow(fcx->icx, e->ty);
        if (t->kind == T_CLOSURE || t->kind == T_FNDEF)
            return coerce(fcx, e, cm->merged);
    }
    return e;
}

/* ------------------------------------------------------------------ */
/* Literals                                                            */
/* ------------------------------------------------------------------ */

static Type *suffix_type(Sym suffix, Span sp) {
    const char *s = sym_str(suffix);
    if (strcmp(s, "f32") == 0)
        return ty_float(FK_F32);
    if (strcmp(s, "f64") == 0)
        return ty_float(FK_F64);
    for (int k = 0; k < IK_COUNT; k++)
        if (strcmp(s, ik_name((IntKind)k)) == 0)
            return ty_int((IntKind)k);
    fatal(sp, "unknown literal suffix `%s`", s);
}

static TExpr *check_lit(FnCtxt *fcx, Expr *e, Type *expected) {
    Lit *l = e->lit;
    Type *x = expected ? infer_shallow(fcx->icx, expected) : NULL;
    TExpr *r;
    switch (l->kind) {
    case LIT_INT: {
        Type *t;
        if (l->suffix)
            t = suffix_type(l->suffix, e->span);
        else if (x && x->kind == T_INT)
            t = x;
        else if (x && x->kind == T_INFER && x->ik == INFER_INT)
            t = x;
        else
            t = infer_var(fcx->icx, INFER_INT, e->span);
        if (t->kind == T_FLOAT) {
            r = te_new(TE_LIT_FLOAT, e->span, t);
            r->fval = (double)l->int_lo;
            return r;
        }
        r = te_new(TE_LIT_INT, e->span, t);
        r->int_.lo = l->int_lo;
        r->int_.hi = l->int_hi;
        return r;
    }
    case LIT_FLOAT: {
        Type *t;
        if (l->suffix)
            t = suffix_type(l->suffix, e->span);
        else if (x && x->kind == T_FLOAT)
            t = x;
        else if (x && x->kind == T_INFER && x->ik == INFER_FLOAT)
            t = x;
        else
            t = infer_var(fcx->icx, INFER_FLOAT, e->span);
        r = te_new(TE_LIT_FLOAT, e->span, t);
        r->fval = strtod(l->float_text, NULL);
        return r;
    }
    case LIT_STR:
        r = te_new(TE_LIT_STR, e->span, ty_ref(ty_str(), false));
        r->str.s = l->str;
        r->str.len = l->str_len;
        return r;
    case LIT_BYTE_STR:
        r = te_new(TE_LIT_BYTESTR, e->span,
                   ty_ref(ty_array(ty_int(IK_U8), ty_const(l->str_len)), false));
        r->str.s = l->str;
        r->str.len = l->str_len;
        return r;
    case LIT_C_STR:
        fatal(e->span, "C string literals are not supported");
    case LIT_CHAR:
        r = te_new(TE_LIT_INT, e->span, ty_char());
        r->int_.lo = l->char_val;
        return r;
    case LIT_BYTE:
        r = te_new(TE_LIT_INT, e->span, ty_int(IK_U8));
        r->int_.lo = l->char_val;
        return r;
    case LIT_BOOL:
        r = te_new(TE_LIT_INT, e->span, ty_bool());
        r->int_.lo = l->bool_val;
        return r;
    }
    fatal(e->span, "internal: bad literal");
}

/* ------------------------------------------------------------------ */
/* Operators                                                           */
/* ------------------------------------------------------------------ */

static bool is_num_like(Type *t) {
    return t->kind == T_INT || t->kind == T_FLOAT ||
           (t->kind == T_INFER && (t->ik == INFER_INT || t->ik == INFER_FLOAT));
}

static bool is_int_like(Type *t) {
    return t->kind == T_INT || (t->kind == T_INFER && t->ik == INFER_INT);
}

static bool is_scalar_prim(Type *t) {
    return t->kind == T_INT || t->kind == T_FLOAT || t->kind == T_BOOL || t->kind == T_CHAR;
}

typedef enum OpClass { OPC_ARITH, OPC_BIT, OPC_SHIFT, OPC_CMP } OpClass;

static OpClass op_class(BinOp op) {
    switch (op) {
    case BIN_ADD: case BIN_SUB: case BIN_MUL: case BIN_DIV: case BIN_REM: return OPC_ARITH;
    case BIN_BITXOR: case BIN_BITAND: case BIN_BITOR: return OPC_BIT;
    case BIN_SHL: case BIN_SHR: return OPC_SHIFT;
    default: return OPC_CMP;
    }
}

static bool builtin_binop_ok(OpClass c, Type *l, Type *r) {
    switch (c) {
    case OPC_ARITH: return is_num_like(l) && is_num_like(r);
    case OPC_BIT:
        return (is_int_like(l) && is_int_like(r)) || (l->kind == T_BOOL && r->kind == T_BOOL);
    case OPC_SHIFT: return is_int_like(l) && is_int_like(r);
    case OPC_CMP:
        return (is_num_like(l) && is_num_like(r)) || (l->kind == T_BOOL && r->kind == T_BOOL) ||
               (l->kind == T_CHAR && r->kind == T_CHAR);
    }
    return false;
}

/* `&prim` (possibly nested) used as an arithmetic operand: auto-deref. */
static TExpr *peel_prim_refs(FnCtxt *fcx, TExpr *e) {
    for (;;) {
        Type *t = infer_shallow(fcx->icx, e->ty);
        if (t->kind != T_REF)
            return e;
        Type *inner = infer_shallow(fcx->icx, t->elem);
        while (inner->kind == T_REF)
            inner = infer_shallow(fcx->icx, inner->elem);
        if (!is_scalar_prim(inner) && !(inner->kind == T_INFER && inner->ik != INFER_TY))
            return e;
        e = te1(TE_DEREF, e->span, t->elem, e);
    }
}

static DefId binop_trait(BinOp op, bool assign) {
    switch (op_class(op)) {
    case OPC_CMP:
        return (op == BIN_EQ || op == BIN_NE) ? lang.partial_eq : lang.partial_ord;
    default:
        return assign ? lang.binop_assign[op] : lang.binop[op];
    }
}

static const char *binop_method(BinOp op, bool assign) {
    static const char *names[18] = {"add", "sub", "mul", "div", "rem", NULL, NULL, "bitxor",
                                    "bitand", "bitor", "shl", "shr", "eq", "lt", "le", "ne",
                                    "ge", "gt"};
    return assign ? arena_printf("%s_assign", names[op]) : names[op];
}

static TExpr *check_binop(FnCtxt *fcx, Expr *e, bool assign) {
    BinOp op = assign ? e->assign.op : e->binary.op;
    Expr *le = assign ? e->assign.lhs : e->binary.lhs;
    Expr *re = assign ? e->assign.rhs : e->binary.rhs;
    OpClass cls = op_class(op);
    InferCtxt *icx = fcx->icx;
    TExpr *l = check_expr(fcx, le, NULL);
    if (cls != OPC_CMP && !assign)
        l = peel_prim_refs(fcx, l);
    Type *lt = structurally_resolve(fcx, l->ty);
    TExpr *r = NULL;
    Type *rt = NULL;
    bool lhs_prim = is_num_like(lt) || lt->kind == T_BOOL || lt->kind == T_CHAR;
    if (lhs_prim) {
        r = check_expr(fcx, re, cls == OPC_SHIFT ? NULL : lt);
        if (cls != OPC_CMP)
            r = peel_prim_refs(fcx, r);
        rt = structurally_resolve(fcx, r->ty);
        if (builtin_binop_ok(cls, lt, rt) && (cls == OPC_SHIFT || infer_unify(icx, lt, rt))) {
            TExpr *x = te_new(assign ? TE_ASSIGN_OP : TE_BINARY, e->span,
                              assign ? ty_unit() : cls == OPC_CMP ? ty_bool() : l->ty);
            x->op = (uint8_t)op;
            x->a = l;
            x->b = r;
            if (assign)
                fix_place_mut(fcx, l);
            return x;
        }
    }
    /* Trait path: `lt: Op<?R>`; whether this ends up as a builtin operation
       or a method call is decided at writeback when types are final. */
    DefId trait = binop_trait(op, assign);
    if (!trait)
        fatal(e->span, "operator `%s` needs its lang trait (not found in std)", binop_str(op));
    Type *rv = infer_var(icx, INFER_TY, e->span);
    TraitRef tr = {trait, substs_2(l->ty, rv)};
    Predicate p = {PRED_TRAIT, tr, 0, NULL};
    infer_register(icx, p, e->span);
    if (r) {
        unify_or_fatal(fcx, rv, r->ty, r->span);
    } else {
        infer_select_pending(icx);
        r = check_expr_coerce(fcx, re, rv);
    }
    Type *out;
    if (assign)
        out = ty_unit();
    else if (cls == OPC_CMP)
        out = ty_bool();
    else
        out = fcx_normalize(fcx, ty_proj(trait_item(trait, sym_intern_cstr("Output")), tr.substs),
                            e->span);
    TExpr *x = te_new(assign ? TE_ASSIGN_OP : TE_BINARY, e->span, out);
    x->op = (uint8_t)op;
    x->a = l;
    x->b = r;
    if (assign)
        fix_place_mut(fcx, l);
    vec_push(fcx->deferred_ops, x);
    return x;
}

/* Called from writeback with fully resolved operand types: turn a deferred
   operator node into a trait method call unless it is a builtin operation. */
void finalize_deferred_op(TExpr *x) {
    if (x->kind == TE_UNARY) {
        Type *t = x->a->ty;
        if (t->kind == T_INT || t->kind == T_FLOAT || (t->kind == T_BOOL && x->op == UN_NOT))
            return;
        DefId trait = x->op == UN_NEG ? lang.neg : lang.not_;
        DefId m = trait_item(trait, sym_intern_cstr(x->op == UN_NEG ? "neg" : "not"));
        TExpr *call = make_call(x->span, m, substs_1(t), x->ty, args1(x->a), 1);
        *x = *call;
        return;
    }
    bool assign = x->kind == TE_ASSIGN_OP;
    BinOp op = (BinOp)x->op;
    OpClass cls = op_class(op);
    Type *lt = x->a->ty, *rt = x->b->ty;
    bool builtin = false;
    switch (cls) {
    case OPC_ARITH: builtin = ty_is_numeric(lt) && lt == rt; break;
    case OPC_BIT: builtin = (lt->kind == T_INT || lt->kind == T_BOOL) && lt == rt; break;
    case OPC_SHIFT: builtin = lt->kind == T_INT && rt->kind == T_INT; break;
    case OPC_CMP: builtin = is_scalar_prim(lt) && lt == rt; break;
    }
    if (builtin)
        return;
    DefId trait = binop_trait(op, assign);
    DefId m = trait_item(trait, sym_intern_cstr(binop_method(op, assign)));
    if (!m)
        fatal(x->span, "std trait for operator `%s` has no method `%s`", binop_str(op),
              binop_method(op, assign));
    TExpr *a = x->a, *b = x->b;
    if (assign) {
        a = make_ref(a->span, a, true);
    } else if (cls == OPC_CMP) {
        a = make_ref(a->span, a, false);
        b = make_ref(b->span, b, false);
    }
    TExpr *call = make_call(x->span, m, substs_2(lt, rt), x->ty, args2(a, b), 2);
    *x = *call;
}

static TExpr *check_unary(FnCtxt *fcx, Expr *e, Type *expected) {
    UnOp op = e->unary.op;
    if (op == UN_DEREF) {
        TExpr *a = check_expr(fcx, e->unary.operand, NULL);
        Type *t = resolve_or_fatal(fcx, a->ty, e->span);
        if (t->kind == T_PTR)
            return te1(TE_DEREF, e->span, t->elem, a);
        TExpr *d = deref_once(fcx, a);
        if (!d)
            fatal(e->span, "type `%s` cannot be dereferenced", fcx_ty_str(fcx, t));
        d->span = e->span;
        return d;
    }
    TExpr *a = check_expr(fcx, e->unary.operand, expected);
    a = peel_prim_refs(fcx, a);
    Type *t = structurally_resolve(fcx, a->ty);
    TExpr *x = te1(TE_UNARY, e->span, a->ty, a);
    x->op = (uint8_t)op;
    if (op == UN_NEG ? is_num_like(t) : (is_int_like(t) || t->kind == T_BOOL))
        return x;
    DefId trait = op == UN_NEG ? lang.neg : lang.not_;
    if (!trait)
        fatal(e->span, "operator `%s` needs its lang trait", unop_str(op));
    TraitRef tr = {trait, substs_1(a->ty)};
    Predicate p = {PRED_TRAIT, tr, 0, NULL};
    infer_register(fcx->icx, p, e->span);
    x->ty = fcx_normalize(fcx, ty_proj(trait_item(trait, sym_intern_cstr("Output")), tr.substs),
                          e->span);
    vec_push(fcx->deferred_ops, x);
    return x;
}

/* ------------------------------------------------------------------ */
/* Calls                                                               */
/* ------------------------------------------------------------------ */

Type **expected_inputs(FnCtxt *fcx, Type *expected, Type *output, Substs *formals) {
    if (!expected)
        return NULL;
    Type *x = infer_shallow(fcx->icx, expected);
    if (x->kind == T_INFER || is_unsized_kind(x))
        return NULL;
    Snapshot s = infer_snapshot(fcx->icx);
    Type **hints = NULL;
    if (infer_unify(fcx->icx, output, expected)) {
        hints = NEW_ARRAY(Type *, formals->len ? formals->len : 1);
        for (uint32_t i = 0; i < formals->len; i++)
            hints[i] = infer_resolve(fcx->icx, formals->t[i]);
    }
    infer_rollback(fcx->icx, s);
    return hints;
}

TExpr **check_args(FnCtxt *fcx, Substs *formals, uint32_t first_formal, ExprList *args,
                   Type **hints, bool variadic, Span sp, uint32_t *nout) {
    uint32_t nformal = formals->len - first_formal;
    if (variadic ? args->len < nformal : args->len != nformal)
        fatal(sp, "this function takes %u argument(s) but %u were supplied", nformal, args->len);
    TExpr **out = NEW_ARRAY(TExpr *, args->len ? args->len : 1);
    for (int pass = 0; pass < 2; pass++) {
        if (pass == 1)
            infer_select_pending(fcx->icx);
        for (uint32_t i = 0; i < args->len; i++) {
            Expr *a = args->data[i];
            bool is_closure = a->kind == EXPR_CLOSURE;
            if (is_closure != (pass == 1))
                continue;
            if (i >= nformal) {
                out[i] = check_expr(fcx, a, NULL);
                continue;
            }
            Type *formal = formals->t[first_formal + i];
            Type *hint = hints ? hints[first_formal + i] : formal;
            TExpr *arg = check_expr(fcx, a, hint);
            if (hints) {
                /* Coerce to the type expected from the call's context
                   first (e.g. `&[T; 3]` -> `&[T]` under an enum
                   constructor), then relate that to the formal type. */
                Type *h = infer_shallow(fcx->icx, hint);
                if (h->kind != T_INFER && !is_unsized_kind(h)) {
                    Snapshot s = infer_snapshot(fcx->icx);
                    TExpr *c = try_coerce(fcx, arg, hint);
                    if (c && infer_unify(fcx->icx, hint, formal)) {
                        out[i] = c;
                        continue;
                    }
                    infer_rollback(fcx->icx, s);
                }
            }
            out[i] = coerce(fcx, arg, formal);
        }
    }
    *nout = args->len;
    return out;
}

static TExpr *check_call(FnCtxt *fcx, Expr *e, Type *expected) {
    InferCtxt *icx = fcx->icx;
    TExpr *callee = check_expr(fcx, e->call.callee, NULL);
    Substs *inputs = NULL;
    Type *output = NULL;
    bool variadic = false;
    for (;;) {
        Type *t = resolve_or_fatal(fcx, callee->ty, e->span);
        if (t->kind == T_FNDEF) {
            FnSigT *sig = fn_sig(t->def);
            inputs = fcx_normalize(fcx, ty_tuple(substs_subst(sig->inputs, t->substs)), e->span)->substs;
            output = fcx_normalize(fcx, ty_subst(sig->output, t->substs), e->span);
            variadic = sig->variadic;
            break;
        }
        if (t->kind == T_FNPTR) {
            inputs = substs_slice(t->substs, 0, t->substs->len - 1);
            output = fnptr_ret(t);
            break;
        }
        if (t->kind == T_CLOSURE) {
            ClosureInfo *ci = def_get(t->def)->closure;
            inputs = substs_subst(ci->inputs, t->substs);
            output = ty_subst(ci->output, t->substs);
            break;
        }
        if (t->kind == T_REF || is_box(t)) {
            callee = deref_once(fcx, callee);
            continue;
        }
        /* Overloaded call through Fn / FnMut / FnOnce. */
        DefId traits[3] = {lang.fn, lang.fn_mut, lang.fn_once};
        static const char *names[3] = {"call", "call_mut", "call_once"};
        for (int k = 0; k < 3; k++) {
            if (!traits[k])
                continue;
            Type *av = infer_var(icx, INFER_TY, e->span);
            TraitRef tr = {traits[k], substs_2(t, av)};
            Snapshot s = infer_snapshot(icx);
            Selection sel = trait_select(icx, tr, e->span);
            if (sel.kind == SEL_NONE || sel.kind == SEL_AMBIG) {
                infer_rollback(icx, s);
                continue;
            }
            infer_select_pending(icx);
            Type *args_ty = infer_resolve(icx, av);
            if (args_ty->kind != T_TUPLE)
                fatal(e->span, "cannot determine the argument types of this call");
            Type *out = fcx_normalize(fcx, ty_proj(lang.fn_once_output, substs_2(t, args_ty)), e->span);
            TExpr *recv = callee;
            if (k == 0) {
                recv = make_ref(callee->span, callee, false);
            } else if (k == 1) {
                fix_place_mut(fcx, callee);
                recv = make_ref(callee->span, callee, true);
            }
            Type **hints = expected_inputs(fcx, expected, out, args_ty->substs);
            uint32_t n;
            TExpr **args = check_args(fcx, args_ty->substs, 0, &e->call.args, hints, false, e->span, &n);
            TExpr *tuple = te_new(TE_TUPLE, e->span, args_ty);
            tuple->args = args;
            tuple->nargs = n;
            DefId m = trait_item(traits[k], sym_intern_cstr(names[k]));
            return make_call(e->span, m, substs_2(t, args_ty), out, args2(recv, tuple), 2);
        }
        TExpr *next = deref_once(fcx, callee);
        if (!next)
            fatal(e->span, "expected a function, found `%s`", fcx_ty_str(fcx, t));
        callee = next;
    }
    Type **hints = expected_inputs(fcx, expected, output, inputs);
    TExpr *call = te1(TE_CALL, e->span, output, callee);
    call->args = check_args(fcx, inputs, 0, &e->call.args, hints, variadic, e->span, &call->nargs);
    return call;
}

/* ------------------------------------------------------------------ */
/* Field access, indexing                                              */
/* ------------------------------------------------------------------ */

static bool field_visible(FnCtxt *fcx, DefId adt) {
    DefId m = def_get(adt)->module;
    while (def_get(m)->mod->is_block)
        m = def_get(m)->parent;
    for (DefId cur = scope_module(fcx->scope);;) {
        if (cur == m)
            return true;
        DefId p = def_get(cur)->parent;
        if (!p)
            return false;
        cur = def_get(p)->module;
        while (def_get(cur)->mod->is_block)
            cur = def_get(cur)->parent;
    }
}

static TExpr *check_field(FnCtxt *fcx, Expr *e) {
    TExpr *cur = check_expr(fcx, e->field.base, NULL);
    for (int steps = 0; steps < 32; steps++) {
        Type *t = resolve_or_fatal(fcx, cur->ty, e->span);
        if (t->kind == T_ADT) {
            AdtDef *adt = adt_def(t->def);
            if (!adt->is_enum) {
                int idx = variant_field_index(&adt->variants[0], e->field.name);
                /* A private field of a type from another module is
                   invisible: keep auto-dereferencing (`rc.value` must reach
                   the pointee's field, not a private field of Rc). */
                if (idx >= 0 && adt->variants[0].fields[idx].is_private &&
                    !field_visible(fcx, t->def))
                    idx = -1;
                if (idx >= 0) {
                    Type *fty = fcx_normalize(fcx, ty_subst(adt->variants[0].fields[idx].ty, t->substs),
                                              e->span);
                    TExpr *f = te1(TE_FIELD, e->span, fty, cur);
                    f->index = (uint32_t)idx;
                    return f;
                }
            }
        } else if (t->kind == T_TUPLE && e->field.is_index) {
            if (e->field.index >= t->substs->len)
                fatal(e->span, "tuple index %u out of range for `%s`", e->field.index,
                      fcx_ty_str(fcx, t));
            TExpr *f = te1(TE_FIELD, e->span, t->substs->t[e->field.index], cur);
            f->index = e->field.index;
            return f;
        }
        TExpr *next = deref_once(fcx, cur);
        if (!next)
            fatal(e->span, "no field `%s` on type `%s`", sym_str(e->field.name), fcx_ty_str(fcx, t));
        cur = next;
    }
    fatal(e->span, "auto-deref recursion limit reached looking for field `%s`", sym_str(e->field.name));
}

static TExpr *check_index(FnCtxt *fcx, Expr *e) {
    InferCtxt *icx = fcx->icx;
    TExpr *cur = check_expr(fcx, e->index.base, NULL);
    TExpr *idx = check_expr(fcx, e->index.index, NULL);
    init_method_defs();
    for (int steps = 0; steps < 32; steps++) {
        Type *t = resolve_or_fatal(fcx, cur->ty, e->span);
        Type *it = structurally_resolve(fcx, idx->ty);
        if ((t->kind == T_ARRAY || t->kind == T_SLICE) &&
            ((it->kind == T_INT && it->ik == IK_USIZE) ||
             (it->kind == T_INFER && it->ik == INFER_INT))) {
            unify_or_fatal(fcx, ty_usize(), it, idx->span);
            TExpr *x = te1(TE_INDEX, e->span, t->elem, cur);
            x->b = idx;
            return x;
        }
        if (lang.index && t->kind != T_REF) {
            if (it->kind == T_REF && it->is_mut) {
                /* `&mut K` where the impl wants `&K`: reborrow */
                TraitRef tr0 = {lang.index, substs_2(t, idx->ty)};
                Predicate p0 = {PRED_TRAIT, tr0, 0, NULL};
                if (trait_eval(icx, &p0) == EVAL_NO) {
                    idx = make_ref(idx->span, te1(TE_DEREF, idx->span, it->elem, idx), false);
                    it = idx->ty;
                }
            }
            TraitRef tr = {lang.index, substs_2(t, idx->ty)};
            Predicate p = {PRED_TRAIT, tr, 0, NULL};
            if (trait_eval(icx, &p) != EVAL_NO) {
                infer_register(icx, p, e->span);
                Type *out = fcx_normalize(
                    fcx, ty_proj(trait_item(lang.index, sym_intern_cstr("Output")), tr.substs),
                    e->span);
                TExpr *call = make_call(e->span, m_index, tr.substs, ty_ref(out, false),
                                        args2(make_ref(cur->span, cur, false), idx), 2);
                return te1(TE_DEREF, e->span, out, call);
            }
        }
        TExpr *next = deref_once(fcx, cur);
        if (!next)
            fatal(e->span, "cannot index into a value of type `%s` with `%s`", fcx_ty_str(fcx, t),
                  fcx_ty_str(fcx, it));
        cur = next;
    }
    fatal(e->span, "auto-deref recursion limit reached while indexing");
}

/* ------------------------------------------------------------------ */
/* Aggregates                                                          */
/* ------------------------------------------------------------------ */

static uint32_t variant_by_name(AdtDef *adt, const char *name, Span sp) {
    Sym s = sym_intern_cstr(name);
    for (uint32_t i = 0; i < adt->nvariants; i++)
        if (adt->variants[i].name == s)
            return i;
    fatal(sp, "std enum `%s` has no variant `%s`", sym_str(def_get(adt->def)->name), name);
}

static TExpr *make_adt(Span sp, Type *ty, uint32_t variant, TExpr **args, uint32_t nargs) {
    TExpr *x = te_new(TE_ADT, sp, ty);
    x->variant = variant;
    x->args = args;
    x->nargs = nargs;
    x->field_idx = NEW_ARRAY(uint32_t, nargs ? nargs : 1);
    for (uint32_t i = 0; i < nargs; i++)
        x->field_idx[i] = i;
    return x;
}

static TExpr *check_struct_expr(FnCtxt *fcx, Expr *e, Type *expected) {
    uint32_t variant;
    DefId ctor;
    Type *ty = resolve_variant_path(fcx, e->struct_.qself, e->struct_.path, NS_TYPE, &variant, &ctor,
                                    e->span);
    if (expected)
        (void)infer_unify(fcx->icx, ty, expected);
    AdtDef *adt = adt_def(ty->def);
    VariantDef *v = &adt->variants[variant];
    uint32_t n = e->struct_.fields.len;
    TExpr *x = te_new(TE_ADT, e->span, ty);
    x->variant = variant;
    x->args = NEW_ARRAY(TExpr *, n ? n : 1);
    x->field_idx = NEW_ARRAY(uint32_t, n ? n : 1);
    x->nargs = n;
    for (uint32_t i = 0; i < n; i++) {
        ExprField *f = e->struct_.fields.data[i];
        int idx = variant_field_index(v, f->name);
        if (idx < 0)
            fatal(f->span, "`%s` has no field named `%s`", sym_str(v->name), sym_str(f->name));
        Type *fty = fcx_normalize(fcx, ty_subst(v->fields[idx].ty, ty->substs), f->span);
        x->args[i] = check_expr_coerce(fcx, f->expr, fty);
        x->field_idx[i] = (uint32_t)idx;
    }
    if (e->struct_.base)
        x->b = check_expr_coerce(fcx, e->struct_.base, ty);
    else if (n != v->nfields && !adt->is_union && !e->struct_.has_rest)
        fatal(e->span, "missing fields in initializer of `%s`", sym_str(v->name));
    return x;
}

static TExpr *check_tuple(FnCtxt *fcx, Expr *e, Type *expected) {
    uint32_t n = e->elems.len;
    Type *x = expected ? infer_shallow(fcx->icx, expected) : NULL;
    bool have = x && x->kind == T_TUPLE && x->substs->len == n;
    TExpr *t = te_new(TE_TUPLE, e->span, NULL);
    t->args = NEW_ARRAY(TExpr *, n ? n : 1);
    t->nargs = n;
    Type **tys = NEW_ARRAY(Type *, n ? n : 1);
    for (uint32_t i = 0; i < n; i++) {
        Type *h = have ? infer_shallow(fcx->icx, x->substs->t[i]) : NULL;
        TExpr *a = check_expr(fcx, e->elems.data[i], h);
        if (h && h->kind != T_INFER && !is_unsized_kind(h))
            a = coerce(fcx, a, h);
        t->args[i] = a;
        tys[i] = a->ty;
    }
    t->ty = ty_tuple(substs_new(tys, n));
    return t;
}

static TExpr *check_array(FnCtxt *fcx, Expr *e, Type *expected) {
    Type *x = expected ? infer_shallow(fcx->icx, expected) : NULL;
    Type *elem_hint = x && (x->kind == T_ARRAY || x->kind == T_SLICE) ? x->elem : NULL;
    if (e->kind == EXPR_REPEAT) {
        TExpr *a = check_expr(fcx, e->repeat.elem, elem_hint);
        if (elem_hint && infer_shallow(fcx->icx, elem_hint)->kind != T_INFER)
            a = coerce(fcx, a, elem_hint);
        fcx->lc.scope = fcx->scope;
        Type *len = lower_const_expr(&fcx->lc, e->repeat.count);
        return te1(TE_REPEAT, e->span, ty_array(a->ty, len), a);
    }
    uint32_t n = e->elems.len;
    TExpr *t = te_new(TE_ARRAY, e->span, NULL);
    t->args = NEW_ARRAY(TExpr *, n ? n : 1);
    t->nargs = n;
    CoerceMany cm = {NULL, elem_hint, {0}};
    for (uint32_t i = 0; i < n; i++)
        t->args[i] = cm_add(fcx, &cm, check_expr(fcx, e->elems.data[i], cm.merged ? cm.merged : elem_hint));
    Type *elem = cm.merged ? cm.merged
                 : elem_hint ? elem_hint : infer_var(fcx->icx, INFER_TY, e->span);
    for (uint32_t i = 0; i < n; i++)
        t->args[i] = cm_fix(fcx, &cm, t->args[i]);
    t->ty = ty_array(elem, ty_const(n));
    return t;
}

static TExpr *check_range(FnCtxt *fcx, Expr *e, Type *expected) {
    Expr *lo = e->range.lo, *hi = e->range.hi;
    DefId def;
    if (lo && hi)
        def = e->range.inclusive ? lang.range_inclusive : lang.range;
    else if (lo)
        def = lang.range_from;
    else if (hi)
        def = e->range.inclusive ? lang.range_to_inclusive : lang.range_to;
    else
        def = lang.range_full;
    if (!def)
        fatal(e->span, "range expression needs the std range lang items");
    if (!lo && !hi)
        return make_adt(e->span, ty_adt(def, substs_empty()), 0, NULL, 0);
    Type *x = expected ? infer_shallow(fcx->icx, expected) : NULL;
    Type *hint = x && x->kind == T_ADT && x->def == def ? x->substs->t[0] : NULL;
    TExpr *a = lo ? check_expr(fcx, lo, hint) : NULL;
    Type *idx = a ? a->ty : NULL;
    TExpr *b = NULL;
    if (hi) {
        b = check_expr(fcx, hi, idx ? idx : hint);
        if (idx)
            b = coerce(fcx, b, idx);
        else
            idx = b->ty;
    }
    Type *ty = ty_adt(def, substs_1(idx));
    AdtDef *adt = adt_def(def);
    VariantDef *v = &adt->variants[0];
    TExpr **args = NEW_ARRAY(TExpr *, 3);
    uint32_t n = 0;
    if (a)
        args[n++] = a;
    if (b)
        args[n++] = b;
    TExpr *r = make_adt(e->span, ty, 0, args, n);
    n = 0;
    if (a)
        r->field_idx[n++] = (uint32_t)variant_field_index(v, sym_intern_cstr("start"));
    if (b)
        r->field_idx[n++] = (uint32_t)variant_field_index(v, sym_intern_cstr("end"));
    int ex = variant_field_index(v, sym_intern_cstr("exhausted"));
    if (ex >= 0) {
        TExpr *f = te_new(TE_LIT_INT, e->span, ty_bool());
        r->args[n] = f;
        r->field_idx[n++] = (uint32_t)ex;
        r->nargs = n;
    }
    return r;
}

/* ------------------------------------------------------------------ */
/* Control flow                                                        */
/* ------------------------------------------------------------------ */

static BreakScope *push_break(FnCtxt *fcx, BreakScope *bs, BreakKind kind, Sym label, Type *ty) {
    bs->parent = fcx->breaks;
    bs->label = label;
    bs->id = ++fcx->next_label;
    bs->kind = (uint8_t)kind;
    bs->ty = ty;
    bs->has_break = false;
    fcx->breaks = bs;
    return bs;
}

static BreakScope *find_break(FnCtxt *fcx, Sym label, bool for_continue, Span sp) {
    for (BreakScope *b = fcx->breaks; b; b = b->parent) {
        if (label ? b->label == label : b->kind != BS_BLOCK) {
            if (for_continue && b->kind == BS_BLOCK)
                fatal(sp, "`continue` cannot target a labeled block");
            return b;
        }
    }
    fatal(sp, label ? "use of undeclared label `'%s`" : "`break`/`continue` outside of a loop",
          sym_str(label));
}

static TExpr *check_let_cond(FnCtxt *fcx, Expr *e) {
    TExpr *s = check_expr(fcx, e->let.scrutinee, NULL);
    TExpr *x = te1(TE_LET, e->span, ty_bool(), s);
    x->pat = check_pat_top(fcx, e->let.pat, s->ty);
    return x;
}

static TExpr *check_if(FnCtxt *fcx, Expr *e, Type *expected) {
    Scope *saved = fcx->scope;
    fcx->scope = scope_push(saved, SCOPE_LOCALS, 0);
    TExpr *cond = check_expr_coerce(fcx, e->if_.cond, ty_bool());
    bool d_cond = fcx->diverges;
    CoerceMany cm = {NULL, e->if_.els ? expected : NULL, {0}};
    TExpr *then = check_block(fcx, e->if_.then, e->if_.els ? expected : ty_unit(), 0);
    bool d_then = fcx->diverges;
    fcx->scope = saved;
    fcx->diverges = d_cond;
    TExpr *x = te1(TE_IF, e->span, NULL, cond);
    if (!e->if_.els) {
        x->b = coerce(fcx, then, ty_unit());
        x->ty = ty_unit();
        return x;
    }
    then = cm_add(fcx, &cm, then);
    TExpr *els = cm_add(fcx, &cm, check_expr(fcx, e->if_.els, cm.merged ? cm.merged : expected));
    bool d_else = fcx->diverges;
    fcx->diverges = d_cond || (d_then && d_else);
    x->b = cm_fix(fcx, &cm, then);
    x->c = cm_fix(fcx, &cm, els);
    x->ty = cm_type(&cm);
    return x;
}

static TExpr *check_match(FnCtxt *fcx, Expr *e, Type *expected) {
    TExpr *scrut = check_expr(fcx, e->match.scrutinee, NULL);
    bool d_s = fcx->diverges;
    uint32_t n = e->match.arms.len;
    TExpr *x = te1(TE_MATCH, e->span, NULL, scrut);
    x->arms = NEW_ARRAY(TArm, n ? n : 1);
    x->narms = n;
    CoerceMany cm = {NULL, expected, {0}};
    bool all_div = true;
    Scope *saved = fcx->scope;
    for (uint32_t i = 0; i < n; i++) {
        Arm *arm = e->match.arms.data[i];
        fcx->scope = scope_push(saved, SCOPE_LOCALS, 0);
        fcx->diverges = d_s;
        x->arms[i].span = arm->span;
        x->arms[i].pat = check_pat_top(fcx, arm->pat, scrut->ty);
        if (arm->guard)
            x->arms[i].guard = check_expr_coerce(fcx, arm->guard, ty_bool());
        x->arms[i].body = cm_add(fcx, &cm, check_expr(fcx, arm->body, cm.merged ? cm.merged : expected));
        all_div = all_div && fcx->diverges;
        fcx->scope = saved;
    }
    for (uint32_t i = 0; i < n; i++)
        x->arms[i].body = cm_fix(fcx, &cm, x->arms[i].body);
    fcx->diverges = d_s || all_div;
    x->ty = cm_type(&cm);
    return x;
}

static TExpr *check_while(FnCtxt *fcx, Expr *e) {
    BreakScope bs;
    push_break(fcx, &bs, BS_WHILE, e->while_.label, NULL);
    Scope *saved = fcx->scope;
    fcx->scope = scope_push(saved, SCOPE_LOCALS, 0);
    TExpr *cond = check_expr_coerce(fcx, e->while_.cond, ty_bool());
    bool d = fcx->diverges;
    TExpr *body = coerce(fcx, check_block(fcx, e->while_.body, ty_unit(), 0), ty_unit());
    fcx->scope = saved;
    fcx->breaks = bs.parent;
    fcx->diverges = d;
    TExpr *x = te1(TE_WHILE, e->span, ty_unit(), cond);
    x->b = body;
    x->label = bs.id;
    return x;
}

static TExpr *check_loop(FnCtxt *fcx, Expr *e, Type *expected) {
    BreakScope bs;
    Type *ty = infer_var(fcx->icx, INFER_TY, e->span);
    if (expected)
        (void)expected; /* the break values are coerced to `ty`; the caller coerces the loop */
    push_break(fcx, &bs, BS_LOOP, e->loop.label, ty);
    bool d = fcx->diverges;
    TExpr *body = coerce(fcx, check_block(fcx, e->loop.body, ty_unit(), 0), ty_unit());
    fcx->breaks = bs.parent;
    fcx->diverges = d || !bs.has_break;
    TExpr *x = te1(TE_LOOP, e->span, bs.has_break ? ty : ty_never(), body);
    x->label = bs.id;
    return x;
}

/*
 * for PAT in ITER BODY   =>
 *   match IntoIterator::into_iter(ITER) {
 *       mut it => loop { match Iterator::next(&mut it) {
 *           None => break, Some(PAT) => BODY } } }
 * (a `match`, so temporaries in ITER live for the whole loop.)
 */
static TExpr *check_for(FnCtxt *fcx, Expr *e) {
    InferCtxt *icx = fcx->icx;
    Span sp = e->span;
    if (!lang.into_iterator || !lang.iterator || !lang.option)
        fatal(sp, "`for` loops need the IntoIterator/Iterator/Option lang items");
    TExpr *iter = check_expr(fcx, e->for_.iter, NULL);
    TraitRef tr = {lang.into_iterator, substs_1(iter->ty)};
    Predicate p = {PRED_TRAIT, tr, 0, NULL};
    infer_register(icx, p, sp);
    Type *it_ty = fcx_normalize(
        fcx, ty_proj(trait_item(lang.into_iterator, sym_intern_cstr("IntoIter")), tr.substs), sp);
    TExpr *into = make_call(sp, trait_item(lang.into_iterator, sym_intern_cstr("into_iter")),
                            tr.substs, it_ty, args1(iter), 1);
    TraitRef itr = {lang.iterator, substs_1(it_ty)};
    Predicate p2 = {PRED_TRAIT, itr, 0, NULL};
    infer_register(icx, p2, sp);
    Type *item = fcx_normalize(
        fcx, ty_proj(trait_item(lang.iterator, sym_intern_cstr("Item")), itr.substs), sp);
    Type *opt = ty_adt(lang.option, substs_1(item));
    AdtDef *oadt = adt_def(lang.option);

    Local *it = new_local(fcx, sym_intern_cstr("__iter"), it_ty, true, sp);
    TExpr *itref = te_new(TE_LOCAL, sp, it_ty);
    itref->local = it;
    TExpr *next = make_call(sp, trait_item(lang.iterator, sym_intern_cstr("next")), itr.substs, opt,
                            args1(make_ref(sp, itref, true)), 1);

    BreakScope bs;
    push_break(fcx, &bs, BS_WHILE, e->for_.label, NULL);
    bool d = fcx->diverges;
    Scope *saved = fcx->scope;
    fcx->scope = scope_push(saved, SCOPE_LOCALS, 0);
    TPat *pat = check_pat_top(fcx, e->for_.pat, item);
    TExpr *body = coerce(fcx, check_block(fcx, e->for_.body, ty_unit(), 0), ty_unit());
    fcx->scope = saved;
    fcx->breaks = bs.parent;
    fcx->diverges = d;

    TExpr *inner = te1(TE_MATCH, sp, ty_unit(), next);
    inner->arms = NEW_ARRAY(TArm, 2);
    inner->narms = 2;
    TPat *none = NEW(TPat);
    none->kind = TP_VARIANT;
    none->span = sp;
    none->ty = opt;
    none->adt = lang.option;
    none->variant = variant_by_name(oadt, "None", sp);
    TExpr *brk = te_new(TE_BREAK, sp, ty_never());
    brk->label = bs.id;
    inner->arms[0].pat = none;
    inner->arms[0].body = te1(TE_NEVER_TO_ANY, sp, ty_unit(), brk);
    inner->arms[0].span = sp;
    TPat *some = NEW(TPat);
    some->kind = TP_VARIANT;
    some->span = sp;
    some->ty = opt;
    some->adt = lang.option;
    some->variant = variant_by_name(oadt, "Some", sp);
    some->subs = NEW_ARRAY(TPat *, 1);
    some->subs[0] = pat;
    some->field_idx = NEW_ARRAY(uint32_t, 1);
    some->nsubs = 1;
    inner->arms[1].pat = some;
    inner->arms[1].body = body;
    inner->arms[1].span = sp;

    TExpr *lp = te1(TE_LOOP, sp, ty_unit(), inner);
    lp->label = bs.id;
    TExpr *outer = te1(TE_MATCH, sp, ty_unit(), into);
    outer->arms = NEW_ARRAY(TArm, 1);
    outer->narms = 1;
    TPat *bind = NEW(TPat);
    bind->kind = TP_BIND;
    bind->span = sp;
    bind->ty = it_ty;
    bind->local = it;
    outer->arms[0].pat = bind;
    outer->arms[0].body = lp;
    outer->arms[0].span = sp;
    return outer;
}

static TPat *bind_pat(Local *l, Span sp) {
    TPat *p = NEW(TPat);
    p->kind = TP_BIND;
    p->span = sp;
    p->ty = l->ty;
    p->local = l;
    return p;
}

static TPat *variant_pat1(Type *ty, uint32_t variant, TPat *sub, Span sp) {
    TPat *p = NEW(TPat);
    p->kind = TP_VARIANT;
    p->span = sp;
    p->ty = ty;
    p->adt = ty->def;
    p->variant = variant;
    if (sub) {
        p->subs = NEW_ARRAY(TPat *, 1);
        p->subs[0] = sub;
        p->field_idx = NEW_ARRAY(uint32_t, 1);
        p->nsubs = 1;
    }
    return p;
}

static TExpr *local_ref(Local *l, Span sp) {
    TExpr *x = te_new(TE_LOCAL, sp, l->ty);
    x->local = l;
    return x;
}

/*
 * EXPR?  on Result<T, E> in a fn returning Result<_, F>:
 *   match EXPR { Ok(v) => v, Err(e) => return Err(From::from(e)) }
 * on Option<T>:
 *   match EXPR { Some(v) => v, None => return None }
 */
static TExpr *check_try(FnCtxt *fcx, Expr *e) {
    InferCtxt *icx = fcx->icx;
    Span sp = e->span;
    TExpr *operand = check_expr(fcx, e->try_, NULL);
    Type *t = resolve_or_fatal(fcx, operand->ty, sp);
    bool is_result = t->kind == T_ADT && t->def == lang.result && lang.result;
    bool is_option = t->kind == T_ADT && t->def == lang.option && lang.option;
    if (!is_result && !is_option)
        fatal(sp, "the `?` operator is only supported on Option and Result (found `%s`)",
              fcx_ty_str(fcx, t));
    Type *ret = structurally_resolve(fcx, fcx->ret_ty);
    if (ret->kind == T_INFER) {
        Type *nt = is_result ? ty_adt(lang.result, substs_2(infer_var(icx, INFER_TY, sp),
                                                            infer_var(icx, INFER_TY, sp)))
                             : ty_adt(lang.option, substs_1(infer_var(icx, INFER_TY, sp)));
        unify_or_fatal(fcx, ret, nt, sp);
        ret = nt;
    }
    if (ret->kind != T_ADT || ret->def != t->def)
        fatal(sp, "`?` on `%s` in a function returning `%s`", fcx_ty_str(fcx, t),
              fcx_ty_str(fcx, ret));
    AdtDef *adt = adt_def(t->def);
    Type *val_ty = t->substs->t[0];
    Local *v = new_local(fcx, sym_intern_cstr("__val"), val_ty, false, sp);
    TExpr *m = te1(TE_MATCH, sp, val_ty, operand);
    m->arms = NEW_ARRAY(TArm, 2);
    m->narms = 2;
    m->arms[0].span = m->arms[1].span = sp;
    TExpr *retval;
    if (is_result) {
        Type *err_ty = t->substs->t[1], *ret_err = ret->substs->t[1];
        Local *er = new_local(fcx, sym_intern_cstr("__err"), err_ty, false, sp);
        m->arms[0].pat = variant_pat1(t, variant_by_name(adt, "Ok", sp), bind_pat(v, sp), sp);
        m->arms[1].pat = variant_pat1(t, variant_by_name(adt, "Err", sp), bind_pat(er, sp), sp);
        TExpr *conv = local_ref(er, sp);
        if (infer_resolve(icx, err_ty) != infer_resolve(icx, ret_err) ||
            (infer_resolve(icx, err_ty)->flags & TYF_HAS_INFER)) {
            if (!lang.from)
                fatal(sp, "`?` needs the `From` lang item to convert errors");
            TraitRef tr = {lang.from, substs_2(ret_err, err_ty)};
            Predicate p = {PRED_TRAIT, tr, 0, NULL};
            infer_register(icx, p, sp);
            conv = make_call(sp, trait_item(lang.from, sym_intern_cstr("from")), tr.substs, ret_err,
                             args1(conv), 1);
        }
        retval = make_adt(sp, ret, variant_by_name(adt, "Err", sp), args1(conv), 1);
    } else {
        m->arms[0].pat = variant_pat1(t, variant_by_name(adt, "Some", sp), bind_pat(v, sp), sp);
        m->arms[1].pat = variant_pat1(t, variant_by_name(adt, "None", sp), NULL, sp);
        retval = make_adt(sp, ret, variant_by_name(adt, "None", sp), NULL, 0);
    }
    m->arms[0].body = local_ref(v, sp);
    m->arms[1].body = te1(TE_NEVER_TO_ANY, sp, val_ty, te1(TE_RETURN, sp, ty_never(), retval));
    return m;
}

/* ------------------------------------------------------------------ */
/* Casts, assignment                                                   */
/* ------------------------------------------------------------------ */

static TExpr *check_cast(FnCtxt *fcx, Expr *e) {
    Type *target = lower_body_ty(fcx, e->cast.ty);
    /* the target type only guides the typing of a literal operand
       (`1 as u64`); `if c { a } else { b } as u32` must not be coerced */
    Expr *ce = e->cast.expr;
    bool lit_like = ce->kind == EXPR_LIT ||
                    (ce->kind == EXPR_UNARY && ce->unary.op == UN_NEG &&
                     ce->unary.operand->kind == EXPR_LIT);
    TExpr *a = check_expr(fcx, ce, lit_like ? target : NULL);
    Type *src = structurally_resolve(fcx, a->ty);
    Type *tgt = infer_shallow(fcx->icx, target);
    /* Casts to scalars are classified at MIR building from the final types
       (int/float/bool/char/enum/pointer sources). Everything else must be a
       coercion (unsizing, reification, & -> *const) or a pointer cast. */
    bool tgt_scalar = tgt->kind == T_INT || tgt->kind == T_FLOAT || tgt->kind == T_CHAR ||
                      tgt->kind == T_BOOL;
    if (tgt_scalar) {
        if (src->kind == T_FNDEF)
            a = te1(TE_REIFY, e->span, fndef_as_fnptr(fcx, src, e->span), a);
        return te1(TE_CAST, e->span, target, a);
    }
    TExpr *c = try_coerce(fcx, a, target);
    if (c)
        return c;
    if (tgt->kind == T_PTR && src->kind == T_REF) {
        /* &T as *const U: go through *const T. */
        a = te1(TE_CAST, e->span, ty_ptr(src->elem, src->is_mut), a);
        src = a->ty;
    }
    if (tgt->kind == T_PTR && (src->kind == T_PTR || src->kind == T_INT ||
                               (src->kind == T_INFER && src->ik == INFER_INT)))
        return te1(TE_CAST, e->span, target, a);
    if (tgt->kind == T_FNPTR && src->kind == T_PTR)
        return te1(TE_CAST, e->span, target, a);
    fatal(e->span, "unsupported cast from `%s` to `%s`", fcx_ty_str(fcx, src), fcx_ty_str(fcx, tgt));
}

/*
 * Destructuring assignment: `(a, s.f, _) = rhs` becomes
 *   { let (__lhs0, __lhs1, _) = rhs; a = __lhs0; s.f = __lhs1; }
 * The left-hand side expression is turned into a pattern whose leaves are
 * fresh locals, each followed by an ordinary assignment to the place.
 */
static bool is_rest_expr(const Expr *e) {
    return e->kind == EXPR_RANGE && !e->range.lo && !e->range.hi && !e->range.inclusive;
}

static bool lhs_is_destructuring(const Expr *e) {
    if (e->kind == EXPR_TUPLE || e->kind == EXPR_ARRAY || e->kind == EXPR_STRUCT)
        return true;
    return e->kind == EXPR_CALL && e->call.callee->kind == EXPR_PATH;
}

/* Tuples (without `..`) of places: the type can be found from the left. */
static bool lhs_is_simple(const Expr *e) {
    if (e->kind == EXPR_TUPLE) {
        for (uint32_t i = 0; i < e->elems.len; i++)
            if (is_rest_expr(e->elems.data[i]) || !lhs_is_simple(e->elems.data[i]))
                return false;
        return true;
    }
    return !lhs_is_destructuring(e);
}

static TPat *pat_new(TPatKind k, Span sp, Type *ty) {
    TPat *p = NEW(TPat);
    p->kind = (uint8_t)k;
    p->span = sp;
    p->ty = ty;
    return p;
}

static TPat *destructure_lhs(FnCtxt *fcx, Expr *e, Type *expected, TStmtVec *assigns);

static void destructure_positional(FnCtxt *fcx, ExprList *elems, uint32_t nfields, Type **field_tys,
                                   TPat *out, TStmtVec *assigns, Span sp) {
    int rest = -1;
    for (uint32_t i = 0; i < elems->len; i++)
        if (is_rest_expr(elems->data[i])) {
            if (rest >= 0)
                fatal(sp, "`..` can only be used once per pattern");
            rest = (int)i;
        }
    uint32_t n = elems->len - (rest >= 0 ? 1 : 0);
    if (rest < 0 ? n != nfields : n > nfields)
        fatal(sp, "this assignment has %u element(s) on the left, but the value has %u", n, nfields);
    out->subs = NEW_ARRAY(TPat *, n ? n : 1);
    out->field_idx = NEW_ARRAY(uint32_t, n ? n : 1);
    out->nsubs = n;
    uint32_t k = 0;
    for (uint32_t i = 0; i < elems->len; i++) {
        if ((int)i == rest)
            continue;
        uint32_t idx = (rest >= 0 && (int)i > rest) ? nfields - (elems->len - i) : i;
        out->field_idx[k] = idx;
        out->subs[k] = destructure_lhs(fcx, elems->data[i], field_tys[idx], assigns);
        k++;
    }
}

static TPat *destructure_lhs(FnCtxt *fcx, Expr *e, Type *expected, TStmtVec *assigns) {
    InferCtxt *icx = fcx->icx;
    switch (e->kind) {
    case EXPR_UNDERSCORE:
        return pat_new(TP_WILD, e->span, expected);
    case EXPR_TUPLE: {
        Type *t = structurally_resolve(fcx, expected);
        if (t->kind == T_INFER) {
            uint32_t k = e->elems.len;
            Type **v = NEW_ARRAY(Type *, k ? k : 1);
            for (uint32_t i = 0; i < k; i++) {
                if (is_rest_expr(e->elems.data[i]))
                    fatal(e->span, "type annotations needed for a destructuring assignment with `..`");
                v[i] = infer_var(icx, INFER_TY, e->span);
            }
            Type *nt = ty_tuple(substs_new(v, k));
            unify_or_fatal(fcx, t, nt, e->span);
            t = nt;
        }
        if (t->kind != T_TUPLE)
            fatal(e->span, "mismatched types: expected `%s`, found a tuple", fcx_ty_str(fcx, t));
        TPat *r = pat_new(TP_TUPLE, e->span, expected);
        destructure_positional(fcx, &e->elems, t->substs->len, t->substs->t, r, assigns, e->span);
        return r;
    }
    case EXPR_ARRAY: {
        Type *t = resolve_or_fatal(fcx, expected, e->span);
        if (t->kind != T_ARRAY && t->kind != T_SLICE)
            fatal(e->span, "mismatched types: expected `%s`, found an array", fcx_ty_str(fcx, t));
        int rest = -1;
        for (uint32_t i = 0; i < e->elems.len; i++)
            if (is_rest_expr(e->elems.data[i]))
                rest = (int)i;
        uint32_t n = e->elems.len - (rest >= 0 ? 1 : 0);
        TPat *r = pat_new(TP_SLICE, e->span, expected);
        r->subs = NEW_ARRAY(TPat *, n ? n : 1);
        r->nsubs = n;
        r->nprefix = rest >= 0 ? (uint32_t)rest : n;
        r->has_rest = rest >= 0;
        uint32_t k = 0;
        for (uint32_t i = 0; i < e->elems.len; i++)
            if ((int)i != rest)
                r->subs[k++] = destructure_lhs(fcx, e->elems.data[i], t->elem, assigns);
        return r;
    }
    case EXPR_CALL:
    case EXPR_STRUCT:
        if (e->kind == EXPR_STRUCT || e->call.callee->kind == EXPR_PATH) {
            bool is_struct = e->kind == EXPR_STRUCT;
            uint32_t variant;
            DefId ctor;
            Type *aty = is_struct
                            ? resolve_variant_path(fcx, e->struct_.qself, e->struct_.path, NS_TYPE,
                                                   &variant, &ctor, e->span)
                            : resolve_variant_path(fcx, e->call.callee->path.qself,
                                                   e->call.callee->path.path, NS_VALUE, &variant, &ctor,
                                                   e->span);
            unify_or_fatal(fcx, expected, aty, e->span);
            AdtDef *adt = adt_def(aty->def);
            VariantDef *v = &adt->variants[variant];
            TPat *r = pat_new(adt->is_enum ? TP_VARIANT : TP_TUPLE, e->span, expected);
            r->adt = aty->def;
            r->variant = variant;
            Type **ftys = NEW_ARRAY(Type *, v->nfields ? v->nfields : 1);
            for (uint32_t i = 0; i < v->nfields; i++)
                ftys[i] = fcx_normalize(fcx, ty_subst(v->fields[i].ty, aty->substs), e->span);
            if (!is_struct) {
                destructure_positional(fcx, &e->call.args, v->nfields, ftys, r, assigns, e->span);
            } else {
                uint32_t k = e->struct_.fields.len;
                r->subs = NEW_ARRAY(TPat *, k ? k : 1);
                r->field_idx = NEW_ARRAY(uint32_t, k ? k : 1);
                r->nsubs = k;
                for (uint32_t i = 0; i < k; i++) {
                    ExprField *f = e->struct_.fields.data[i];
                    int idx = variant_field_index(v, f->name);
                    if (idx < 0)
                        fatal(f->span, "`%s` has no field named `%s`", sym_str(v->name), sym_str(f->name));
                    r->field_idx[i] = (uint32_t)idx;
                    r->subs[i] = destructure_lhs(fcx, f->expr, ftys[idx], assigns);
                }
            }
            return r;
        }
        break;
    default:
        break;
    }
    /* a place: bind a fresh local and assign it afterwards */
    TExpr *l = check_expr(fcx, e, NULL);
    fix_place_mut(fcx, l);
    unify_or_fatal(fcx, expected, l->ty, e->span);
    Local *tmp = new_local(fcx, sym_intern_cstr("__lhs"), l->ty, false, e->span);
    TExpr *asg = te1(TE_ASSIGN, e->span, ty_unit(), l);
    asg->b = local_ref(tmp, e->span);
    TStmt *s = NEW(TStmt);
    s->kind = TS_EXPR;
    s->span = e->span;
    s->expr = asg;
    vec_push(*assigns, s);
    return bind_pat(tmp, e->span);
}

static TExpr *check_destructuring_assign(FnCtxt *fcx, Expr *e) {
    Expr *lhs = e->assign.lhs;
    TStmtVec stmts = {0};
    TStmt *let = NEW(TStmt);
    let->kind = TS_LET;
    let->span = e->span;
    vec_push(stmts, let);
    if (lhs_is_simple(lhs)) {
        Type *ty = infer_var(fcx->icx, INFER_TY, e->span);
        let->pat = destructure_lhs(fcx, lhs, ty, &stmts);
        let->init = check_expr_coerce(fcx, e->assign.rhs, ty);
    } else {
        let->init = check_expr(fcx, e->assign.rhs, NULL);
        let->pat = destructure_lhs(fcx, lhs, let->init->ty, &stmts);
    }
    TExpr *blk = te_new(TE_BLOCK, e->span, ty_unit());
    blk->stmts = stmts.data;
    blk->nstmts = stmts.len;
    return blk;
}

static TExpr *check_assign(FnCtxt *fcx, Expr *e) {
    Expr *lhs = e->assign.lhs;
    if (lhs->kind == EXPR_UNDERSCORE) {
        /* `_ = expr;` evaluates and drops the value. */
        TExpr *r = check_expr(fcx, e->assign.rhs, NULL);
        TExpr *blk = te_new(TE_BLOCK, e->span, ty_unit());
        TStmt *s = NEW(TStmt);
        s->kind = TS_EXPR;
        s->span = e->span;
        s->expr = r;
        blk->stmts = NEW_ARRAY(TStmt *, 1);
        blk->stmts[0] = s;
        blk->nstmts = 1;
        return blk;
    }
    if (lhs_is_destructuring(lhs))
        return check_destructuring_assign(fcx, e);
    TExpr *l = check_expr(fcx, lhs, NULL);
    fix_place_mut(fcx, l);
    TExpr *r = check_expr_coerce(fcx, e->assign.rhs, l->ty);
    TExpr *x = te1(TE_ASSIGN, e->span, ty_unit(), l);
    x->b = r;
    return x;
}

/* ------------------------------------------------------------------ */
/* Blocks                                                              */
/* ------------------------------------------------------------------ */

TExpr *check_block(FnCtxt *fcx, Block *b, Type *expected, Sym label) {
    Scope *saved = fcx->scope;
    DefId bm = resolve_block_module(b);
    if (bm)
        fcx->scope = scope_push(fcx->scope, SCOPE_MODULE, bm);
    fcx->scope = scope_push(fcx->scope, SCOPE_LOCALS, 0);
    BreakScope bs;
    if (label)
        push_break(fcx, &bs, BS_BLOCK, label,
                   expected ? expected : infer_var(fcx->icx, INFER_TY, b->span));
    TStmtVec stmts = {0};
    Expr *tail = block_tail(b);
    for (uint32_t i = 0; i < b->stmts.len; i++) {
        Stmt *s = b->stmts.data[i];
        switch (s->kind) {
        case STMT_LET: {
            TStmt *ts = NEW(TStmt);
            ts->kind = TS_LET;
            ts->span = s->span;
            Type *ty = s->let.ty ? lower_body_ty(fcx, s->let.ty) : NULL;
            if (s->let.init) {
                if (ty) {
                    ts->init = check_expr_coerce(fcx, s->let.init, ty);
                } else {
                    ts->init = check_expr(fcx, s->let.init, NULL);
                    ty = ts->init->ty;
                }
            } else if (!ty) {
                ty = infer_var(fcx->icx, INFER_TY, s->span);
            }
            if (s->let.els) {
                bool d = fcx->diverges;
                TExpr *els = check_block(fcx, s->let.els, ty_never(), 0);
                ts->els = coerce(fcx, els, ty_never());
                fcx->diverges = d;
            }
            /* Each `let` opens a new rib so that later items/closures see
               exactly the bindings declared before them. */
            fcx->scope = scope_push(fcx->scope, SCOPE_LOCALS, 0);
            ts->pat = check_pat_top(fcx, s->let.pat, ty);
            vec_push(stmts, ts);
            break;
        }
        case STMT_EXPR: {
            if (s->expr.expr == tail && i == b->stmts.len - 1)
                break;
            TStmt *ts = NEW(TStmt);
            ts->kind = TS_EXPR;
            ts->span = s->span;
            if (s->expr.semi) {
                ts->expr = check_expr(fcx, s->expr.expr, NULL);
            } else {
                ts->expr = coerce(fcx, check_expr(fcx, s->expr.expr, ty_unit()), ty_unit());
            }
            vec_push(stmts, ts);
            break;
        }
        case STMT_ITEM:
        case STMT_EMPTY:
            break;
        case STMT_MAC:
            fatal(s->span, "unexpanded macro invocation in statement position");
        }
    }
    TExpr *blk = te_new(TE_BLOCK, b->span, NULL);
    blk->stmts = stmts.data;
    blk->nstmts = stmts.len;
    if (tail) {
        blk->a = check_expr(fcx, tail, label ? bs.ty : expected);
        blk->ty = blk->a->ty;
    } else {
        blk->ty = fcx->diverges ? ty_never() : ty_unit();
    }
    if (label) {
        fcx->breaks = bs.parent;
        if (bs.has_break) {
            if (blk->a)
                blk->a = coerce(fcx, blk->a, bs.ty);
            else if (infer_shallow(fcx->icx, blk->ty)->kind != T_NEVER)
                unify_or_fatal(fcx, bs.ty, ty_unit(), b->span);
            blk->ty = bs.ty;
            fcx->diverges = false;
        }
        blk->label = bs.id;
    }
    fcx->scope = saved;
    return blk;
}

/* ------------------------------------------------------------------ */
/* check_expr                                                          */
/* ------------------------------------------------------------------ */

static TExpr *check_expr_kind(FnCtxt *fcx, Expr *e, Type *expected) {
    InferCtxt *icx = fcx->icx;
    switch (e->kind) {
    case EXPR_LIT:
        return check_lit(fcx, e, expected);
    case EXPR_PATH:
        return check_path_expr(fcx, e, expected);
    case EXPR_UNARY:
        return check_unary(fcx, e, expected);
    case EXPR_ADDR_OF: {
        Type *x = expected ? infer_shallow(icx, expected) : NULL;
        Type *hint = x && (x->kind == T_REF || x->kind == T_PTR) ? x->elem : NULL;
        TExpr *a = check_expr(fcx, e->addr_of.operand, hint);
        if (e->addr_of.is_mut)
            fix_place_mut(fcx, a);
        if (e->addr_of.raw) {
            TExpr *r = te1(TE_ADDR_OF, e->span, ty_ptr(a->ty, e->addr_of.is_mut), a);
            r->is_mut = e->addr_of.is_mut;
            return r;
        }
        TExpr *r = make_ref(e->span, a, e->addr_of.is_mut);
        return r;
    }
    case EXPR_BINARY:
        if (e->binary.op == BIN_AND || e->binary.op == BIN_OR) {
            TExpr *a = check_expr_coerce(fcx, e->binary.lhs, ty_bool());
            bool d = fcx->diverges;
            TExpr *b = check_expr_coerce(fcx, e->binary.rhs, ty_bool());
            fcx->diverges = d;
            TExpr *x = te1(TE_LOGICAL, e->span, ty_bool(), a);
            x->op = (uint8_t)e->binary.op;
            x->b = b;
            return x;
        }
        return check_binop(fcx, e, false);
    case EXPR_ASSIGN:
        return check_assign(fcx, e);
    case EXPR_ASSIGN_OP:
        return check_binop(fcx, e, true);
    case EXPR_CAST:
        return check_cast(fcx, e);
    case EXPR_CALL:
        return check_call(fcx, e, expected);
    case EXPR_METHOD_CALL:
        return check_method_call(fcx, e, expected);
    case EXPR_FIELD:
        return check_field(fcx, e);
    case EXPR_INDEX:
        return check_index(fcx, e);
    case EXPR_STRUCT:
        return check_struct_expr(fcx, e, expected);
    case EXPR_TUPLE:
        return check_tuple(fcx, e, expected);
    case EXPR_ARRAY:
    case EXPR_REPEAT:
        return check_array(fcx, e, expected);
    case EXPR_RANGE:
        return check_range(fcx, e, expected);
    case EXPR_CLOSURE:
        return check_closure(fcx, e, expected);
    case EXPR_BLOCK:
        return check_block(fcx, e->block.block, expected, e->block.label);
    case EXPR_IF:
        return check_if(fcx, e, expected);
    case EXPR_LET:
        return check_let_cond(fcx, e);
    case EXPR_MATCH:
        return check_match(fcx, e, expected);
    case EXPR_WHILE:
        return check_while(fcx, e);
    case EXPR_FOR:
        return check_for(fcx, e);
    case EXPR_LOOP:
        return check_loop(fcx, e, expected);
    case EXPR_BREAK: {
        BreakScope *bs = find_break(fcx, e->break_.label, false, e->span);
        TExpr *x = te_new(TE_BREAK, e->span, ty_never());
        x->label = bs->id;
        if (e->break_.value) {
            if (bs->kind == BS_WHILE)
                fatal(e->span, "`break` with a value inside a `while`/`for` loop");
            x->a = check_expr_coerce(fcx, e->break_.value, bs->ty);
        } else if (bs->kind != BS_WHILE) {
            unify_or_fatal(fcx, bs->ty, ty_unit(), e->span);
        }
        bs->has_break = true;
        return x;
    }
    case EXPR_CONTINUE: {
        BreakScope *bs = find_break(fcx, e->continue_.label, true, e->span);
        TExpr *x = te_new(TE_CONTINUE, e->span, ty_never());
        x->label = bs->id;
        return x;
    }
    case EXPR_RETURN: {
        TExpr *x = te_new(TE_RETURN, e->span, ty_never());
        if (e->ret)
            x->a = check_expr_coerce(fcx, e->ret, fcx->ret_ty);
        else
            unify_or_fatal(fcx, fcx->ret_ty, ty_unit(), e->span);
        return x;
    }
    case EXPR_TRY:
        return check_try(fcx, e);
    case EXPR_MAC:
        fatal(e->span, "unexpanded macro invocation");
    case EXPR_UNDERSCORE:
        fatal(e->span, "`_` is only allowed on the left-hand side of an assignment");
    }
    fatal(e->span, "internal: unknown expression kind");
}

TExpr *check_expr(FnCtxt *fcx, Expr *e, Type *expected) {
    TExpr *r = check_expr_kind(fcx, e, expected);
    if (infer_shallow(fcx->icx, r->ty)->kind == T_NEVER)
        fcx->diverges = true;
    return r;
}
