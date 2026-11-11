/*
 * typeck_collect.c - item signatures and AST type lowering.
 *
 * Every query here is lazy and memoized in the Def. See typeck.h.
 */
#include "typeck.h"

#include <stdio.h>

LangItems lang;

void pred_push(PredList *l, Predicate p) {
    if (l->len == l->cap) {
        uint32_t nc = l->cap ? l->cap * 2 : 4;
        Predicate *n = NEW_ARRAY(Predicate, nc);
        if (l->len)
            memcpy(n, l->data, sizeof(Predicate) * l->len);
        l->data = n;
        l->cap = nc;
    }
    l->data[l->len++] = p;
}

static Sym sym_self_upper(void) { return (Sym)KW_SELF_TYPE; }

/* The AST generics of an item, or NULL. */
static Generics *ast_generics_of(Def *d) {
    Item *it = d->item;
    if (!it)
        return NULL;
    switch (it->kind) {
    case ITEM_FN: return &it->fn_.generics;
    case ITEM_STRUCT: case ITEM_UNION: return &it->adt.generics;
    case ITEM_ENUM: return &it->enum_.generics;
    case ITEM_TRAIT: return &it->trait.generics;
    case ITEM_IMPL: return &it->impl.generics;
    case ITEM_TYPE_ALIAS: return &it->type_alias.generics;
    default: return NULL;
    }
}

/* ------------------------------------------------------------------ */
/* generics_of                                                         */
/* ------------------------------------------------------------------ */

typedef VEC(Ty *) TyVec;

/* Collect argument-position `impl Trait` nodes inside a parameter type. */
static void find_impl_traits(Ty *t, TyVec *out) {
    if (!t)
        return;
    switch (t->kind) {
    case TY_IMPL: vec_push(*out, t); break;
    case TY_REF: find_impl_traits(t->ref.elem, out); break;
    case TY_PTR: find_impl_traits(t->ptr.elem, out); break;
    case TY_SLICE: find_impl_traits(t->slice.elem, out); break;
    case TY_ARRAY: find_impl_traits(t->array.elem, out); break;
    case TY_TUPLE:
        for (uint32_t i = 0; i < t->tuple.elems.len; i++)
            find_impl_traits(t->tuple.elems.data[i], out);
        break;
    case TY_PATH:
        for (uint32_t i = 0; i < t->path.path->segs.len; i++) {
            GenericArgs *ga = t->path.path->segs.data[i]->args;
            if (!ga || ga->kind != GENERIC_ARGS_ANGLE)
                continue;
            for (uint32_t k = 0; k < ga->args.len; k++)
                if (ga->args.data[k]->kind == GA_TYPE)
                    find_impl_traits(ga->args.data[k]->ty, out);
        }
        break;
    default: break;
    }
}

GenericsDef *generics_of(DefId id) {
    Def *d = def_get(id);
    if (d->generics)
        return d->generics;
    switch (d->kind) {
    case DEF_VARIANT: case DEF_CLOSURE: case DEF_TY_PARAM: case DEF_CONST_PARAM:
        return d->generics = generics_of(d->parent);
    default: break;
    }
    GenericsDef *g = NEW(GenericsDef);
    g->owner = id;
    if (d->parent) {
        Def *p = def_get(d->parent);
        if (p->kind == DEF_IMPL || p->kind == DEF_TRAIT) {
            GenericsDef *pg = generics_of(d->parent);
            g->parent = d->parent;
            g->parent_count = pg->count;
            g->has_self = pg->has_self;
        }
    }
    TyVec impls = {0};
    if (d->kind == DEF_FN && d->item)
        for (uint32_t i = 0; i < d->item->fn_.sig.params.len; i++)
            find_impl_traits(d->item->fn_.sig.params.data[i]->ty, &impls);
    uint32_t n = d->gparams.len + (d->kind == DEF_TRAIT ? 1 : 0) + impls.len;
    g->own = NEW_ARRAY(GenericParamDef, n ? n : 1);
    uint32_t k = 0;
    if (d->kind == DEF_TRAIT) {
        g->own[k].name = sym_self_upper();
        g->own[k].index = g->parent_count + k;
        k++;
        g->has_self = true;
    }
    for (uint32_t i = 0; i < d->gparams.len; i++) {
        Def *pd = def_get(d->gparams.data[i]);
        g->own[k].name = pd->name;
        g->own[k].def = d->gparams.data[i];
        g->own[k].index = g->parent_count + k;
        g->own[k].is_const = pd->kind == DEF_CONST_PARAM;
        g->own[k].default_ast = pd->gparam->default_ty;
        k++;
    }
    for (uint32_t i = 0; i < impls.len; i++) {
        g->own[k].name = sym_intern_cstr(arena_printf("impl#%u", i));
        g->own[k].index = g->parent_count + k;
        g->own[k].impl_trait = impls.data[i];
        k++;
    }
    g->nown = k;
    g->count = g->parent_count + k;
    return d->generics = g;
}

GenericParamDef *generics_param(GenericsDef *g, uint32_t index) {
    while (index < g->parent_count)
        g = generics_of(g->parent);
    return &g->own[index - g->parent_count];
}

static HMap identity_cache;

Substs *identity_substs(DefId def) {
    GenericsDef *g = generics_of(def);
    Substs *c = hmap_get(&identity_cache, g->owner);
    if (c)
        return c;
    Type **t = NEW_ARRAY(Type *, g->count ? g->count : 1);
    for (uint32_t i = 0; i < g->count; i++)
        t[i] = ty_param(i, generics_param(g, i)->name);
    c = substs_new(t, g->count);
    hmap_put(&identity_cache, g->owner, c);
    return c;
}

