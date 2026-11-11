/*
 * typeck_method.c - value paths, associated-item lookup, and method
 * probing.
 *
 * Method probing follows rustc: build the auto-deref chain of the receiver
 * type (plus a final array -> slice unsizing step); for each step, try the
 * receiver by value, then auto-ref'd with `&`, then with `&mut`; at each of
 * those, inherent candidates (inherent impls, bounds on type parameters,
 * methods of a trait object) are preferred to extension candidates (methods
 * of traits in scope). A candidate applies if the method's `self` parameter
 * type unifies with the adjusted receiver type and, for trait methods, the
 * `Self: Trait` predicate does not definitely fail.
 */
#include "typeck_fn.h"

#include <stdio.h>

static Sym sym_self_ty(void) { return (Sym)KW_SELF_TYPE; }

static const char *ast_path_str_for_error(const Path *p) {
    StrBuf sb = {0};
    for (uint32_t i = 0; i < p->segs.len; i++) {
        if (i || p->global)
            sb_puts(&sb, "::");
        sb_puts(&sb, sym_str(p->segs.data[i]->name));
    }
    return sb_str(&sb);
}

static Type *lower_here(FnCtxt *fcx, Ty *ast) {
    fcx->lc.scope = fcx->scope;
    return fcx_normalize(fcx, lower_ty(&fcx->lc, ast), ast->span);
}

static Substs *args_here(FnCtxt *fcx, DefId def, GenericArgs *args, Substs *parent, Span sp) {
    fcx->lc.scope = fcx->scope;
    return lower_generic_args(&fcx->lc, def, args, parent, true, sp);
}

/* A fn item (or method) as a value, with its predicates registered. */
static TExpr *value_fn(FnCtxt *fcx, DefId def, Substs *substs, Span sp) {
    infer_register_preds(fcx->icx, predicates_of(def), substs, sp);
    TExpr *x = te_new(TE_FN, sp, ty_fndef(def, substs));
    x->item.def = def;
    x->item.substs = substs;
    return x;
}

static TExpr *value_const(FnCtxt *fcx, DefId def, Substs *substs, Span sp) {
    TExpr *x = te_new(TE_CONST, sp, fcx_normalize(fcx, ty_subst(type_of(def), substs), sp));
    x->item.def = def;
    x->item.substs = substs;
    return x;
}

/* The value of a struct or variant constructor: a unit value or a fn. */
static TExpr *value_ctor(FnCtxt *fcx, DefId ctor, Type *adt_ty, uint32_t variant, Span sp) {
    AdtDef *adt = adt_def(adt_ty->def);
    VariantDef *v = &adt->variants[variant];
    (void)fcx;
    if (v->ctor == VD_UNIT) {
        TExpr *x = te_new(TE_ADT, sp, adt_ty);
        x->variant = variant;
        return x;
    }
    if (v->ctor != VD_TUPLE)
        fatal(sp, "`%s` is a struct-like constructor; use `%s { .. }`", sym_str(v->name),
              sym_str(v->name));
    TExpr *x = te_new(TE_FN, sp, ty_fndef(ctor, adt_ty->substs));
    x->item.def = ctor;
    x->item.substs = adt_ty->substs;
    return x;
}

static TExpr *trait_item_value(FnCtxt *fcx, DefId item, Substs *trait_substs, PathSegment *seg,
                               Span sp) {
    Def *d = def_get(item);
    if (d->kind == DEF_FN)
        return value_fn(fcx, item, args_here(fcx, item, seg->args, trait_substs, sp), sp);
    if (d->kind == DEF_ASSOC_CONST) {
        Predicate p = {PRED_TRAIT, {d->parent, trait_substs}, 0, NULL};
        infer_register(fcx->icx, p, sp);
        return value_const(fcx, item, trait_substs, sp);
    }
    fatal(sp, "`%s` is not a value", sym_str(d->name));
}

static int find_variant(AdtDef *adt, Sym name) {
    if (!adt->is_enum)
        return -1;
    for (uint32_t i = 0; i < adt->nvariants; i++)
        if (adt->variants[i].name == name)
            return (int)i;
    return -1;
}

