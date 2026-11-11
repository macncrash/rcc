/* resolve.c - def collection, module tree, imports, scope lookup. See resolve.h. */
#include "resolve.h"

#include <stdio.h>

/* ------------------------------------------------------------------ */
/* State                                                               */
/* ------------------------------------------------------------------ */

struct Import {
    DefId in_module;  /* module containing the `use` */
    Sym *segs;        /* full path, prefixes flattened */
    uint32_t nsegs;
    bool global;      /* leading `::` */
    Sym name;         /* binding name; 0 for globs and `as _` */
    bool is_glob;
    bool is_pub;
    Span span;
    uint8_t state[2]; /* per namespace: 0 unresolved, 1 in progress, 2 done */
    Res res[2];
    uint8_t glob_state;
    DefId glob_target;
    Import *next;     /* next import with the same name in the module */
};

static VEC(CrateInfo) crates;
static uint32_t std_crate = UINT32_MAX;
static DefId prelude_mod;
static DefIdList all_impls;
static DefIdList all_bodies;
static HMap block_modules; /* Block NodeId -> DefId */
static HMap lang_items;    /* Sym -> DefId */
static HMap prim_types;    /* Sym -> Type* */
static HMap scope_cache;   /* DefId -> Scope* */

#define ID2P(id) ((void *)(uintptr_t)(id))
#define P2ID(p) ((DefId)(uintptr_t)(p))

uint32_t crate_count(void) { return crates.len; }
CrateInfo *crate_get(uint32_t idx) { return &crates.data[idx]; }
const DefIdList *resolve_all_impls(void) { return &all_impls; }
const DefIdList *resolve_all_bodies(void) { return &all_bodies; }

DefId lang_item(const char *name) {
    return P2ID(hmap_get(&lang_items, sym_intern_cstr(name)));
}

DefId lang_req(const char *name) {
    DefId d = lang_item(name);
    if (!d)
        fatal((Span){0}, "std is missing the lang item `%s`", name);
    return d;
}

/* ------------------------------------------------------------------ */
/* Def collection                                                      */
/* ------------------------------------------------------------------ */

static void collect_items(ItemList *items, DefId module);
static void collect_expr(Expr *e, DefId module);
static void collect_block(Block *b, DefId module);

static bool vis_is_pub(const Visibility *v) { return v->kind != VIS_INHERITED; }

static DefId new_module(Sym name, DefId parent, Span sp, bool is_block) {
    DefId id = def_new(DEF_MOD, name, parent, sp);
    Def *d = def_get(id);
    d->module = id;
    d->mod = NEW(ModData);
    d->mod->is_block = is_block;
    return id;
}

static void declare(DefId module, Sym name, Namespace ns, DefId def) {
    ModData *m = def_get(module)->mod;
    if (name == 0 || name == KW_UNDERSCORE)
        return;
    /* First declaration wins (duplicate names only arise from cfg'd items the
       expander already removed, or from value/type namespace sharing). */
    if (!hmap_get(&m->items[ns], name))
        hmap_put(&m->items[ns], name, ID2P(def));
}

static void note_lang(DefId id, const AttrList *attrs) {
    const char *l = attrs_str_value(attrs, "lang");
    if (l) {
        Sym s = sym_intern_cstr(l);
        def_get(id)->lang = s;
        hmap_put(&lang_items, s, ID2P(id));
    }
}

static void collect_generics(DefId owner, Generics *g) {
    uint32_t idx = 0;
    for (uint32_t i = 0; i < g->params.len; i++) {
        GenericParam *p = g->params.data[i];
        if (p->kind == GP_LIFETIME)
            continue;
        DefId id = def_new(p->kind == GP_TYPE ? DEF_TY_PARAM : DEF_CONST_PARAM, p->name,
                           owner, p->span);
        Def *d = def_get(id);
        d->gparam = p;
        d->index = idx++;
        vec_push(def_get(owner)->gparams, id);
    }
}

static DefId new_item_def(DefKind kind, Item *it, DefId parent, DefId module) {
    DefId id = def_new(kind, it->name, parent, it->span);
    Def *d = def_get(id);
    d->item = it;
    d->module = module;
    note_lang(id, &it->attrs);
    vec_push(def_get(module)->mod->members, id);
    return id;
}

static void collect_fn_body(Item *it, DefId module) {
    if (it->fn_.body)
        collect_block(it->fn_.body, module);
}

/* Associated items of a trait or impl. */
static void collect_assoc_items(ItemList *items, DefId owner, DefId module) {
    for (uint32_t i = 0; i < items->len; i++) {
        Item *it = items->data[i];
        DefKind k;
        switch (it->kind) {
        case ITEM_FN: k = DEF_FN; break;
        case ITEM_CONST: k = DEF_ASSOC_CONST; break;
        case ITEM_TYPE_ALIAS: k = DEF_ASSOC_TY; break;
        case ITEM_MAC:
            fatal(it->span, "unexpanded macro invocation in associated item position");
        default:
            fatal(it->span, "unsupported associated item kind");
        }
        DefId id = def_new(k, it->name, owner, it->span);
        Def *d = def_get(id);
        d->item = it;
        d->module = module;
        note_lang(id, &it->attrs);
        vec_push(def_get(owner)->children, id);
        if (k == DEF_FN) {
            collect_generics(id, &it->fn_.generics);
            collect_fn_body(it, module);
            vec_push(all_bodies, id);
        } else if (k == DEF_ASSOC_CONST) {
            if (it->const_.expr)
                collect_expr(it->const_.expr, module);
        } else {
            collect_generics(id, &it->type_alias.generics);
        }
    }
}