static Type *param_ty(DefId param) {
    Def *d = def_get(param);
    GenericsDef *g = generics_of(d->parent);
    for (uint32_t i = 0; i < g->nown; i++)
        if (g->own[i].def == param)
            return ty_param(g->own[i].index, d->name);
    fatal(d->span, "internal: generic parameter not found in its owner");
}

/* ------------------------------------------------------------------ */
/* trait_def / impl_def / adt_def                                      */
/* ------------------------------------------------------------------ */

TraitDef *trait_def(DefId id) {
    Def *d = def_get(id);
    if (d->trait)
        return d->trait;
    if (d->kind != DEF_TRAIT)
        fatal(d->span, "internal: trait_def of non-trait %s", def_path_str(id));
    TraitDef *t = NEW(TraitDef);
    t->def = id;
    for (uint32_t i = 0; i < d->children.len; i++)
        hmap_put(&t->items, def_get(d->children.data[i])->name,
                 (void *)(uintptr_t)d->children.data[i]);
    t->is_auto = d->item->trait.is_auto || attrs_have(&d->item->attrs, "rcc_auto");
    return d->trait = t;
}

DefId trait_item(DefId trait, Sym name) {
    return (DefId)(uintptr_t)hmap_get(&trait_def(trait)->items, name);
}

static void lower_bounds(LowerCtx *lc, Type *self, BoundList *bounds, PredList *out);

static void trait_fill_supers(DefId id) {
    TraitDef *t = trait_def(id);
    if (t->supers_done)
        return;
    t->supers_done = true;
    Def *d = def_get(id);
    LowerCtx lc = {scope_for_def(id), id, NULL, IT_ERROR};
    lower_bounds(&lc, ty_param(0, sym_self_upper()), &d->item->trait.supertraits, &t->supers);
    for (uint32_t i = 0; i < d->children.len; i++) {
        Def *c = def_get(d->children.data[i]);
        if (c->kind != DEF_ASSOC_TY || c->item->type_alias.bounds.len == 0)
            continue;
        Type *proj = ty_proj(d->children.data[i], identity_substs(id));
        lower_bounds(&lc, proj, &c->item->type_alias.bounds, &t->item_bounds);
    }
}

PredList *trait_supers(DefId trait) {
    trait_fill_supers(trait);
    return &trait_def(trait)->supers;
}

PredList *trait_item_bounds(DefId trait) {
    trait_fill_supers(trait);
    return &trait_def(trait)->item_bounds;
}

DefId trait_item_deep(TraitRef tr, Sym name, TraitRef *owner) {
    DefId it = trait_item(tr.trait, name);
    if (it) {
        if (owner)
            *owner = tr;
        return it;
    }
    PredList *sup = trait_supers(tr.trait);
    for (uint32_t i = 0; i < sup->len; i++) {
        if (sup->data[i].kind != PRED_TRAIT)
            continue;
        TraitRef s = {sup->data[i].tr.trait, substs_subst(sup->data[i].tr.substs, tr.substs)};
        it = trait_item_deep(s, name, owner);
        if (it)
            return it;
    }
    return 0;
}

void elaborate_pred(PredList *out, const Predicate *p) {
    if (p->kind != PRED_TRAIT)
        return;
    PredList *sup = trait_supers(p->tr.trait);
    for (uint32_t i = 0; i < sup->len; i++) {
        Predicate s = sup->data[i];
        s.tr.substs = substs_subst(s.tr.substs, p->tr.substs);
        if (s.kind == PRED_PROJ)
            s.ty = ty_subst(s.ty, p->tr.substs);
        bool dup = false;
        for (uint32_t k = 0; k < out->len && !dup; k++)
            dup = out->data[k].kind == s.kind && out->data[k].tr.trait == s.tr.trait &&
                  out->data[k].tr.substs == s.tr.substs && out->data[k].assoc == s.assoc &&
                  out->data[k].ty == s.ty;
        if (dup)
            continue;
        pred_push(out, s);
        elaborate_pred(out, &s);
    }
}

ImplDef *impl_def(DefId id) {
    Def *d = def_get(id);
    if (d->impl)
        return d->impl;
    ImplDef *im = NEW(ImplDef);
    im->def = id;
    d->impl = im;
    for (uint32_t i = 0; i < d->children.len; i++)
        hmap_put(&im->items, def_get(d->children.data[i])->name,
                 (void *)(uintptr_t)d->children.data[i]);
    Item *it = d->item;
    LowerCtx lc = {scope_for_def(id), id, NULL, IT_ERROR};
    im->self_ty = lower_ty(&lc, it->impl.self_ty);
    im->negative = it->impl.negative;
    if (it->impl.trait_path) {
        PathRes r = resolve_path(lc.scope, it->impl.trait_path, NS_TYPE);
        if (r.res.kind != RES_DEF || def_get(r.res.def)->kind != DEF_TRAIT ||
            r.nsegs != it->impl.trait_path->segs.len)
            fatal(it->impl.trait_path->span, "cannot find trait `%s`",
                  sym_str(vec_last(it->impl.trait_path->segs)->name));
        im->of_trait = true;
        im->trait_ref = lower_trait_ref(&lc, r.res.def, vec_last(it->impl.trait_path->segs),
                                        im->self_ty, NULL, it->span);
    }
    return im;
}

DefId impl_item(DefId impl, Sym name) {
    return (DefId)(uintptr_t)hmap_get(&impl_def(impl)->items, name);
}

