/*
 * mir_build.c - THIR -> MIR for one monomorphic instance.
 *
 * Shape (after rustc's MIR builder):
 *   expr_into(dest, e)   evaluate e into a place; the workhorse
 *   lower_place(e)       evaluate e to a place (temporaries for rvalues)
 *   lower_operand(e)     evaluate e to an operand (constant / copy / move)
 *   cond_branch(e, t, f) evaluate a condition (incl. `let` patterns)
 *   pat_test / pat_bind  pattern matching: tests branch to a fail block,
 *                        bindings are assigned once all tests passed
 *
 * Scopes and drops. b->scopes is a stack; each scope lists the locals whose
 * value must be dropped when the scope is left. Variables are scheduled in
 * their block's scope, temporaries in the innermost statement scope
 * (b->temp_scope; the tail expression of a block uses the enclosing one).
 * Leaving a scope normally (pop_scope) or by break/continue/return
 * (exit_scopes) emits ST_DROP for the scheduled locals in reverse order.
 * Those drops mean "drop if still initialized": mir_drop.c turns them into
 * unconditional or flag-guarded drops. A local scheduled into a scope that
 * was opened outside the conditional construct currently being lowered is
 * marked maybe_uninit.
 */
#include "mir.h"

#include <stdio.h>

typedef VEC(LocalId) LocalIdVec;

typedef struct ScopeM {
    LocalIdVec drops;
    uint32_t cond_depth;
} ScopeM;

typedef struct BreakTarget {
    struct BreakTarget *parent;
    uint32_t label;
    BlockId brk, cont;
    Place dest;
    uint32_t scope_depth; /* scopes above this index are exited */
} BreakTarget;

typedef VEC(const TExpr *) ConstExprVec;

typedef struct Builder {
    MirBody *mir;
    Instance *inst;
    Body *body;
    Substs *substs;
    ClosureInfo *closure; /* non-NULL when building a closure body */
    Type *closure_ty;
    BlockId cur;
    LocalId *vars; /* THIR Local.id -> MIR local (0 = not yet created) */
    ScopeM *scopes;
    uint32_t nscopes, scopes_cap;
    uint32_t temp_scope;
    uint32_t cond_depth;
    BreakTarget *breaks;
    bool in_global_init;
    ConstExprVec ext_refs; /* borrow expressions with extended temporaries */
    uint32_t ext_scope;
    Span span;
} Builder;

/* ------------------------------------------------------------------ */
/* MIR construction primitives                                         */
/* ------------------------------------------------------------------ */

LocalId mir_new_local(MirBody *mir, Type *ty, Sym name) {
    if (mir->nlocals == mir->locals_cap) {
        uint32_t nc = mir->locals_cap ? mir->locals_cap * 2 : 16;
        MirLocal *n = NEW_ARRAY(MirLocal, nc);
        if (mir->nlocals)
            memcpy(n, mir->locals, sizeof(MirLocal) * mir->nlocals);
        mir->locals = n;
        mir->locals_cap = nc;
    }
    MirLocal *l = &mir->locals[mir->nlocals];
    memset(l, 0, sizeof *l);
    l->ty = ty;
    l->name = name;
    return mir->nlocals++;
}

BlockId mir_new_block(MirBody *mir) {
    if (mir->nblocks == mir->blocks_cap) {
        uint32_t nc = mir->blocks_cap ? mir->blocks_cap * 2 : 16;
        MirBlock *n = NEW_ARRAY(MirBlock, nc);
        if (mir->nblocks)
            memcpy(n, mir->blocks, sizeof(MirBlock) * mir->nblocks);
        mir->blocks = n;
        mir->blocks_cap = nc;
    }
    memset(&mir->blocks[mir->nblocks], 0, sizeof(MirBlock));
    return mir->nblocks++;
}

MirStmt *mir_push_stmt(MirBody *mir, BlockId b, StmtKindM kind, Span sp) {
    MirBlock *blk = &mir->blocks[b];
    if (blk->nstmts == blk->cap) {
        uint32_t nc = blk->cap ? blk->cap * 2 : 8;
        MirStmt *n = NEW_ARRAY(MirStmt, nc);
        if (blk->nstmts)
            memcpy(n, blk->stmts, sizeof(MirStmt) * blk->nstmts);
        blk->stmts = n;
        blk->cap = nc;
    }
    MirStmt *s = &blk->stmts[blk->nstmts++];
    memset(s, 0, sizeof *s);
    s->kind = (uint8_t)kind;
    s->span = sp;
    return s;
}

Place mir_place_local(MirBody *mir, LocalId l) {
    Place p = {l, NULL, 0, mir->locals[l].ty};
    return p;
}

Place mir_place_proj(Place base, ProjKind kind, uint32_t index, Type *ty) {
    Place p = base;
    p.projs = NEW_ARRAY(Proj, base.nprojs + 1);
    if (base.nprojs)
        memcpy(p.projs, base.projs, sizeof(Proj) * base.nprojs);
    Proj *pj = &p.projs[base.nprojs];
    pj->kind = (uint8_t)kind;
    pj->index = index;
    pj->ty = ty;
    p.nprojs = base.nprojs + 1;
    p.ty = ty;
    return p;
}

Operand mir_const_int(Type *ty, uint64_t lo, uint64_t hi) {
    Operand o;
    memset(&o, 0, sizeof o);
    o.kind = OP_CONST;
    o.c.kind = MC_INT;
    o.c.ty = ty;
    o.c.lo = lo;
    o.c.hi = hi;
    return o;
}

static Operand op_zst(Type *ty) {
    Operand o;
    memset(&o, 0, sizeof o);
    o.kind = OP_CONST;
    o.c.kind = MC_ZST;
    o.c.ty = ty;
    return o;
}

static Operand op_copy(Place p) {
    Operand o;
    memset(&o, 0, sizeof o);
    o.kind = OP_COPY;
    o.place = p;
    return o;
}

static Operand op_move(Place p) {
    Operand o = op_copy(p);
    o.kind = OP_MOVE;
    return o;
}

/* Copy for Copy types, move otherwise. */
static Operand op_use(Place p) { return ty_is_copy(p.ty) ? op_copy(p) : op_move(p); }

static Type *T(Builder *b, Type *t) { return mono_ty(t, b->substs, b->span); }

static Place local_place(Builder *b, LocalId l) { return mir_place_local(b->mir, l); }

static Place new_temp(Builder *b, Type *ty) {
    return local_place(b, mir_new_local(b->mir, ty, 0));
}

static Rvalue rv_new(RvalueKind k, Type *ty) {
    Rvalue rv;
    memset(&rv, 0, sizeof rv);
    rv.kind = (uint8_t)k;
    rv.ty = ty;
    return rv;
}

static void emit_assign(Builder *b, Place dest, Rvalue rv, Span sp) {
    MirStmt *s = mir_push_stmt(b->mir, b->cur, ST_ASSIGN, sp);
    s->place = dest;
    s->rv = rv;
}

static void emit_use(Builder *b, Place dest, Operand o, Span sp) {
    Rvalue rv = rv_new(RV_USE, dest.ty);
    rv.a = o;
    emit_assign(b, dest, rv, sp);
}

static void emit_drop(Builder *b, Place p, Span sp) {
    MirStmt *s = mir_push_stmt(b->mir, b->cur, ST_DROP, sp);
    s->place = p;
}

static void emit_forget(Builder *b, Place p, Span sp) {
    if (!ty_needs_drop(p.ty))
        return;
    MirStmt *s = mir_push_stmt(b->mir, b->cur, ST_FORGET, sp);
    s->place = p;
}

static void term_goto(Builder *b, BlockId target) {
    Terminator *t = &b->mir->blocks[b->cur].term;
    if (t->kind == TM_NONE) {
        t->kind = TM_GOTO;
        t->target = target;
    }
}

static void term_unreachable(Builder *b) {
    Terminator *t = &b->mir->blocks[b->cur].term;
    if (t->kind == TM_NONE)
        t->kind = TM_UNREACHABLE;
    b->cur = mir_new_block(b->mir);
}

/* switch on a bool operand */
static void term_if(Builder *b, Operand cond, BlockId then_blk, BlockId else_blk) {
    Terminator *t = &b->mir->blocks[b->cur].term;
    t->kind = TM_SWITCH;
    t->discr = cond;
    t->values = NEW_ARRAY(ConstVal, 1);
    t->targets = NEW_ARRAY(BlockId, 1);
    t->values[0].lo = 0;
    t->targets[0] = else_blk;
    t->ntargets = 1;
    t->target = then_blk;
}

/* if (discr == value) goto eq_blk else goto ne_blk */
static void term_switch_eq(Builder *b, Operand discr, ConstVal value, BlockId eq_blk, BlockId ne_blk) {
    Terminator *t = &b->mir->blocks[b->cur].term;
    t->kind = TM_SWITCH;
    t->discr = discr;
    t->values = NEW_ARRAY(ConstVal, 1);
    t->targets = NEW_ARRAY(BlockId, 1);
    t->values[0] = value;
    t->targets[0] = eq_blk;
    t->ntargets = 1;
    t->target = ne_blk;
}

/* ------------------------------------------------------------------ */
/* Scopes                                                              */
/* ------------------------------------------------------------------ */

static uint32_t push_scope(Builder *b) {
    if (b->nscopes == b->scopes_cap) {
        uint32_t nc = b->scopes_cap ? b->scopes_cap * 2 : 16;
        ScopeM *n = NEW_ARRAY(ScopeM, nc);
        if (b->nscopes)
            memcpy(n, b->scopes, sizeof(ScopeM) * b->nscopes);
        b->scopes = n;
        b->scopes_cap = nc;
    }
    ScopeM *s = &b->scopes[b->nscopes];
    memset(s, 0, sizeof *s);
    s->cond_depth = b->cond_depth;
    return b->nscopes++;
}

static void schedule_drop(Builder *b, uint32_t scope, LocalId l) {
    MirLocal *ml = &b->mir->locals[l];
    if (!ty_needs_drop(ml->ty))
        return;
    ScopeM *s = &b->scopes[scope];
    if (b->cond_depth > s->cond_depth)
        ml->maybe_uninit = true;
    vec_push(s->drops, l);
}

static void emit_scope_drops(Builder *b, uint32_t scope, Span sp) {
    ScopeM *s = &b->scopes[scope];
    for (uint32_t i = s->drops.len; i-- > 0;)
        emit_drop(b, local_place(b, s->drops.data[i]), sp);
}

static void pop_scope(Builder *b, Span sp) {
    emit_scope_drops(b, b->nscopes - 1, sp);
    b->nscopes--;
}

/* Emit the drops for leaving all scopes with index >= depth (no pop). */
static void exit_scopes(Builder *b, uint32_t depth, Span sp) {
    for (uint32_t i = b->nscopes; i-- > depth;)
        emit_scope_drops(b, i, sp);
}