static void flatten_use(UseTree *t, DefId module, Sym *prefix, uint32_t nprefix, bool global,
                        bool is_pub) {
    uint32_t n = nprefix + t->prefix->segs.len;
    Sym *segs = NEW_ARRAY(Sym, n + 1);
    if (nprefix)
        memcpy(segs, prefix, sizeof(Sym) * nprefix);
    for (uint32_t i = 0; i < t->prefix->segs.len; i++)
        segs[nprefix + i] = t->prefix->segs.data[i]->name;
    if (nprefix == 0)
        global = t->prefix->global;
    ModData *m = def_get(module)->mod;
    if (t->kind == USE_NESTED) {
        for (uint32_t i = 0; i < t->items.len; i++)
            flatten_use(t->items.data[i], module, segs, n, global, is_pub);
        return;
    }
    Import *imp = NEW(Import);
    imp->in_module = module;
    imp->global = global;
    imp->is_pub = is_pub;
    imp->span = t->span;
    if (t->kind == USE_GLOB) {
        imp->segs = segs;
        imp->nsegs = n;
        imp->is_glob = true;
        vec_push(m->globs, imp);
        return;
    }
    /* `use a::b::{self}`: the trailing `self` names the prefix itself. */
    if (n > 1 && segs[n - 1] == KW_SELF_VALUE)
        n--;
    if (n == 0)
        fatal(t->span, "empty `use` path");
    imp->segs = segs;
    imp->nsegs = n;
    Sym name = t->rename ? t->rename : segs[n - 1];
    if (name == KW_UNDERSCORE)
        name = 0;
    imp->name = name;
    vec_push(m->all_imports, imp);
    if (name) {
        imp->next = hmap_get(&m->imports, name);
        hmap_put(&m->imports, name, imp);
    }
}