static void fill_variant(VariantDef *v, DefId def, Sym name, VariantData *data) {
    v->def = def;
    v->name = name;
    v->ctor = (uint8_t)data->kind;
    v->nfields = data->fields.len;
    v->fields = NEW_ARRAY(FieldT, v->nfields ? v->nfields : 1);
    for (uint32_t i = 0; i < v->nfields; i++) {
        FieldDef *f = data->fields.data[i];
        v->fields[i].name = f->name ? f->name : sym_intern_cstr(arena_printf("%u", i));
        v->fields[i].ast = f->ty;
        v->fields[i].is_private = f->vis.kind == VIS_INHERITED;
    }
}

AdtDef *adt_def(DefId id) {
    Def *d = def_get(id);
    if (d->kind == DEF_VARIANT)
        return adt_def(d->parent);
    if (d->adt)
        return d->adt;
    if (d->kind != DEF_STRUCT && d->kind != DEF_ENUM && d->kind != DEF_UNION)
        fatal(d->span, "internal: adt_def of %s", def_path_str(id));
    AdtDef *a = NEW(AdtDef);
    d->adt = a;
    a->def = id;
    Item *it = d->item;
    if (d->kind == DEF_ENUM) {
        a->is_enum = true;
        a->nvariants = it->enum_.variants.len;
        a->variants = NEW_ARRAY(VariantDef, a->nvariants ? a->nvariants : 1);
        for (uint32_t i = 0; i < a->nvariants; i++) {
            Variant *v = it->enum_.variants.data[i];
            fill_variant(&a->variants[i], d->children.data[i], v->name, &v->data);
            a->variants[i].disc_expr = v->disc;
        }
    } else {
        a->is_union = d->kind == DEF_UNION;
        a->nvariants = 1;
        a->variants = NEW(VariantDef);
        fill_variant(&a->variants[0], id, d->name, &it->adt.data);
    }
    for (uint32_t i = 0; i < it->attrs.len; i++) {
        Attr *at = it->attrs.data[i];
        if (at->path->segs.len != 1 || strcmp(sym_str(at->path->segs.data[0]->name), "repr") != 0)
            continue;
        for (uint32_t k = 0; k < at->tokens.len; k++) {
            if (at->tokens.toks[k].kind != TK_IDENT)
                continue;
            const char *s = sym_str(at->tokens.toks[k].v.sym);
            for (int ik = 0; ik < IK_COUNT; ik++)
                if (strcmp(s, ik_name((IntKind)ik)) == 0)
                    a->repr = ty_int((IntKind)ik);
        }
    }
    LowerCtx lc = {scope_for_def(id), id, NULL, IT_ERROR};
    for (uint32_t i = 0; i < a->nvariants; i++)
        for (uint32_t k = 0; k < a->variants[i].nfields; k++)
            a->variants[i].fields[k].ty = lower_ty(&lc, a->variants[i].fields[k].ast);
    a->fields_done = true;
    return a;
}

void adt_compute_discs(AdtDef *a) {
    if (a->discs_done)
        return;
    a->discs_done = true;
    int64_t next = 0;
    for (uint32_t i = 0; i < a->nvariants; i++) {
        VariantDef *v = &a->variants[i];
        if (v->disc_expr) {
            Type *ty = a->repr ? a->repr : ty_int(IK_ISIZE);
            Body *b = typeck_anon_const(scope_for_def(a->def), a->def, v->disc_expr, ty);
            ConstVal cv;
            if (!const_eval(b->value, NULL, &cv))
                fatal(v->disc_expr->span, "cannot evaluate enum discriminant");
            next = (int64_t)cv.lo;
        }
        v->disc = next++;
    }
}

int variant_field_index(const VariantDef *v, Sym name) {
    for (uint32_t i = 0; i < v->nfields; i++)
        if (v->fields[i].name == name)
            return (int)i;
    return -1;
}

/* ------------------------------------------------------------------ */
/* type_of / fn_sig                                                    */
/* ------------------------------------------------------------------ */

enum { QS_TYPE_OF = 1, QS_PREDS = 2 };

Type *type_of(DefId id) {
    Def *d = def_get(id);
    if (d->type_of)
        return d->type_of;
    if (d->query_state & QS_TYPE_OF)
        fatal(d->span, "cycle while computing the type of `%s`", def_path_str(id));
    d->query_state |= QS_TYPE_OF;
    Type *t = NULL;
    LowerCtx lc = {NULL, id, NULL, IT_ERROR};
    switch (d->kind) {
    case DEF_STRUCT: case DEF_ENUM: case DEF_UNION:
        t = ty_adt(id, identity_substs(id));
        break;
    case DEF_VARIANT:
        t = type_of(d->parent);
        break;
    case DEF_TYPE_ALIAS:
        lc.scope = scope_for_def(id);
        if (!d->item->type_alias.ty)
            fatal(d->span, "type alias without a type");
        t = lower_ty(&lc, d->item->type_alias.ty);
        break;
    case DEF_ASSOC_TY:
        if (def_get(d->parent)->kind == DEF_TRAIT) {
            t = ty_proj(id, identity_substs(d->parent));
        } else {
            lc.scope = scope_for_def(id);
            if (!d->item->type_alias.ty)
                fatal(d->span, "associated type without a value");
            t = lower_ty(&lc, d->item->type_alias.ty);
        }
        break;
    case DEF_CONST: case DEF_ASSOC_CONST:
        lc.scope = scope_for_def(id);
        t = lower_ty(&lc, d->item->const_.ty);
        break;
    case DEF_STATIC:
        lc.scope = scope_for_def(id);
        t = lower_ty(&lc, d->item->static_.ty);
        break;
    case DEF_IMPL:
        t = impl_def(id)->self_ty;
        break;
    case DEF_TY_PARAM: case DEF_CONST_PARAM:
        t = param_ty(id);
        break;
    case DEF_FN:
        t = ty_fndef(id, identity_substs(id));
        break;
    default:
        fatal(d->span, "internal: type_of(%s %s)", def_kind_str(d->kind), def_path_str(id));
    }
    d->query_state &= (uint8_t)~QS_TYPE_OF;
    return d->type_of = t;
}

