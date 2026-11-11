/*
 * typeck_closure.c - closures, capture analysis, writeback, and the body
 * checking entry points (typeck_body, typeck_anon_const).
 *
 * Closure checking: the signature comes from annotations, else from the
 * expected type (a fn pointer, a `dyn Fn*` type, or an inference variable
 * with pending `?F: Fn*(Args) -> R` obligations, which is what a generic
 * callee's bound looks like while its arguments are being checked), else
 * fresh variables. The body is checked immediately.
 *
 * Capture analysis runs after writeback (types are final): every use of an
 * enclosing-scope variable inside the closure body is classified as a
 * borrow, a mutable borrow or a move, as rustc does per variable. A
 * non-`move` closure captures by the strongest use; a `move` closure
 * captures everything by value. The closure kind is FnOnce if a captured
 * variable is moved out of, FnMut if one is mutated, else Fn.
 * By-value captures are precise places (edition-2021 disjoint capture:
 * `move || s.a.len()` captures only `s.a`); by-reference captures take the
 * whole variable, which is indistinguishable without a borrow checker.
 */
#include "typeck_fn.h"

#include <stdio.h>

/* ------------------------------------------------------------------ */
/* Closure expressions                                                 */
/* ------------------------------------------------------------------ */

static bool is_fn_family(DefId t) {
    return t && (t == lang.fn || t == lang.fn_mut || t == lang.fn_once);
}

/* Expected signature of a closure, from the expected type. */
static void deduce_sig(FnCtxt *fcx, Type *expected, Substs **inputs, Type **output) {
    InferCtxt *icx = fcx->icx;
    *inputs = NULL;
    *output = NULL;
    if (!expected)
        return;
    Type *x = infer_shallow(icx, expected);
    if (x->kind == T_FNPTR) {
        *inputs = substs_slice(x->substs, 0, x->substs->len - 1);
        *output = fnptr_ret(x);
        return;
    }
    if (x->kind == T_DYN && is_fn_family(x->def)) {
        Type *args = infer_shallow(icx, x->substs->t[0]);
        if (args->kind == T_TUPLE)
            *inputs = args->substs;
        for (uint32_t i = 0; i < x->nbinds; i++)
            if (x->binds[i].assoc == lang.fn_once_output)
                *output = x->binds[i].ty;
        return;
    }
    if (x->kind != T_INFER)
        return;
    for (uint32_t i = 0; i < icx->npending; i++) {
        Predicate *p = &icx->pending[i].pred;
        if (!p->tr.trait || infer_shallow(icx, p->tr.substs->t[0]) != x)
            continue;
        if (p->kind == PRED_TRAIT && is_fn_family(p->tr.trait)) {
            Type *args = infer_shallow(icx, p->tr.substs->t[1]);
            if (args->kind == T_TUPLE)
                *inputs = args->substs;
        } else if (p->kind == PRED_PROJ && p->assoc == lang.fn_once_output) {
            *output = p->ty;
            Type *args = infer_shallow(icx, p->tr.substs->t[1]);
            if (args->kind == T_TUPLE && !*inputs)
                *inputs = args->substs;
        }
    }
}