/* ------------------------------------------------------------------ */
/* Forward declarations                                                */
/* ------------------------------------------------------------------ */

static void expr_into(Builder *b, Place dest, const TExpr *e);
static Place lower_place(Builder *b, const TExpr *e);
static Operand lower_operand(Builder *b, const TExpr *e);
static void cond_branch(Builder *b, const TExpr *e, BlockId then_blk, BlockId else_blk,
                        uint32_t bind_scope);
static void pat_test(Builder *b, const TPat *p, Place place, BlockId fail);
static void pat_bind(Builder *b, const TPat *p, Place place, bool real_moves, int scope);
static void emit_call(Builder *b, Place dest, DefId method, CalleeRes cr, Operand *ops, uint32_t n,
                      Span sp);

static bool is_place_expr(const TExpr *e) {
    return e->kind == TE_LOCAL || e->kind == TE_FIELD || e->kind == TE_INDEX ||
           e->kind == TE_DEREF || e->kind == TE_STATIC;
}

/* ------------------------------------------------------------------ */
/* Variables                                                           */
/* ------------------------------------------------------------------ */

static LocalId var_local(Builder *b, Local *l) {
    if (!b->vars[l->id])
        b->vars[l->id] = mir_new_local(b->mir, T(b, l->ty), l->name);
    return b->vars[l->id];
}

/*
 * The place `l.path` as seen from the body being built. Inside a closure,
 * a variable of an enclosing scope is reached through the environment: by
 * the by-value capture with the longest matching field path, else by the
 * capture of the whole variable.
 */
static Place var_path_place(Builder *b, Local *l, const uint32_t *path, uint32_t npath, Span sp) {
    Place p;
    uint32_t used = 0;
    if (b->closure && l->closure != b->closure->def) {
        ClosureInfo *ci = b->closure;
        int best = -1;
        for (uint32_t i = 0; i < ci->ncaptures; i++) {
            Capture *c = &ci->captures[i];
            if (c->var != l || c->npath > npath)
                continue;
            if (c->npath && memcmp(c->path, path, sizeof(uint32_t) * c->npath) != 0)
                continue;
            if (best < 0 || c->npath > ci->captures[best].npath)
                best = (int)i;
        }
        if (best < 0)
            fatal(sp, "internal: variable `%s` is used in a closure but was not captured",
                  sym_str(l->name));
        Capture *c = &ci->captures[best];
        Place env = local_place(b, 1);
        if (ci->kind != CK_FN_ONCE)
            env = mir_place_proj(env, PJ_DEREF, 0, b->closure_ty);
        p = mir_place_proj(env, PJ_FIELD, (uint32_t)best, T(b, c->ty));
        if (c->mode != CAP_VALUE)
            p = mir_place_proj(p, PJ_DEREF, 0, T(b, c->place_ty));
        used = c->npath;
    } else {
        p = local_place(b, var_local(b, l));
    }
    for (uint32_t i = used; i < npath; i++)
        p = mir_place_proj(p, PJ_FIELD, path[i], ty_field_type(p.ty, 0, path[i], sp));
    return p;
}

static Place var_place(Builder *b, Local *l, Span sp) { return var_path_place(b, l, NULL, 0, sp); }

/* ------------------------------------------------------------------ */
/* Constants                                                           */
/* ------------------------------------------------------------------ */

static const TExpr *peel_blocks(const TExpr *e) {
    while (e->kind == TE_BLOCK && e->nstmts == 0 && e->a)
        e = e->a;
    return e;
}

/* Resolve a (possibly trait-associated) const to the item that defines its
   value. */
static void resolve_const_item(Builder *b, const TExpr *e, DefId *def, Substs **substs) {
    *def = e->item.def;
    *substs = norm_closed_substs(substs_subst(e->item.substs, b->substs ? b->substs : substs_empty()),
                                 e->span);
    Def *d = def_get(*def);
    if (d->kind == DEF_ASSOC_CONST && def_get(d->parent)->kind == DEF_TRAIT) {
        TraitRef tr = {d->parent, *substs};
        Selection sel = select_closed(tr, e->span);
        if (sel.kind == SEL_IMPL) {
            DefId it = impl_item(sel.impl, d->name);
            if (it) {
                *def = it;
                *substs = sel.substs;
                return;
            }
        }
        if (!d->item->const_.expr)
            fatal(e->span, "associated constant `%s` has no value for `%s`", sym_str(d->name),
                  ty_to_str((*substs)->t[0]));
    }
}

/* Try to express e as a MIR constant. */
static bool try_const(Builder *b, const TExpr *e, Operand *out) {
    Type *ty = T(b, e->ty);
    memset(out, 0, sizeof *out);
    out->kind = OP_CONST;
    out->c.ty = ty;
    switch ((TExprKind)e->kind) {
    case TE_LIT_INT: {
        ConstVal v = const_wrap((ConstVal){e->int_.lo, e->int_.hi}, ty);
        out->c.kind = MC_INT;
        out->c.lo = v.lo;
        out->c.hi = v.hi;
        return true;
    }
    case TE_LIT_FLOAT:
        out->c.kind = MC_FLOAT;
        out->c.fval = e->fval;
        return true;
    case TE_LIT_STR:
        out->c.kind = MC_STR;
        out->c.str = e->str.s;
        out->c.len = e->str.len;
        return true;
    case TE_LIT_BYTESTR:
        out->c.kind = MC_BYTESTR;
        out->c.str = e->str.s;
        out->c.len = e->str.len;
        return true;
    case TE_FN:
        out->c.kind = MC_ZST;
        return true;
    case TE_TUPLE:
        if (e->nargs == 0) {
            out->c.kind = MC_ZST;
            return true;
        }
        return false;
    case TE_UNARY:
        if (e->op == UN_NEG && e->a->kind == TE_LIT_FLOAT) {
            out->c.kind = MC_FLOAT;
            out->c.fval = -e->a->fval;
            return true;
        }
        /* fallthrough */
    case TE_CONST_PARAM: case TE_BINARY: case TE_CAST: {
        ConstVal v;
        if (!const_eval(e, b->substs, &v))
            return false;
        out->c.kind = MC_INT;
        out->c.lo = v.lo;
        out->c.hi = v.hi;
        return true;
    }
    case TE_CONST: {
        ConstVal v;
        if (const_eval(e, b->substs, &v)) {
            out->c.kind = MC_INT;
            out->c.lo = v.lo;
            out->c.hi = v.hi;
            return true;
        }
        DefId def;
        Substs *substs;
        resolve_const_item(b, e, &def, &substs);
        Body *cb = typeck_body(def);
        if (!cb)
            return false;
        const TExpr *v0 = peel_blocks(cb->value);
        if (v0->kind == TE_LIT_STR || v0->kind == TE_LIT_FLOAT ||
            (v0->kind == TE_UNARY && v0->op == UN_NEG && v0->a->kind == TE_LIT_FLOAT)) {
            Builder tmp = *b;
            tmp.substs = substs;
            return try_const(&tmp, v0, out);
        }
        return false;
    }
    default:
        return false;
    }
}

static Place global_place(Builder *b, Global *g, Span sp) {
    Place ptr = new_temp(b, ty_ptr(g->ty, true));
    Rvalue rv = rv_new(RV_GLOBAL_ADDR, ptr.ty);
    rv.global = g;
    emit_assign(b, ptr, rv, sp);
    return mir_place_proj(ptr, PJ_DEREF, 0, g->ty);
}

static Place const_global_place(Builder *b, const TExpr *e) {
    DefId def;
    Substs *substs;
    resolve_const_item(b, e, &def, &substs);
    return global_place(b, mono_const_global(def, substs, e->span), e->span);
}

/* ------------------------------------------------------------------ */
/* Places and operands                                                 */
/* ------------------------------------------------------------------ */

static Place temp_for(Builder *b, const TExpr *e) {
    Place t = new_temp(b, T(b, e->ty));
    expr_into(b, t, e);
    schedule_drop(b, b->temp_scope, t.local);
    return t;
}

static Place lower_place(Builder *b, const TExpr *e) {
    b->span = e->span;
    switch ((TExprKind)e->kind) {
    case TE_LOCAL:
        return var_place(b, e->local, e->span);
    case TE_STATIC:
        return global_place(b, mono_static(e->item.def, e->span), e->span);
    case TE_FIELD: {
        if (b->closure) {
            /* a field path of a captured variable may have its own capture */
            uint32_t rev[32], n = 0;
            const TExpr *root = e;
            while (root->kind == TE_FIELD && n < 32) {
                rev[n++] = root->index;
                root = root->a;
            }
            if (root->kind == TE_LOCAL && root->local->closure != b->closure->def) {
                uint32_t path[32];
                for (uint32_t i = 0; i < n; i++)
                    path[i] = rev[n - 1 - i];
                return var_path_place(b, root->local, path, n, e->span);
            }
        }
        Place base = lower_place(b, e->a);
        return mir_place_proj(base, PJ_FIELD, e->index, T(b, e->ty));
    }
    case TE_INDEX: {
        Place base = lower_place(b, e->a);
        Place idx = new_temp(b, ty_usize());
        emit_use(b, idx, lower_operand(b, e->b), e->span);
        Operand len;
        if (base.ty->kind == T_ARRAY) {
            len = mir_const_int(ty_usize(), base.ty->len->cval, 0);
        } else {
            Place lt = new_temp(b, ty_usize());
            Rvalue rv = rv_new(RV_LEN, ty_usize());
            rv.place = base;
            emit_assign(b, lt, rv, e->span);
            len = op_copy(lt);
        }
        MirStmt *s = mir_push_stmt(b->mir, b->cur, ST_BOUNDS_CHECK, e->span);
        s->rv.a = op_copy(idx);
        s->rv.b = len;
        Place p = mir_place_proj(base, PJ_INDEX, 0, T(b, e->ty));
        p.projs[p.nprojs - 1].local = idx.local;
        return p;
    }
    case TE_DEREF: {
        Place ptr = lower_place(b, e->a);
        return mir_place_proj(ptr, PJ_DEREF, 0, T(b, e->ty));
    }
    case TE_CONST: {
        Operand c;
        if (!try_const(b, e, &c))
            return const_global_place(b, e);
        break;
    }
    default:
        break;
    }
    return temp_for(b, e);
}

static Operand lower_operand(Builder *b, const TExpr *e) {
    b->span = e->span;
    Operand c;
    if (try_const(b, e, &c))
        return c;
    if (e->kind == TE_CONST) {
        /* a non-scalar constant: every use is a fresh bitwise copy */
        return op_copy(const_global_place(b, e));
    }
    if (is_place_expr(e))
        return op_use(lower_place(b, e));
    Place t = new_temp(b, T(b, e->ty));
    expr_into(b, t, e);
    schedule_drop(b, b->temp_scope, t.local);
    return op_use(t);
}