/* The `Self` type visible inside an associated item (or NULL). */
static Type *self_ty_in(DefId item) {
    for (DefId id = item; id; id = def_get(id)->parent) {
        Def *d = def_get(id);
        if (d->kind == DEF_IMPL)
            return type_of(id);
        if (d->kind == DEF_TRAIT)
            return ty_param(0, sym_self_upper());
        if (d->kind == DEF_MOD)
            break;
    }
    return NULL;
}

FnSigT *fn_sig(DefId id) {
    Def *d = def_get(id);
    if (d->sig)
        return d->sig;
    FnSigT *s = NEW(FnSigT);
    if (d->kind == DEF_STRUCT || d->kind == DEF_VARIANT) {
        AdtDef *a = adt_def(id);
        VariantDef *v = d->kind == DEF_VARIANT ? &a->variants[d->index] : &a->variants[0];
        Type **in = NEW_ARRAY(Type *, v->nfields ? v->nfields : 1);
        for (uint32_t i = 0; i < v->nfields; i++)
            in[i] = v->fields[i].ty;
        s->inputs = substs_new(in, v->nfields);
        s->output = type_of(id);
        return d->sig = s;
    }
    if (d->kind != DEF_FN)
        fatal(d->span, "internal: fn_sig of %s", def_path_str(id));
    FnSig *ast = &d->item->fn_.sig;
    LowerCtx lc = {scope_for_def(id), id, NULL, IT_ARG};
    uint32_t n = ast->params.len + (ast->self_param ? 1 : 0);
    Type **in = NEW_ARRAY(Type *, n ? n : 1);
    uint32_t k = 0;
    if (ast->self_param) {
        SelfParam *sp = ast->self_param;
        Type *self = self_ty_in(id);
        if (!self)
            fatal(sp->span, "`self` parameter outside of an impl or trait");
        if (sp->kind == SELF_VALUE)
            in[k++] = self;
        else if (sp->kind == SELF_REF)
            in[k++] = ty_ref(self, sp->is_mut);
        else
            in[k++] = lower_ty(&lc, sp->ty);
        s->has_self = true;
    }
    for (uint32_t i = 0; i < ast->params.len; i++)
        in[k++] = lower_ty(&lc, ast->params.data[i]->ty);
    s->inputs = substs_new(in, n);
    s->variadic = ast->variadic;
    lc.impl_trait = IT_RET;
    s->output = ast->ret ? lower_ty(&lc, ast->ret) : ty_unit();
    return d->sig = s;
}

/* ------------------------------------------------------------------ */
/* predicates_of                                                       */
/* ------------------------------------------------------------------ */

static DefId resolve_trait_path(LowerCtx *lc, Path *p) {
    PathRes r = resolve_path(lc->scope, p, NS_TYPE);
    if (r.res.kind != RES_DEF || def_get(r.res.def)->kind != DEF_TRAIT || r.nsegs != p->segs.len)
        fatal(p->span, "cannot find trait `%s`", sym_str(vec_last(p->segs)->name));
    return r.res.def;
}

static void lower_bounds(LowerCtx *lc, Type *self, BoundList *bounds, PredList *out) {
    for (uint32_t i = 0; i < bounds->len; i++) {
        GenericBound *b = bounds->data[i];
        if (b->kind != BOUND_TRAIT || b->maybe)
            continue;
        DefId trait = resolve_trait_path(lc, b->path);
        PredList extra = {0};
        Predicate p = {PRED_TRAIT, lower_trait_ref(lc, trait, vec_last(b->path->segs), self, &extra,
                                                   b->span), 0, NULL};
        pred_push(out, p);
        for (uint32_t k = 0; k < extra.len; k++)
            pred_push(out, extra.data[k]);
    }
}

PredList *predicates_of(DefId id) {
    Def *d = def_get(id);
    if (d->preds)
        return d->preds; /* possibly still being filled (see lower_assoc_ty_path) */
    switch (d->kind) {
    case DEF_VARIANT: case DEF_CLOSURE: case DEF_TY_PARAM: case DEF_CONST_PARAM:
        return d->preds = predicates_of(d->parent);
    default: break;
    }
    PredList *l = NEW(PredList);
    d->preds = l;
    GenericsDef *g = generics_of(id);
    if (g->parent) {
        PredList *pp = predicates_of(g->parent);
        for (uint32_t i = 0; i < pp->len; i++)
            pred_push(l, pp->data[i]);
    }
    LowerCtx lc = {scope_for_def(id), id, NULL, IT_ERROR};
    if (d->kind == DEF_TRAIT) {
        Predicate self = {PRED_TRAIT, {id, identity_substs(id)}, 0, NULL};
        pred_push(l, self);
    }
    Generics *ag = ast_generics_of(d);
    if (ag) {
        for (uint32_t i = 0; i < d->gparams.len; i++) {
            Def *pd = def_get(d->gparams.data[i]);
            if (pd->kind == DEF_TY_PARAM && pd->gparam->bounds.len)
                lower_bounds(&lc, type_of(d->gparams.data[i]), &pd->gparam->bounds, l);
        }
        for (uint32_t i = 0; i < ag->where.len; i++) {
            WherePredicate *wp = ag->where.data[i];
            if (wp->kind != WP_BOUND)
                continue;
            lower_bounds(&lc, lower_ty(&lc, wp->ty), &wp->bounds, l);
        }
    }
    for (uint32_t i = 0; i < g->nown; i++)
        if (g->own[i].impl_trait)
            lower_bounds(&lc, ty_param(g->own[i].index, g->own[i].name),
                         &g->own[i].impl_trait->bounds, l);
    return l;
}