TExpr *check_closure(FnCtxt *fcx, Expr *e, Type *expected) {
    InferCtxt *icx = fcx->icx;
    DefId def = def_new(DEF_CLOSURE, 0, fcx->owner ? fcx->owner : 0, e->span);
    Def *d = def_get(def);
    d->closure_expr = e;
    ClosureInfo *ci = NEW(ClosureInfo);
    ci->def = def;
    ci->owner = fcx->owner;
    ci->parent_closure = fcx->closure;
    ci->is_move = e->closure.is_move;
    ci->span = e->span;
    d->closure = ci;
    vec_push(fcx->closures, def);

    Substs *exp_in;
    Type *exp_out;
    deduce_sig(fcx, expected, &exp_in, &exp_out);
    uint32_t n = e->closure.params.len;
    if (exp_in && exp_in->len != n)
        exp_in = NULL;
    Type **in = NEW_ARRAY(Type *, n ? n : 1);
    for (uint32_t i = 0; i < n; i++) {
        Param *p = e->closure.params.data[i];
        if (p->ty) {
            in[i] = lower_body_ty(fcx, p->ty);
            if (exp_in)
                (void)infer_unify(icx, in[i], exp_in->t[i]);
        } else {
            in[i] = exp_in ? exp_in->t[i] : infer_var(icx, INFER_TY, p->span);
        }
    }
    Type *ret;
    if (e->closure.ret) {
        ret = lower_body_ty(fcx, e->closure.ret);
        if (exp_out)
            (void)infer_unify(icx, ret, exp_out);
    } else {
        ret = exp_out ? exp_out : infer_var(icx, INFER_TY, e->span);
    }
    ci->inputs = substs_new(in, n);
    ci->output = ret;

    Type *saved_ret = fcx->ret_ty;
    BreakScope *saved_breaks = fcx->breaks;
    DefId saved_closure = fcx->closure;
    bool saved_div = fcx->diverges;
    Scope *saved_scope = fcx->scope;
    fcx->ret_ty = ret;
    fcx->breaks = NULL;
    fcx->closure = def;
    fcx->diverges = false;
    fcx->scope = scope_push(saved_scope, SCOPE_LOCALS, 0);
    ci->params = NEW_ARRAY(TPat *, n ? n : 1);
    ci->nparams = n;
    for (uint32_t i = 0; i < n; i++)
        ci->params[i] = check_pat_top(fcx, e->closure.params.data[i]->pat, in[i]);
    ci->body = check_expr_coerce(fcx, e->closure.body, ret);
    fcx->ret_ty = saved_ret;
    fcx->breaks = saved_breaks;
    fcx->closure = saved_closure;
    fcx->diverges = saved_div;
    fcx->scope = saved_scope;

    TExpr *x = te_new(TE_CLOSURE, e->span, ty_closure(def, fcx->identity));
    x->item.def = def;
    x->item.substs = fcx->identity;
    return x;
}

/* ------------------------------------------------------------------ */
/* Writeback                                                           */
/* ------------------------------------------------------------------ */

static Type *wb_type(FnCtxt *fcx, Type *t, Span sp) {
    t = infer_resolve(fcx->icx, t);
    if (t->flags & TYF_HAS_INFER)
        fatal(sp, "type annotations needed: could not infer the type `%s`", ty_to_str(t));
    return t;
}

static void wb_expr(FnCtxt *fcx, TExpr *e);

static void wb_pat(FnCtxt *fcx, TPat *p) {
    if (!p)
        return;
    p->ty = wb_type(fcx, p->ty, p->span);
    wb_pat(fcx, p->sub);
    wb_pat(fcx, p->rest);
    for (uint32_t i = 0; i < p->nsubs; i++)
        wb_pat(fcx, p->subs[i]);
    if (p->value)
        wb_expr(fcx, p->value);
    if (p->lo)
        wb_expr(fcx, p->lo);
    if (p->hi)
        wb_expr(fcx, p->hi);
}

static void wb_expr(FnCtxt *fcx, TExpr *e) {
    if (!e)
        return;
    e->ty = wb_type(fcx, e->ty, e->span);
    switch ((TExprKind)e->kind) {
    case TE_FN: case TE_CONST: case TE_STATIC:
        e->item.substs = infer_resolve_substs(fcx->icx, e->item.substs);
        if (e->item.substs->flags & TYF_HAS_INFER)
            fatal(e->span, "type annotations needed: cannot infer the generic arguments of `%s`%s",
                  def_path_str(e->item.def), substs_to_str(e->item.substs));
        break;
    case TE_CLOSURE: {
        ClosureInfo *ci = def_get(e->item.def)->closure;
        for (uint32_t i = 0; i < ci->nparams; i++)
            wb_pat(fcx, ci->params[i]);
        ci->inputs = infer_resolve_substs(fcx->icx, ci->inputs);
        if (ci->inputs->flags & TYF_HAS_INFER)
            fatal(e->span, "type annotations needed for closure parameters");
        ci->output = wb_type(fcx, ci->output, e->span);
        wb_expr(fcx, ci->body);
        break;
    }
    default:
        break;
    }
    wb_expr(fcx, e->a);
    wb_expr(fcx, e->b);
    wb_expr(fcx, e->c);
    for (uint32_t i = 0; i < e->nargs; i++)
        wb_expr(fcx, e->args[i]);
    for (uint32_t i = 0; i < e->nstmts; i++) {
        TStmt *s = e->stmts[i];
        wb_pat(fcx, s->pat);
        wb_expr(fcx, s->init);
        wb_expr(fcx, s->els);
        wb_expr(fcx, s->expr);
    }
    for (uint32_t i = 0; i < e->narms; i++) {
        wb_pat(fcx, e->arms[i].pat);
        wb_expr(fcx, e->arms[i].guard);
        wb_expr(fcx, e->arms[i].body);
    }
    wb_pat(fcx, e->pat);
}

