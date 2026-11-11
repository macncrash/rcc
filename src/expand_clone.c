/*
 * expand_clone.c - deep copies of AST nodes with fresh NodeIds.
 *
 * Used when a macro_rules fragment ($e:expr etc.) is transcribed: the same
 * parsed node may be pasted several times, and every paste must be a
 * distinct tree with its own ids.
 */
#include "expand_int.h"

static GenericArgs *c_gargs(const GenericArgs *a);
static BoundList c_bounds(BoundList l);
static GenericParamList c_gparams(GenericParamList l);
static MacCall *c_mac(const MacCall *m);
static UseTree *c_use(const UseTree *u);

#define CLONE_LIST(T, list, fn)                                                \
    do {                                                                       \
        T out__ = {0};                                                         \
        for (uint32_t i__ = 0; i__ < (list).len; i__++)                        \
            vec_push(out__, fn((list).data[i__]));                             \
        (list) = out__;                                                        \
    } while (0)

TokenList clone_tokens(TokenList tl) {
    TokenList out = {NULL, tl.len};
    if (!tl.len)
        return out;
    out.toks = NEW_ARRAY(Token, tl.len);
    for (uint32_t i = 0; i < tl.len; i++) {
        out.toks[i] = tl.toks[i];
        if (tl.toks[i].kind == TK_INTERP)
            out.toks[i].v.node = clone_nt((NtKind)tl.toks[i].nt_kind, tl.toks[i].v.node);
    }
    return out;
}

Attr *clone_attr(const Attr *a) {
    Attr *n = NEW(Attr);
    *n = *a;
    n->id = ast_fresh_id();
    n->path = clone_path(a->path);
    n->tokens = clone_tokens(a->tokens);
    if (a->value)
        n->value = clone_expr(a->value);
    return n;
}

static AttrList c_attrs(AttrList l) {
    CLONE_LIST(AttrList, l, clone_attr);
    return l;
}

static PathSegment *c_seg(const PathSegment *s) {
    PathSegment *n = NEW(PathSegment);
    *n = *s;
    n->id = ast_fresh_id();
    if (s->args)
        n->args = c_gargs(s->args);
    return n;
}

Path *clone_path(const Path *p) {
    if (!p)
        return NULL;
    Path *n = NEW(Path);
    *n = *p;
    n->id = ast_fresh_id();
    CLONE_LIST(PathSegList, n->segs, c_seg);
    return n;
}

static QSelf *c_qself(const QSelf *q) {
    if (!q)
        return NULL;
    QSelf *n = NEW(QSelf);
    *n = *q;
    n->ty = clone_ty(q->ty);
    return n;
}

static GenericArg *c_garg(const GenericArg *a) {
    GenericArg *n = NEW(GenericArg);
    *n = *a;
    if (a->ty)
        n->ty = clone_ty(a->ty);
    if (a->expr)
        n->expr = clone_expr(a->expr);
    if (a->name_args)
        n->name_args = c_gargs(a->name_args);
    n->bounds = c_bounds(a->bounds);
    return n;
}

static GenericArgs *c_gargs(const GenericArgs *a) {
    GenericArgs *n = NEW(GenericArgs);
    *n = *a;
    CLONE_LIST(GenericArgList, n->args, c_garg);
    CLONE_LIST(TyList, n->inputs, clone_ty);
    if (a->output)
        n->output = clone_ty(a->output);
    return n;
}

static GenericBound *c_bound(const GenericBound *b) {
    GenericBound *n = NEW(GenericBound);
    *n = *b;
    if (b->path)
        n->path = clone_path(b->path);
    n->for_lifetimes = c_gparams(b->for_lifetimes);
    return n;
}

static BoundList c_bounds(BoundList l) {
    CLONE_LIST(BoundList, l, c_bound);
    return l;
}