/* ------------------------------------------------------------------ */
/* Promotion of constant expressions                                   */
/* ------------------------------------------------------------------ */

static bool promotable(const TExpr *e) {
    switch ((TExprKind)e->kind) {
    case TE_LIT_INT: case TE_LIT_FLOAT: case TE_LIT_STR: case TE_LIT_BYTESTR: case TE_FN:
    case TE_CONST: case TE_CONST_PARAM:
        return true;
    case TE_UNARY: case TE_CAST: case TE_UNSIZE: case TE_REIFY: case TE_REPEAT:
        return promotable(e->a);
    case TE_BINARY:
        return promotable(e->a) && promotable(e->b);
    case TE_REF:
        return !e->is_mut && promotable(e->a);
    case TE_TUPLE: case TE_ARRAY: case TE_ADT:
        if (e->b)
            return false;
        for (uint32_t i = 0; i < e->nargs; i++)
            if (!promotable(e->args[i]))
                return false;
        return true;
    case TE_BLOCK:
        return e->nstmts == 0 && e->a && promotable(e->a);
    default:
        return false;
    }
}

static bool in_ext_refs(Builder *b, const TExpr *e) {
    for (uint32_t i = 0; i < b->ext_refs.len; i++)
        if (b->ext_refs.data[i] == e)
            return true;
    return false;
}

/* Collect the borrow expressions of a `let` initializer whose temporaries
   live until the end of the enclosing block. */
static void mark_extended(Builder *b, const TExpr *e) {
    if (!e)
        return;
    switch ((TExprKind)e->kind) {
    case TE_REF: case TE_ADDR_OF:
        vec_push(b->ext_refs, e);
        mark_extended(b, e->a);
        break;
    case TE_UNSIZE: case TE_CAST:
        mark_extended(b, e->a);
        break;
    case TE_TUPLE: case TE_ARRAY: case TE_ADT:
        for (uint32_t i = 0; i < e->nargs; i++)
            mark_extended(b, e->args[i]);
        break;
    case TE_BLOCK:
        mark_extended(b, e->a);
        break;
    default:
        break;
    }
}

static void lower_ref(Builder *b, Place dest, const TExpr *e) {
    const TExpr *a = e->a;
    Type *aty = T(b, a->ty);
    if (!is_place_expr(a)) {
        Operand c;
        bool is_scalar_const = a->kind == TE_CONST && try_const(b, a, &c);
        if (a->kind == TE_CONST && !is_scalar_const) {
            /* &CONST: the address of the constant's global */
            Rvalue rv = rv_new(RV_REF, dest.ty);
            rv.place = const_global_place(b, a);
            emit_assign(b, dest, rv, e->span);
            return;
        }
        if (!e->is_mut && e->kind == TE_REF && promotable(a) && !ty_needs_drop(aty)) {
            Global *g = mono_promoted(a, b->body, b->substs, b->closure ? b->closure->def : 0, aty,
                                      e->span);
            Rvalue rv = rv_new(RV_REF, dest.ty);
            rv.place = global_place(b, g, e->span);
            emit_assign(b, dest, rv, e->span);
            return;
        }
        if (b->in_global_init) {
            /* a temporary borrowed by a static/const initializer lives
               forever: give it its own global */
            Global *g = mono_hidden_global(a, b->substs, aty);
            Place gp = global_place(b, g, e->span);
            expr_into(b, gp, a);
            Rvalue rv = rv_new(RV_REF, dest.ty);
            rv.place = gp;
            emit_assign(b, dest, rv, e->span);
            return;
        }
        if (in_ext_refs(b, e)) {
            Place t = new_temp(b, aty);
            expr_into(b, t, a);
            schedule_drop(b, b->ext_scope, t.local);
            Rvalue rv = rv_new(RV_REF, dest.ty);
            rv.place = t;
            emit_assign(b, dest, rv, e->span);
            return;
        }
    }
    Rvalue rv = rv_new(RV_REF, dest.ty);
    rv.place = lower_place(b, a);
    emit_assign(b, dest, rv, e->span);
}

/* ------------------------------------------------------------------ */
/* Casts                                                               */
/* ------------------------------------------------------------------ */

static bool is_intlike(Type *t) { return t->kind == T_INT || t->kind == T_BOOL || t->kind == T_CHAR; }
static bool is_ptrlike(Type *t) { return t->kind == T_PTR || t->kind == T_REF; }

static void lower_cast(Builder *b, Place dest, const TExpr *e) {
    Type *src = T(b, e->a->ty), *dst = dest.ty;
    Operand a = lower_operand(b, e->a);
    Rvalue rv = rv_new(RV_CAST, dst);
    rv.a = a;
    if (src == dst) {
        emit_use(b, dest, a, e->span);
        return;
    }
    if (src->kind == T_ADT && is_intlike(dst)) {
        AdtDef *adt = adt_def(src->def);
        if (!adt->is_enum)
            fatal(e->span, "unsupported cast from `%s` to `%s`", ty_to_str(src), ty_to_str(dst));
        Place t = new_temp(b, src);
        emit_use(b, t, a, e->span);
        Place d = new_temp(b, enum_tag_type(src));
        Rvalue dr = rv_new(RV_DISCR, d.ty);
        dr.place = t;
        emit_assign(b, d, dr, e->span);
        rv.a = op_copy(d);
        rv.cast = CAST_INT_INT;
    } else if (is_intlike(src) && is_intlike(dst)) {
        rv.cast = CAST_INT_INT;
    } else if (is_intlike(src) && dst->kind == T_FLOAT) {
        rv.cast = CAST_INT_FLOAT;
    } else if (src->kind == T_FLOAT && is_intlike(dst)) {
        rv.cast = CAST_FLOAT_INT;
    } else if (src->kind == T_FLOAT && dst->kind == T_FLOAT) {
        rv.cast = CAST_FLOAT_FLOAT;
    } else if (is_ptrlike(src) && is_ptrlike(dst)) {
        rv.cast = CAST_PTR_PTR;
    } else if (is_ptrlike(src) && dst->kind == T_INT) {
        rv.cast = CAST_PTR_INT;
    } else if (src->kind == T_INT && is_ptrlike(dst)) {
        rv.cast = CAST_INT_PTR;
    } else if (src->kind == T_FNPTR && dst->kind == T_INT) {
        rv.cast = CAST_FNPTR_INT;
    } else if ((src->kind == T_FNPTR && is_ptrlike(dst)) || (is_ptrlike(src) && dst->kind == T_FNPTR)) {
        rv.cast = CAST_TRANSMUTE;
    } else {
        fatal(e->span, "unsupported cast from `%s` to `%s`", ty_to_str(src), ty_to_str(dst));
    }
    emit_assign(b, dest, rv, e->span);
}

static void lower_unsize(Builder *b, Place dest, const TExpr *e) {
    Type *src = T(b, e->a->ty), *dst = dest.ty;
    Type *sp = ty_pointee(src), *dp = ty_pointee(dst);
    if (!sp && src->kind == T_ADT && dst->kind == T_ADT && src->substs->len && dst->substs->len) {
        /* Rc<T> / Arc<T>: field 0 is the pointer to the first type argument */
        sp = src->substs->t[0];
        dp = dst->substs->t[0];
    }
    if (!sp || !dp)
        fatal(e->span, "internal: unsizing of non-pointer types `%s` -> `%s`", ty_to_str(src),
              ty_to_str(dst));
    Rvalue rv = rv_new(RV_CAST, dst);
    rv.a = lower_operand(b, e->a);
    if (dp->kind == T_SLICE && sp->kind == T_ARRAY) {
        rv.cast = CAST_UNSIZE_ARRAY;
        rv.count = sp->len->cval;
    } else if (dp->kind == T_DYN) {
        rv.cast = CAST_UNSIZE_DYN;
        rv.vtable = mono_vtable(sp, dp, e->span);
    } else {
        fatal(e->span, "unsupported unsizing coercion `%s` -> `%s`", ty_to_str(src), ty_to_str(dst));
    }
    emit_assign(b, dest, rv, e->span);
}

/* ------------------------------------------------------------------ */
/* Calls                                                               */
/* ------------------------------------------------------------------ */

static MirStmt *emit_call_stmt(Builder *b, Place dest, CalleeKind ck, Operand *ops, uint32_t n,
                               Span sp) {
    MirStmt *s = mir_push_stmt(b->mir, b->cur, ST_CALL, sp);
    s->place = dest;
    s->callee = (uint8_t)ck;
    s->args = ops;
    s->nargs = n;
    return s;
}

static void finish_call(Builder *b, Place dest) {
    if (dest.ty->kind == T_NEVER) {
        b->mir->blocks[b->cur].stmts[b->mir->blocks[b->cur].nstmts - 1].diverges = true;
        term_unreachable(b);
    }
}

/* Spread a tuple operand into its fields. */
static Operand *spread_tuple(Builder *b, Operand tuple, Type *tuple_ty, uint32_t *n, Span sp) {
    uint32_t k = tuple_ty->substs->len;
    Operand *out = NEW_ARRAY(Operand, k ? k : 1);
    *n = k;
    if (k == 0)
        return out;
    Place tp;
    if (tuple.kind == OP_CONST) {
        tp = new_temp(b, tuple_ty);
        emit_use(b, tp, tuple, sp);
    } else {
        tp = tuple.place;
    }
    for (uint32_t i = 0; i < k; i++)
        out[i] = op_copy(mir_place_proj(tp, PJ_FIELD, i, tuple_ty->substs->t[i]));
    if (tuple.kind == OP_MOVE)
        emit_forget(b, tp, sp);
    return out;
}

static bool sym_is(Sym s, const char *name) { return strcmp(sym_str(s), name) == 0; }

static Place operand_place(Builder *b, Operand o, Span sp) {
    if (o.kind != OP_CONST)
        return o.place;
    Place t = new_temp(b, o.c.ty);
    emit_use(b, t, o, sp);
    return t;
}