static HMap env_cache;

PredList *param_env(DefId id) {
    DefId owner = generics_of(id)->owner;
    Def *d = def_get(id);
    if (d->kind == DEF_CLOSURE || d->kind == DEF_VARIANT)
        owner = generics_of(id)->owner;
    else
        owner = id;
    PredList *c = hmap_get(&env_cache, owner);
    if (c)
        return c;
    PredList *base = predicates_of(owner);
    c = NEW(PredList);
    for (uint32_t i = 0; i < base->len; i++)
        pred_push(c, base->data[i]);
    for (uint32_t i = 0; i < base->len; i++)
        elaborate_pred(c, &base->data[i]);
    hmap_put(&env_cache, owner, c);
    return c;
}

/* ------------------------------------------------------------------ */
/* Type lowering                                                       */
/* ------------------------------------------------------------------ */

Type *lower_const_expr(LowerCtx *lc, Expr *e) {
    while (e->kind == EXPR_BLOCK && e->block.block->stmts.len == 1 && block_tail(e->block.block))
        e = block_tail(e->block.block);
    if (e->kind == EXPR_LIT && e->lit->kind == LIT_INT)
        return ty_const(e->lit->int_lo);
    if (e->kind == EXPR_UNDERSCORE && lc->fcx)
        return infer_var(*(InferCtxt **)lc->fcx, INFER_TY, e->span);
    if (e->kind == EXPR_PATH && !e->path.qself && e->path.path->segs.len == 1) {
        Res r = resolve_ident(lc->scope, e->path.path->segs.data[0]->name, NS_VALUE);
        if (r.kind == RES_DEF && def_get(r.def)->kind == DEF_CONST_PARAM)
            return type_of(r.def);
    }
    Body *b = typeck_anon_const(lc->scope, lc->item, e, ty_usize());
    ConstVal cv;
    if (!const_eval(b->value, NULL, &cv))
        fatal(e->span, "cannot evaluate constant expression (array length / const argument)");
    return ty_const(cv.lo);
}

Substs *lower_generic_args(LowerCtx *lc, DefId def, GenericArgs *args, Substs *parent,
                           bool infer_missing, Span sp) {
    GenericsDef *g = generics_of(def);
    uint32_t first = def_get(g->owner)->kind == DEF_TRAIT ? 1 : 0;
    if (g->owner != def && def_get(def)->kind != DEF_VARIANT)
        first = g->nown; /* def shares its parent's generics: nothing own */
    uint32_t total = parent->len + (g->nown - first);
    if (def_get(def)->kind == DEF_VARIANT)
        total = parent->len + g->nown, first = 0;
    Type *small[8];
    Type **t = total <= 8 ? small : NEW_ARRAY(Type *, total);
    memcpy(t, parent->t, sizeof(Type *) * parent->len);
    uint32_t n = parent->len;
    uint32_t ai = 0;
    uint32_t nargs = args && args->kind == GENERIC_ARGS_ANGLE ? args->args.len : 0;
    for (uint32_t i = first; i < g->nown; i++) {
        GenericParamDef *p = &g->own[i];
        GenericArg *a = NULL;
        while (ai < nargs) {
            GenericArg *c = args->args.data[ai];
            if (c->kind == GA_TYPE || c->kind == GA_CONST) {
                a = c;
                ai++;
                break;
            }
            if (c->kind == GA_LIFETIME) {
                ai++;
                continue;
            }
            break; /* bindings come last */
        }
        if (p->impl_trait)
            a = NULL; /* synthetic params cannot be given explicitly */
        Type *ty;
        if (a) {
            if (p->is_const) {
                if (a->kind == GA_CONST) {
                    ty = lower_const_expr(lc, a->expr);
                } else if (a->ty->kind == TY_INFER && lc->fcx) {
                    ty = infer_var(*(InferCtxt **)lc->fcx, INFER_TY, a->span);
                } else {
                    if (a->ty->kind != TY_PATH || a->ty->path.qself)
                        fatal(a->span, "unsupported const generic argument");
                    Expr *pe = ast_new_expr(EXPR_PATH, a->span);
                    pe->path.path = a->ty->path.path;
                    ty = lower_const_expr(lc, pe);
                }
            } else {
                if (a->kind != GA_TYPE)
                    fatal(a->span, "expected a type argument");
                ty = lower_ty(lc, a->ty);
            }
        } else if (infer_missing && lc->fcx) {
            ty = infer_var(*(InferCtxt **)lc->fcx, INFER_TY, sp);
        } else if (p->default_ast) {
            LowerCtx dl = {scope_for_def(g->owner), g->owner, NULL, IT_ERROR};
            Type *dt = lower_ty(&dl, p->default_ast);
            /* Defaults may mention earlier parameters. */
            Type **pad = NEW_ARRAY(Type *, g->count ? g->count : 1);
            for (uint32_t k = 0; k < g->count; k++)
                pad[k] = k < n ? t[k] : ty_error();
            ty = ty_subst(dt, substs_new(pad, g->count));
        } else if (lc->fcx) {
            ty = infer_var(*(InferCtxt **)lc->fcx, INFER_TY, sp);
        } else {
            fatal(sp, "missing generic argument `%s` for `%s`", sym_str(p->name),
                  def_path_str(def));
        }
        t[n++] = ty;
    }
    return substs_new(t, n);
}