static GenericParam *c_gparam(const GenericParam *g) {
    GenericParam *n = NEW(GenericParam);
    *n = *g;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(g->attrs);
    n->bounds = c_bounds(g->bounds);
    if (g->default_ty)
        n->default_ty = clone_ty(g->default_ty);
    if (g->const_ty)
        n->const_ty = clone_ty(g->const_ty);
    if (g->const_default)
        n->const_default = clone_expr(g->const_default);
    return n;
}

static GenericParamList c_gparams(GenericParamList l) {
    CLONE_LIST(GenericParamList, l, c_gparam);
    return l;
}

static WherePredicate *c_wp(const WherePredicate *w) {
    WherePredicate *n = NEW(WherePredicate);
    *n = *w;
    n->for_lifetimes = c_gparams(w->for_lifetimes);
    if (w->ty)
        n->ty = clone_ty(w->ty);
    n->bounds = c_bounds(w->bounds);
    return n;
}

Generics clone_generics(const Generics *g) {
    Generics n = *g;
    n.params = c_gparams(g->params);
    CLONE_LIST(WherePredList, n.where, c_wp);
    return n;
}

static MacCall *c_mac(const MacCall *m) {
    MacCall *n = NEW(MacCall);
    *n = *m;
    n->id = ast_fresh_id();
    n->path = clone_path(m->path);
    n->tokens = clone_tokens(m->tokens);
    if (ex_is_frozen(m))
        ex_freeze(n);
    return n;
}

static FnPtrParam *c_fnparam(const FnPtrParam *p) {
    FnPtrParam *n = NEW(FnPtrParam);
    *n = *p;
    n->ty = clone_ty(p->ty);
    return n;
}

Ty *clone_ty(const Ty *t) {
    if (!t)
        return NULL;
    Ty *n = NEW(Ty);
    *n = *t;
    n->id = ast_fresh_id();
    switch (t->kind) {
    case TY_PATH:
        n->path.qself = c_qself(t->path.qself);
        n->path.path = clone_path(t->path.path);
        break;
    case TY_REF: n->ref.elem = clone_ty(t->ref.elem); break;
    case TY_PTR: n->ptr.elem = clone_ty(t->ptr.elem); break;
    case TY_SLICE: n->slice.elem = clone_ty(t->slice.elem); break;
    case TY_ARRAY:
        n->array.elem = clone_ty(t->array.elem);
        n->array.len = clone_expr(t->array.len);
        break;
    case TY_TUPLE: CLONE_LIST(TyList, n->tuple.elems, clone_ty); break;
    case TY_FN:
        n->fn_.for_lifetimes = c_gparams(t->fn_.for_lifetimes);
        CLONE_LIST(FnPtrParamList, n->fn_.params, c_fnparam);
        n->fn_.ret = clone_ty(t->fn_.ret);
        break;
    case TY_DYN:
    case TY_IMPL: n->bounds = c_bounds(t->bounds); break;
    case TY_MAC: n->mac = c_mac(t->mac); break;
    case TY_NEVER:
    case TY_INFER: break;
    }
    return n;
}

static PatField *c_patfield(const PatField *f) {
    PatField *n = NEW(PatField);
    *n = *f;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(f->attrs);
    n->pat = clone_pat(f->pat);
    return n;
}

