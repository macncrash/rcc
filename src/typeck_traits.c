/*
 * typeck_traits.c - the trait system: candidate assembly and selection,
 * projection normalization, pending obligations, and the closed-type
 * queries used after monomorphization.
 *
 * Selection follows rustc's shape:
 *   1. Self is an unresolved inference variable  -> ambiguous.
 *   2. Auto traits / Sized                        -> builtin.
 *   3. Caller bounds (icx->env), bounds declared on associated types (for
 *      projection Self types) and on opaque types -> SEL_PARAM. These take
 *      precedence over impls.
 *   4. dyn Trait: the principal trait and its supertraits -> SEL_OBJECT.
 *   5. Fn* for closures, fn items and fn pointers; Copy/Clone for those.
 *   6. impls: every impl whose header unifies and whose where-clauses do
 *      not definitely fail.
 * Exactly one surviving candidate is confirmed (unified for real, nested
 * obligations registered); several mean "ambiguous for now".
 */
#include "typeck.h"

#include <stdio.h>

#define MAX_DEPTH 48
#define MAX_CANDS 16

typedef struct Cand {
    SelKind kind;
    DefId impl;
    Substs *substs; /* SEL_PARAM/SEL_OBJECT: the bound's trait substs */
    Type *args;     /* SEL_CLOSURE/SEL_FNPTR: the argument tuple */
} Cand;

typedef struct CandList {
    Cand c[MAX_CANDS];
    uint32_t n;
    bool ambig;
} CandList;

static EvalResult eval_trait(InferCtxt *icx, TraitRef tr, int depth);
static Type *project(InferCtxt *icx, DefId assoc, Substs *substs, Span sp, bool *ambig);

static void cand_add(CandList *cl, Cand c) {
    if (cl->n < MAX_CANDS)
        cl->c[cl->n++] = c;
    else
        cl->ambig = true;
}

static bool probe_unify_substs(InferCtxt *icx, Substs *a, Substs *b) {
    Snapshot s = infer_snapshot(icx);
    bool ok = infer_unify_substs(icx, a, b);
    infer_rollback(icx, s);
    return ok;
}

void supertraits_of(TraitRef tr, PredList *out) {
    Predicate p = {PRED_TRAIT, tr, 0, NULL};
    pred_push(out, p);
    elaborate_pred(out, &p);
}

/* The bound lists attached to a projection or opaque Self type. */
static void self_bounds(Type *self, PredList *out) {
    if (self->kind == T_PROJ) {
        PredList *ib = trait_item_bounds(def_get(self->def)->parent);
        for (uint32_t i = 0; i < ib->len; i++) {
            Predicate p = ib->data[i];
            p.tr.substs = substs_subst(p.tr.substs, self->substs);
            if (p.kind == PRED_PROJ)
                p.ty = ty_subst(p.ty, self->substs);
            pred_push(out, p);
            elaborate_pred(out, &p);
        }
    } else if (self->kind == T_OPAQUE) {
        (void)fn_sig(self->def);
        OpaqueInfo *oi = def_get(self->def)->opaques;
        if (!oi || self->index >= oi->n)
            return;
        PredList *ob = &oi->bounds[self->index];
        for (uint32_t i = 0; i < ob->len; i++) {
            Predicate p = ob->data[i];
            p.tr.substs = substs_subst(p.tr.substs, self->substs);
            if (p.kind == PRED_PROJ)
                p.ty = ty_subst(p.ty, self->substs);
            pred_push(out, p);
            elaborate_pred(out, &p);
        }
    }
}

static Type *closure_args_tuple(Type *self) {
    ClosureInfo *ci = def_get(self->def)->closure;
    return ty_tuple(substs_subst(ci->inputs, self->substs));
}

static bool is_fn_trait(DefId t) {
    return t && (t == lang.fn || t == lang.fn_mut || t == lang.fn_once);
}

static uint64_t assemble_steps;
static const char *pred_str(InferCtxt *icx, TraitRef tr);