static void collect_item(Item *it, DefId module) {
    switch (it->kind) {
    case ITEM_USE:
        flatten_use(it->use_tree, module, NULL, 0, false, vis_is_pub(&it->vis));
        break;
    case ITEM_EXTERN_CRATE: {
        Import *imp = NEW(Import);
        imp->in_module = module;
        imp->global = it->extern_crate.orig != KW_SELF_VALUE;
        imp->segs = NEW_ARRAY(Sym, 1);
        imp->segs[0] = it->extern_crate.orig == KW_SELF_VALUE ? KW_CRATE : it->extern_crate.orig;
        imp->nsegs = 1;
        imp->name = it->name;
        imp->is_pub = vis_is_pub(&it->vis);
        imp->span = it->span;
        ModData *m = def_get(module)->mod;
        vec_push(m->all_imports, imp);
        imp->next = hmap_get(&m->imports, imp->name);
        hmap_put(&m->imports, imp->name, imp);
        break;
    }
    case ITEM_MOD: {
        if (!it->mod.inline_)
            fatal(it->span, "`mod %s;` was not loaded (the expander loads module files)",
                  sym_str(it->name));
        DefId id = new_module(it->name, module, it->span, false);
        Def *d = def_get(id);
        d->item = it;
        vec_push(def_get(module)->mod->members, id);
        declare(module, it->name, NS_TYPE, id);
        collect_items(&it->mod.items, id);
        break;
    }
    case ITEM_FN: {
        DefId id = new_item_def(DEF_FN, it, module, module);
        declare(module, it->name, NS_VALUE, id);
        collect_generics(id, &it->fn_.generics);
        collect_fn_body(it, module);
        vec_push(all_bodies, id);
        break;
    }
    case ITEM_STRUCT:
    case ITEM_UNION: {
        DefId id = new_item_def(it->kind == ITEM_STRUCT ? DEF_STRUCT : DEF_UNION, it, module, module);
        declare(module, it->name, NS_TYPE, id);
        if (it->adt.data.kind != VD_STRUCT)
            declare(module, it->name, NS_VALUE, id);
        collect_generics(id, &it->adt.generics);
        break;
    }
    case ITEM_ENUM: {
        DefId id = new_item_def(DEF_ENUM, it, module, module);
        declare(module, it->name, NS_TYPE, id);
        collect_generics(id, &it->enum_.generics);
        Def *d = def_get(id);
        d->mod = NEW(ModData);
        for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
            Variant *v = it->enum_.variants.data[i];
            DefId vid = def_new(DEF_VARIANT, v->name, id, v->span);
            Def *vd = def_get(vid);
            vd->variant = v;
            vd->index = i;
            vd->module = module;
            note_lang(vid, &v->attrs);
            vec_push(def_get(id)->children, vid);
            hmap_put(&def_get(id)->mod->items[NS_TYPE], v->name, ID2P(vid));
            hmap_put(&def_get(id)->mod->items[NS_VALUE], v->name, ID2P(vid));
            vec_push(def_get(id)->mod->members, vid);
            if (v->disc)
                collect_expr(v->disc, module);
        }
        break;
    }
    case ITEM_TRAIT: {
        DefId id = new_item_def(DEF_TRAIT, it, module, module);
        declare(module, it->name, NS_TYPE, id);
        collect_generics(id, &it->trait.generics);
        collect_assoc_items(&it->trait.items, id, module);
        break;
    }
    case ITEM_IMPL: {
        DefId id = new_item_def(DEF_IMPL, it, module, module);
        collect_generics(id, &it->impl.generics);
        collect_assoc_items(&it->impl.items, id, module);
        vec_push(all_impls, id);
        break;
    }
    case ITEM_TYPE_ALIAS: {
        DefId id = new_item_def(DEF_TYPE_ALIAS, it, module, module);
        declare(module, it->name, NS_TYPE, id);
        collect_generics(id, &it->type_alias.generics);
        break;
    }
    case ITEM_CONST: {
        DefId id = new_item_def(DEF_CONST, it, module, module);
        declare(module, it->name, NS_VALUE, id);
        if (it->const_.expr)
            collect_expr(it->const_.expr, module);
        vec_push(all_bodies, id);
        break;
    }
    case ITEM_STATIC: {
        DefId id = new_item_def(DEF_STATIC, it, module, module);
        declare(module, it->name, NS_VALUE, id);
        if (it->static_.expr)
            collect_expr(it->static_.expr, module);
        vec_push(all_bodies, id);
        break;
    }
    case ITEM_FOREIGN_MOD:
        for (uint32_t i = 0; i < it->foreign_mod.items.len; i++) {
            Item *f = it->foreign_mod.items.data[i];
            DefKind k = f->kind == ITEM_FN ? DEF_FN
                        : f->kind == ITEM_STATIC ? DEF_STATIC : DEF_NONE;
            if (k == DEF_NONE)
                fatal(f->span, "unsupported item in extern block");
            DefId id = new_item_def(k, f, module, module);
            Def *d = def_get(id);
            d->is_foreign = true;
            d->abi = it->foreign_mod.abi;
            declare(module, f->name, NS_VALUE, id);
            if (k == DEF_FN)
                collect_generics(id, &f->fn_.generics);
        }
        break;
    case ITEM_MAC:
        fatal(it->span, "unexpanded macro invocation `%s!` in item position",
              it->mac->path->segs.len ? sym_str(vec_last(it->mac->path->segs)->name) : "?");
    case ITEM_MACRO_RULES:
        break;
    }
}

static void collect_items(ItemList *items, DefId module) {
    for (uint32_t i = 0; i < items->len; i++)
        collect_item(items->data[i], module);
}

static void collect_block(Block *b, DefId module) {
    bool has_items = false;
    for (uint32_t i = 0; i < b->stmts.len; i++)
        if (b->stmts.data[i]->kind == STMT_ITEM)
            has_items = true;
    if (has_items) {
        DefId outer = module;
        module = new_module(0, outer, b->span, true);
        hmap_put(&block_modules, b->id, ID2P(module));
        for (uint32_t i = 0; i < b->stmts.len; i++)
            if (b->stmts.data[i]->kind == STMT_ITEM)
                collect_item(b->stmts.data[i]->item, module);
    }
    for (uint32_t i = 0; i < b->stmts.len; i++) {
        Stmt *s = b->stmts.data[i];
        switch (s->kind) {
        case STMT_LET:
            if (s->let.init)
                collect_expr(s->let.init, module);
            if (s->let.els)
                collect_block(s->let.els, module);
            break;
        case STMT_EXPR:
            collect_expr(s->expr.expr, module);
            break;
        case STMT_MAC:
            fatal(s->span, "unexpanded macro invocation in statement position");
        case STMT_ITEM:
        case STMT_EMPTY:
            break;
        }
    }
}

static void collect_exprs(ExprList *l, DefId module) {
    for (uint32_t i = 0; i < l->len; i++)
        collect_expr(l->data[i], module);
}