Pat *clone_pat(const Pat *p) {
    if (!p)
        return NULL;
    Pat *n = NEW(Pat);
    *n = *p;
    n->id = ast_fresh_id();
    switch (p->kind) {
    case PAT_IDENT: n->ident.sub = clone_pat(p->ident.sub); break;
    case PAT_LIT: n->lit = clone_expr(p->lit); break;
    case PAT_RANGE:
        n->range.lo = clone_expr(p->range.lo);
        n->range.hi = clone_expr(p->range.hi);
        break;
    case PAT_PATH:
        n->path.qself = c_qself(p->path.qself);
        n->path.path = clone_path(p->path.path);
        break;
    case PAT_TUPLE:
    case PAT_SLICE:
    case PAT_OR: CLONE_LIST(PatList, n->elems, clone_pat); break;
    case PAT_TUPLE_STRUCT:
        n->tuple_struct.qself = c_qself(p->tuple_struct.qself);
        n->tuple_struct.path = clone_path(p->tuple_struct.path);
        CLONE_LIST(PatList, n->tuple_struct.elems, clone_pat);
        break;
    case PAT_STRUCT:
        n->struct_.qself = c_qself(p->struct_.qself);
        n->struct_.path = clone_path(p->struct_.path);
        CLONE_LIST(PatFieldList, n->struct_.fields, c_patfield);
        break;
    case PAT_REF: n->ref.pat = clone_pat(p->ref.pat); break;
    case PAT_MAC: n->mac = c_mac(p->mac); break;
    case PAT_WILD:
    case PAT_REST: break;
    }
    return n;
}

static Arm *c_arm(const Arm *a) {
    Arm *n = NEW(Arm);
    *n = *a;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(a->attrs);
    n->pat = clone_pat(a->pat);
    n->guard = clone_expr(a->guard);
    n->body = clone_expr(a->body);
    return n;
}

static ExprField *c_exprfield(const ExprField *f) {
    ExprField *n = NEW(ExprField);
    *n = *f;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(f->attrs);
    n->expr = clone_expr(f->expr);
    return n;
}

static Param *c_param(const Param *p) {
    Param *n = NEW(Param);
    *n = *p;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(p->attrs);
    n->pat = clone_pat(p->pat);
    n->ty = clone_ty(p->ty);
    return n;
}

Expr *clone_expr(const Expr *e) {
    if (!e)
        return NULL;
    Expr *n = NEW(Expr);
    *n = *e;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(e->attrs);
    switch (e->kind) {
    case EXPR_LIT: break; /* Lit is immutable, share it */
    case EXPR_PATH:
        n->path.qself = c_qself(e->path.qself);
        n->path.path = clone_path(e->path.path);
        break;
    case EXPR_UNARY: n->unary.operand = clone_expr(e->unary.operand); break;
    case EXPR_ADDR_OF: n->addr_of.operand = clone_expr(e->addr_of.operand); break;
    case EXPR_BINARY:
        n->binary.lhs = clone_expr(e->binary.lhs);
        n->binary.rhs = clone_expr(e->binary.rhs);
        break;
    case EXPR_ASSIGN:
    case EXPR_ASSIGN_OP:
        n->assign.lhs = clone_expr(e->assign.lhs);
        n->assign.rhs = clone_expr(e->assign.rhs);
        break;
    case EXPR_CAST:
        n->cast.expr = clone_expr(e->cast.expr);
        n->cast.ty = clone_ty(e->cast.ty);
        break;
    case EXPR_CALL:
        n->call.callee = clone_expr(e->call.callee);
        CLONE_LIST(ExprList, n->call.args, clone_expr);
        break;
    case EXPR_METHOD_CALL:
        n->method_call.receiver = clone_expr(e->method_call.receiver);
        n->method_call.method = c_seg(e->method_call.method);
        CLONE_LIST(ExprList, n->method_call.args, clone_expr);
        break;
    case EXPR_FIELD: n->field.base = clone_expr(e->field.base); break;
    case EXPR_INDEX:
        n->index.base = clone_expr(e->index.base);
        n->index.index = clone_expr(e->index.index);
        break;
    case EXPR_STRUCT:
        n->struct_.qself = c_qself(e->struct_.qself);
        n->struct_.path = clone_path(e->struct_.path);
        CLONE_LIST(ExprFieldList, n->struct_.fields, c_exprfield);
        n->struct_.base = clone_expr(e->struct_.base);
        break;
    case EXPR_TUPLE:
    case EXPR_ARRAY: CLONE_LIST(ExprList, n->elems, clone_expr); break;
    case EXPR_REPEAT:
        n->repeat.elem = clone_expr(e->repeat.elem);
        n->repeat.count = clone_expr(e->repeat.count);
        break;
    case EXPR_RANGE:
        n->range.lo = clone_expr(e->range.lo);
        n->range.hi = clone_expr(e->range.hi);
        break;
    case EXPR_CLOSURE:
        CLONE_LIST(ParamList, n->closure.params, c_param);
        n->closure.ret = clone_ty(e->closure.ret);
        n->closure.body = clone_expr(e->closure.body);
        break;
    case EXPR_BLOCK: n->block.block = clone_block(e->block.block); break;
    case EXPR_IF:
        n->if_.cond = clone_expr(e->if_.cond);
        n->if_.then = clone_block(e->if_.then);
        n->if_.els = clone_expr(e->if_.els);
        break;
    case EXPR_LET:
        n->let.pat = clone_pat(e->let.pat);
        n->let.scrutinee = clone_expr(e->let.scrutinee);
        break;
    case EXPR_MATCH:
        n->match.scrutinee = clone_expr(e->match.scrutinee);
        CLONE_LIST(ArmList, n->match.arms, c_arm);
        break;
    case EXPR_WHILE:
        n->while_.cond = clone_expr(e->while_.cond);
        n->while_.body = clone_block(e->while_.body);
        break;
    case EXPR_FOR:
        n->for_.pat = clone_pat(e->for_.pat);
        n->for_.iter = clone_expr(e->for_.iter);
        n->for_.body = clone_block(e->for_.body);
        break;
    case EXPR_LOOP: n->loop.body = clone_block(e->loop.body); break;
    case EXPR_BREAK: n->break_.value = clone_expr(e->break_.value); break;
    case EXPR_CONTINUE: break;
    case EXPR_RETURN: n->ret = clone_expr(e->ret); break;
    case EXPR_TRY: n->try_ = clone_expr(e->try_); break;
    case EXPR_MAC: n->mac = c_mac(e->mac); break;
    case EXPR_UNDERSCORE: break;
    }
    return n;
}