static void assemble(InferCtxt *icx, TraitRef tr, int depth, CandList *cl) {
    Type *self = infer_shallow(icx, tr.substs->t[0]);
    if (++assemble_steps % 2000000 == 0)
        fprintf(stderr, "rcc: trait selection is taking long (depth %d): %s\n", depth,
                pred_str(icx, tr));
    cl->n = 0;
    cl->ambig = false;
    if (trait_def(tr.trait)->is_auto || tr.trait == lang.sized) {
        cand_add(cl, (Cand){SEL_BUILTIN, 0, NULL, NULL});
        return;
    }
    if (self->kind == T_INFER && self->ik == INFER_TY) {
        cl->ambig = true;
        return;
    }
    if (self->kind == T_ERROR)
        return;
    /* Caller bounds. */
    if (icx->env) {
        for (uint32_t i = 0; i < icx->env->len; i++) {
            Predicate *p = &icx->env->data[i];
            if (p->kind != PRED_TRAIT || p->tr.trait != tr.trait)
                continue;
            if (probe_unify_substs(icx, p->tr.substs, tr.substs)) {
                bool dup = false;
                for (uint32_t k = 0; k < cl->n; k++)
                    dup |= cl->c[k].substs == p->tr.substs;
                if (!dup)
                    cand_add(cl, (Cand){SEL_PARAM, 0, p->tr.substs, NULL});
            }
        }
    }
    if (self->kind == T_PROJ || self->kind == T_OPAQUE) {
        PredList sb = {0};
        self_bounds(self, &sb);
        for (uint32_t i = 0; i < sb.len; i++) {
            Predicate *p = &sb.data[i];
            if (p->kind != PRED_TRAIT || p->tr.trait != tr.trait)
                continue;
            if (probe_unify_substs(icx, p->tr.substs, tr.substs))
                cand_add(cl, (Cand){SEL_PARAM, 0, p->tr.substs, NULL});
        }
    }
    if (cl->n)
        return;
    if (self->kind == T_DYN) {
        PredList sup = {0};
        TraitRef principal = {self->def, substs_concat(substs_1(self), self->substs)};
        supertraits_of(principal, &sup);
        for (uint32_t i = 0; i < sup.len; i++) {
            Predicate *p = &sup.data[i];
            if (p->kind != PRED_TRAIT || p->tr.trait != tr.trait)
                continue;
            if (probe_unify_substs(icx, p->tr.substs, tr.substs)) {
                cand_add(cl, (Cand){SEL_OBJECT, 0, p->tr.substs, NULL});
                return;
            }
        }
    }
    if (is_fn_trait(tr.trait)) {
        Type *args = NULL;
        SelKind k = SEL_NONE;
        if (self->kind == T_CLOSURE) {
            args = closure_args_tuple(self);
            if (args->flags & TYF_HAS_PROJ)
                args = infer_normalize(icx, args, (Span){0});
            k = SEL_CLOSURE;
        } else if (self->kind == T_FNPTR) {
            args = ty_tuple(substs_slice(self->substs, 0, self->substs->len - 1));
            k = SEL_FNPTR;
        } else if (self->kind == T_FNDEF) {
            args = ty_tuple(substs_subst(fn_sig(self->def)->inputs, self->substs));
            if (args->flags & TYF_HAS_PROJ)
                args = infer_normalize(icx, args, (Span){0});
            k = SEL_FNPTR;
        }
        if (k != SEL_NONE) {
            Snapshot s = infer_snapshot(icx);
            bool ok = infer_unify(icx, tr.substs->t[1], args);
            infer_rollback(icx, s);
            if (ok)
                cand_add(cl, (Cand){k, 0, NULL, args});
            return;
        }
    }
    if ((tr.trait == lang.copy || tr.trait == lang.clone) && tr.trait &&
        (self->kind == T_FNDEF || self->kind == T_FNPTR || self->kind == T_CLOSURE ||
         self->kind == T_NEVER)) {
        /* a closure is Copy/Clone iff everything it captured is (until the
           captures are known, during the check of its own body, assume so) */
        bool ok = true;
        if (self->kind == T_CLOSURE && def_get(self->def)->closure->analyzed && depth < MAX_DEPTH) {
            ClosureInfo *ci = def_get(self->def)->closure;
            for (uint32_t i = 0; i < ci->ncaptures && ok; i++) {
                if (ci->captures[i].mode == CAP_REF)
                    continue;
                if (ci->captures[i].mode == CAP_MUT_REF) {
                    ok = false;
                    break;
                }
                Type *ct = ty_subst(ci->captures[i].ty, self->substs);
                if (ct->flags & TYF_HAS_PROJ)
                    ct = infer_normalize(icx, ct, (Span){0});
                TraitRef nt = {tr.trait, substs_1(ct)};
                ok = eval_trait(icx, nt, depth + 1) != EVAL_NO;
            }
        }
        if (ok)
            cand_add(cl, (Cand){SEL_BUILTIN, 0, NULL, NULL});
        return;
    }
    /* Impls. */
    const DefIdList *lists[2];
    trait_impls_for(tr.trait, self, &lists[0], &lists[1]);
    for (int li = 0; li < 2; li++) {
        if (!lists[li])
            continue;
        for (uint32_t i = 0; i < lists[li]->len; i++) {
            DefId impl = lists[li]->data[i];
            ImplDef *im = impl_def(impl);
            Snapshot s = infer_snapshot(icx);
            icx->probe_depth++;
            Substs *fresh = infer_fresh_substs(icx, impl, (Span){0});
            Substs *hdr = substs_subst(im->trait_ref.substs, fresh);
            if (hdr->flags & TYF_HAS_PROJ) {
                Type *n = infer_normalize(icx, ty_tuple(hdr), (Span){0});
                hdr = n->substs;
            }
            bool ok = infer_unify_substs(icx, hdr, tr.substs);
            if (ok && depth < MAX_DEPTH) {
                PredList *preds = predicates_of(impl);
                for (uint32_t k = 0; k < preds->len && ok; k++) {
                    Predicate *p = &preds->data[k];
                    if (p->kind != PRED_TRAIT)
                        continue;
                    TraitRef nt = {p->tr.trait, substs_subst(p->tr.substs, fresh)};
                    if (nt.substs->flags & TYF_HAS_PROJ) {
                        Type *n = infer_normalize(icx, ty_tuple(nt.substs), (Span){0});
                        nt.substs = n->substs;
                    }
                    if (eval_trait(icx, nt, depth + 1) == EVAL_NO)
                        ok = false;
                }
            }
            icx->probe_depth--;
            infer_rollback(icx, s);
            if (ok)
                cand_add(cl, (Cand){SEL_IMPL, impl, NULL, NULL});
        }
    }
}