/* ------------------------------------------------------------------ */
/* Capture analysis                                                    */
/* ------------------------------------------------------------------ */

enum { USE_BORROW = 0, USE_MUT = 1, USE_MOVE = 2 };

typedef struct CapUse {
    Local *var;
    uint32_t path[CAPTURE_MAX_PATH];
    uint32_t npath;
    Type *ty; /* type of the place var.path */
    int use;
} CapUse;

typedef struct CapCtx {
    FnCtxt *fcx;
    ClosureInfo *ci;
    VEC(CapUse) uses;
} CapCtx;

static void analyze_closure(FnCtxt *fcx, ClosureInfo *ci);
static void cap_value(CapCtx *c, TExpr *e);

/* Record a use of the place `var.path`. Borrows always concern the whole
   variable; by-value captures (moves, or any use in a `move` closure) keep
   the precise field path. */
static void cap_record(CapCtx *c, Local *var, const uint32_t *path, uint32_t npath, Type *ty, int use) {
    if (!(c->ci->is_move || use == USE_MOVE) || npath > CAPTURE_MAX_PATH) {
        npath = 0;
        ty = var->ty;
    }
    for (uint32_t i = 0; i < c->uses.len; i++) {
        CapUse *u = &c->uses.data[i];
        if (u->var != var || u->npath != npath || (u->use == USE_MOVE) != (use == USE_MOVE) ||
            (npath && memcmp(u->path, path, sizeof(uint32_t) * npath) != 0))
            continue;
        if (use > u->use)
            u->use = use;
        return;
    }
    CapUse u;
    memset(&u, 0, sizeof u);
    u.var = var;
    u.npath = npath;
    if (npath)
        memcpy(u.path, path, sizeof(uint32_t) * npath);
    u.ty = ty;
    u.use = use;
    vec_push(c->uses, u);
}

/* How a by-value read uses a place of type t. */
static int read_use(CapCtx *c, Type *t) {
    if (fcx_is_copy(c->fcx, t))
        return USE_BORROW;
    if (t->kind == T_REF && t->is_mut)
        return USE_MUT; /* implicit reborrow of a `&mut` */
    return USE_MOVE;
}

static bool is_place(const TExpr *e) {
    return e->kind == TE_LOCAL || e->kind == TE_FIELD || e->kind == TE_INDEX ||
           e->kind == TE_DEREF || e->kind == TE_STATIC;
}

static void cap_place(CapCtx *c, TExpr *e, int use) {
    if (e->kind == TE_FIELD || e->kind == TE_LOCAL) {
        /* a field path rooted at a variable? */
        uint32_t rev[32], n = 0;
        TExpr *root = e;
        while (root->kind == TE_FIELD && n < 32) {
            rev[n++] = root->index;
            root = root->a;
        }
        if (root->kind == TE_LOCAL) {
            if (root->local->closure != c->ci->def) {
                uint32_t path[32];
                for (uint32_t i = 0; i < n; i++)
                    path[i] = rev[n - 1 - i];
                cap_record(c, root->local, path, n, e->ty, use);
            }
            return;
        }
        if (root->kind != TE_FIELD) {
            cap_place(c, root, use);
            return;
        }
    }
    switch ((TExprKind)e->kind) {
    case TE_LOCAL:
        break;
    case TE_FIELD:
        cap_place(c, e->a, use);
        break;
    case TE_INDEX:
        cap_place(c, e->a, use == USE_MOVE ? USE_BORROW : use);
        cap_value(c, e->b);
        break;
    case TE_DEREF: {
        Type *it = e->a->ty;
        if (it->kind == T_ADT)
            cap_place(c, e->a, use); /* Box */
        else if (is_place(e->a))
            cap_place(c, e->a, use == USE_MOVE ? USE_BORROW : use);
        else
            cap_value(c, e->a);
        break;
    }
    case TE_STATIC:
        break;
    default:
        cap_value(c, e);
        break;
    }
}

static void cap_scrutinee(CapCtx *c, TExpr *scrut, int use) {
    if (is_place(scrut))
        cap_place(c, scrut, use);
    else
        cap_value(c, scrut);
}