/* `Type::name` in value position. */
static TExpr *resolve_assoc_value(FnCtxt *fcx, Type *self, PathSegment *seg, Span sp) {
    InferCtxt *icx = fcx->icx;
    Sym name = seg->name;
    Type *sr = structurally_resolve(fcx, self);
    if (sr->kind == T_ADT) {
        AdtDef *adt = adt_def(sr->def);
        int vi = find_variant(adt, name);
        if (vi >= 0)
            return value_ctor(fcx, adt->variants[vi].def, sr, (uint32_t)vi, sp);
    }
    /* Inherent impls. */
    const DefIdList *impls = inherent_impls_for(sr);
    for (uint32_t i = 0; impls && i < impls->len; i++) {
        DefId impl = impls->data[i];
        DefId item = impl_item(impl, name);
        if (!item)
            continue;
        Def *d = def_get(item);
        if (d->kind != DEF_FN && d->kind != DEF_ASSOC_CONST)
            continue;
        Snapshot s = infer_snapshot(icx);
        Substs *fresh = infer_fresh_substs(icx, impl, sp);
        Type *ity = fcx_normalize(fcx, ty_subst(impl_def(impl)->self_ty, fresh), sp);
        if (!infer_unify(icx, ity, sr)) {
            infer_rollback(icx, s);
            continue;
        }
        if (d->kind == DEF_FN)
            return value_fn(fcx, item, args_here(fcx, item, seg->args, fresh, sp), sp);
        infer_register_preds(icx, predicates_of(impl), fresh, sp);
        return value_const(fcx, item, fresh, sp);
    }
    /* Bounds in scope on `self` (type parameters, projections). */
    if (icx->env) {
        for (uint32_t i = 0; i < icx->env->len; i++) {
            Predicate *p = &icx->env->data[i];
            if (p->kind != PRED_TRAIT || infer_resolve(icx, p->tr.substs->t[0]) != sr)
                continue;
            DefId item = trait_item(p->tr.trait, name);
            if (item && def_get(item)->kind != DEF_ASSOC_TY)
                return trait_item_value(fcx, item, p->tr.substs, seg, sp);
        }
    }
    /* Any trait with such an item that `self` may implement; traits in
       scope are preferred. */
    const DefIdList *items = trait_items_named(name);
    for (int pass = 0; pass < 2 && items; pass++) {
        for (uint32_t i = 0; i < items->len; i++) {
            DefId item = items->data[i];
            Def *d = def_get(item);
            if (d->kind == DEF_ASSOC_TY)
                continue;
            if (trait_in_scope(fcx->scope, d->parent) != (pass == 0))
                continue;
            Snapshot s = infer_snapshot(icx);
            Substs *ts = infer_fresh_own_substs(icx, d->parent, substs_1(sr), sp);
            Predicate p = {PRED_TRAIT, {d->parent, ts}, 0, NULL};
            if (trait_eval(icx, &p) == EVAL_NO) {
                infer_rollback(icx, s);
                continue;
            }
            return trait_item_value(fcx, item, ts, seg, sp);
        }
    }
    fatal(sp, "no associated item named `%s` found for type `%s`", sym_str(name),
          fcx_ty_str(fcx, sr));
}

static Type *self_ty_of(Res r) {
    return def_get(r.def)->kind == DEF_IMPL ? type_of(r.def) : ty_param(0, sym_self_ty());
}

