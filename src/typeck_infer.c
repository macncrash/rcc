/* typeck_infer.c - inference variables and unification. See typeck.h. */
#include "typeck.h"

InferCtxt *infer_new(PredList *env) {
    InferCtxt *icx = NEW(InferCtxt);
    icx->env = env;
    return icx;
}

Type *infer_var(InferCtxt *icx, InferKind k, Span origin) {
    if (icx->nvars == icx->vars_cap) {
        uint32_t nc = icx->vars_cap ? icx->vars_cap * 2 : 32;
        TyVar *n = NEW_ARRAY(TyVar, nc);
        if (icx->nvars)
            memcpy(n, icx->vars, sizeof(TyVar) * icx->nvars);
        icx->vars = n;
        icx->vars_cap = nc;
    }
    TyVar *v = &icx->vars[icx->nvars];
    v->node = ty_infer_node(icx->nvars, k);
    v->value = NULL;
    v->origin = origin;
    v->diverging = false;
    icx->nvars++;
    return v->node;
}

Type *infer_shallow(InferCtxt *icx, Type *t) {
    while (t->kind == T_INFER) {
        Type *v = icx->vars[t->index].value;
        if (!v)
            break;
        t = v;
    }
    return t;
}

static Type *resolve_fold(TypeFolder *f, Type *t) {
    if (t->kind == T_INFER) {
        Type *r = infer_shallow(f->ctx, t);
        if (r->kind == T_INFER)
            return r;
        return ty_fold(f, r);
    }
    return ty_super_fold(f, t);
}

Type *infer_resolve(InferCtxt *icx, Type *t) {
    if (!(t->flags & TYF_HAS_INFER))
        return t;
    TypeFolder f = {resolve_fold, TYF_HAS_INFER, icx, NULL};
    return ty_fold(&f, t);
}

Substs *infer_resolve_substs(InferCtxt *icx, Substs *s) {
    if (!(s->flags & TYF_HAS_INFER))
        return s;
    TypeFolder f = {resolve_fold, TYF_HAS_INFER, icx, NULL};
    return substs_fold(&f, s);
}

Snapshot infer_snapshot(InferCtxt *icx) {
    Snapshot s = {icx->nundo, icx->npending};
    return s;
}

void infer_rollback(InferCtxt *icx, Snapshot s) {
    while (icx->nundo > s.nundo)
        icx->vars[icx->undo[--icx->nundo]].value = NULL;
    icx->npending = s.npending;
}

static void bind_var(InferCtxt *icx, Type *var, Type *value) {
    icx->vars[var->index].value = value;
    if (icx->nundo == icx->undo_cap) {
        uint32_t nc = icx->undo_cap ? icx->undo_cap * 2 : 64;
        uint32_t *n = NEW_ARRAY(uint32_t, nc);
        if (icx->nundo)
            memcpy(n, icx->undo, sizeof(uint32_t) * icx->nundo);
        icx->undo = n;
        icx->undo_cap = nc;
    }
    icx->undo[icx->nundo++] = var->index;
}

static bool occurs(InferCtxt *icx, Type *var, Type *t) {
    if (!(t->flags & TYF_HAS_INFER))
        return false;
    t = infer_shallow(icx, t);
    if (t == var)
        return true;
    if (t->kind == T_INFER)
        return false;
    if (t->elem && occurs(icx, var, t->elem))
        return true;
    if (t->len && occurs(icx, var, t->len))
        return true;
    if (t->substs)
        for (uint32_t i = 0; i < t->substs->len; i++)
            if (occurs(icx, var, t->substs->t[i]))
                return true;
    for (uint32_t i = 0; i < t->nbinds; i++)
        if (occurs(icx, var, t->binds[i].ty))
            return true;
    return false;
}

static bool unify(InferCtxt *icx, Type *a, Type *b);