Stmt *clone_stmt(const Stmt *s) {
    Stmt *n = NEW(Stmt);
    *n = *s;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(s->attrs);
    switch (s->kind) {
    case STMT_LET:
        n->let.pat = clone_pat(s->let.pat);
        n->let.ty = clone_ty(s->let.ty);
        n->let.init = clone_expr(s->let.init);
        n->let.els = s->let.els ? clone_block(s->let.els) : NULL;
        break;
    case STMT_ITEM: n->item = clone_item(s->item); break;
    case STMT_EXPR: n->expr.expr = clone_expr(s->expr.expr); break;
    case STMT_MAC: n->mac.mac = c_mac(s->mac.mac); break;
    case STMT_EMPTY: break;
    }
    return n;
}

Block *clone_block(const Block *b) {
    if (!b)
        return NULL;
    Block *n = NEW(Block);
    *n = *b;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(b->attrs);
    CLONE_LIST(StmtList, n->stmts, clone_stmt);
    return n;
}

Visibility clone_vis(Visibility v) {
    if (v.path)
        v.path = clone_path(v.path);
    return v;
}

static FieldDef *c_field(const FieldDef *f) {
    FieldDef *n = NEW(FieldDef);
    *n = *f;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(f->attrs);
    n->vis = clone_vis(f->vis);
    n->ty = clone_ty(f->ty);
    return n;
}

static VariantData c_vdata(VariantData d) {
    CLONE_LIST(FieldDefList, d.fields, c_field);
    return d;
}

static Variant *c_variant(const Variant *v) {
    Variant *n = NEW(Variant);
    *n = *v;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(v->attrs);
    n->data = c_vdata(v->data);
    n->disc = clone_expr(v->disc);
    return n;
}

static UseTree *c_use(const UseTree *u) {
    UseTree *n = NEW(UseTree);
    *n = *u;
    n->id = ast_fresh_id();
    n->prefix = clone_path(u->prefix);
    CLONE_LIST(UseTreeList, n->items, c_use);
    return n;
}