TExpr *instantiate_value_path(FnCtxt *fcx, QSelf *qself, Path *p, Span sp) {
    InferCtxt *icx = fcx->icx;
    uint32_t n = p->segs.len;
    if (qself) {
        Type *self = lower_here(fcx, qself->ty);
        if (qself->position == 0) {
            if (n != 1)
                fatal(sp, "unsupported qualified path");
            return resolve_assoc_value(fcx, self, p->segs.data[0], sp);
        }
        Path tp = *p;
        tp.segs.len = qself->position;
        PathRes tr = resolve_path(fcx->scope, &tp, NS_TYPE);
        if (tr.res.kind != RES_DEF || def_get(tr.res.def)->kind != DEF_TRAIT)
            fatal(sp, "expected a trait in qualified path");
        if (n != qself->position + 1)
            fatal(sp, "unsupported qualified path");
        fcx->lc.scope = fcx->scope;
        Substs *ts = lower_generic_args(&fcx->lc, tr.res.def, p->segs.data[qself->position - 1]->args,
                                        substs_1(self), true, sp);
        TraitRef tref = {tr.res.def, ts}, owner;
        PathSegment *seg = p->segs.data[qself->position];
        DefId item = trait_item_deep(tref, seg->name, &owner);
        if (!item)
            fatal(sp, "trait `%s` has no item `%s`", sym_str(def_get(tr.res.def)->name),
                  sym_str(seg->name));
        return trait_item_value(fcx, item, owner.substs, seg, sp);
    }
    PathRes r = resolve_path(fcx->scope, p, NS_VALUE);
    PathSegment *last = p->segs.data[n - 1];
    Type *self = NULL;
    switch (r.res.kind) {
    case RES_NONE:
        fatal(sp, "cannot find value `%s` in this scope", ast_path_str_for_error(p));
    case RES_LOCAL: {
        Local *l = r.res.local;
        TExpr *x = te_new(TE_LOCAL, sp, l->ty);
        x->local = l;
        return x;
    }
    case RES_PRIM:
        self = r.res.prim;
        break;
    case RES_SELF_TY:
        self = self_ty_of(r.res);
        if (r.nsegs == n) {
            Type *st = structurally_resolve(fcx, self);
            if (st->kind != T_ADT || adt_def(st->def)->is_enum)
                fatal(sp, "`Self` is not a struct constructor here");
            return value_ctor(fcx, st->def, st, 0, sp);
        }
        break;
    case RES_DEF: {
        DefId def = r.res.def;
        Def *d = def_get(def);
        PathSegment *seg = p->segs.data[r.nsegs - 1];
        if (r.nsegs == n) {
            switch (d->kind) {
            case DEF_FN:
                return value_fn(fcx, def, args_here(fcx, def, last->args, substs_empty(), sp), sp);
            case DEF_CONST:
                return value_const(fcx, def, substs_empty(), sp);
            case DEF_STATIC: {
                TExpr *x = te_new(TE_STATIC, sp, type_of(def));
                x->item.def = def;
                x->item.substs = substs_empty();
                return x;
            }
            case DEF_STRUCT: {
                Type *t = ty_adt(def, args_here(fcx, def, last->args, substs_empty(), sp));
                return value_ctor(fcx, def, t, 0, sp);
            }
            case DEF_VARIANT: {
                GenericArgs *ga = last->args;
                if (!ga && n >= 2)
                    ga = p->segs.data[n - 2]->args;
                Type *t = ty_adt(d->parent, args_here(fcx, d->parent, ga, substs_empty(), sp));
                return value_ctor(fcx, def, t, d->index, sp);
            }
            case DEF_CONST_PARAM: {
                Type *pt = type_of(def);
                fcx->lc.scope = fcx->scope;
                TExpr *x = te_new(TE_CONST_PARAM, sp, lower_ty(&fcx->lc, d->gparam->const_ty));
                x->index = pt->index;
                return x;
            }
            default:
                fatal(sp, "expected a value, found %s `%s`", def_kind_str(d->kind),
                      def_path_str(def));
            }
        }
        switch (d->kind) {
        case DEF_STRUCT: case DEF_ENUM: case DEF_UNION:
            self = ty_adt(def, args_here(fcx, def, seg->args, substs_empty(), sp));
            break;
        case DEF_TYPE_ALIAS:
            self = fcx_normalize(
                fcx, ty_subst(type_of(def), args_here(fcx, def, seg->args, substs_empty(), sp)), sp);
            break;
        case DEF_TY_PARAM:
            self = type_of(def);
            break;
        case DEF_TRAIT: {
            if (r.nsegs + 1 != n)
                fatal(sp, "unsupported path through trait `%s`", sym_str(d->name));
            Type *sv = infer_var(icx, INFER_TY, sp);
            Substs *ts = args_here(fcx, def, seg->args, substs_1(sv), sp);
            TraitRef tref = {def, ts}, owner;
            DefId item = trait_item_deep(tref, last->name, &owner);
            if (!item)
                fatal(sp, "trait `%s` has no item `%s`", sym_str(d->name), sym_str(last->name));
            return trait_item_value(fcx, item, owner.substs, last, sp);
        }
        default:
            fatal(sp, "cannot find `%s` in %s `%s`", sym_str(p->segs.data[r.nsegs]->name),
                  def_kind_str(d->kind), def_path_str(def));
        }
        break;
    }
    }
    if (r.nsegs + 1 != n)
        fatal(sp, "unsupported path: too many segments after a type");
    return resolve_assoc_value(fcx, self, last, sp);
}