static void cap_value(CapCtx *c, TExpr *e) {
    if (!e)
        return;
    switch ((TExprKind)e->kind) {
    case TE_LOCAL: case TE_FIELD: case TE_INDEX: case TE_DEREF: case TE_STATIC:
        cap_place(c, e, read_use(c, e->ty));
        return;
    case TE_REF: case TE_ADDR_OF:
        cap_place(c, e->a, e->is_mut ? USE_MUT : USE_BORROW);
        return;
    case TE_ASSIGN: case TE_ASSIGN_OP:
        cap_place(c, e->a, USE_MUT);
        cap_value(c, e->b);
        return;
    case TE_CALL:
        if (e->a->ty->kind == T_CLOSURE && is_place(e->a)) {
            ClosureInfo *callee = def_get(e->a->ty->def)->closure;
            if (!callee->analyzed && callee->owner == c->fcx->owner)
                analyze_closure(c->fcx, callee);
            cap_place(c, e->a, callee->kind == CK_FN ? USE_BORROW
                               : callee->kind == CK_FN_MUT ? USE_MUT : USE_MOVE);
        } else {
            cap_value(c, e->a);
        }
        for (uint32_t i = 0; i < e->nargs; i++)
            cap_value(c, e->args[i]);
        return;
    case TE_MATCH: {
        int use = 0;
        for (uint32_t i = 0; i < e->narms; i++) {
            int u = pat_use_kind(c->fcx, e->arms[i].pat);
            if (u > use)
                use = u;
        }
        cap_scrutinee(c, e->a, use);
        for (uint32_t i = 0; i < e->narms; i++) {
            cap_value(c, e->arms[i].guard);
            cap_value(c, e->arms[i].body);
        }
        return;
    }
    case TE_LET:
        cap_scrutinee(c, e->a, pat_use_kind(c->fcx, e->pat));
        return;
    case TE_BLOCK:
        for (uint32_t i = 0; i < e->nstmts; i++) {
            TStmt *s = e->stmts[i];
            if (s->kind == TS_LET) {
                if (s->init)
                    cap_scrutinee(c, s->init, pat_use_kind(c->fcx, s->pat));
                cap_value(c, s->els);
            } else {
                cap_value(c, s->expr);
            }
        }
        cap_value(c, e->a);
        return;
    case TE_CLOSURE: {
        ClosureInfo *inner = def_get(e->item.def)->closure;
        analyze_closure(c->fcx, inner);
        for (uint32_t i = 0; i < inner->ncaptures; i++) {
            Capture *cp = &inner->captures[i];
            if (cp->var->closure == c->ci->def)
                continue;
            int use = cp->mode == CAP_REF ? USE_BORROW
                      : cp->mode == CAP_MUT_REF ? USE_MUT : read_use(c, cp->place_ty);
            cap_record(c, cp->var, cp->path, cp->npath, cp->place_ty, use);
        }
        return;
    }
    default:
        break;
    }
    cap_value(c, e->a);
    cap_value(c, e->b);
    cap_value(c, e->c);
    for (uint32_t i = 0; i < e->nargs; i++)
        cap_value(c, e->args[i]);
}

static void analyze_closure(FnCtxt *fcx, ClosureInfo *ci) {
    if (ci->analyzed)
        return;
    ci->analyzed = true;
    CapCtx c = {fcx, ci, {0}};
    cap_value(&c, ci->body);
    /* A by-value capture of a place makes by-value captures of its
       sub-places redundant: the ancestor wins. */
    for (uint32_t i = 0; i < c.uses.len; i++) {
        CapUse *u = &c.uses.data[i];
        bool u_val = ci->is_move || u->use == USE_MOVE;
        if (!u_val || !u->var)
            continue;
        for (uint32_t k = 0; k < c.uses.len; k++) {
            CapUse *w = &c.uses.data[k];
            bool w_val = ci->is_move || w->use == USE_MOVE;
            if (k == i || !w->var || w->var != u->var || !w_val || w->npath < u->npath ||
                (w->npath == u->npath && k < i))
                continue;
            if (u->npath && memcmp(u->path, w->path, sizeof(uint32_t) * u->npath) != 0)
                continue;
            if (w->use > u->use)
                u->use = w->use;
            w->var = NULL; /* absorbed */
        }
    }
    uint32_t n = 0;
    for (uint32_t i = 0; i < c.uses.len; i++)
        if (c.uses.data[i].var)
            n++;
    ci->ncaptures = n;
    ci->captures = NEW_ARRAY(Capture, n ? n : 1);
    int kind = CK_FN;
    n = 0;
    for (uint32_t i = 0; i < c.uses.len; i++) {
        CapUse *u = &c.uses.data[i];
        if (!u->var)
            continue;
        Capture *cp = &ci->captures[n++];
        cp->var = u->var;
        cp->npath = (uint8_t)u->npath;
        memcpy(cp->path, u->path, sizeof(uint32_t) * u->npath);
        cp->place_ty = u->ty;
        if (ci->is_move || u->use == USE_MOVE)
            cp->mode = CAP_VALUE;
        else
            cp->mode = u->use == USE_MUT ? CAP_MUT_REF : CAP_REF;
        cp->ty = cp->mode == CAP_VALUE ? u->ty : ty_ref(u->ty, cp->mode == CAP_MUT_REF);
        if (u->use == USE_MOVE)
            kind = CK_FN_ONCE;
        else if (u->use == USE_MUT && kind == CK_FN)
            kind = CK_FN_MUT;
    }
    ci->kind = (uint8_t)kind;
}