static bool substs_have_infer(InferCtxt *icx, Substs *s) {
    return (infer_resolve_substs(icx, s)->flags & TYF_HAS_INFER) != 0;
}

static EvalResult eval_trait(InferCtxt *icx, TraitRef tr, int depth) {
    if (depth >= MAX_DEPTH)
        return EVAL_AMBIG;
    CandList cl;
    assemble(icx, tr, depth, &cl);
    if (cl.n == 0)
        return cl.ambig ? EVAL_AMBIG : EVAL_NO;
    if (cl.n == 1 && !cl.ambig)
        return EVAL_YES;
    return substs_have_infer(icx, tr.substs) ? EVAL_AMBIG : EVAL_YES;
}

EvalResult trait_eval(InferCtxt *icx, const Predicate *p) {
    Snapshot s = infer_snapshot(icx);
    icx->probe_depth++;
    EvalResult r = eval_trait(icx, p->tr, 0);
    icx->probe_depth--;
    infer_rollback(icx, s);
    return r;
}

void infer_register(InferCtxt *icx, Predicate p, Span sp) {
    if (icx->npending == icx->pending_cap) {
        uint32_t nc = icx->pending_cap ? icx->pending_cap * 2 : 16;
        Obligation *n = NEW_ARRAY(Obligation, nc);
        if (icx->npending)
            memcpy(n, icx->pending, sizeof(Obligation) * icx->npending);
        icx->pending = n;
        icx->pending_cap = nc;
    }
    icx->pending[icx->npending].pred = p;
    icx->pending[icx->npending].span = sp;
    icx->npending++;
}

void infer_register_preds(InferCtxt *icx, PredList *preds, Substs *substs, Span sp) {
    for (uint32_t i = 0; i < preds->len; i++) {
        Predicate p = preds->data[i];
        p.tr.substs = substs_subst(p.tr.substs, substs);
        if (p.tr.substs->flags & TYF_HAS_PROJ)
            p.tr.substs = infer_normalize(icx, ty_tuple(p.tr.substs), sp)->substs;
        if (p.kind == PRED_PROJ)
            p.ty = infer_normalize(icx, ty_subst(p.ty, substs), sp);
        if (p.kind == PRED_TRAIT && (trait_def(p.tr.trait)->is_auto || p.tr.trait == lang.sized))
            continue;
        infer_register(icx, p, sp);
    }
}