TExpr *check_path_expr(FnCtxt *fcx, Expr *e, Type *expected) {
    (void)expected;
    return instantiate_value_path(fcx, e->path.qself, e->path.path, e->span);
}

Type *resolve_variant_path(FnCtxt *fcx, QSelf *qself, Path *p, Namespace ns, uint32_t *variant,
                           DefId *ctor_def, Span sp) {
    if (qself)
        fatal(sp, "qualified paths in struct expressions/patterns are not supported");
    uint32_t n = p->segs.len;
    PathRes r = resolve_path(fcx->scope, p, ns);
    Type *t = NULL;
    *variant = 0;
    if (r.res.kind == RES_DEF) {
        DefId def = r.res.def;
        Def *d = def_get(def);
        PathSegment *seg = p->segs.data[r.nsegs - 1];
        if (d->kind == DEF_VARIANT && r.nsegs == n) {
            GenericArgs *ga = seg->args;
            if (!ga && n >= 2)
                ga = p->segs.data[n - 2]->args;
            *variant = d->index;
            *ctor_def = def;
            return ty_adt(d->parent, args_here(fcx, d->parent, ga, substs_empty(), sp));
        }
        if (d->kind == DEF_STRUCT || d->kind == DEF_UNION || d->kind == DEF_ENUM)
            t = ty_adt(def, args_here(fcx, def, seg->args, substs_empty(), sp));
        else if (d->kind == DEF_TYPE_ALIAS)
            t = structurally_resolve(
                fcx, fcx_normalize(fcx, ty_subst(type_of(def),
                                                 args_here(fcx, def, seg->args, substs_empty(), sp)),
                                   sp));
    } else if (r.res.kind == RES_SELF_TY) {
        t = structurally_resolve(fcx, self_ty_of(r.res));
    }
    if (!t || t->kind != T_ADT)
        fatal(sp, "cannot find struct, variant or union type `%s`", sym_str(p->segs.data[n - 1]->name));
    AdtDef *adt = adt_def(t->def);
    if (r.nsegs == n) {
        if (adt->is_enum)
            fatal(sp, "expected a struct or variant, found enum `%s`", sym_str(def_get(t->def)->name));
        *ctor_def = t->def;
        return t;
    }
    if (r.nsegs + 1 != n)
        fatal(sp, "unsupported path in struct expression/pattern");
    int vi = find_variant(adt, p->segs.data[n - 1]->name);
    if (vi < 0)
        fatal(sp, "no variant `%s` in `%s`", sym_str(p->segs.data[n - 1]->name),
              sym_str(def_get(t->def)->name));
    *variant = (uint32_t)vi;
    *ctor_def = adt->variants[vi].def;
    return t;
}

/* ------------------------------------------------------------------ */
/* Method probing                                                      */
/* ------------------------------------------------------------------ */

typedef struct Probe {
    FnCtxt *fcx;
    Sym name;
    GenericArgs *turbofish;
    Span sp;
    DefId def;
    Substs *substs;
} Probe;