/* ------------------------------------------------------------------ */
/* Body entry points                                                   */
/* ------------------------------------------------------------------ */

static FnCtxt *fcx_new(DefId owner, Scope *scope) {
    FnCtxt *fcx = NEW(FnCtxt);
    fcx->icx = infer_new(owner ? param_env(owner) : NULL);
    fcx->owner = owner;
    fcx->scope = scope_push(scope, SCOPE_LOCALS, 0);
    fcx->lc.scope = fcx->scope;
    fcx->lc.item = owner;
    fcx->lc.fcx = fcx;
    fcx->lc.impl_trait = IT_ERROR;
    fcx->identity = owner ? identity_substs(owner) : substs_empty();
    return fcx;
}

static Body *fcx_finish(FnCtxt *fcx, Body *body) {
    InferCtxt *icx = fcx->icx;
    infer_select_pending(icx);
    /* Numeric fallback, then `()` for variables that only arose from
       diverging expressions. */
    for (int round = 0; round < 2; round++) {
        for (uint32_t i = 0; i < icx->nvars; i++) {
            TyVar *v = &icx->vars[i];
            if (v->value)
                continue;
            if (v->node->ik == INFER_INT)
                (void)infer_unify(icx, v->node, ty_int(IK_I32));
            else if (v->node->ik == INFER_FLOAT)
                (void)infer_unify(icx, v->node, ty_float(FK_F64));
            else if (round == 1 && v->diverging)
                (void)infer_unify(icx, v->node, ty_unit());
        }
        infer_select_pending(icx);
    }
    for (uint32_t i = 0; i < fcx->locals.len; i++)
        fcx->locals.data[i]->ty = wb_type(fcx, fcx->locals.data[i]->ty, fcx->locals.data[i]->span);
    for (uint32_t i = 0; i < body->nparams; i++)
        wb_pat(fcx, body->params[i]);
    wb_expr(fcx, body->value);
    body->ret_ty = wb_type(fcx, body->ret_ty, body->value->span);
    for (uint32_t i = 0; i < fcx->deferred_ops.len; i++)
        finalize_deferred_op(fcx->deferred_ops.data[i]);
    for (uint32_t i = 0; i < fcx->closures.len; i++)
        analyze_closure(fcx, def_get(fcx->closures.data[i])->closure);
    body->locals = fcx->locals.data;
    body->nlocals = fcx->locals.len;
    body->closures = fcx->closures;
    return body;
}

typedef struct OpaqueFold {
    FnCtxt *fcx;
    Span sp;
} OpaqueFold;

static Type *opaque_fold(TypeFolder *f, Type *t) {
    OpaqueFold *of = f->ctx;
    FnCtxt *fcx = of->fcx;
    if (t->kind == T_OPAQUE && t->def == fcx->owner) {
        OpaqueInfo *oi = def_get(fcx->owner)->opaques;
        if (!fcx->opaque_vars)
            fcx->opaque_vars = NEW_ARRAY(Type *, oi->n);
        if (!fcx->opaque_vars[t->index]) {
            Type *v = infer_var(fcx->icx, INFER_TY, of->sp);
            fcx->opaque_vars[t->index] = v;
            /* The hidden type must satisfy the declared bounds; registering
               them also lets them drive inference (closure signatures, the
               Item type of an iterator, ...). */
            PredList *bl = &oi->bounds[t->index];
            for (uint32_t i = 0; i < bl->len; i++) {
                Predicate p = bl->data[i];
                if (p.tr.substs->t[0] != t)
                    continue;
                Type **ts = NEW_ARRAY(Type *, p.tr.substs->len);
                memcpy(ts, p.tr.substs->t, sizeof(Type *) * p.tr.substs->len);
                ts[0] = v;
                p.tr.substs = substs_new(ts, p.tr.substs->len);
                infer_register(fcx->icx, p, of->sp);
            }
        }
        return fcx->opaque_vars[t->index];
    }
    return ty_super_fold(f, t);
}