Selection trait_select(InferCtxt *icx, TraitRef tr, Span sp) {
    Selection sel = {SEL_NONE, 0, NULL};
    CandList cl;
    assemble(icx, tr, 0, &cl);
    if (cl.n == 0) {
        sel.kind = cl.ambig ? SEL_AMBIG : SEL_NONE;
        return sel;
    }
    if (cl.n > 1 || cl.ambig) {
        if (substs_have_infer(icx, tr.substs)) {
            sel.kind = SEL_AMBIG;
            return sel;
        }
        /* Closed and still several: caller bounds repeated through
           different paths, or overlapping impls. Take the first. */
    }
    Cand *c = &cl.c[0];
    sel.kind = c->kind;
    switch (c->kind) {
    case SEL_IMPL: {
        ImplDef *im = impl_def(c->impl);
        Substs *fresh = infer_fresh_substs(icx, c->impl, sp);
        Substs *hdr = substs_subst(im->trait_ref.substs, fresh);
        if (hdr->flags & TYF_HAS_PROJ)
            hdr = infer_normalize(icx, ty_tuple(hdr), sp)->substs;
        if (!infer_unify_substs(icx, hdr, tr.substs))
            fatal(sp, "internal: impl candidate no longer unifies");
        infer_register_preds(icx, predicates_of(c->impl), fresh, sp);
        sel.impl = c->impl;
        sel.substs = fresh;
        break;
    }
    case SEL_PARAM:
    case SEL_OBJECT:
        if (!infer_unify_substs(icx, c->substs, tr.substs))
            fatal(sp, "internal: bound candidate no longer unifies");
        sel.substs = c->substs;
        break;
    case SEL_CLOSURE:
    case SEL_FNPTR:
        if (!infer_unify(icx, tr.substs->t[1], c->args))
            fatal(sp, "internal: fn candidate no longer unifies");
        break;
    default:
        break;
    }
    return sel;
}

/* ------------------------------------------------------------------ */
/* Projection                                                          */
/* ------------------------------------------------------------------ */

static const char *pred_str(InferCtxt *icx, TraitRef tr) {
    Substs *s = infer_resolve_substs(icx, tr.substs);
    StrBuf sb = {0};
    ty_print(&sb, s->t[0]);
    sb_puts(&sb, ": ");
    sb_puts(&sb, sym_str(def_get(tr.trait)->name));
    if (s->len > 1)
        sb_puts(&sb, substs_to_str(substs_slice(s, 1, s->len)));
    return sb_str(&sb);
}

/* Look for a `<..>::assoc == T` bound matching (assoc, substs). */
static Type *find_proj_bound(InferCtxt *icx, DefId assoc, Substs *substs) {
    Type *self = infer_shallow(icx, substs->t[0]);
    PredList sb = {0};
    self_bounds(self, &sb);
    PredList *lists[2] = {icx->env, &sb};
    for (int li = 0; li < 2; li++) {
        if (!lists[li])
            continue;
        for (uint32_t i = 0; i < lists[li]->len; i++) {
            Predicate *p = &lists[li]->data[i];
            if (p->kind != PRED_PROJ || p->assoc != assoc)
                continue;
            if (infer_unify_substs(icx, p->tr.substs, substs))
                return p->ty;
        }
    }
    return NULL;
}

/* 1 = arithmetic/bit operator trait, 2 = shift, 3 = Neg/Not, 0 = other. */
static int arith_trait_kind(DefId trait) {
    if (!trait)
        return 0;
    if (trait == lang.neg || trait == lang.not_)
        return 3;
    for (int i = 0; i < 18; i++)
        if (lang.binop[i] == trait)
            return (i == BIN_SHL || i == BIN_SHR) ? 2 : 1;
    return 0;
}

static int project_depth;
static Type *project_inner(InferCtxt *icx, DefId assoc, Substs *substs, Span sp, bool *ambig);

static Type *project(InferCtxt *icx, DefId assoc, Substs *substs, Span sp, bool *ambig) {
    if (project_depth > 150)
        fatal(sp, "overflow normalizing `%s`",
              ty_to_str(ty_proj(assoc, infer_resolve_substs(icx, substs))));
    project_depth++;
    Type *r = project_inner(icx, assoc, substs, sp, ambig);
    project_depth--;
    return r;
}