/* Does method `m` with the given parent substs accept `self_ty` as its
   receiver? On success the unification is kept and pr->def/substs set. */
static bool try_method(Probe *pr, DefId m, Substs *parent, Type *self_ty, const Predicate *must_hold,
                       EvalResult *eval_out) {
    FnCtxt *fcx = pr->fcx;
    InferCtxt *icx = fcx->icx;
    if (def_get(m)->kind != DEF_FN)
        return false;
    FnSigT *sig = fn_sig(m);
    if (!sig->has_self)
        return false;
    Snapshot s = infer_snapshot(icx);
    icx->probe_depth++;
    Substs *ms = args_here(fcx, m, pr->turbofish, parent, pr->sp);
    Type *xform = fcx_normalize(fcx, ty_subst(sig->inputs->t[0], ms), pr->sp);
    bool ok = infer_unify(icx, xform, self_ty);
    icx->probe_depth--;
    EvalResult ev = EVAL_YES;
    if (ok && must_hold) {
        ev = trait_eval(icx, must_hold);
        ok = ev != EVAL_NO;
    }
    if (!ok) {
        infer_rollback(icx, s);
        return false;
    }
    if (eval_out)
        *eval_out = ev;
    pr->def = m;
    pr->substs = ms;
    return true;
}

static bool probe_bounds(Probe *pr, PredList *preds, Type *step, Type *self_ty) {
    InferCtxt *icx = pr->fcx->icx;
    for (uint32_t i = 0; preds && i < preds->len; i++) {
        Predicate *p = &preds->data[i];
        if (p->kind != PRED_TRAIT || infer_resolve(icx, p->tr.substs->t[0]) != step)
            continue;
        DefId m = trait_item(p->tr.trait, pr->name);
        if (m && try_method(pr, m, p->tr.substs, self_ty, NULL, NULL))
            return true;
    }
    return false;
}

/* Inherent candidates contributed by the type `step` (its inherent impls,
   bounds on a type parameter, the methods of a trait object) whose `self`
   parameter accepts `self_ty`. */
static bool probe_inherent(Probe *pr, Type *step, Type *self_ty) {
    FnCtxt *fcx = pr->fcx;
    InferCtxt *icx = fcx->icx;
    const DefIdList *impls = inherent_impls_for(step);
    for (uint32_t i = 0; impls && i < impls->len; i++) {
        DefId impl = impls->data[i];
        DefId m = impl_item(impl, pr->name);
        if (!m)
            continue;
        Snapshot s = infer_snapshot(icx);
        Substs *fresh = infer_fresh_substs(icx, impl, pr->sp);
        Type *ity = fcx_normalize(fcx, ty_subst(impl_def(impl)->self_ty, fresh), pr->sp);
        if (infer_unify(icx, ity, step)) {
            /* the impl's where-clauses must not definitely fail (several
               inherent impls may define the same method name) */
            bool ok = true;
            PredList *ip = predicates_of(impl);
            for (uint32_t k = 0; k < ip->len && ok; k++) {
                if (ip->data[k].kind != PRED_TRAIT)
                    continue;
                Predicate p = ip->data[k];
                p.tr.substs = substs_subst(p.tr.substs, fresh);
                if (p.tr.substs->flags & TYF_HAS_PROJ)
                    continue;
                ok = trait_eval(icx, &p) != EVAL_NO;
            }
            if (ok && try_method(pr, m, fresh, self_ty, NULL, NULL))
                return true;
        }
        infer_rollback(icx, s);
    }
    if (step->kind == T_PARAM || step->kind == T_PROJ || step->kind == T_OPAQUE) {
        if (probe_bounds(pr, icx->env, step, self_ty))
            return true;
        if (step->kind != T_PARAM) {
            /* bounds declared on the associated / opaque type */
            PredList sb = {0};
            if (step->kind == T_PROJ) {
                PredList *ib = trait_item_bounds(def_get(step->def)->parent);
                for (uint32_t i = 0; i < ib->len; i++) {
                    Predicate p = ib->data[i];
                    p.tr.substs = substs_subst(p.tr.substs, step->substs);
                    pred_push(&sb, p);
                    elaborate_pred(&sb, &p);
                }
            } else {
                (void)fn_sig(step->def);
                OpaqueInfo *oi = def_get(step->def)->opaques;
                if (oi && step->index < oi->n)
                    for (uint32_t i = 0; i < oi->bounds[step->index].len; i++) {
                        Predicate p = oi->bounds[step->index].data[i];
                        p.tr.substs = substs_subst(p.tr.substs, step->substs);
                        pred_push(&sb, p);
                        elaborate_pred(&sb, &p);
                    }
            }
            if (probe_bounds(pr, &sb, step, self_ty))
                return true;
        }
    }
    if (step->kind == T_DYN) {
        PredList sup = {0};
        TraitRef principal = {step->def, substs_concat(substs_1(step), step->substs)};
        supertraits_of(principal, &sup);
        if (probe_bounds(pr, &sup, step, self_ty))
            return true;
    }
    return false;
}