static void lower_intrinsic(Builder *b, Place dest, Sym name, Substs *substs, Operand *ops,
                            uint32_t n, Span sp) {
    Type *t0 = substs->len ? substs->t[0] : NULL;
    if (sym_is(name, "forget")) {
        if (ops[0].kind == OP_MOVE)
            emit_forget(b, ops[0].place, sp);
        emit_use(b, dest, op_zst(dest.ty), sp);
        return;
    }
    if (sym_is(name, "read")) {
        Place p = mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, t0);
        emit_use(b, dest, op_copy(p), sp);
        return;
    }
    if (sym_is(name, "write")) {
        Place p = mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, t0);
        emit_use(b, p, ops[1], sp);
        emit_use(b, dest, op_zst(dest.ty), sp);
        return;
    }
    if (sym_is(name, "needs_drop")) {
        emit_use(b, dest, mir_const_int(ty_bool(), ty_needs_drop(t0), 0), sp);
        return;
    }
    if (sym_is(name, "drop_in_place")) {
        if (ty_needs_drop(t0)) {
            Place p = mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, t0);
            emit_drop(b, p, sp);
        }
        emit_use(b, dest, op_zst(dest.ty), sp);
        return;
    }
    if (sym_is(name, "slice_len")) {
        Type *pointee = ty_pointee(ops[0].kind == OP_CONST ? ops[0].c.ty : ops[0].place.ty);
        Rvalue rv = rv_new(RV_LEN, ty_usize());
        rv.place = mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, pointee);
        emit_assign(b, dest, rv, sp);
        return;
    }
    if (sym_is(name, "discriminant_value")) {
        Place p = mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, t0);
        if (t0->kind == T_ADT && adt_def(t0->def)->is_enum) {
            Place d = new_temp(b, enum_tag_type(t0));
            Rvalue dr = rv_new(RV_DISCR, d.ty);
            dr.place = p;
            emit_assign(b, d, dr, sp);
            Rvalue rv = rv_new(RV_CAST, dest.ty);
            rv.cast = CAST_INT_INT;
            rv.a = op_copy(d);
            emit_assign(b, dest, rv, sp);
        } else {
            emit_use(b, dest, mir_const_int(dest.ty, 0, 0), sp);
        }
        return;
    }
    if (sym_is(name, "transmute")) {
        Rvalue rv = rv_new(RV_CAST, dest.ty);
        rv.cast = CAST_TRANSMUTE;
        rv.a = ops[0];
        emit_assign(b, dest, rv, sp);
        return;
    }
    if (sym_is(name, "unreachable")) {
        term_unreachable(b);
        return;
    }
    if (sym_is(name, "type_name")) {
        Operand o;
        memset(&o, 0, sizeof o);
        o.kind = OP_CONST;
        o.c.kind = MC_STR;
        o.c.ty = dest.ty;
        o.c.str = ty_to_str(t0);
        o.c.len = (uint32_t)strlen(o.c.str);
        emit_use(b, dest, o, sp);
        return;
    }
    if (sym_is(name, "type_id")) {
        emit_use(b, dest, mir_const_int(dest.ty, t0->id, 0), sp);
        return;
    }
    /* Everything else is implemented by cgen. */
    MirStmt *s = emit_call_stmt(b, dest, CALLEE_INTRINSIC, ops, n, sp);
    s->intrinsic = name;
    s->substs = substs;
    finish_call(b, dest);
}

static void emit_ctor(Builder *b, Place dest, DefId ctor, Operand *ops, uint32_t n, Span sp) {
    Def *d = def_get(ctor);
    Rvalue rv = rv_new(RV_AGGREGATE, dest.ty);
    rv.agg = AGG_ADT;
    rv.variant = d->kind == DEF_VARIANT ? d->index : 0;
    rv.ops = ops;
    rv.nops = n;
    emit_assign(b, dest, rv, sp);
}

static void emit_call(Builder *b, Place dest, DefId method, CalleeRes cr, Operand *ops, uint32_t n,
                      Span sp) {
    switch (cr.kind) {
    case CR_INSTANCE: {
        MirStmt *s = emit_call_stmt(b, dest, CALLEE_DIRECT, ops, n, sp);
        s->inst = cr.inst;
        finish_call(b, dest);
        return;
    }
    case CR_VIRTUAL: {
        MirStmt *s = emit_call_stmt(b, dest, CALLEE_VIRTUAL, ops, n, sp);
        s->slot = cr.slot;
        finish_call(b, dest);
        return;
    }
    case CR_CTOR:
        emit_ctor(b, dest, method, ops, n, sp);
        return;
    case CR_INTRINSIC:
        lower_intrinsic(b, dest, cr.intrinsic, cr.substs, ops, n, sp);
        return;
    case CR_CLONE_COPY: {
        Type *t = cr.self_ty;
        Place p = mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, t);
        emit_use(b, dest, op_copy(p), sp);
        return;
    }
    case CR_CLOSURE: {
        ClosureInfo *ci = def_get(cr.self_ty->def)->closure;
        bool once = sym_is(def_get(method)->name, "call_once");
        Instance *inst;
        if (once && ci->kind != CK_FN_ONCE)
            inst = mono_closure_once_shim(cr.self_ty, sp);
        else
            inst = mono_closure_instance(cr.self_ty, sp);
        if (!once && ci->kind == CK_FN_ONCE)
            fatal(sp, "internal: FnOnce closure called through Fn/FnMut");
        MirStmt *s = emit_call_stmt(b, dest, CALLEE_DIRECT, ops, n, sp);
        s->inst = inst;
        finish_call(b, dest);
        return;
    }
    case CR_FN_UNTUPLED: {
        Type *ft = cr.self_ty;
        bool once = sym_is(def_get(method)->name, "call_once");
        Type *tuple_ty = ops[1].kind == OP_CONST ? ops[1].c.ty : ops[1].place.ty;
        uint32_t k;
        Operand *args = spread_tuple(b, ops[1], tuple_ty, &k, sp);
        if (ft->kind == T_FNDEF) {
            CalleeRes inner = mono_resolve_callee(ft->def, ft->substs, sp);
            emit_call(b, dest, ft->def, inner, args, k, sp);
        } else {
            Operand f = ops[0];
            if (!once)
                f = op_copy(mir_place_proj(operand_place(b, ops[0], sp), PJ_DEREF, 0, ft));
            MirStmt *s = emit_call_stmt(b, dest, CALLEE_INDIRECT, args, k, sp);
            s->rv.a = f;
            finish_call(b, dest);
        }
        return;
    }
    }
}

static void lower_call(Builder *b, Place dest, const TExpr *e) {
    const TExpr *callee = e->a;
    Type *cty = T(b, callee->ty);
    Span sp = e->span;
    if (cty->kind == T_CLOSURE) {
        ClosureInfo *ci = def_get(cty->def)->closure;
        Place env = lower_place(b, callee);
        Operand *ops = NEW_ARRAY(Operand, 2);
        if (ci->kind == CK_FN_ONCE) {
            ops[0] = op_use(env);
        } else {
            Place r = new_temp(b, ty_ref(cty, ci->kind == CK_FN_MUT));
            Rvalue rv = rv_new(RV_REF, r.ty);
            rv.place = env;
            emit_assign(b, r, rv, sp);
            ops[0] = op_copy(r);
        }
        Instance *inst = mono_closure_instance(cty, sp);
        Type *tuple_ty = inst->params[1];
        if (e->nargs == 0) {
            ops[1] = op_zst(tuple_ty);
        } else {
            Place tp = new_temp(b, tuple_ty);
            Rvalue rv = rv_new(RV_AGGREGATE, tuple_ty);
            rv.agg = AGG_TUPLE;
            rv.ops = NEW_ARRAY(Operand, e->nargs);
            rv.nops = e->nargs;
            for (uint32_t i = 0; i < e->nargs; i++)
                rv.ops[i] = lower_operand(b, e->args[i]);
            emit_assign(b, tp, rv, sp);
            ops[1] = op_move(tp);
        }
        MirStmt *s = emit_call_stmt(b, dest, CALLEE_DIRECT, ops, 2, sp);
        s->inst = inst;
        finish_call(b, dest);
        return;
    }
    Operand fnptr;
    memset(&fnptr, 0, sizeof fnptr);
    if (cty->kind == T_FNPTR)
        fnptr = lower_operand(b, callee);
    else if (callee->kind != TE_FN)
        (void)lower_operand(b, callee); /* evaluate for side effects */
    Operand *ops = NEW_ARRAY(Operand, e->nargs ? e->nargs : 1);
    for (uint32_t i = 0; i < e->nargs; i++) {
        const TExpr *a = e->args[i];
        /* A by-value unsized argument can only be `*boxed` (calling a
           Box<dyn FnOnce>): pass the box itself; the vtable shim unboxes. */
        if (a->kind == TE_DEREF && ty_is_unsized(T(b, a->ty)) && ty_pointee(T(b, a->a->ty)) &&
            T(b, a->a->ty)->kind == T_ADT)
            a = a->a;
        ops[i] = lower_operand(b, a);
    }
    b->span = sp;
    if (cty->kind == T_FNPTR) {
        MirStmt *s = emit_call_stmt(b, dest, CALLEE_INDIRECT, ops, e->nargs, sp);
        s->rv.a = fnptr;
        finish_call(b, dest);
        return;
    }
    if (cty->kind != T_FNDEF)
        fatal(sp, "internal: call of non-function type `%s`", ty_to_str(cty));
    CalleeRes cr = mono_resolve_callee(cty->def, cty->substs, sp);
    emit_call(b, dest, cty->def, cr, ops, e->nargs, sp);
}

/* ------------------------------------------------------------------ */
/* Patterns                                                            */
/* ------------------------------------------------------------------ */

static bool pat_has_bindings(const TPat *p) {
    if (!p)
        return false;
    if (p->kind == TP_BIND)
        return true;
    if (pat_has_bindings(p->sub) || pat_has_bindings(p->rest))
        return true;
    for (uint32_t i = 0; i < p->nsubs; i++)
        if (pat_has_bindings(p->subs[i]))
            return true;
    return false;
}

static bool pat_has_ref_bindings(const TPat *p) {
    if (!p)
        return false;
    if (p->kind == TP_BIND && p->bind_mode != BIND_VALUE)
        return true;
    if (pat_has_ref_bindings(p->sub) || pat_has_ref_bindings(p->rest))
        return true;
    for (uint32_t i = 0; i < p->nsubs; i++)
        if (pat_has_ref_bindings(p->subs[i]))
            return true;
    return false;
}

static ConstVal variant_disc(Type *adt_ty, uint32_t variant) {
    AdtDef *adt = adt_def(adt_ty->def);
    adt_compute_discs(adt);
    ConstVal v = {(uint64_t)adt->variants[variant].disc, adt->variants[variant].disc < 0 ? ~0ull : 0};
    return const_wrap(v, enum_tag_type(adt_ty));
}

static Place slice_elem_place(Builder *b, Place place, const TPat *p, uint32_t i) {
    Type *elem = place.ty->elem;
    Place ep = mir_place_proj(place, PJ_CONST_INDEX, 0, elem);
    Proj *pj = &ep.projs[ep.nprojs - 1];
    if (i < p->nprefix) {
        pj->index = i;
    } else {
        pj->index = p->nsubs - i; /* distance from the end */
        pj->from_end = true;
    }
    (void)b;
    return ep;
}