/* Walk an expression looking for blocks (which may declare items). */
static void collect_expr(Expr *e, DefId module) {
    if (!e)
        return;
    switch (e->kind) {
    case EXPR_LIT: case EXPR_PATH: case EXPR_CONTINUE: case EXPR_UNDERSCORE:
        break;
    case EXPR_UNARY: collect_expr(e->unary.operand, module); break;
    case EXPR_ADDR_OF: collect_expr(e->addr_of.operand, module); break;
    case EXPR_BINARY:
        collect_expr(e->binary.lhs, module);
        collect_expr(e->binary.rhs, module);
        break;
    case EXPR_ASSIGN: case EXPR_ASSIGN_OP:
        collect_expr(e->assign.lhs, module);
        collect_expr(e->assign.rhs, module);
        break;
    case EXPR_CAST: collect_expr(e->cast.expr, module); break;
    case EXPR_CALL:
        collect_expr(e->call.callee, module);
        collect_exprs(&e->call.args, module);
        break;
    case EXPR_METHOD_CALL:
        collect_expr(e->method_call.receiver, module);
        collect_exprs(&e->method_call.args, module);
        break;
    case EXPR_FIELD: collect_expr(e->field.base, module); break;
    case EXPR_INDEX:
        collect_expr(e->index.base, module);
        collect_expr(e->index.index, module);
        break;
    case EXPR_STRUCT:
        for (uint32_t i = 0; i < e->struct_.fields.len; i++)
            collect_expr(e->struct_.fields.data[i]->expr, module);
        collect_expr(e->struct_.base, module);
        break;
    case EXPR_TUPLE: case EXPR_ARRAY: collect_exprs(&e->elems, module); break;
    case EXPR_REPEAT:
        collect_expr(e->repeat.elem, module);
        collect_expr(e->repeat.count, module);
        break;
    case EXPR_RANGE:
        collect_expr(e->range.lo, module);
        collect_expr(e->range.hi, module);
        break;
    case EXPR_CLOSURE: collect_expr(e->closure.body, module); break;
    case EXPR_BLOCK: collect_block(e->block.block, module); break;
    case EXPR_IF:
        collect_expr(e->if_.cond, module);
        collect_block(e->if_.then, module);
        collect_expr(e->if_.els, module);
        break;
    case EXPR_LET: collect_expr(e->let.scrutinee, module); break;
    case EXPR_MATCH:
        collect_expr(e->match.scrutinee, module);
        for (uint32_t i = 0; i < e->match.arms.len; i++) {
            collect_expr(e->match.arms.data[i]->guard, module);
            collect_expr(e->match.arms.data[i]->body, module);
        }
        break;
    case EXPR_WHILE:
        collect_expr(e->while_.cond, module);
        collect_block(e->while_.body, module);
        break;
    case EXPR_FOR:
        collect_expr(e->for_.iter, module);
        collect_block(e->for_.body, module);
        break;
    case EXPR_LOOP: collect_block(e->loop.body, module); break;
    case EXPR_BREAK: collect_expr(e->break_.value, module); break;
    case EXPR_RETURN: collect_expr(e->ret, module); break;
    case EXPR_TRY: collect_expr(e->try_, module); break;
    case EXPR_MAC:
        fatal(e->span, "unexpanded macro invocation `%s!`",
              e->mac->path->segs.len ? sym_str(vec_last(e->mac->path->segs)->name) : "?");
    }
}

static void init_prims(void) {
    if (prim_types.len)
        return;
    types_init();
    hmap_put(&prim_types, sym_intern_cstr("bool"), ty_bool());
    hmap_put(&prim_types, sym_intern_cstr("char"), ty_char());
    hmap_put(&prim_types, sym_intern_cstr("str"), ty_str());
    hmap_put(&prim_types, sym_intern_cstr("f32"), ty_float(FK_F32));
    hmap_put(&prim_types, sym_intern_cstr("f64"), ty_float(FK_F64));
    for (int k = 0; k < IK_COUNT; k++)
        hmap_put(&prim_types, sym_intern_cstr(ik_name((IntKind)k)), ty_int((IntKind)k));
}

uint32_t resolve_add_crate(Sym name, Crate *ast) {
    init_prims();
    DefId root = new_module(name, 0, ast->span, false);
    Def *d = def_get(root);
    d->crate_ = crates.len;
    CrateInfo ci = {name, root, ast};
    vec_push(crates, ci);
    if (std_crate == UINT32_MAX && strcmp(sym_str(name), "std") == 0)
        std_crate = crates.len - 1;
    collect_items(&ast->items, root);
    return crates.len - 1;
}

void resolve_finish(void) {
    if (std_crate == UINT32_MAX)
        return;
    Res p = module_lookup(crates.data[std_crate].root, sym_intern_cstr("prelude"), NS_TYPE);
    if (p.kind == RES_DEF && def_get(p.def)->kind == DEF_MOD) {
        Res v1 = module_lookup(p.def, sym_intern_cstr("v1"), NS_TYPE);
        prelude_mod = v1.kind == RES_DEF ? v1.def : p.def;
    }
}

/* ------------------------------------------------------------------ */
/* Scopes                                                              */
/* ------------------------------------------------------------------ */

Scope *scope_push(Scope *parent, ScopeKind kind, DefId def) {
    Scope *s = NEW(Scope);
    s->parent = parent;
    s->kind = kind;
    s->def = def;
    return s;
}