static SelfParam *c_selfp(const SelfParam *s) {
    if (!s)
        return NULL;
    SelfParam *n = NEW(SelfParam);
    *n = *s;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(s->attrs);
    n->ty = clone_ty(s->ty);
    return n;
}

Item *clone_item(const Item *it) {
    Item *n = NEW(Item);
    *n = *it;
    n->id = ast_fresh_id();
    n->attrs = c_attrs(it->attrs);
    n->vis = clone_vis(it->vis);
    switch (it->kind) {
    case ITEM_USE: n->use_tree = c_use(it->use_tree); break;
    case ITEM_EXTERN_CRATE: break;
    case ITEM_MOD: CLONE_LIST(ItemList, n->mod.items, clone_item); break;
    case ITEM_FN:
        n->fn_.sig.self_param = c_selfp(it->fn_.sig.self_param);
        CLONE_LIST(ParamList, n->fn_.sig.params, c_param);
        n->fn_.sig.ret = clone_ty(it->fn_.sig.ret);
        n->fn_.generics = clone_generics(&it->fn_.generics);
        n->fn_.body = clone_block(it->fn_.body);
        break;
    case ITEM_STRUCT:
    case ITEM_UNION:
        n->adt.generics = clone_generics(&it->adt.generics);
        n->adt.data = c_vdata(it->adt.data);
        break;
    case ITEM_ENUM:
        n->enum_.generics = clone_generics(&it->enum_.generics);
        CLONE_LIST(VariantList, n->enum_.variants, c_variant);
        break;
    case ITEM_TRAIT:
        n->trait.generics = clone_generics(&it->trait.generics);
        n->trait.supertraits = c_bounds(it->trait.supertraits);
        CLONE_LIST(ItemList, n->trait.items, clone_item);
        break;
    case ITEM_IMPL:
        n->impl.generics = clone_generics(&it->impl.generics);
        n->impl.trait_path = clone_path(it->impl.trait_path);
        n->impl.self_ty = clone_ty(it->impl.self_ty);
        CLONE_LIST(ItemList, n->impl.items, clone_item);
        break;
    case ITEM_TYPE_ALIAS:
        n->type_alias.generics = clone_generics(&it->type_alias.generics);
        n->type_alias.bounds = c_bounds(it->type_alias.bounds);
        n->type_alias.ty = clone_ty(it->type_alias.ty);
        break;
    case ITEM_CONST:
        n->const_.generics = clone_generics(&it->const_.generics);
        n->const_.ty = clone_ty(it->const_.ty);
        n->const_.expr = clone_expr(it->const_.expr);
        break;
    case ITEM_STATIC:
        n->static_.ty = clone_ty(it->static_.ty);
        n->static_.expr = clone_expr(it->static_.expr);
        break;
    case ITEM_FOREIGN_MOD:
        CLONE_LIST(ItemList, n->foreign_mod.items, clone_item);
        break;
    case ITEM_MAC: n->mac = c_mac(it->mac); break;
    case ITEM_MACRO_RULES:
        n->macro_rules.body = clone_tokens(it->macro_rules.body);
        break;
    }
    return n;
}

void *clone_nt(NtKind k, const void *node) {
    switch (k) {
    case NT_EXPR:
    case NT_LIT: return clone_expr((const Expr *)node);
    case NT_TY: return clone_ty((const Ty *)node);
    case NT_PAT: return clone_pat((const Pat *)node);
    case NT_STMT: return clone_stmt((const Stmt *)node);
    case NT_ITEM: return clone_item((const Item *)node);
    case NT_BLOCK: return clone_block((const Block *)node);
    case NT_PATH: return clone_path((const Path *)node);
    case NT_VIS: {
        Visibility *v = NEW(Visibility);
        *v = clone_vis(*(const Visibility *)node);
        return v;
    }
    }
    return NULL;
}