static Place sub_place(Builder *b, const TPat *p, Place place, uint32_t i) {
    Type *pt = place.ty;
    switch ((TPatKind)p->kind) {
    case TP_TUPLE:
        return mir_place_proj(place, PJ_FIELD, p->field_idx[i],
                              ty_field_type(pt, 0, p->field_idx[i], b->span));
    case TP_VARIANT: {
        Place dc = mir_place_proj(place, PJ_DOWNCAST, p->variant, pt);
        return mir_place_proj(dc, PJ_FIELD, p->field_idx[i],
                              ty_field_type(pt, p->variant, p->field_idx[i], b->span));
    }
    case TP_SLICE:
        return slice_elem_place(b, place, p, i);
    default:
        fatal(p->span, "internal: sub_place of pattern kind %d", p->kind);
    }
}

static Place deref_place(Builder *b, const TPat *p, Place place) {
    return mir_place_proj(place, PJ_DEREF, 0, T(b, p->sub->ty));
}

static void test_scalar(Builder *b, Operand val, BinOp op, Operand rhs, BlockId fail, Span sp) {
    Place t = new_temp(b, ty_bool());
    Rvalue rv = rv_new(RV_BINARY, ty_bool());
    rv.op = (uint8_t)op;
    rv.a = val;
    rv.b = rhs;
    emit_assign(b, t, rv, sp);
    BlockId ok = mir_new_block(b->mir);
    term_if(b, op_copy(t), ok, fail);
    b->cur = ok;
}

static void pat_test(Builder *b, const TPat *p, Place place, BlockId fail) {
    b->span = p->span;
    switch ((TPatKind)p->kind) {
    case TP_WILD:
        return;
    case TP_BIND:
        if (p->sub)
            pat_test(b, p->sub, place, fail);
        return;
    case TP_CONST: {
        Type *ty = place.ty;
        if (ty->kind == T_INT || ty->kind == T_BOOL || ty->kind == T_CHAR) {
            ConstVal v;
            if (const_eval(p->value, b->substs, &v)) {
                BlockId ok = mir_new_block(b->mir);
                term_switch_eq(b, op_copy(place), v, ok, fail);
                b->cur = ok;
                return;
            }
        }
        if (ty->kind == T_INT || ty->kind == T_BOOL || ty->kind == T_CHAR || ty->kind == T_FLOAT) {
            test_scalar(b, op_copy(place), BIN_EQ, lower_operand(b, p->value), fail, p->span);
            return;
        }
        if (ty->kind == T_REF && (ty->elem->kind == T_STR || ty->elem->kind == T_SLICE)) {
            DefId eq = lang_req(ty->elem->kind == T_STR ? "str_eq" : "slice_eq");
            Substs *substs = ty->elem->kind == T_STR ? substs_empty() : substs_1(ty->elem->elem);
            Operand *ops = NEW_ARRAY(Operand, 2);
            ops[0] = op_copy(place);
            ops[1] = lower_operand(b, p->value);
            Place t = new_temp(b, ty_bool());
            MirStmt *s = emit_call_stmt(b, t, CALLEE_DIRECT, ops, 2, p->span);
            s->inst = mono_fn_instance(eq, substs, p->span);
            BlockId ok = mir_new_block(b->mir);
            term_if(b, op_copy(t), ok, fail);
            b->cur = ok;
            return;
        }
        if (lang.partial_eq) {
            /* A constant of aggregate type: its type derives PartialEq
               (rustc requires structural equality), so `==` decides. */
            DefId eq = trait_item(lang.partial_eq, sym_intern_cstr("eq"));
            CalleeRes cr = mono_resolve_callee(eq, substs_2(ty, ty), p->span);
            Operand *ops = NEW_ARRAY(Operand, 2);
            Type *rty = ty_ref(ty, false);
            Place r1 = new_temp(b, rty), r2 = new_temp(b, rty);
            Rvalue rv = rv_new(RV_REF, rty);
            rv.place = place;
            emit_assign(b, r1, rv, p->span);
            rv.place = lower_place(b, p->value);
            emit_assign(b, r2, rv, p->span);
            ops[0] = op_copy(r1);
            ops[1] = op_copy(r2);
            Place t = new_temp(b, ty_bool());
            emit_call(b, t, eq, cr, ops, 2, p->span);
            BlockId ok = mir_new_block(b->mir);
            term_if(b, op_copy(t), ok, fail);
            b->cur = ok;
            return;
        }
        fatal(p->span, "constant patterns of type `%s` are not supported", ty_to_str(ty));
    }
    case TP_RANGE:
        if (p->lo)
            test_scalar(b, op_copy(place), BIN_GE, lower_operand(b, p->lo), fail, p->span);
        if (p->hi)
            test_scalar(b, op_copy(place), p->inclusive ? BIN_LE : BIN_LT, lower_operand(b, p->hi),
                        fail, p->span);
        return;
    case TP_TUPLE:
        for (uint32_t i = 0; i < p->nsubs; i++)
            pat_test(b, p->subs[i], sub_place(b, p, place, i), fail);
        return;
    case TP_VARIANT: {
        AdtDef *adt = adt_def(place.ty->def);
        if (adt->nvariants > 1) {
            Place d = new_temp(b, enum_tag_type(place.ty));
            Rvalue rv = rv_new(RV_DISCR, d.ty);
            rv.place = place;
            emit_assign(b, d, rv, p->span);
            BlockId ok = mir_new_block(b->mir);
            term_switch_eq(b, op_copy(d), variant_disc(place.ty, p->variant), ok, fail);
            b->cur = ok;
        }
        for (uint32_t i = 0; i < p->nsubs; i++)
            pat_test(b, p->subs[i], sub_place(b, p, place, i), fail);
        return;
    }
    case TP_DEREF:
        pat_test(b, p->sub, deref_place(b, p, place), fail);
        return;
    case TP_SLICE: {
        if (place.ty->kind == T_SLICE) {
            Place len = new_temp(b, ty_usize());
            Rvalue rv = rv_new(RV_LEN, ty_usize());
            rv.place = place;
            emit_assign(b, len, rv, p->span);
            test_scalar(b, op_copy(len), p->has_rest ? BIN_GE : BIN_EQ,
                        mir_const_int(ty_usize(), p->nsubs, 0), fail, p->span);
        }
        for (uint32_t i = 0; i < p->nsubs; i++)
            pat_test(b, p->subs[i], sub_place(b, p, place, i), fail);
        return;
    }
    case TP_OR: {
        BlockId ok = mir_new_block(b->mir);
        for (uint32_t i = 0; i < p->nsubs; i++) {
            BlockId next = i + 1 < p->nsubs ? mir_new_block(b->mir) : fail;
            pat_test(b, p->subs[i], place, next);
            term_goto(b, ok);
            b->cur = next;
        }
        b->cur = ok;
        return;
    }
    }
}

/*
 * Assign the bindings of p. With real_moves, by-value bindings of non-Copy
 * types move out of the scrutinee (and are scheduled for drop in `scope`
 * if scope >= 0); otherwise every binding is a bitwise copy (used while a
 * match guard is evaluated).
 */
static void pat_bind(Builder *b, const TPat *p, Place place, bool real_moves, int scope) {
    if (!pat_has_bindings(p))
        return;
    switch ((TPatKind)p->kind) {
    case TP_WILD: case TP_CONST: case TP_RANGE:
        return;
    case TP_BIND: {
        if (p->sub)
            pat_bind(b, p->sub, place, real_moves, scope);
        LocalId v = var_local(b, p->local);
        Place vp = local_place(b, v);
        if (p->bind_mode == BIND_VALUE) {
            bool mv = real_moves && !ty_is_copy(place.ty);
            emit_use(b, vp, mv ? op_move(place) : op_copy(place), p->span);
        } else {
            Rvalue rv = rv_new(RV_REF, vp.ty);
            rv.place = place;
            emit_assign(b, vp, rv, p->span);
        }
        if (real_moves && scope >= 0)
            schedule_drop(b, (uint32_t)scope, v);
        return;
    }
    case TP_TUPLE: case TP_VARIANT:
        for (uint32_t i = 0; i < p->nsubs; i++)
            pat_bind(b, p->subs[i], sub_place(b, p, place, i), real_moves, scope);
        return;
    case TP_DEREF:
        pat_bind(b, p->sub, deref_place(b, p, place), real_moves, scope);
        return;
    case TP_SLICE:
        for (uint32_t i = 0; i < p->nsubs; i++)
            pat_bind(b, p->subs[i], sub_place(b, p, place, i), real_moves, scope);
        if (p->rest) {
            Place sp = mir_place_proj(place, PJ_SUBSLICE, p->nprefix, T(b, p->rest->ty));
            sp.projs[sp.nprojs - 1].to = p->nsubs - p->nprefix;
            pat_bind(b, p->rest, sp, real_moves, scope);
        }
        return;
    case TP_OR: {
        /* Re-test to find the alternative that matched (tests are pure). */
        BlockId done = mir_new_block(b->mir);
        for (uint32_t i = 0; i < p->nsubs; i++) {
            bool last = i + 1 == p->nsubs;
            BlockId next = mir_new_block(b->mir);
            if (!last)
                pat_test(b, p->subs[i], place, next);
            pat_bind(b, p->subs[i], place, real_moves, i == 0 ? scope : -1);
            term_goto(b, done);
            b->cur = next;
        }
        term_unreachable(b);
        b->cur = done;
        return;
    }
    }
}

/* ------------------------------------------------------------------ */
/* Control flow                                                        */
/* ------------------------------------------------------------------ */

/* Branch on a condition. Bindings of `let` patterns are scheduled for drop
   in scope `bind_scope`. */
static void cond_branch(Builder *b, const TExpr *e, BlockId then_blk, BlockId else_blk,
                        uint32_t bind_scope) {
    if (e->kind == TE_LOGICAL) {
        BlockId mid = mir_new_block(b->mir);
        if (e->op == BIN_AND)
            cond_branch(b, e->a, mid, else_blk, bind_scope);
        else
            cond_branch(b, e->a, then_blk, mid, bind_scope);
        b->cur = mid;
        b->cond_depth++;
        cond_branch(b, e->b, then_blk, else_blk, bind_scope);
        b->cond_depth--;
        return;
    }
    if (e->kind == TE_LET) {
        Place scrut = lower_place(b, e->a);
        pat_test(b, e->pat, scrut, else_blk);
        pat_bind(b, e->pat, scrut, true, (int)bind_scope);
        term_goto(b, then_blk);
        return;
    }
    Operand c = lower_operand(b, e);
    term_if(b, c, then_blk, else_blk);
}

static bool cond_has_let(const TExpr *e) {
    if (e->kind == TE_LET)
        return true;
    if (e->kind == TE_LOGICAL)
        return cond_has_let(e->a) || cond_has_let(e->b);
    return false;
}