static bool unify_var(InferCtxt *icx, Type *var, Type *t) {
    if (t->kind == T_INFER) {
        /* var := var. Keep the more specific kind as the root. */
        if (var->ik != INFER_TY && t->ik == INFER_TY) {
            Type *tmp = var;
            var = t;
            t = tmp;
        } else if (var->ik != INFER_TY && t->ik != INFER_TY && var->ik != t->ik) {
            return false;
        }
        TyVar *vv = &icx->vars[var->index], *tv = &icx->vars[t->index];
        tv->diverging = tv->diverging && vv->diverging;
        bind_var(icx, var, t);
        return true;
    }
    if (var->ik == INFER_INT && t->kind != T_INT)
        return false;
    if (var->ik == INFER_FLOAT && t->kind != T_FLOAT)
        return false;
    if (occurs(icx, var, t))
        return false;
    bind_var(icx, var, t);
    return true;
}

static bool unify_substs(InferCtxt *icx, Substs *a, Substs *b) {
    if (a == b)
        return true;
    if (a->len != b->len)
        return false;
    for (uint32_t i = 0; i < a->len; i++)
        if (!unify(icx, a->t[i], b->t[i]))
            return false;
    return true;
}

static bool unify(InferCtxt *icx, Type *a, Type *b) {
    a = infer_shallow(icx, a);
    b = infer_shallow(icx, b);
    if (a == b)
        return true;
    if (a->kind == T_INFER)
        return unify_var(icx, a, b);
    if (b->kind == T_INFER)
        return unify_var(icx, b, a);
    if (a->kind != b->kind)
        return false;
    switch ((TypeKind)a->kind) {
    case T_ADT: case T_FNDEF: case T_CLOSURE: case T_PROJ:
        return a->def == b->def && unify_substs(icx, a->substs, b->substs);
    case T_OPAQUE:
        return a->def == b->def && a->index == b->index && unify_substs(icx, a->substs, b->substs);
    case T_TUPLE: case T_FNPTR:
        return unify_substs(icx, a->substs, b->substs);
    case T_REF: case T_PTR:
        return a->is_mut == b->is_mut && unify(icx, a->elem, b->elem);
    case T_SLICE:
        return unify(icx, a->elem, b->elem);
    case T_ARRAY:
        return unify(icx, a->elem, b->elem) && unify(icx, a->len, b->len);
    case T_DYN:
        if (a->def != b->def || a->nbinds != b->nbinds || !unify_substs(icx, a->substs, b->substs))
            return false;
        for (uint32_t i = 0; i < a->nbinds; i++)
            if (a->binds[i].assoc != b->binds[i].assoc || !unify(icx, a->binds[i].ty, b->binds[i].ty))
                return false;
        return true;
    default:
        return false; /* interned leaves: equal iff pointer-equal */
    }
}

bool infer_unify(InferCtxt *icx, Type *a, Type *b) {
    uint32_t mark = icx->nundo;
    if (unify(icx, a, b))
        return true;
    while (icx->nundo > mark)
        icx->vars[icx->undo[--icx->nundo]].value = NULL;
    return false;
}

bool infer_unify_substs(InferCtxt *icx, Substs *a, Substs *b) {
    uint32_t mark = icx->nundo;
    if (unify_substs(icx, a, b))
        return true;
    while (icx->nundo > mark)
        icx->vars[icx->undo[--icx->nundo]].value = NULL;
    return false;
}

Substs *infer_fresh_own_substs(InferCtxt *icx, DefId def, Substs *parent, Span sp) {
    GenericsDef *g = generics_of(def);
    if (parent->len > g->count)
        fatal(sp, "internal: too many parent substs for %s", def_path_str(def));
    Type **t = NEW_ARRAY(Type *, g->count ? g->count : 1);
    memcpy(t, parent->t, sizeof(Type *) * parent->len);
    for (uint32_t i = parent->len; i < g->count; i++)
        t[i] = infer_var(icx, INFER_TY, sp);
    return substs_new(t, g->count);
}

Substs *infer_fresh_substs(InferCtxt *icx, DefId def, Span sp) {
    return infer_fresh_own_substs(icx, def, substs_empty(), sp);
}