void scope_bind(Scope *sc, Sym name, void *local) {
    if (sc->nbinds == sc->cap) {
        uint32_t nc = sc->cap ? sc->cap * 2 : 4;
        ScopeBinding *nb = NEW_ARRAY(ScopeBinding, nc);
        if (sc->nbinds)
            memcpy(nb, sc->binds, sizeof(ScopeBinding) * sc->nbinds);
        sc->binds = nb;
        sc->cap = nc;
    }
    sc->binds[sc->nbinds].name = name;
    sc->binds[sc->nbinds].local = local;
    sc->nbinds++;
}

DefId resolve_block_module(const Block *b) { return P2ID(hmap_get(&block_modules, b->id)); }

Scope *scope_for_def(DefId id) {
    Scope *c = hmap_get(&scope_cache, id);
    if (c)
        return c;
    Def *d = def_get(id);
    Scope *s;
    switch (d->kind) {
    case DEF_MOD:
        /* Block modules chain to the enclosing module; real modules are
           the end of the lexical chain. */
        s = scope_push(d->mod->is_block ? scope_for_def(d->parent) : NULL, SCOPE_MODULE, id);
        break;
    case DEF_IMPL:
    case DEF_TRAIT:
        s = scope_push(scope_for_def(d->module), SCOPE_GENERICS, id);
        s = scope_push(s, SCOPE_SELF, id);
        break;
    case DEF_VARIANT:
        s = scope_for_def(d->parent);
        break;
    default: {
        Def *p = d->parent ? def_get(d->parent) : NULL;
        Scope *outer = p && (p->kind == DEF_IMPL || p->kind == DEF_TRAIT)
                           ? scope_for_def(d->parent)
                           : scope_for_def(d->module);
        s = scope_push(outer, SCOPE_GENERICS, id);
        break;
    }
    }
    hmap_put(&scope_cache, id, s);
    return s;
}

static DefId real_module(DefId m) {
    while (def_get(m)->mod->is_block)
        m = def_get(m)->parent;
    return m;
}

DefId scope_module(const Scope *sc) {
    for (; sc; sc = sc->parent)
        if (sc->kind == SCOPE_MODULE)
            return real_module(sc->def);
    fatal((Span){0}, "internal: scope without a module");
}

/* ------------------------------------------------------------------ */
/* Module member lookup                                                */
/* ------------------------------------------------------------------ */

#define MAX_GLOB_DEPTH 64
typedef struct LookupGuard {
    DefId visiting[MAX_GLOB_DEPTH];
    uint32_t n;
} LookupGuard;

static Res res_none(void) { return (Res){RES_NONE, 0, NULL, NULL}; }
static Res res_def(DefId d) { return (Res){RES_DEF, d, NULL, NULL}; }

static Res lookup_in_module(DefId module, Sym name, Namespace ns, bool pub_only, LookupGuard *g);
static Res resolve_segs(DefId from_module, Sym *segs, uint32_t n, bool global, Namespace ns,
                        LookupGuard *g);

static bool is_ancestor_or_self(DefId anc, DefId m) {
    for (;;) {
        if (m == anc)
            return true;
        DefId p = def_get(m)->parent;
        if (!p)
            return false;
        m = def_get(p)->module;
    }
}

static Res import_resolve(Import *imp, Namespace ns, LookupGuard *g) {
    if (imp->state[ns] == 2)
        return imp->res[ns];
    if (imp->state[ns] == 1)
        return res_none(); /* cycle: not found through this route */
    imp->state[ns] = 1;
    Res r = resolve_segs(imp->in_module, imp->segs, imp->nsegs, imp->global, ns, g);
    imp->res[ns] = r;
    /* A failure seen while other lookups are in progress may be an artifact
       of cycle cutting; only memoize definitive answers. */
    imp->state[ns] = (r.kind != RES_NONE || g->n == 0) ? 2 : 0;
    return r;
}

static DefId glob_target(Import *imp, LookupGuard *g) {
    if (imp->glob_state == 2)
        return imp->glob_target;
    if (imp->glob_state == 1)
        return 0;
    imp->glob_state = 1;
    Res r = resolve_segs(imp->in_module, imp->segs, imp->nsegs, imp->global, NS_TYPE, g);
    DefId t = 0;
    if (r.kind == RES_DEF && def_get(r.def)->mod)
        t = r.def;
    imp->glob_target = t;
    imp->glob_state = (t || g->n == 0) ? 2 : 0;
    return t;
}

static bool def_is_pub(DefId id) {
    Def *d = def_get(id);
    if (d->kind == DEF_VARIANT)
        return true;
    if (d->item)
        return vis_is_pub(&d->item->vis);
    return true;
}