static void lower_if(Builder *b, Place dest, const TExpr *e) {
    BlockId then_blk = mir_new_block(b->mir), else_blk = mir_new_block(b->mir),
            end = mir_new_block(b->mir);
    /* The `if` scope holds pattern bindings; a plain condition's
       temporaries die before the branch, a `let` scrutinee's live on. */
    uint32_t is = push_scope(b);
    uint32_t saved_temp = b->temp_scope;
    bool has_let = cond_has_let(e->a);
    if (!has_let) {
        uint32_t cs = push_scope(b);
        b->temp_scope = cs;
        Operand c = lower_operand(b, e->a);
        Place ct = new_temp(b, ty_bool());
        emit_use(b, ct, c, e->span);
        pop_scope(b, e->span);
        b->temp_scope = saved_temp;
        term_if(b, op_copy(ct), then_blk, else_blk);
    } else {
        cond_branch(b, e->a, then_blk, else_blk, is);
    }
    b->cond_depth++;
    b->cur = then_blk;
    expr_into(b, dest, e->b);
    pop_scope(b, e->span);
    term_goto(b, end);
    b->cur = else_blk;
    if (e->c)
        expr_into(b, dest, e->c);
    else if (ty_is_unit(dest.ty))
        emit_use(b, dest, op_zst(dest.ty), e->span);
    term_goto(b, end);
    b->cond_depth--;
    b->cur = end;
}

static void lower_match(Builder *b, Place dest, const TExpr *e) {
    Place scrut = lower_place(b, e->a);
    BlockId end = mir_new_block(b->mir);
    b->cond_depth++;
    for (uint32_t i = 0; i < e->narms; i++) {
        const TArm *arm = &e->arms[i];
        BlockId next = mir_new_block(b->mir);
        uint32_t as = push_scope(b);
        pat_test(b, arm->pat, scrut, next);
        if (arm->guard) {
            pat_bind(b, arm->pat, scrut, false, -1);
            uint32_t saved_temp = b->temp_scope;
            uint32_t gs = push_scope(b);
            b->temp_scope = gs;
            Operand g = lower_operand(b, arm->guard);
            Place gt = new_temp(b, ty_bool());
            emit_use(b, gt, g, arm->span);
            pop_scope(b, arm->span);
            b->temp_scope = saved_temp;
            BlockId ok = mir_new_block(b->mir);
            term_if(b, op_copy(gt), ok, next);
            b->cur = ok;
        }
        pat_bind(b, arm->pat, scrut, true, (int)as);
        expr_into(b, dest, arm->body);
        pop_scope(b, arm->span);
        term_goto(b, end);
        b->cur = next;
    }
    b->cond_depth--;
    /* exhaustive: falling off the last arm is impossible */
    Terminator *t = &b->mir->blocks[b->cur].term;
    if (t->kind == TM_NONE)
        t->kind = TM_UNREACHABLE;
    b->cur = end;
}

static BreakTarget *find_target(Builder *b, uint32_t label, Span sp) {
    for (BreakTarget *t = b->breaks; t; t = t->parent)
        if (t->label == label)
            return t;
    fatal(sp, "internal: break/continue target not found");
}

static void lower_loop(Builder *b, Place dest, const TExpr *e) {
    BlockId head = mir_new_block(b->mir), end = mir_new_block(b->mir);
    term_goto(b, head);
    b->cur = head;
    BreakTarget bt = {b->breaks, e->label, end, head, dest, b->nscopes};
    b->breaks = &bt;
    b->cond_depth++;
    uint32_t saved_temp = b->temp_scope;
    uint32_t ls = push_scope(b);
    b->temp_scope = ls;
    if (e->kind == TE_WHILE) {
        BlockId body = mir_new_block(b->mir), exit = mir_new_block(b->mir);
        uint32_t bs = push_scope(b); /* `while let` bindings */
        if (cond_has_let(e->a)) {
            /* the scrutinee's temporaries live through the body */
            cond_branch(b, e->a, body, exit, bs);
            b->cur = exit;
            emit_scope_drops(b, ls, e->span);
        } else {
            /* a plain condition's temporaries die before the body runs */
            uint32_t cs = push_scope(b);
            b->temp_scope = cs;
            Operand c = lower_operand(b, e->a);
            Place ct = new_temp(b, ty_bool());
            emit_use(b, ct, c, e->span);
            pop_scope(b, e->span);
            b->temp_scope = ls;
            term_if(b, op_copy(ct), body, exit);
            b->cur = exit;
        }
        if (ty_is_unit(dest.ty))
            emit_use(b, dest, op_zst(dest.ty), e->span);
        term_goto(b, end);
        b->cur = body;
        Place unit = new_temp(b, ty_unit());
        expr_into(b, unit, e->b);
        pop_scope(b, e->span);
    } else {
        Place unit = new_temp(b, ty_unit());
        expr_into(b, unit, e->a);
    }
    pop_scope(b, e->span);
    b->temp_scope = saved_temp;
    term_goto(b, head);
    b->cond_depth--;
    b->breaks = bt.parent;
    b->cur = end;
}

/* `let (a, b);`: declare every binding of the pattern, uninitialized. */
static void declare_uninit(Builder *b, const TPat *p, uint32_t block_scope) {
    if (!p)
        return;
    if (p->kind == TP_BIND) {
        LocalId v = var_local(b, p->local);
        b->mir->locals[v].maybe_uninit = true;
        schedule_drop(b, block_scope, v);
    }
    declare_uninit(b, p->sub, block_scope);
    declare_uninit(b, p->rest, block_scope);
    for (uint32_t i = 0; i < p->nsubs; i++)
        declare_uninit(b, p->subs[i], block_scope);
}

static void lower_let(Builder *b, const TStmt *s, uint32_t block_scope) {
    const TPat *pat = s->pat;
    b->span = s->span;
    if (!s->init) {
        declare_uninit(b, pat, block_scope);
        return;
    }
    b->ext_refs.len = 0;
    b->ext_scope = block_scope;
    mark_extended(b, s->init);
    if (pat->kind == TP_BIND && pat->bind_mode == BIND_VALUE && !pat->sub && !s->els) {
        LocalId v = var_local(b, pat->local);
        expr_into(b, local_place(b, v), s->init);
        schedule_drop(b, block_scope, v);
        b->ext_refs.len = 0;
        return;
    }
    Place scrut;
    if (is_place_expr(s->init)) {
        scrut = lower_place(b, s->init);
    } else {
        scrut = new_temp(b, T(b, s->init->ty));
        expr_into(b, scrut, s->init);
        schedule_drop(b, pat_has_ref_bindings(pat) ? block_scope : b->temp_scope, scrut.local);
    }
    b->ext_refs.len = 0;
    BlockId fail = mir_new_block(b->mir);
    pat_test(b, pat, scrut, fail);
    pat_bind(b, pat, scrut, true, (int)block_scope);
    BlockId cont = b->cur;
    b->cur = fail;
    if (s->els) {
        b->cond_depth++;
        Place never = new_temp(b, ty_never());
        expr_into(b, never, s->els);
        b->cond_depth--;
    }
    Terminator *t = &b->mir->blocks[b->cur].term;
    if (t->kind == TM_NONE)
        t->kind = TM_UNREACHABLE;
    b->cur = cont;
}

static void lower_block(Builder *b, Place dest, const TExpr *e) {
    uint32_t bs = push_scope(b);
    BlockId end = 0;
    BreakTarget bt = {b->breaks, e->label, 0, 0, dest, bs};
    if (e->label) {
        end = mir_new_block(b->mir);
        bt.brk = end;
        b->breaks = &bt;
    }
    uint32_t saved_temp = b->temp_scope;
    for (uint32_t i = 0; i < e->nstmts; i++) {
        const TStmt *s = e->stmts[i];
        uint32_t ss = push_scope(b);
        b->temp_scope = ss;
        if (s->kind == TS_LET) {
            lower_let(b, s, bs);
        } else {
            Place t = new_temp(b, T(b, s->expr->ty));
            expr_into(b, t, s->expr);
            schedule_drop(b, ss, t.local);
        }
        pop_scope(b, s->span);
        b->temp_scope = saved_temp;
    }
    if (e->a)
        expr_into(b, dest, e->a);
    else if (ty_is_unit(dest.ty))
        emit_use(b, dest, op_zst(dest.ty), e->span);
    pop_scope(b, e->span);
    if (e->label) {
        b->breaks = bt.parent;
        term_goto(b, end);
        b->cur = end;
    }
}

/* ------------------------------------------------------------------ */
/* expr_into                                                           */
/* ------------------------------------------------------------------ */

static void lower_adt(Builder *b, Place dest, const TExpr *e) {
    Type *ty = dest.ty;
    VariantDef *v = &adt_def(ty->def)->variants[e->variant];
    uint32_t nf = e->b ? v->nfields : e->nargs;
    Rvalue rv = rv_new(RV_AGGREGATE, ty);
    rv.agg = AGG_ADT;
    rv.variant = e->variant;
    rv.ops = NEW_ARRAY(Operand, nf ? nf : 1);
    rv.field_idx = NEW_ARRAY(uint32_t, nf ? nf : 1);
    rv.nops = e->nargs;
    for (uint32_t i = 0; i < e->nargs; i++) {
        rv.ops[i] = lower_operand(b, e->args[i]);
        rv.field_idx[i] = e->field_idx[i];
    }
    if (e->b) {
        /* struct update: remaining fields come from the base */
        Place base = lower_place(b, e->b);
        for (uint32_t f = 0; f < v->nfields; f++) {
            bool given = false;
            for (uint32_t i = 0; i < e->nargs; i++)
                given |= e->field_idx[i] == f;
            if (given)
                continue;
            Place fp = mir_place_proj(base, PJ_FIELD, f, ty_field_type(ty, e->variant, f, e->span));
            rv.ops[rv.nops] = op_use(fp);
            rv.field_idx[rv.nops] = f;
            rv.nops++;
        }
    }
    emit_assign(b, dest, rv, e->span);
}

static void lower_closure_expr(Builder *b, Place dest, const TExpr *e) {
    ClosureInfo *ci = def_get(e->item.def)->closure;
    Rvalue rv = rv_new(RV_AGGREGATE, dest.ty);
    rv.agg = AGG_CLOSURE;
    rv.nops = ci->ncaptures;
    rv.ops = NEW_ARRAY(Operand, ci->ncaptures ? ci->ncaptures : 1);
    for (uint32_t i = 0; i < ci->ncaptures; i++) {
        Capture *c = &ci->captures[i];
        Place vp = var_path_place(b, c->var, c->path, c->npath, e->span);
        if (c->mode == CAP_VALUE) {
            rv.ops[i] = op_use(vp);
        } else {
            Place r = new_temp(b, T(b, c->ty));
            Rvalue rr = rv_new(RV_REF, r.ty);
            rr.place = vp;
            emit_assign(b, r, rr, e->span);
            rv.ops[i] = op_copy(r);
        }
    }
    emit_assign(b, dest, rv, e->span);
}