static Type *project_inner(InferCtxt *icx, DefId assoc, Substs *substs, Span sp, bool *ambig) {
    *ambig = false;
    Def *ad = def_get(assoc);
    TraitRef tr = {ad->parent, substs};
    Type *self = infer_shallow(icx, substs->t[0]);
    /* Builtin operators on numeric types, where the operands may still be
       integer/float inference variables: `?int + ?int` has type `?int`
       (rustc types builtin binary operations the same way, without going
       through the Add impls). */
    int ok = arith_trait_kind(ad->parent);
    if (ok && strcmp(sym_str(ad->name), "Output") == 0) {
        bool self_numvar = self->kind == T_INFER && self->ik != INFER_TY;
        bool self_num = self_numvar || self->kind == T_INT || self->kind == T_FLOAT;
        if (self_num && ok == 3)
            return self; /* Neg / Not */
        if (self_num && substs->len == 2) {
            Type *rhs = infer_shallow(icx, substs->t[1]);
            bool rhs_numvar = rhs->kind == T_INFER && rhs->ik != INFER_TY;
            if (ok == 2) {
                if (rhs_numvar || rhs->kind == T_INT)
                    return self; /* shifts: any integer amount */
            } else {
                if (rhs != self && (rhs_numvar || self_numvar) &&
                    (rhs_numvar || rhs->kind == T_INT || rhs->kind == T_FLOAT))
                    (void)infer_unify(icx, self, rhs);
                if (infer_shallow(icx, substs->t[0]) == infer_shallow(icx, substs->t[1]))
                    return infer_shallow(icx, substs->t[0]);
            }
        }
    }
    if (self->kind == T_INFER) {
        *ambig = true;
        return NULL;
    }
    if (self->kind == T_ERROR)
        return ty_error();
    Selection sel = trait_select(icx, tr, sp);
    switch (sel.kind) {
    case SEL_IMPL: {
        DefId it = impl_item(sel.impl, ad->name);
        if (!it)
            fatal(sp, "impl of `%s` is missing associated type `%s`", sym_str(def_get(ad->parent)->name),
                  sym_str(ad->name));
        return infer_normalize(icx, ty_subst(type_of(it), sel.substs), sp);
    }
    case SEL_PARAM: {
        Type *b = find_proj_bound(icx, assoc, substs);
        if (b)
            return infer_normalize(icx, b, sp);
        return NULL; /* rigid */
    }
    case SEL_OBJECT: {
        for (uint32_t i = 0; i < self->nbinds; i++)
            if (self->binds[i].assoc == assoc)
                return self->binds[i].ty;
        return NULL;
    }
    case SEL_CLOSURE: {
        ClosureInfo *ci = def_get(self->def)->closure;
        return infer_normalize(icx, ty_subst(ci->output, self->substs), sp);
    }
    case SEL_FNPTR:
        if (self->kind == T_FNPTR)
            return fnptr_ret(self);
        return infer_normalize(icx, ty_subst(fn_sig(self->def)->output, self->substs), sp);
    case SEL_AMBIG:
        *ambig = true;
        return NULL;
    case SEL_BUILTIN:
        fatal(sp, "associated type `%s` of a builtin impl is not supported", sym_str(ad->name));
    case SEL_NONE:
        if (icx->probe_depth > 0)
            return ty_error();
        fatal(sp, "the trait bound `%s` is not satisfied (needed for `%s`)", pred_str(icx, tr),
              sym_str(ad->name));
    }
    return NULL;
}

typedef struct NormCtx {
    InferCtxt *icx;
    Span sp;
} NormCtx;

static Type *norm_fold(TypeFolder *f, Type *t) {
    NormCtx *nc = f->ctx;
    if (t->kind != T_PROJ)
        return ty_super_fold(f, t);
    Substs *s = substs_fold(f, t->substs);
    bool ambig;
    Type *r = project(nc->icx, t->def, s, nc->sp, &ambig);
    if (r)
        return r;
    if (ambig) {
        Type *v = infer_var(nc->icx, INFER_TY, nc->sp);
        Predicate p = {PRED_PROJ, {def_get(t->def)->parent, s}, t->def, v};
        infer_register(nc->icx, p, nc->sp);
        return v;
    }
    return s == t->substs ? t : ty_proj(t->def, s);
}

Type *infer_normalize(InferCtxt *icx, Type *t, Span sp) {
    if (!(t->flags & TYF_HAS_PROJ))
        return t;
    NormCtx nc = {icx, sp};
    TypeFolder f = {norm_fold, TYF_HAS_PROJ, &nc, NULL};
    return ty_fold(&f, t);
}