static Res lookup_in_module(DefId module, Sym name, Namespace ns, bool pub_only, LookupGuard *g) {
    ModData *m = def_get(module)->mod;
    DefId own = P2ID(hmap_get(&m->items[ns], name));
    if (own && (!pub_only || def_is_pub(own)))
        return res_def(own);
    for (Import *imp = hmap_get(&m->imports, name); imp; imp = imp->next) {
        if (pub_only && !imp->is_pub)
            continue;
        Res r = import_resolve(imp, ns, g);
        if (r.kind != RES_NONE)
            return r;
    }
    if (m->globs.len == 0)
        return res_none();
    for (uint32_t i = 0; i < g->n; i++)
        if (g->visiting[i] == module)
            return res_none();
    if (g->n == MAX_GLOB_DEPTH)
        return res_none();
    g->visiting[g->n++] = module;
    Res found = res_none();
    for (uint32_t i = 0; i < m->globs.len; i++) {
        Import *imp = m->globs.data[i];
        if (pub_only && !imp->is_pub)
            continue;
        DefId t = glob_target(imp, g);
        if (!t)
            continue;
        bool sees_private = def_get(t)->kind == DEF_MOD && is_ancestor_or_self(t, real_module(module));
        Res r = lookup_in_module(t, name, ns, !sees_private, g);
        if (r.kind != RES_NONE) {
            found = r;
            break;
        }
    }
    g->n--;
    return found;
}

Res module_lookup(DefId module, Sym name, Namespace ns) {
    LookupGuard g;
    g.n = 0;
    return lookup_in_module(module, name, ns, false, &g);
}

static DefId crate_by_name(Sym name) {
    const char *s = sym_str(name);
    if (std_crate != UINT32_MAX &&
        (strcmp(s, "std") == 0 || strcmp(s, "core") == 0 || strcmp(s, "alloc") == 0))
        return crates.data[std_crate].root;
    for (uint32_t i = 0; i < crates.len; i++)
        if (crates.data[i].name == name)
            return crates.data[i].root;
    return 0;
}

static DefId parent_module(DefId m) {
    DefId p = def_get(real_module(m))->parent;
    return p ? real_module(def_get(p)->module) : 0;
}

/* Lookup of a first path segment at module level (no locals/generics):
   the module chain (through block modules), then extern crates, the
   prelude and primitive types. */
static Res lookup_module_chain(DefId module, Sym name, Namespace ns, LookupGuard *g) {
    for (DefId m = module;;) {
        Res r = lookup_in_module(m, name, ns, false, g);
        if (r.kind != RES_NONE)
            return r;
        if (!def_get(m)->mod->is_block)
            break;
        m = def_get(m)->parent;
    }
    if (ns == NS_TYPE) {
        DefId c = crate_by_name(name);
        if (c)
            return res_def(c);
    }
    if (prelude_mod) {
        Res r = lookup_in_module(prelude_mod, name, ns, false, g);
        if (r.kind != RES_NONE)
            return r;
    }
    if (ns == NS_TYPE) {
        Type *p = hmap_get(&prim_types, name);
        if (p)
            return (Res){RES_PRIM, 0, p, NULL};
    }
    return res_none();
}

/* Continue a path from `r` through modules and enums. Returns the number
   of segments consumed in *used. */
static Res walk_segs(Res r, Sym *segs, uint32_t n, uint32_t start, Namespace ns, uint32_t *used,
                     LookupGuard *g) {
    uint32_t i = start;
    while (i < n && r.kind == RES_DEF) {
        Def *d = def_get(r.def);
        if (!d->mod)
            break;
        bool last = i == n - 1;
        Res nx = lookup_in_module(r.def, segs[i], last ? ns : NS_TYPE, false, g);
        if (nx.kind == RES_NONE) {
            if (d->kind == DEF_ENUM)
                break; /* associated item of the enum type */
            *used = n;
            return res_none();
        }
        r = nx;
        i++;
    }
    *used = i;
    return r;
}

static Res special_first(DefId module, Sym *segs, uint32_t n, uint32_t *i) {
    DefId m = real_module(module);
    if (segs[0] == KW_CRATE) {
        *i = 1;
        return res_def(crates.data[def_get(m)->crate_].root);
    }
    if (segs[0] == KW_SELF_VALUE || segs[0] == KW_SUPER) {
        uint32_t k = 0;
        if (segs[0] == KW_SELF_VALUE)
            k = 1;
        while (k < n && segs[k] == KW_SUPER) {
            m = parent_module(m);
            if (!m)
                return res_none();
            k++;
        }
        *i = k;
        return res_def(m);
    }
    *i = 0;
    return res_none();
}

/* Resolve an import path (module-level: no locals or generics). */
static Res resolve_segs(DefId from_module, Sym *segs, uint32_t n, bool global, Namespace ns,
                        LookupGuard *g) {
    Res r;
    uint32_t i = 0;
    if (global) {
        DefId c = crate_by_name(segs[0]);
        if (!c)
            return res_none();
        r = res_def(c);
        i = 1;
    } else {
        r = special_first(from_module, segs, n, &i);
        if (r.kind == RES_NONE) {
            if (i != 0 || segs[0] == KW_SUPER)
                return r;
            r = lookup_module_chain(from_module, segs[0], n == 1 ? ns : NS_TYPE, g);
            if (r.kind == RES_NONE)
                return r;
            i = 1;
        }
    }
    uint32_t used;
    r = walk_segs(r, segs, n, i, ns, &used, g);
    if (used != n)
        return res_none(); /* imports cannot name associated items */
    if (n == 1 && ns == NS_VALUE && r.kind == RES_DEF && def_get(r.def)->kind == DEF_MOD)
        return res_none();
    return r;
}