static void expr_into(Builder *b, Place dest, const TExpr *e) {
    Span sp = e->span;
    b->span = sp;
    Operand c;
    if (try_const(b, e, &c)) {
        emit_use(b, dest, c, sp);
        return;
    }
    switch ((TExprKind)e->kind) {
    case TE_LIT_INT: case TE_LIT_FLOAT: case TE_LIT_STR: case TE_LIT_BYTESTR: case TE_FN:
    case TE_CONST_PARAM:
        fatal(sp, "internal: constant expression was not lowered as a constant");
    case TE_CONST:
        emit_use(b, dest, op_copy(const_global_place(b, e)), sp);
        return;
    case TE_LOCAL: case TE_STATIC: case TE_FIELD: case TE_INDEX: case TE_DEREF:
        emit_use(b, dest, op_use(lower_place(b, e)), sp);
        return;
    case TE_UNARY: {
        Rvalue rv = rv_new(RV_UNARY, dest.ty);
        rv.op = e->op;
        rv.a = lower_operand(b, e->a);
        emit_assign(b, dest, rv, sp);
        return;
    }
    case TE_BINARY: {
        Rvalue rv = rv_new(RV_BINARY, dest.ty);
        rv.op = e->op;
        rv.a = lower_operand(b, e->a);
        /* keep the left value stable while the right side is evaluated */
        if (rv.a.kind != OP_CONST && !(e->b->kind == TE_LIT_INT || e->b->kind == TE_LOCAL)) {
            Place t = new_temp(b, rv.a.place.ty);
            emit_use(b, t, rv.a, sp);
            rv.a = op_copy(t);
        }
        rv.b = lower_operand(b, e->b);
        emit_assign(b, dest, rv, sp);
        return;
    }
    case TE_LOGICAL: {
        BlockId rhs = mir_new_block(b->mir), sc = mir_new_block(b->mir), end = mir_new_block(b->mir);
        Operand a = lower_operand(b, e->a);
        if (e->op == BIN_AND)
            term_if(b, a, rhs, sc);
        else
            term_if(b, a, sc, rhs);
        b->cur = sc;
        emit_use(b, dest, mir_const_int(ty_bool(), e->op == BIN_OR, 0), sp);
        term_goto(b, end);
        b->cur = rhs;
        b->cond_depth++;
        uint32_t saved_temp = b->temp_scope;
        uint32_t rs = push_scope(b);
        b->temp_scope = rs;
        expr_into(b, dest, e->b);
        pop_scope(b, sp);
        b->temp_scope = saved_temp;
        b->cond_depth--;
        term_goto(b, end);
        b->cur = end;
        return;
    }
    case TE_ASSIGN: {
        Type *vt = T(b, e->b->ty);
        Operand v = lower_operand(b, e->b);
        if (v.kind != OP_CONST && is_place_expr(e->b)) {
            /* read the source before the destination is dropped */
            Place t = new_temp(b, vt);
            emit_use(b, t, v, sp);
            v = op_use(t);
        }
        Place p = lower_place(b, e->a);
        if (ty_needs_drop(p.ty))
            emit_drop(b, p, sp);
        emit_use(b, p, v, sp);
        if (ty_is_unit(dest.ty))
            emit_use(b, dest, op_zst(dest.ty), sp);
        return;
    }
    case TE_ASSIGN_OP: {
        Operand v = lower_operand(b, e->b);
        if (v.kind != OP_CONST) {
            Place t = new_temp(b, v.place.ty);
            emit_use(b, t, v, sp);
            v = op_copy(t);
        }
        Place p = lower_place(b, e->a);
        Rvalue rv = rv_new(RV_BINARY, p.ty);
        rv.op = e->op;
        rv.a = op_copy(p);
        rv.b = v;
        emit_assign(b, p, rv, sp);
        if (ty_is_unit(dest.ty))
            emit_use(b, dest, op_zst(dest.ty), sp);
        return;
    }
    case TE_CAST:
        lower_cast(b, dest, e);
        return;
    case TE_CALL:
        lower_call(b, dest, e);
        return;
    case TE_REF: case TE_ADDR_OF:
        lower_ref(b, dest, e);
        return;
    case TE_ADT:
        lower_adt(b, dest, e);
        return;
    case TE_TUPLE: case TE_ARRAY: {
        Rvalue rv = rv_new(RV_AGGREGATE, dest.ty);
        rv.agg = e->kind == TE_TUPLE ? AGG_TUPLE : AGG_ARRAY;
        rv.nops = e->nargs;
        rv.ops = NEW_ARRAY(Operand, e->nargs ? e->nargs : 1);
        for (uint32_t i = 0; i < e->nargs; i++)
            rv.ops[i] = lower_operand(b, e->args[i]);
        emit_assign(b, dest, rv, sp);
        return;
    }
    case TE_REPEAT: {
        Rvalue rv = rv_new(RV_REPEAT, dest.ty);
        rv.a = lower_operand(b, e->a);
        if (dest.ty->len->kind != T_CONST)
            fatal(sp, "internal: array length is not a constant");
        rv.count = dest.ty->len->cval;
        if (rv.a.kind == OP_MOVE && rv.count != 1) {
            /* `[expr; N]` of a non-Copy constant expression: N bitwise
               copies are only right if the value owns nothing */
            rv.a.kind = OP_COPY;
            if (rv.count == 0)
                emit_drop(b, rv.a.place, sp);
            else
                emit_forget(b, rv.a.place, sp);
        }
        emit_assign(b, dest, rv, sp);
        return;
    }
    case TE_CLOSURE:
        lower_closure_expr(b, dest, e);
        return;
    case TE_BLOCK:
        lower_block(b, dest, e);
        return;
    case TE_IF:
        lower_if(b, dest, e);
        return;
    case TE_LET: {
        BlockId t = mir_new_block(b->mir), f = mir_new_block(b->mir), end = mir_new_block(b->mir);
        cond_branch(b, e, t, f, b->nscopes - 1);
        b->cur = t;
        emit_use(b, dest, mir_const_int(ty_bool(), 1, 0), sp);
        term_goto(b, end);
        b->cur = f;
        emit_use(b, dest, mir_const_int(ty_bool(), 0, 0), sp);
        term_goto(b, end);
        b->cur = end;
        return;
    }
    case TE_MATCH:
        lower_match(b, dest, e);
        return;
    case TE_LOOP: case TE_WHILE:
        lower_loop(b, dest, e);
        return;
    case TE_BREAK: {
        BreakTarget *t = find_target(b, e->label, sp);
        if (e->a)
            expr_into(b, t->dest, e->a);
        else if (ty_is_unit(t->dest.ty))
            emit_use(b, t->dest, op_zst(t->dest.ty), sp);
        exit_scopes(b, t->scope_depth, sp);
        term_goto(b, t->brk);
        b->cur = mir_new_block(b->mir);
        return;
    }
    case TE_CONTINUE: {
        BreakTarget *t = find_target(b, e->label, sp);
        exit_scopes(b, t->scope_depth, sp);
        term_goto(b, t->cont);
        b->cur = mir_new_block(b->mir);
        return;
    }
    case TE_RETURN: {
        Place ret = local_place(b, 0);
        if (e->a)
            expr_into(b, ret, e->a);
        else
            emit_use(b, ret, op_zst(ret.ty), sp);
        exit_scopes(b, 0, sp);
        Terminator *t = &b->mir->blocks[b->cur].term;
        if (t->kind == TM_NONE)
            t->kind = TM_RETURN;
        b->cur = mir_new_block(b->mir);
        return;
    }
    case TE_NEVER_TO_ANY: {
        Place never = new_temp(b, ty_never());
        expr_into(b, never, e->a);
        term_unreachable(b);
        return;
    }
    case TE_UNSIZE:
        lower_unsize(b, dest, e);
        return;
    case TE_REIFY: {
        Type *ft = T(b, e->a->ty);
        if (e->a->kind != TE_FN)
            (void)lower_operand(b, e->a);
        CalleeRes cr = mono_resolve_callee(ft->def, ft->substs, sp);
        if (cr.kind != CR_INSTANCE)
            fatal(sp, "taking a fn pointer to `%s` is not supported", def_path_str(ft->def));
        Operand o;
        memset(&o, 0, sizeof o);
        o.kind = OP_CONST;
        o.c.kind = MC_FN;
        o.c.ty = dest.ty;
        o.c.inst = cr.inst;
        emit_use(b, dest, o, sp);
        return;
    }
    case TE_CLOSURE_FNPTR: {
        Type *ct = T(b, e->a->ty);
        (void)lower_operand(b, e->a);
        if (def_get(ct->def)->closure->ncaptures)
            fatal(sp, "only non-capturing closures can be coerced to fn pointers");
        Operand o;
        memset(&o, 0, sizeof o);
        o.kind = OP_CONST;
        o.c.kind = MC_FN;
        o.c.ty = dest.ty;
        o.c.inst = mono_fnptr_shim(ct, sp);
        emit_use(b, dest, o, sp);
        return;
    }
    }
    fatal(sp, "internal: unhandled THIR expression kind %d in MIR building", e->kind);
}

/* ------------------------------------------------------------------ */
/* Instance bodies                                                     */
/* ------------------------------------------------------------------ */

/* ------------------------------------------------------------------ */
/* Switch merging                                                      */
/* ------------------------------------------------------------------ */

static bool place_same(const Place *a, const Place *b) {
    if (a->local != b->local || a->nprojs != b->nprojs)
        return false;
    for (uint32_t i = 0; i < a->nprojs; i++) {
        const Proj *x = &a->projs[i], *y = &b->projs[i];
        if (x->kind != y->kind || x->index != y->index || x->from_end != y->from_end || x->to != y->to ||
            x->local != y->local)
            return false;
    }
    return true;
}

/* What a block that only tests one value switches on: the place whose
   discriminant it reads (is_discr) or the place it compares directly.
   Returns false if the block is not of that shape. */
static bool switch_subject(const MirBody *mir, const MirBlock *blk, bool allow_stmts, Place *subject,
                           bool *is_discr) {
    const Terminator *t = &blk->term;
    if (t->kind != TM_SWITCH || t->discr.kind != OP_COPY)
        return false;
    Type *dt = t->discr.place.ty;
    if (dt->kind == T_BOOL || (dt->kind != T_INT && dt->kind != T_CHAR))
        return false;
    if (blk->nstmts) {
        const MirStmt *s = &blk->stmts[blk->nstmts - 1];
        if (s->kind == ST_ASSIGN && s->rv.kind == RV_DISCR && t->discr.place.nprojs == 0 &&
            s->place.nprojs == 0 && s->place.local == t->discr.place.local &&
            !mir->locals[s->place.local].name) {
            if (!allow_stmts && blk->nstmts != 1)
                return false;
            *subject = s->rv.place;
            *is_discr = true;
            return true;
        }
        if (!allow_stmts)
            return false;
    }
    *subject = t->discr.place;
    *is_discr = false;
    return true;
}