/* The trait of the enclosing `impl Trait for ..` / `trait` item is in scope
   for method calls even when it was not imported by name. */
static bool trait_usable(FnCtxt *fcx, DefId trait) {
    if (trait_in_scope(fcx->scope, trait))
        return true;
    for (Scope *s = fcx->scope; s; s = s->parent) {
        if (s->kind != SCOPE_SELF)
            continue;
        Def *d = def_get(s->def);
        if (d->kind == DEF_TRAIT && s->def == trait)
            return true;
        if (d->kind == DEF_IMPL) {
            ImplDef *id = impl_def(s->def);
            if (id->of_trait && id->trait_ref.trait == trait)
                return true;
        }
    }
    return false;
}

/* Extension candidates: methods of traits in scope. */
static bool probe_extension(Probe *pr, Type *self_ty) {
    FnCtxt *fcx = pr->fcx;
    InferCtxt *icx = fcx->icx;
    const DefIdList *items = trait_items_named(pr->name);
    DefId maybe_def = 0;
    for (uint32_t i = 0; items && i < items->len; i++) {
        DefId m = items->data[i];
        Def *d = def_get(m);
        if (d->kind != DEF_FN || !trait_usable(fcx, d->parent))
            continue;
        Snapshot s = infer_snapshot(icx);
        Substs *ts = infer_fresh_substs(icx, d->parent, pr->sp);
        Predicate p = {PRED_TRAIT, {d->parent, ts}, 0, NULL};
        EvalResult ev;
        if (!try_method(pr, m, ts, self_ty, &p, &ev))
            continue;
        if (ev == EVAL_YES) {
            infer_register(icx, p, pr->sp);
            return true;
        }
        /* Ambiguous: remember the first and keep looking for a sure one. */
        if (!maybe_def)
            maybe_def = m;
        infer_rollback(icx, s);
    }
    if (maybe_def) {
        Def *d = def_get(maybe_def);
        Substs *ts = infer_fresh_substs(icx, d->parent, pr->sp);
        Predicate p = {PRED_TRAIT, {d->parent, ts}, 0, NULL};
        if (try_method(pr, maybe_def, ts, self_ty, &p, NULL)) {
            infer_register(icx, p, pr->sp);
            return true;
        }
    }
    return false;
}

#define MAX_STEPS 24