enum { QS_BODY = 4 };

Body *typeck_body(DefId def) {
    Def *d = def_get(def);
    if (d->body)
        return d->body;
    Item *it = d->item;
    if (!it)
        return NULL;
    Expr *init = NULL;
    Block *block = NULL;
    switch (d->kind) {
    case DEF_FN: block = it->fn_.body; break;
    case DEF_CONST: case DEF_ASSOC_CONST: init = it->const_.expr; break;
    case DEF_STATIC: init = it->static_.expr; break;
    default: fatal(d->span, "internal: typeck_body of %s", def_path_str(def));
    }
    if (!block && !init)
        return NULL;
    if (d->query_state & QS_BODY)
        fatal(d->span, "cycle while type-checking `%s` (e.g. a recursive `impl Trait` return type)",
              def_path_str(def));
    d->query_state |= QS_BODY;
    FnCtxt *fcx = fcx_new(def, scope_for_def(def));
    Body *body = NEW(Body);
    body->owner = def;
    if (block) {
        FnSigT *sig = fn_sig(def);
        FnSig *ast = &it->fn_.sig;
        uint32_t n = sig->inputs->len;
        body->params = NEW_ARRAY(TPat *, n ? n : 1);
        body->nparams = n;
        uint32_t k = 0;
        if (ast->self_param) {
            Local *l = new_local(fcx, (Sym)KW_SELF_VALUE, sig->inputs->t[0],
                                 ast->self_param->kind != SELF_REF && ast->self_param->is_mut,
                                 ast->self_param->span);
            scope_bind(fcx->scope, l->name, l);
            TPat *p = NEW(TPat);
            p->kind = TP_BIND;
            p->span = l->span;
            p->ty = l->ty;
            p->local = l;
            body->params[k++] = p;
        }
        for (uint32_t i = 0; i < ast->params.len; i++, k++)
            body->params[k] = check_pat_top(fcx, ast->params.data[i]->pat, sig->inputs->t[k]);
        Type *ret = sig->output;
        if (ret->flags & TYF_HAS_PROJ) {
            OpaqueFold of = {fcx, d->span};
            TypeFolder f = {opaque_fold, TYF_HAS_PROJ, &of, NULL};
            ret = ty_fold(&f, ret);
        }
        fcx->ret_ty = ret;
        body->ret_ty = ret;
        body->value = coerce(fcx, check_block(fcx, block, ret, 0), ret);
    } else {
        Type *ty = type_of(def);
        fcx->ret_ty = ty;
        body->ret_ty = ty;
        body->value = check_expr_coerce(fcx, init, ty);
    }
    fcx_finish(fcx, body);
    if (fcx->opaque_vars) {
        OpaqueInfo *oi = d->opaques;
        for (uint32_t i = 0; i < oi->n; i++)
            if (fcx->opaque_vars[i])
                oi->concrete[i] = wb_type(fcx, fcx->opaque_vars[i], d->span);
    }
    d->query_state &= (uint8_t)~QS_BODY;
    return d->body = body;
}

Body *typeck_anon_const(Scope *scope, DefId item, Expr *e, Type *expected) {
    FnCtxt *fcx = fcx_new(item, scope);
    Body *body = NEW(Body);
    body->owner = item;
    fcx->ret_ty = expected;
    body->ret_ty = expected;
    body->value = check_expr_coerce(fcx, e, expected);
    return fcx_finish(fcx, body);
}

Type *opaque_concrete(DefId fn, uint32_t ordinal) {
    Def *d = def_get(fn);
    (void)fn_sig(fn);
    if (!d->opaques || ordinal >= d->opaques->n)
        fatal(d->span, "internal: no opaque type #%u in `%s`", ordinal, def_path_str(fn));
    if (!d->opaques->concrete[ordinal]) {
        if (!typeck_body(fn))
            fatal(d->span, "`impl Trait` return type of a fn without a body: `%s`", def_path_str(fn));
        if (!d->opaques->concrete[ordinal])
            fatal(d->span, "could not determine the hidden type of `impl Trait` in `%s`",
                  def_path_str(fn));
    }
    return d->opaques->concrete[ordinal];
}