/*
 * Pattern tests are generated arm by arm: `if tag == A goto a; if tag == B
 * goto b; ..`. Fold such a chain of single-value tests of the same place
 * into one multi-way TM_SWITCH (the first test of a value wins, so earlier
 * arms keep their priority). The later blocks of the chain stay in place
 * for any other predecessor.
 */
static void merge_switch_chains(MirBody *mir) {
    for (BlockId b = 0; b < mir->nblocks; b++) {
        MirBlock *blk = &mir->blocks[b];
        Place subject;
        bool is_discr;
        if (!switch_subject(mir, blk, true, &subject, &is_discr))
            continue;
        Terminator *t = &blk->term;
        VEC(ConstVal) values = {0};
        VEC(BlockId) targets = {0};
        for (uint32_t i = 0; i < t->ntargets; i++) {
            vec_push(values, t->values[i]);
            vec_push(targets, t->targets[i]);
        }
        BlockId other = t->target;
        uint32_t steps = 0;
        bool changed = false;
        while (steps++ < 4096 && other != b) {
            const MirBlock *nb = &mir->blocks[other];
            Place ns;
            bool nd;
            if (!switch_subject(mir, nb, false, &ns, &nd) || nd != is_discr || !place_same(&ns, &subject))
                break;
            const Terminator *nt = &nb->term;
            for (uint32_t i = 0; i < nt->ntargets; i++) {
                bool dup = false;
                for (uint32_t k = 0; k < values.len && !dup; k++)
                    dup = values.data[k].lo == nt->values[i].lo && values.data[k].hi == nt->values[i].hi;
                if (!dup) {
                    vec_push(values, nt->values[i]);
                    vec_push(targets, nt->targets[i]);
                }
            }
            other = nt->target;
            changed = true;
        }
        if (!changed)
            continue;
        t->values = values.data;
        t->targets = targets.data;
        t->ntargets = values.len;
        t->target = other;
    }
}

static void builder_init(Builder *b, Instance *inst, Body *body, Substs *substs) {
    memset(b, 0, sizeof *b);
    b->inst = inst;
    b->mir = NEW(MirBody);
    b->mir->inst = inst;
    b->body = body;
    b->substs = substs;
    b->span = inst->span;
    if (body)
        b->vars = NEW_ARRAY(LocalId, body->nlocals ? body->nlocals : 1);
    mir_new_local(b->mir, inst->ret, 0);
    for (uint32_t i = 0; i < inst->nparams; i++) {
        LocalId l = mir_new_local(b->mir, inst->params[i], 0);
        b->mir->locals[l].is_arg = true;
    }
    b->mir->nargs = inst->nparams;
    b->cur = mir_new_block(b->mir);
    push_scope(b);
    b->temp_scope = 0;
}

static void builder_finish(Builder *b) {
    pop_scope(b, b->inst->span);
    Terminator *t = &b->mir->blocks[b->cur].term;
    if (t->kind == TM_NONE)
        t->kind = TM_RETURN;
    for (uint32_t i = 0; i < b->mir->nblocks; i++)
        if (b->mir->blocks[i].term.kind == TM_NONE)
            b->mir->blocks[i].term.kind = TM_UNREACHABLE;
    merge_switch_chains(b->mir);
    b->inst->mir = b->mir;
}

/* Bind a parameter pattern to an argument place. */
static void bind_param(Builder *b, const TPat *p, Place arg) {
    if (p->kind == TP_BIND && p->bind_mode == BIND_VALUE && !p->sub && arg.nprojs == 0) {
        b->vars[p->local->id] = arg.local;
        b->mir->locals[arg.local].name = p->local->name;
        schedule_drop(b, 0, arg.local);
        return;
    }
    if (arg.nprojs == 0)
        schedule_drop(b, 0, arg.local);
    BlockId fail = mir_new_block(b->mir);
    pat_test(b, p, arg, fail);
    pat_bind(b, p, arg, true, 0);
    b->mir->blocks[fail].term.kind = TM_UNREACHABLE;
}

void mir_build_instance(Instance *inst) {
    Builder b;
    Span sp = inst->span;
    switch (inst->kind) {
    case INST_FN: {
        Body *body = typeck_body(inst->def);
        if (!body)
            fatal(sp, "internal: no body for `%s`", def_path_str(inst->def));
        builder_init(&b, inst, body, inst->substs);
        for (uint32_t i = 0; i < body->nparams; i++)
            bind_param(&b, body->params[i], local_place(&b, i + 1));
        expr_into(&b, local_place(&b, 0), body->value);
        builder_finish(&b);
        return;
    }
    case INST_CLOSURE: {
        ClosureInfo *ci = def_get(inst->ty->def)->closure;
        DefKind ok = ci->owner ? def_get(ci->owner)->kind : DEF_NONE;
        Body *body = (ok == DEF_FN || ok == DEF_CONST || ok == DEF_ASSOC_CONST || ok == DEF_STATIC)
                         ? typeck_body(ci->owner) : NULL;
        if (!body) {
            /* closure inside an anonymous constant: locals are private */
            body = NEW(Body);
            body->nlocals = 4096;
        }
        builder_init(&b, inst, body, inst->substs);
        b.closure = ci;
        b.closure_ty = inst->ty;
        if (ci->kind == CK_FN_ONCE)
            schedule_drop(&b, 0, 1);
        Place tuple = local_place(&b, 2);
        schedule_drop(&b, 0, 2);
        for (uint32_t i = 0; i < ci->nparams; i++) {
            Place f = mir_place_proj(tuple, PJ_FIELD, i, inst->params[1]->substs->t[i]);
            bind_param(&b, ci->params[i], f);
        }
        expr_into(&b, local_place(&b, 0), ci->body);
        builder_finish(&b);
        return;
    }
    case INST_CLOSURE_ONCE: {
        builder_init(&b, inst, NULL, inst->substs);
        ClosureInfo *ci = def_get(inst->ty->def)->closure;
        schedule_drop(&b, 0, 1);
        Place r = new_temp(&b, ty_ref(inst->ty, ci->kind == CK_FN_MUT));
        Rvalue rv = rv_new(RV_REF, r.ty);
        rv.place = local_place(&b, 1);
        emit_assign(&b, r, rv, sp);
        Operand *ops = NEW_ARRAY(Operand, 2);
        ops[0] = op_copy(r);
        ops[1] = op_use(local_place(&b, 2));
        MirStmt *s = emit_call_stmt(&b, local_place(&b, 0), CALLEE_DIRECT, ops, 2, sp);
        s->inst = mono_closure_instance(inst->ty, sp);
        finish_call(&b, local_place(&b, 0));
        builder_finish(&b);
        return;
    }
    case INST_FNPTR_SHIM: {
        builder_init(&b, inst, NULL, inst->substs);
        ClosureInfo *ci = def_get(inst->ty->def)->closure;
        Instance *body = mono_closure_instance(inst->ty, sp);
        Place env = new_temp(&b, inst->ty);
        Rvalue ev = rv_new(RV_AGGREGATE, inst->ty);
        ev.agg = AGG_CLOSURE;
        emit_assign(&b, env, ev, sp);
        Operand *ops = NEW_ARRAY(Operand, 2);
        if (ci->kind == CK_FN_ONCE) {
            ops[0] = op_copy(env);
        } else {
            Place r = new_temp(&b, body->params[0]);
            Rvalue rv = rv_new(RV_REF, r.ty);
            rv.place = env;
            emit_assign(&b, r, rv, sp);
            ops[0] = op_copy(r);
        }
        Type *tuple_ty = body->params[1];
        if (inst->nparams == 0) {
            ops[1] = op_zst(tuple_ty);
        } else {
            Place tp = new_temp(&b, tuple_ty);
            Rvalue rv = rv_new(RV_AGGREGATE, tuple_ty);
            rv.agg = AGG_TUPLE;
            rv.nops = inst->nparams;
            rv.ops = NEW_ARRAY(Operand, inst->nparams);
            for (uint32_t i = 0; i < inst->nparams; i++)
                rv.ops[i] = op_use(local_place(&b, i + 1));
            emit_assign(&b, tp, rv, sp);
            ops[1] = op_move(tp);
        }
        MirStmt *s = emit_call_stmt(&b, local_place(&b, 0), CALLEE_DIRECT, ops, 2, sp);
        s->inst = body;
        finish_call(&b, local_place(&b, 0));
        builder_finish(&b);
        return;
    }
    case INST_FN_TRAIT_SHIM: {
        builder_init(&b, inst, NULL, inst->substs);
        CalleeRes cr;
        memset(&cr, 0, sizeof cr);
        cr.kind = CR_FN_UNTUPLED;
        cr.self_ty = inst->ty;
        Operand *ops = NEW_ARRAY(Operand, 2);
        ops[0] = op_copy(local_place(&b, 1));
        ops[1] = op_use(local_place(&b, 2));
        static const char *names[3] = {"call", "call_mut", "call_once"};
        DefId traits[3] = {lang.fn, lang.fn_mut, lang.fn_once};
        DefId method = trait_item(traits[inst->shim_self], sym_intern_cstr(names[inst->shim_self]));
        emit_call(&b, local_place(&b, 0), method, cr, ops, 2, sp);
        builder_finish(&b);
        return;
    }
    case INST_BOX_ONCE_SHIM: {
        builder_init(&b, inst, NULL, inst->substs);
        Type *ct = inst->ty;
        schedule_drop(&b, 0, 1);
        Place val = new_temp(&b, ct);
        emit_use(&b, val, op_use(mir_place_proj(local_place(&b, 1), PJ_DEREF, 0, ct)), sp);
        schedule_drop(&b, 0, val.local);
        CalleeRes cr;
        memset(&cr, 0, sizeof cr);
        cr.kind = ct->kind == T_CLOSURE ? CR_CLOSURE : CR_FN_UNTUPLED;
        cr.self_ty = ct;
        Operand *ops = NEW_ARRAY(Operand, 2);
        ops[0] = op_use(val);
        ops[1] = op_use(local_place(&b, 2));
        emit_call(&b, local_place(&b, 0), trait_item(lang.fn_once, sym_intern_cstr("call_once")), cr,
                  ops, 2, sp);
        builder_finish(&b);
        return;
    }
    case INST_GLOBAL_INIT: {
        Global *g = inst->global;
        if (g->kind == GLOBAL_PROMOTED) {
            Body *body = g->body;
            builder_init(&b, inst, body, g->body_substs);
            if (g->closure) {
                b.closure = def_get(g->closure)->closure;
            }
            b.in_global_init = true;
            expr_into(&b, local_place(&b, 0), g->expr);
        } else {
            Body *body = typeck_body(g->def);
            if (!body)
                fatal(sp, "internal: global `%s` has no initializer", def_path_str(g->def));
            builder_init(&b, inst, body, g->substs);
            b.in_global_init = true;
            expr_into(&b, local_place(&b, 0), body->value);
        }
        builder_finish(&b);
        return;
    }
    case INST_EXTERN:
        return;
    }
}