bool infer_select_pending(InferCtxt *icx) {
    bool any = false;
    for (;;) {
        bool progress = false;
        for (uint32_t i = 0; i < icx->npending; i++) {
            Obligation ob = icx->pending[i];
            if (ob.pred.tr.trait == 0)
                continue;
            if (ob.pred.kind == PRED_TRAIT) {
                Selection sel = trait_select(icx, ob.pred.tr, ob.span);
                if (sel.kind == SEL_AMBIG)
                    continue;
                if (sel.kind == SEL_NONE)
                    fatal(ob.span, "the trait bound `%s` is not satisfied",
                          pred_str(icx, ob.pred.tr));
            } else {
                bool ambig;
                Type *r = project(icx, ob.pred.assoc, ob.pred.tr.substs, ob.span, &ambig);
                if (!r && ambig)
                    continue;
                if (!r)
                    r = ty_proj(ob.pred.assoc, infer_resolve_substs(icx, ob.pred.tr.substs));
                if (!infer_unify(icx, r, ob.pred.ty))
                    fatal(ob.span, "type mismatch: `<%s>::%s` is `%s`, expected `%s`",
                          pred_str(icx, ob.pred.tr), sym_str(def_get(ob.pred.assoc)->name),
                          ty_to_str(infer_resolve(icx, r)),
                          ty_to_str(infer_resolve(icx, ob.pred.ty)));
            }
            icx->pending[i].pred.tr.trait = 0;
            progress = true;
        }
        uint32_t k = 0;
        for (uint32_t i = 0; i < icx->npending; i++)
            if (icx->pending[i].pred.tr.trait)
                icx->pending[k++] = icx->pending[i];
        icx->npending = k;
        if (!progress)
            break;
        any = true;
    }
    return any;
}

Type *infer_overloaded_deref(InferCtxt *icx, Type *ty, Span sp) {
    if (!lang.deref)
        return NULL;
    ty = infer_shallow(icx, ty);
    if (ty->kind == T_INFER)
        return NULL;
    Predicate p = {PRED_TRAIT, {lang.deref, substs_1(ty)}, 0, NULL};
    if (trait_eval(icx, &p) == EVAL_NO)
        return NULL;
    return infer_normalize(icx, ty_proj(lang.deref_target, substs_1(ty)), sp);
}

/* ------------------------------------------------------------------ */
/* Closed-type queries                                                 */
/* ------------------------------------------------------------------ */

#define SCRATCH_MAX 64
static InferCtxt *scratch[SCRATCH_MAX];
static int scratch_depth;

static InferCtxt *scratch_enter(void) {
    if (scratch_depth >= SCRATCH_MAX)
        fatal((Span){0}, "trait selection recursion too deep");
    if (!scratch[scratch_depth])
        scratch[scratch_depth] = infer_new(NULL);
    InferCtxt *icx = scratch[scratch_depth++];
    icx->nvars = 0;
    icx->nundo = 0;
    icx->npending = 0;
    icx->probe_depth = 0;
    return icx;
}

static void scratch_leave(void) { scratch_depth--; }

static HMap closed_sel_cache; /* (trait, substs id) -> Selection* */
static HMap closed_norm_cache; /* type id -> Type* */
static HMap copy_cache;        /* type id -> 1 (no) / 2 (yes) */

Selection select_closed(TraitRef tr, Span sp) {
    Selection *c = hmap_get2(&closed_sel_cache, tr.trait, tr.substs->id);
    if (c)
        return *c;
    InferCtxt *icx = scratch_enter();
    Selection sel = trait_select(icx, tr, sp);
    if (sel.kind == SEL_NONE || sel.kind == SEL_AMBIG)
        fatal(sp, "cannot find an implementation for `%s` during monomorphization",
              pred_str(icx, tr));
    if (sel.kind == SEL_IMPL) {
        infer_select_pending(icx);
        sel.substs = infer_resolve_substs(icx, sel.substs);
        if (sel.substs->flags & TYF_HAS_INFER)
            fatal(sp, "impl parameters of `%s` are not determined by `%s`",
                  def_path_str(sel.impl), pred_str(icx, tr));
    }
    scratch_leave();
    if (sel.kind == SEL_IMPL)
        sel.substs = norm_closed_substs(sel.substs, sp);
    c = NEW(Selection);
    *c = sel;
    hmap_put2(&closed_sel_cache, tr.trait, tr.substs->id, c);
    return sel;
}