TraitRef lower_trait_ref(LowerCtx *lc, DefId trait, PathSegment *seg, Type *self, PredList *projs,
                         Span sp) {
    TraitRef tr = {trait, NULL};
    GenericArgs *args = seg ? seg->args : NULL;
    if (args && args->kind == GENERIC_ARGS_PAREN) {
        Type **in = NEW_ARRAY(Type *, args->inputs.len ? args->inputs.len : 1);
        for (uint32_t i = 0; i < args->inputs.len; i++)
            in[i] = lower_ty(lc, args->inputs.data[i]);
        Type *tuple = ty_tuple(substs_new(in, args->inputs.len));
        tr.substs = substs_2(self, tuple);
        if (projs && lang.fn_once) {
            Predicate p = {PRED_PROJ, {lang.fn_once, tr.substs}, lang.fn_once_output,
                           args->output ? lower_ty(lc, args->output) : ty_unit()};
            pred_push(projs, p);
        }
        return tr;
    }
    tr.substs = lower_generic_args(lc, trait, args, substs_1(self), false, sp);
    if (args && projs) {
        for (uint32_t i = 0; i < args->args.len; i++) {
            GenericArg *a = args->args.data[i];
            if (a->kind != GA_BINDING && a->kind != GA_CONSTRAINT)
                continue;
            TraitRef owner;
            DefId assoc = trait_item_deep(tr, a->name, &owner);
            if (!assoc || def_get(assoc)->kind != DEF_ASSOC_TY)
                fatal(a->span, "trait `%s` has no associated type `%s`",
                      sym_str(def_get(trait)->name), sym_str(a->name));
            if (a->kind == GA_BINDING) {
                Predicate p = {PRED_PROJ, owner, assoc, lower_ty(lc, a->ty)};
                pred_push(projs, p);
            } else {
                lower_bounds(lc, ty_proj(assoc, owner.substs), &a->bounds, projs);
            }
        }
    }
    return tr;
}

Type *lower_assoc_ty_path(LowerCtx *lc, Type *self, Sym name, Span sp) {
    /* 1. Inside an impl, `Self::Name`. */
    for (DefId id = lc->item; id; id = def_get(id)->parent) {
        Def *d = def_get(id);
        if (d->kind == DEF_MOD)
            break;
        if (d->kind != DEF_IMPL)
            continue;
        ImplDef *im = impl_def(id);
        if (im->self_ty != self)
            break;
        DefId it = impl_item(id, name);
        if (it && def_get(it)->kind == DEF_ASSOC_TY) {
            /* While the associated type itself is being defined
               (`type Item = I::Item` in `impl<I: Iterator> IntoIterator for
               I`), the name must come from the bounds on Self instead. */
            if (it == lc->item || (def_get(it)->query_state & 1))
                break;
            return type_of(it);
        }
        if (im->of_trait) {
            TraitRef owner;
            DefId assoc = trait_item_deep(im->trait_ref, name, &owner);
            if (assoc && def_get(assoc)->kind == DEF_ASSOC_TY)
                return ty_proj(assoc, owner.substs);
        }
        break;
    }
    /* 2. Bounds in scope on `self`. */
    if (lc->item) {
        PredList *preds = predicates_of(lc->item);
        for (uint32_t i = 0; i < preds->len; i++) {
            Predicate *p = &preds->data[i];
            if (p->kind != PRED_TRAIT || p->tr.substs->t[0] != self)
                continue;
            TraitRef owner;
            DefId assoc = trait_item_deep(p->tr, name, &owner);
            if (assoc && def_get(assoc)->kind == DEF_ASSOC_TY)
                return ty_proj(assoc, owner.substs);
        }
    }
    /* 3. Bounds on a projection / opaque self (`T::Item::Output`). */
    if (self->kind == T_PROJ) {
        PredList *ib = trait_item_bounds(def_get(self->def)->parent);
        for (uint32_t i = 0; i < ib->len; i++) {
            Predicate p = ib->data[i];
            if (p.kind != PRED_TRAIT)
                continue;
            p.tr.substs = substs_subst(p.tr.substs, self->substs);
            if (p.tr.substs->t[0] != self)
                continue;
            TraitRef owner;
            DefId assoc = trait_item_deep(p.tr, name, &owner);
            if (assoc && def_get(assoc)->kind == DEF_ASSOC_TY)
                return ty_proj(assoc, owner.substs);
        }
    }
    (void)sp;
    return NULL;
}

static Type *lower_dyn(LowerCtx *lc, Ty *ast) {
    DefId principal = 0;
    GenericBound *pb = NULL;
    for (uint32_t i = 0; i < ast->bounds.len; i++) {
        GenericBound *b = ast->bounds.data[i];
        if (b->kind != BOUND_TRAIT)
            continue;
        DefId t = resolve_trait_path(lc, b->path);
        if (trait_def(t)->is_auto)
            continue;
        if (principal)
            fatal(b->span, "trait object with more than one non-auto trait");
        principal = t;
        pb = b;
    }
    if (!principal)
        fatal(ast->span, "trait object without a principal trait is not supported");
    PredList extra = {0};
    TraitRef tr = lower_trait_ref(lc, principal, vec_last(pb->path->segs), ty_error(), &extra,
                                  ast->span);
    DynBinding *binds = NEW_ARRAY(DynBinding, extra.len ? extra.len : 1);
    uint32_t nb = 0;
    for (uint32_t i = 0; i < extra.len; i++)
        if (extra.data[i].kind == PRED_PROJ) {
            binds[nb].assoc = extra.data[i].assoc;
            binds[nb].ty = extra.data[i].ty;
            nb++;
        }
    return ty_dyn(principal, substs_slice(tr.substs, 1, tr.substs->len), binds, nb);
}