/* ------------------------------------------------------------------ */
/* Lexical lookup                                                      */
/* ------------------------------------------------------------------ */

static Res lookup_generic(DefId owner, Sym name, Namespace ns) {
    Def *d = def_get(owner);
    for (uint32_t i = 0; i < d->gparams.len; i++) {
        Def *p = def_get(d->gparams.data[i]);
        if (p->name != name)
            continue;
        if (p->kind == DEF_TY_PARAM ? ns == NS_TYPE : true)
            return res_def(d->gparams.data[i]);
    }
    return res_none();
}

Res resolve_ident(Scope *sc, Sym name, Namespace ns) {
    LookupGuard g;
    g.n = 0;
    for (Scope *s = sc; s; s = s->parent) {
        switch (s->kind) {
        case SCOPE_LOCALS:
            if (ns == NS_VALUE)
                for (uint32_t i = s->nbinds; i-- > 0;)
                    if (s->binds[i].name == name)
                        return (Res){RES_LOCAL, 0, NULL, s->binds[i].local};
            break;
        case SCOPE_GENERICS: {
            Res r = lookup_generic(s->def, name, ns);
            if (r.kind != RES_NONE)
                return r;
            break;
        }
        case SCOPE_SELF:
            if (name == KW_SELF_TYPE && ns == NS_TYPE)
                return (Res){RES_SELF_TY, s->def, NULL, NULL};
            break;
        case SCOPE_MODULE: {
            Res r = lookup_in_module(s->def, name, ns, false, &g);
            if (r.kind != RES_NONE)
                return r;
            if (!def_get(s->def)->mod->is_block && !s->parent)
                goto tail;
            break;
        }
        }
    }
tail:
    if (ns == NS_TYPE) {
        DefId c = crate_by_name(name);
        if (c)
            return res_def(c);
    }
    if (prelude_mod) {
        Res r = lookup_in_module(prelude_mod, name, ns, false, &g);
        if (r.kind != RES_NONE)
            return r;
    }
    if (ns == NS_TYPE) {
        Type *p = hmap_get(&prim_types, name);
        if (p)
            return (Res){RES_PRIM, 0, p, NULL};
    }
    return res_none();
}

PathRes resolve_path(Scope *sc, const Path *p, Namespace ns) {
    uint32_t n = p->segs.len;
    Sym small[8];
    Sym *segs = n <= 8 ? small : NEW_ARRAY(Sym, n);
    for (uint32_t i = 0; i < n; i++)
        segs[i] = p->segs.data[i]->name;
    PathRes out = {res_none(), n};
    LookupGuard g;
    g.n = 0;
    Res r;
    uint32_t i = 0;
    if (p->global) {
        DefId c = crate_by_name(segs[0]);
        if (!c)
            return out;
        r = res_def(c);
        i = 1;
    } else if (segs[0] == KW_SELF_TYPE) {
        r = resolve_ident(sc, KW_SELF_TYPE, NS_TYPE);
        out.res = r;
        out.nsegs = 1;
        return out;
    } else {
        DefId cur = 0;
        for (Scope *s = sc; s; s = s->parent)
            if (s->kind == SCOPE_MODULE) {
                cur = s->def;
                break;
            }
        r = (n > 1 || ns == NS_TYPE || segs[0] == KW_CRATE) ? special_first(cur, segs, n, &i)
                                                            : res_none();
        if (r.kind == RES_NONE) {
            if (i != 0 || segs[0] == KW_SUPER)
                return out;
            r = resolve_ident(sc, segs[0], n == 1 ? ns : NS_TYPE);
            if (r.kind == RES_NONE)
                return out;
            i = 1;
            /* A module that shares its name with a primitive type
               (`std::str`, `std::u32`): prefer the primitive unless the next
               segment names a member of the module. */
            if (r.kind == RES_DEF && def_get(r.def)->kind == DEF_MOD) {
                Type *prim = hmap_get(&prim_types, segs[0]);
                if (prim && (n == 1 ||
                             (lookup_in_module(r.def, segs[1], NS_TYPE, false, &g).kind == RES_NONE &&
                              lookup_in_module(r.def, segs[1], NS_VALUE, false, &g).kind == RES_NONE)))
                    r = (Res){RES_PRIM, 0, prim, NULL};
            }
        }
    }
    uint32_t used = i;
    if (r.kind == RES_DEF)
        r = walk_segs(r, segs, n, i, ns, &used, &g);
    out.res = r;
    out.nsegs = used;
    return out;
}

/* ------------------------------------------------------------------ */
/* Traits in scope                                                     */
/* ------------------------------------------------------------------ */

static void add_traits_of(HMap *set, DefId module, bool pub_only, LookupGuard *g);

static void add_trait_res(HMap *set, Res r) {
    if (r.kind == RES_DEF && def_get(r.def)->kind == DEF_TRAIT)
        hmap_put(set, r.def, set);
}