bool implements_closed(DefId trait, Type *self) {
    if (!trait)
        return false;
    InferCtxt *icx = scratch_enter();
    Predicate p = {PRED_TRAIT, {trait, substs_1(self)}, 0, NULL};
    EvalResult r = trait_eval(icx, &p);
    scratch_leave();
    return r != EVAL_NO;
}

Type *project_closed(DefId assoc, Substs *substs, Span sp) {
    Def *ad = def_get(assoc);
    TraitRef tr = {ad->parent, substs};
    Type *self = substs->t[0];
    Selection sel = select_closed(tr, sp);
    switch (sel.kind) {
    case SEL_IMPL: {
        DefId it = impl_item(sel.impl, ad->name);
        if (!it)
            fatal(sp, "impl of `%s` is missing associated type `%s`",
                  sym_str(def_get(ad->parent)->name), sym_str(ad->name));
        return norm_closed(ty_subst(type_of(it), sel.substs), sp);
    }
    case SEL_OBJECT:
        for (uint32_t i = 0; i < self->nbinds; i++)
            if (self->binds[i].assoc == assoc)
                return self->binds[i].ty;
        break;
    case SEL_CLOSURE:
        return norm_closed(ty_subst(def_get(self->def)->closure->output, self->substs), sp);
    case SEL_FNPTR:
        if (self->kind == T_FNPTR)
            return fnptr_ret(self);
        return norm_closed(ty_subst(fn_sig(self->def)->output, self->substs), sp);
    default:
        break;
    }
    fatal(sp, "cannot normalize `%s` during monomorphization", ty_to_str(ty_proj(assoc, substs)));
}

static Type *closed_fold(TypeFolder *f, Type *t) {
    Span sp = *(Span *)f->ctx;
    if (t->kind == T_PROJ) {
        Substs *s = substs_fold(f, t->substs);
        return project_closed(t->def, s, sp);
    }
    if (t->kind == T_OPAQUE) {
        Substs *s = substs_fold(f, t->substs);
        return norm_closed(ty_subst(opaque_concrete(t->def, t->index), s), sp);
    }
    return ty_super_fold(f, t);
}

Type *norm_closed(Type *t, Span sp) {
    if (!(t->flags & TYF_HAS_PROJ))
        return t;
    Type *c = hmap_get(&closed_norm_cache, t->id);
    if (c)
        return c;
    TypeFolder f = {closed_fold, TYF_HAS_PROJ, &sp, NULL};
    c = ty_fold(&f, t);
    hmap_put(&closed_norm_cache, t->id, c);
    return c;
}

Substs *norm_closed_substs(Substs *s, Span sp) {
    if (!(s->flags & TYF_HAS_PROJ))
        return s;
    return norm_closed(ty_tuple(s), sp)->substs;
}

bool ty_is_copy(Type *t) {
    switch ((TypeKind)t->kind) {
    case T_BOOL: case T_CHAR: case T_INT: case T_FLOAT: case T_NEVER:
    case T_PTR: case T_FNPTR: case T_FNDEF: case T_CONST:
        return true;
    case T_REF:
        return !t->is_mut;
    case T_STR: case T_SLICE: case T_DYN:
        return false;
    default:
        break;
    }
    void *c = hmap_get(&copy_cache, t->id);
    if (c)
        return c == (void *)2;
    bool r = false;
    switch ((TypeKind)t->kind) {
    case T_TUPLE:
        r = true;
        for (uint32_t i = 0; i < t->substs->len && r; i++)
            r = ty_is_copy(t->substs->t[i]);
        break;
    case T_ARRAY:
        r = ty_is_copy(t->elem);
        break;
    case T_ADT:
        r = implements_closed(lang.copy, t);
        break;
    case T_CLOSURE: {
        ClosureInfo *ci = def_get(t->def)->closure;
        r = true;
        for (uint32_t i = 0; i < ci->ncaptures && r; i++) {
            if (ci->captures[i].mode == CAP_REF)
                continue;
            if (ci->captures[i].mode == CAP_MUT_REF)
                r = false;
            else
                r = ty_is_copy(norm_closed(ty_subst(ci->captures[i].ty, t->substs), ci->span));
        }
        break;
    }
    default:
        fatal((Span){0}, "internal: ty_is_copy on non-closed type `%s`", ty_to_str(t));
    }
    hmap_put(&copy_cache, t->id, r ? (void *)2 : (void *)1);
    return r;
}