static Type *lower_path_ty(LowerCtx *lc, Ty *ast) {
    Path *p = ast->path.path;
    QSelf *q = ast->path.qself;
    Type *ty;
    uint32_t next;
    if (q) {
        Type *self = lower_ty(lc, q->ty);
        if (q->position == 0) {
            ty = self;
            next = 0;
        } else {
            Path tp = *p;
            tp.segs.len = q->position;
            DefId trait = resolve_trait_path(lc, &tp);
            TraitRef tr = lower_trait_ref(lc, trait, p->segs.data[q->position - 1], self, NULL,
                                          ast->span);
            if (p->segs.len <= q->position)
                fatal(ast->span, "expected an associated type after the qualified path");
            TraitRef owner;
            DefId assoc = trait_item_deep(tr, p->segs.data[q->position]->name, &owner);
            if (!assoc || def_get(assoc)->kind != DEF_ASSOC_TY)
                fatal(ast->span, "trait `%s` has no associated type `%s`",
                      sym_str(def_get(trait)->name), sym_str(p->segs.data[q->position]->name));
            ty = ty_proj(assoc, owner.substs);
            next = q->position + 1;
        }
    } else {
        PathRes r = resolve_path(lc->scope, p, NS_TYPE);
        PathSegment *seg = p->segs.data[r.nsegs ? r.nsegs - 1 : 0];
        next = r.nsegs;
        switch (r.res.kind) {
        case RES_PRIM:
            ty = r.res.prim;
            break;
        case RES_SELF_TY:
            ty = def_get(r.res.def)->kind == DEF_IMPL ? type_of(r.res.def)
                                                      : ty_param(0, sym_self_upper());
            break;
        case RES_DEF: {
            Def *d = def_get(r.res.def);
            switch (d->kind) {
            case DEF_STRUCT: case DEF_ENUM: case DEF_UNION:
                ty = ty_adt(r.res.def, lower_generic_args(lc, r.res.def, seg->args, substs_empty(),
                                                          false, ast->span));
                break;
            case DEF_TYPE_ALIAS:
                ty = ty_subst(type_of(r.res.def),
                              lower_generic_args(lc, r.res.def, seg->args, substs_empty(), false,
                                                 ast->span));
                break;
            case DEF_TY_PARAM:
                ty = type_of(r.res.def);
                break;
            case DEF_TRAIT:
                fatal(ast->span, "trait `%s` used as a type (write `dyn %s`)", sym_str(d->name),
                      sym_str(d->name));
            default:
                fatal(ast->span, "expected a type, found %s `%s`", def_kind_str(d->kind),
                      def_path_str(r.res.def));
            }
            break;
        }
        default:
            fatal(ast->span, "cannot find type `%s` in this scope",
                  sym_str(vec_last(p->segs)->name));
        }
    }
    for (; next < p->segs.len; next++) {
        Type *a = lower_assoc_ty_path(lc, ty, p->segs.data[next]->name, ast->span);
        if (!a)
            fatal(ast->span, "cannot resolve associated type `%s::%s`", ty_to_str(ty),
                  sym_str(p->segs.data[next]->name));
        ty = a;
    }
    return ty;
}

Type *lower_ty(LowerCtx *lc, Ty *ast) {
    switch (ast->kind) {
    case TY_PATH:
        return lower_path_ty(lc, ast);
    case TY_REF:
        return ty_ref(lower_ty(lc, ast->ref.elem), ast->ref.is_mut);
    case TY_PTR:
        return ty_ptr(lower_ty(lc, ast->ptr.elem), ast->ptr.is_mut);
    case TY_SLICE:
        return ty_slice(lower_ty(lc, ast->slice.elem));
    case TY_ARRAY: {
        Type *elem = lower_ty(lc, ast->array.elem);
        return ty_array(elem, lower_const_expr(lc, ast->array.len));
    }
    case TY_TUPLE: {
        uint32_t n = ast->tuple.elems.len;
        Type **t = NEW_ARRAY(Type *, n ? n : 1);
        for (uint32_t i = 0; i < n; i++)
            t[i] = lower_ty(lc, ast->tuple.elems.data[i]);
        return ty_tuple(substs_new(t, n));
    }
    case TY_NEVER:
        return ty_never();
    case TY_FN: {
        uint32_t n = ast->fn_.params.len;
        Type **t = NEW_ARRAY(Type *, n + 1);
        for (uint32_t i = 0; i < n; i++)
            t[i] = lower_ty(lc, ast->fn_.params.data[i]->ty);
        t[n] = ast->fn_.ret ? lower_ty(lc, ast->fn_.ret) : ty_unit();
        return ty_fnptr(substs_new(t, n + 1));
    }
    case TY_DYN:
        return lower_dyn(lc, ast);
    case TY_IMPL: {
        if (lc->impl_trait == IT_ARG && lc->item) {
            GenericsDef *g = generics_of(lc->item);
            for (uint32_t i = 0; i < g->nown; i++)
                if (g->own[i].impl_trait == ast)
                    return ty_param(g->own[i].index, g->own[i].name);
        }
        if (lc->impl_trait == IT_RET && lc->item) {
            Def *d = def_get(lc->item);
            if (!d->opaques)
                d->opaques = NEW(OpaqueInfo);
            OpaqueInfo *oi = d->opaques;
            uint32_t ord = oi->n++;
            PredList *nb = NEW_ARRAY(PredList, oi->n);
            Type **nc = NEW_ARRAY(Type *, oi->n);
            if (ord) {
                memcpy(nb, oi->bounds, sizeof(PredList) * ord);
                memcpy(nc, oi->concrete, sizeof(Type *) * ord);
            }
            oi->bounds = nb;
            oi->concrete = nc;
            Type *t = ty_opaque(lc->item, ord, identity_substs(lc->item));
            LowerCtx inner = *lc;
            inner.impl_trait = IT_ERROR;
            PredList bl = {0};
            lower_bounds(&inner, t, &ast->bounds, &bl);
            d->opaques->bounds[ord] = bl;
            return t;
        }
        fatal(ast->span, "`impl Trait` is only supported in fn parameter and return types");
    }
    case TY_INFER:
        if (!lc->fcx)
            fatal(ast->span, "`_` is not allowed in item signatures");
        return infer_var(*(InferCtxt **)lc->fcx, INFER_TY, ast->span);
    case TY_MAC:
        fatal(ast->span, "unexpanded macro in type position");
    }
    fatal(ast->span, "internal: bad type node");
}