TExpr *lookup_method(FnCtxt *fcx, TExpr *recv, Sym name, GenericArgs *turbofish, Span sp,
                     DefId *out_def, Substs **out_substs) {
    InferCtxt *icx = fcx->icx;
    Probe pr = {fcx, name, turbofish, sp, 0, NULL};
    for (int attempt = 0; attempt < 2; attempt++) {
        Type *steps[MAX_STEPS + 1];
        int n = 0;
        Type *t = structurally_resolve(fcx, recv->ty);
        if (t->kind == T_INFER && t->ik == INFER_TY)
            fatal(sp, "type annotations needed: cannot call method `%s` on a value of unknown type",
                  sym_str(name));
        while (n < MAX_STEPS) {
            steps[n++] = t;
            Type *next = autoderef_ty(fcx, t, sp);
            if (!next)
                break;
            t = structurally_resolve(fcx, next);
            if (t->kind == T_INFER && t->ik == INFER_TY)
                break;
        }
        bool unsize = false;
        if (steps[n - 1]->kind == T_ARRAY) {
            steps[n] = ty_slice(steps[n - 1]->elem);
            n++;
            unsize = true;
        }
        for (int i = 0; i < n; i++) {
            bool is_unsize_step = unsize && i == n - 1;
            /* a by-value receiver must be Sized: `slice[a..b].try_into()`
               skips to the autoref step */
            bool unsized_step = steps[i]->kind == T_SLICE || steps[i]->kind == T_STR ||
                                steps[i]->kind == T_DYN;
            for (int mode = (is_unsize_step || unsized_step) ? 1 : 0; mode < 3; mode++) {
                Type *self_ty = mode == 0 ? steps[i] : ty_ref(steps[i], mode == 2);
                /* Inherent candidates come from this and every later step
                   (`self: Box<Self>` methods are declared on the pointee). */
                bool found = false;
                for (int j = i; j < n && !found; j++)
                    found = probe_inherent(&pr, steps[j], self_ty);
                if (!found && !probe_extension(&pr, self_ty))
                    continue;
                TExpr *e = recv;
                int derefs = is_unsize_step ? i - 1 : i;
                for (int k = 0; k < derefs; k++) {
                    e = deref_once(fcx, e);
                    if (!e)
                        fatal(sp, "internal: auto-deref step failed");
                }
                if (mode == 2)
                    fix_place_mut(fcx, e);
                if (mode != 0)
                    e = make_ref(recv->span, e, mode == 2);
                if (is_unsize_step) {
                    TExpr *u = te_new(TE_UNSIZE, recv->span, self_ty);
                    u->a = e;
                    e = u;
                }
                *out_def = pr.def;
                *out_substs = pr.substs;
                return e;
            }
        }
        /* Lenient fallback: a method on an unsuffixed numeric literal. */
        Type *t0 = infer_shallow(icx, recv->ty);
        if (attempt == 0 && t0->kind == T_INFER && t0->ik != INFER_TY)
            (void)infer_unify(icx, t0, t0->ik == INFER_INT ? ty_int(IK_I32) : ty_float(FK_F64));
        else
            break;
    }
    return NULL;
}

TExpr *check_method_call(FnCtxt *fcx, Expr *e, Type *expected) {
    TExpr *recv = check_expr(fcx, e->method_call.receiver, NULL);
    PathSegment *seg = e->method_call.method;
    DefId def;
    Substs *substs;
    TExpr *adj = lookup_method(fcx, recv, seg->name, seg->args, e->span, &def, &substs);
    if (!adj)
        fatal(e->span, "no method named `%s` found for type `%s`", sym_str(seg->name),
              fcx_ty_str(fcx, recv->ty));
    infer_register_preds(fcx->icx, predicates_of(def), substs, e->span);
    FnSigT *sig = fn_sig(def);
    Substs *inputs = fcx_normalize(fcx, ty_tuple(substs_subst(sig->inputs, substs)), e->span)->substs;
    Type *output = fcx_normalize(fcx, ty_subst(sig->output, substs), e->span);
    unify_or_fatal(fcx, inputs->t[0], adj->ty, e->span);
    Type **hints = expected_inputs(fcx, expected, output, inputs);
    uint32_t n;
    TExpr **args = check_args(fcx, inputs, 1, &e->method_call.args, hints, false, e->span, &n);
    TExpr **all = NEW_ARRAY(TExpr *, n + 1);
    all[0] = adj;
    for (uint32_t i = 0; i < n; i++)
        all[i + 1] = args[i];
    return make_call(e->span, def, substs, output, all, n + 1);
}