static void add_traits_of(HMap *set, DefId module, bool pub_only, LookupGuard *g) {
    for (uint32_t i = 0; i < g->n; i++)
        if (g->visiting[i] == module)
            return;
    if (g->n == MAX_GLOB_DEPTH)
        return;
    g->visiting[g->n++] = module;
    ModData *m = def_get(module)->mod;
    for (uint32_t i = 0; i < m->members.len; i++) {
        DefId id = m->members.data[i];
        if (def_get(id)->kind == DEF_TRAIT && (!pub_only || def_is_pub(id)))
            hmap_put(set, id, set);
    }
    for (uint32_t i = 0; i < m->all_imports.len; i++) {
        Import *imp = m->all_imports.data[i];
        if (pub_only && !imp->is_pub)
            continue;
        LookupGuard g2;
        g2.n = 0;
        add_trait_res(set, import_resolve(imp, NS_TYPE, &g2));
    }
    for (uint32_t i = 0; i < m->globs.len; i++) {
        Import *imp = m->globs.data[i];
        if (pub_only && !imp->is_pub)
            continue;
        LookupGuard g2;
        g2.n = 0;
        DefId t = glob_target(imp, &g2);
        if (!t || def_get(t)->kind != DEF_MOD)
            continue;
        add_traits_of(set, t, !is_ancestor_or_self(t, real_module(module)), g);
    }
    g->n--;
}

static HMap *module_trait_set(DefId module) {
    ModData *m = def_get(module)->mod;
    if (!m->trait_set) {
        HMap *set = NEW(HMap);
        LookupGuard g;
        g.n = 0;
        add_traits_of(set, module, false, &g);
        m->trait_set = set;
    }
    return m->trait_set;
}

bool trait_in_scope(Scope *sc, DefId trait) {
    for (Scope *s = sc; s; s = s->parent)
        if (s->kind == SCOPE_MODULE && hmap_get(module_trait_set(s->def), trait))
            return true;
    return prelude_mod && hmap_get(module_trait_set(prelude_mod), trait);
}

/* ------------------------------------------------------------------ */
/* Dump                                                                */
/* ------------------------------------------------------------------ */

static void dump_module(FILE *out, DefId module, int depth) {
    Def *d = def_get(module);
    ModData *m = d->mod;
    fprintf(out, "%*s%s %s (def %u)\n", depth * 2, "", def_kind_str(d->kind),
            d->name ? sym_str(d->name) : "{block}", module);
    for (uint32_t i = 0; i < m->members.len; i++) {
        DefId id = m->members.data[i];
        Def *c = def_get(id);
        if (c->kind == DEF_MOD) {
            dump_module(out, id, depth + 1);
            continue;
        }
        fprintf(out, "%*s%s %s (def %u)", depth * 2 + 2, "", def_kind_str(c->kind),
                c->name ? sym_str(c->name) : "_", id);
        if (c->lang)
            fprintf(out, " lang=%s", sym_str(c->lang));
        if (c->gparams.len)
            fprintf(out, " generics=%u", c->gparams.len);
        fprintf(out, "\n");
        for (uint32_t k = 0; k < c->children.len; k++) {
            Def *cc = def_get(c->children.data[k]);
            fprintf(out, "%*s%s %s (def %u)\n", depth * 2 + 4, "", def_kind_str(cc->kind),
                    sym_str(cc->name), c->children.data[k]);
        }
    }
    for (uint32_t i = 0; i < m->all_imports.len; i++) {
        Import *imp = m->all_imports.data[i];
        fprintf(out, "%*suse ", depth * 2 + 2, "");
        for (uint32_t k = 0; k < imp->nsegs; k++)
            fprintf(out, "%s%s", k || imp->global ? "::" : "", sym_str(imp->segs[k]));
        fprintf(out, " as %s", imp->name ? sym_str(imp->name) : "_");
        for (int ns = 0; ns < 2; ns++) {
            LookupGuard g;
            g.n = 0;
            Res r = import_resolve(imp, (Namespace)ns, &g);
            if (r.kind == RES_DEF)
                fprintf(out, " %s=%s", ns == NS_TYPE ? "type" : "value", def_path_str(r.def));
        }
        fprintf(out, "\n");
    }
    for (uint32_t i = 0; i < m->globs.len; i++) {
        Import *imp = m->globs.data[i];
        LookupGuard g;
        g.n = 0;
        DefId t = glob_target(imp, &g);
        fprintf(out, "%*suse ", depth * 2 + 2, "");
        for (uint32_t k = 0; k < imp->nsegs; k++)
            fprintf(out, "%s%s", k || imp->global ? "::" : "", sym_str(imp->segs[k]));
        fprintf(out, "::* -> %s\n", t ? def_path_str(t) : "<unresolved>");
    }
}

void resolve_dump(FILE *out) {
    for (uint32_t i = 0; i < crates.len; i++)
        dump_module(out, crates.data[i].root, 0);
}