/* ------------------------------------------------------------------ */
/* Impl tables                                                         */
/* ------------------------------------------------------------------ */

static HMap trait_impl_tab; /* (trait, key) -> DefIdList* */
static HMap inherent_tab;   /* key -> DefIdList* */
static HMap item_name_tab;  /* Sym -> DefIdList* */

#define KEY_BLANKET 1
#define KEY_ALL 2

static uint64_t simp_key(Type *t) {
    uint64_t k = ((uint64_t)(t->kind + 1)) << 40;
    switch ((TypeKind)t->kind) {
    case T_ADT: case T_DYN: return k | t->def;
    case T_INT: case T_FLOAT: return k | t->ik;
    case T_REF: case T_PTR: return k | t->is_mut;
    case T_TUPLE: return k | t->substs->len;
    case T_FNPTR: return k | t->substs->len;
    case T_PARAM: case T_PROJ: case T_OPAQUE: return KEY_BLANKET;
    case T_INFER: return KEY_ALL;
    default: return k;
    }
}

static void tab_push(HMap *m, uint64_t a, uint64_t b, DefId id) {
    DefIdList *l = hmap_get2(m, a, b);
    if (!l) {
        l = NEW(DefIdList);
        hmap_put2(m, a, b, l);
    }
    vec_push(*l, id);
}

static DefId lang_assoc(DefId trait, const char *name) {
    return trait ? trait_item(trait, sym_intern_cstr(name)) : 0;
}

void typeck_init(void) {
    static const char *binop_names[18] = {"add", "sub", "mul", "div", "rem", NULL, NULL,
                                          "bitxor", "bitand", "bitor", "shl", "shr",
                                          NULL, NULL, NULL, NULL, NULL, NULL};
    lang.sized = lang_item("sized");
    lang.copy = lang_item("copy");
    lang.clone = lang_item("clone");
    lang.drop = lang_item("drop");
    lang.deref = lang_item("deref");
    lang.deref_mut = lang_item("deref_mut");
    lang.index = lang_item("index");
    lang.index_mut = lang_item("index_mut");
    lang.fn = lang_item("fn");
    lang.fn_mut = lang_item("fn_mut");
    lang.fn_once = lang_item("fn_once");
    lang.box = lang_item("owned_box");
    lang.rc = lang_item("Rc");
    lang.arc = lang_item("Arc");
    lang.option = lang_item("Option");
    lang.result = lang_item("Result");
    lang.string = lang_item("String");
    lang.range = lang_item("Range");
    lang.range_from = lang_item("RangeFrom");
    lang.range_to = lang_item("RangeTo");
    lang.range_inclusive = lang_item("RangeInclusive");
    lang.range_to_inclusive = lang_item("RangeToInclusive");
    lang.range_full = lang_item("RangeFull");
    lang.into_iterator = lang_item("IntoIterator");
    lang.iterator = lang_item("Iterator");
    lang.from = lang_item("From");
    lang.partial_eq = lang_item("eq");
    lang.partial_ord = lang_item("partial_ord");
    lang.neg = lang_item("neg");
    lang.not_ = lang_item("not");
    for (int i = 0; i < 18; i++) {
        if (!binop_names[i])
            continue;
        lang.binop[i] = lang_item(binop_names[i]);
        lang.binop_assign[i] = lang_item(arena_printf("%s_assign", binop_names[i]));
    }
    lang.fn_once_output = lang_assoc(lang.fn_once, "Output");
    lang.deref_target = lang_assoc(lang.deref, "Target");

    uint32_t ndefs = def_count();
    for (DefId id = 1; id <= ndefs; id++) {
        Def *d = def_get(id);
        if (d->kind != DEF_TRAIT)
            continue;
        for (uint32_t i = 0; i < d->children.len; i++)
            tab_push(&item_name_tab, def_get(d->children.data[i])->name, 0, d->children.data[i]);
    }
    const DefIdList *impls = resolve_all_impls();
    for (uint32_t i = 0; i < impls->len; i++) {
        DefId id = impls->data[i];
        ImplDef *im = impl_def(id);
        if (im->negative)
            continue;
        uint64_t key = simp_key(im->self_ty);
        if (im->of_trait) {
            tab_push(&trait_impl_tab, im->trait_ref.trait, key, id);
            tab_push(&trait_impl_tab, im->trait_ref.trait, KEY_ALL, id);
        } else {
            tab_push(&inherent_tab, key, 0, id);
        }
    }
}

void trait_impls_for(DefId trait, Type *self, const DefIdList **exact, const DefIdList **blanket) {
    uint64_t key = simp_key(self);
    *exact = hmap_get2(&trait_impl_tab, trait, key);
    *blanket = key == KEY_ALL || key == KEY_BLANKET ? NULL
                                                    : hmap_get2(&trait_impl_tab, trait, KEY_BLANKET);
}

const DefIdList *inherent_impls_for(Type *self) {
    return hmap_get2(&inherent_tab, simp_key(self), 0);
}

const DefIdList *trait_items_named(Sym name) { return hmap_get2(&item_name_tab, name, 0); }
