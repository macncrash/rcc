/*
 * mono.c - monomorphization: the instance table and worklist, resolution
 * of calls to concrete instances, vtables, globals, drop-glue discovery,
 * and layout-ish queries on closed types. See mir.h.
 */
#include "mir.h"

#include <stdio.h>

static Program prog;
static uint32_t next_build;
static HMap inst_tab;     /* (kind<<32|def, substs/type id) -> Instance* */
static HMap global_tab;   /* (kind<<32|def, substs id) -> Global* */
static HMap vtable_tab;   /* (concrete id, dyn id) -> Vtable* */
static HMap used_names;   /* Sym -> any */
static HMap needs_drop_cache; /* type id -> 1 no, 2 yes, 3 in progress */
static HMap glue_set;     /* type id -> any */
static HMap drop_impl_cache; /* type id -> Instance* or (void*)1 for none */

Type *mono_ty(Type *t, Substs *substs, Span sp) {
    if (t->flags & TYF_HAS_PARAM) {
        if (!substs)
            fatal(sp, "internal: generic type `%s` without substitutions", ty_to_str(t));
        t = ty_subst(t, substs);
    }
    return norm_closed(t, sp);
}

static Substs *mono_substs(Substs *s, Span sp) { return norm_closed_substs(s, sp); }

/* ------------------------------------------------------------------ */
/* Names                                                               */
/* ------------------------------------------------------------------ */

static void mangle_ident(StrBuf *sb, const char *s) {
    for (; *s; s++) {
        char c = *s;
        if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_')
            sb_putc(sb, c);
        else
            sb_putc(sb, '_');
    }
}

static void mangle_type_head(StrBuf *sb, Type *t) {
    switch ((TypeKind)t->kind) {
    case T_ADT: mangle_ident(sb, sym_str(def_get(t->def)->name)); break;
    case T_REF: sb_puts(sb, "ref_"); mangle_type_head(sb, t->elem); break;
    case T_PTR: sb_puts(sb, "ptr_"); mangle_type_head(sb, t->elem); break;
    case T_SLICE: sb_puts(sb, "slice_"); mangle_type_head(sb, t->elem); break;
    case T_ARRAY: sb_puts(sb, "array_"); mangle_type_head(sb, t->elem); break;
    case T_TUPLE: sb_printf(sb, "tuple%u", t->substs->len); break;
    case T_DYN: sb_puts(sb, "dyn_"); mangle_ident(sb, sym_str(def_get(t->def)->name)); break;
    case T_PARAM: mangle_ident(sb, sym_str(t->name)); break;
    case T_FNPTR: sb_puts(sb, "fnptr"); break;
    case T_CLOSURE: sb_puts(sb, "closure"); break;
    default: mangle_ident(sb, ty_to_str(t)); break;
    }
}

static void mangle_def(StrBuf *sb, DefId id) {
    Def *d = def_get(id);
    if (d->parent) {
        Def *p = def_get(d->parent);
        /* skip anonymous block modules in names */
        if (!(p->kind == DEF_MOD && p->name == 0)) {
            mangle_def(sb, d->parent);
            sb_puts(sb, "__");
        } else if (p->parent) {
            mangle_def(sb, def_get(d->parent)->parent);
            sb_puts(sb, "__");
        }
    }
    if (d->kind == DEF_IMPL) {
        ImplDef *im = impl_def(id);
        if (im->of_trait) {
            mangle_ident(sb, sym_str(def_get(im->trait_ref.trait)->name));
            sb_puts(sb, "_for_");
        }
        mangle_type_head(sb, im->self_ty);
    } else if (d->kind == DEF_CLOSURE) {
        sb_printf(sb, "closure%u", id);
    } else if (d->name == 0) {
        sb_printf(sb, "anon%u", id);
    } else {
        mangle_ident(sb, sym_str(d->name));
    }
}

static const char *unique_name(const char *base, uint32_t id, bool force_suffix) {
    Sym s = sym_intern_cstr(base);
    if (force_suffix || hmap_get(&used_names, s)) {
        base = arena_printf("%s_%u", base, id);
        s = sym_intern_cstr(base);
    }
    hmap_put(&used_names, s, &used_names);
    return sym_str(s);
}

/* ------------------------------------------------------------------ */
/* Layout-ish helpers                                                  */
/* ------------------------------------------------------------------ */

Type *ty_pointee(Type *t) {
    if (t->kind == T_REF || t->kind == T_PTR)
        return t->elem;
    if (t->kind == T_ADT && t->def == lang.box && lang.box)
        return t->substs->t[0];
    return NULL;
}

uint32_t ty_field_count(Type *t, uint32_t variant) {
    switch ((TypeKind)t->kind) {
    case T_TUPLE: return t->substs->len;
    case T_ADT: return adt_def(t->def)->variants[variant].nfields;
    case T_CLOSURE: return def_get(t->def)->closure->ncaptures;
    default: return 0;
    }
}

Type *ty_field_type(Type *t, uint32_t variant, uint32_t field, Span sp) {
    switch ((TypeKind)t->kind) {
    case T_TUPLE:
        return t->substs->t[field];
    case T_ADT:
        return mono_ty(adt_def(t->def)->variants[variant].fields[field].ty, t->substs, sp);
    case T_CLOSURE:
        return mono_ty(def_get(t->def)->closure->captures[field].ty, t->substs, sp);
    default:
        fatal(sp, "internal: field of non-aggregate type `%s`", ty_to_str(t));
    }
}

Type *enum_tag_type(Type *t) {
    AdtDef *adt = adt_def(t->def);
    if (adt->repr)
        return adt->repr;
    adt_compute_discs(adt);
    int64_t lo = 0, hi = 0;
    for (uint32_t i = 0; i < adt->nvariants; i++) {
        if (adt->variants[i].disc < lo)
            lo = adt->variants[i].disc;
        if (adt->variants[i].disc > hi)
            hi = adt->variants[i].disc;
    }
    if (lo < 0) {
        if (lo >= -128 && hi <= 127)
            return ty_int(IK_I8);
        if (lo >= -32768 && hi <= 32767)
            return ty_int(IK_I16);
        if (lo >= INT32_MIN && hi <= INT32_MAX)
            return ty_int(IK_I32);
        return ty_int(IK_I64);
    }
    if (hi <= 255)
        return ty_int(IK_U8);
    if (hi <= 65535)
        return ty_int(IK_U16);
    if (hi <= UINT32_MAX)
        return ty_int(IK_U32);
    return ty_int(IK_U64);
}

Type *closure_env_param_ty(Type *cty) {
    ClosureInfo *ci = def_get(cty->def)->closure;
    if (ci->kind == CK_FN)
        return ty_ref(cty, false);
    if (ci->kind == CK_FN_MUT)
        return ty_ref(cty, true);
    return cty;
}

Instance *mono_drop_impl(Type *t, Span sp) {
    if (t->kind != T_ADT || !lang.drop)
        return NULL;
    void *c = hmap_get(&drop_impl_cache, t->id);
    if (c)
        return c == (void *)1 ? NULL : c;
    Instance *inst = NULL;
    if (implements_closed(lang.drop, t)) {
        TraitRef tr = {lang.drop, substs_1(t)};
        Selection sel = select_closed(tr, sp);
        if (sel.kind == SEL_IMPL) {
            DefId m = impl_item(sel.impl, sym_intern_cstr("drop"));
            if (m)
                inst = mono_fn_instance(m, sel.substs, sp);
        }
    }
    hmap_put(&drop_impl_cache, t->id, inst ? (void *)inst : (void *)1);
    return inst;
}

static bool has_drop_impl(Type *t) {
    if (t->kind != T_ADT || !lang.drop)
        return false;
    void *c = hmap_get(&drop_impl_cache, t->id);
    if (c)
        return c != (void *)1;
    return implements_closed(lang.drop, t);
}

bool ty_needs_drop(Type *t) {
    switch ((TypeKind)t->kind) {
    case T_BOOL: case T_CHAR: case T_INT: case T_FLOAT: case T_STR: case T_NEVER:
    case T_REF: case T_PTR: case T_FNPTR: case T_FNDEF: case T_CONST:
        return false;
    case T_DYN:
        return true;
    case T_SLICE: case T_ARRAY:
        return ty_needs_drop(t->elem);
    default:
        break;
    }
    void *c = hmap_get(&needs_drop_cache, t->id);
    if (c)
        return c == (void *)2;
    hmap_put(&needs_drop_cache, t->id, (void *)3);
    bool r = false;
    Span sp = {0};
    switch ((TypeKind)t->kind) {
    case T_TUPLE:
        for (uint32_t i = 0; i < t->substs->len && !r; i++)
            r = ty_needs_drop(t->substs->t[i]);
        break;
    case T_CLOSURE: {
        ClosureInfo *ci = def_get(t->def)->closure;
        for (uint32_t i = 0; i < ci->ncaptures && !r; i++)
            if (ci->captures[i].mode == CAP_VALUE)
                r = ty_needs_drop(ty_field_type(t, 0, i, sp));
        break;
    }
    case T_ADT: {
        Def *d = def_get(t->def);
        if (t->def == lang.box && lang.box) {
            r = true;
            break;
        }
        if (d->lang && strcmp(sym_str(d->lang), "manually_drop") == 0)
            break;
        if (has_drop_impl(t)) {
            r = true;
            break;
        }
        AdtDef *adt = adt_def(t->def);
        if (adt->is_union)
            break;
        for (uint32_t v = 0; v < adt->nvariants && !r; v++)
            for (uint32_t f = 0; f < adt->variants[v].nfields && !r; f++)
                r = ty_needs_drop(ty_field_type(t, v, f, sp));
        break;
    }
    default:
        fatal(sp, "internal: ty_needs_drop on non-closed type `%s`", ty_to_str(t));
    }
    hmap_put(&needs_drop_cache, t->id, r ? (void *)2 : (void *)1);
    return r;
}

void mono_require_drop_glue(Type *t, Span sp) {
    if (!ty_needs_drop(t) || t->kind == T_DYN)
        return;
    if (hmap_get(&glue_set, t->id))
        return;
    hmap_put(&glue_set, t->id, &glue_set);
    vec_push(prog.drop_glue, t);
    switch ((TypeKind)t->kind) {
    case T_TUPLE:
        for (uint32_t i = 0; i < t->substs->len; i++)
            mono_require_drop_glue(t->substs->t[i], sp);
        break;
    case T_SLICE: case T_ARRAY:
        mono_require_drop_glue(t->elem, sp);
        break;
    case T_CLOSURE: {
        ClosureInfo *ci = def_get(t->def)->closure;
        for (uint32_t i = 0; i < ci->ncaptures; i++)
            if (ci->captures[i].mode == CAP_VALUE)
                mono_require_drop_glue(ty_field_type(t, 0, i, sp), sp);
        break;
    }
    case T_ADT: {
        if (t->def == lang.box && lang.box) {
            mono_require_drop_glue(t->substs->t[0], sp);
            break;
        }
        (void)mono_drop_impl(t, sp);
        AdtDef *adt = adt_def(t->def);
        if (adt->is_union)
            break;
        for (uint32_t v = 0; v < adt->nvariants; v++)
            for (uint32_t f = 0; f < adt->variants[v].nfields; f++)
                mono_require_drop_glue(ty_field_type(t, v, f, sp), sp);
        break;
    }
    default:
        break;
    }
}

/* ------------------------------------------------------------------ */
/* Instances                                                           */
/* ------------------------------------------------------------------ */

static Instance *inst_new(InstanceKind kind, Span sp) {
    Instance *i = NEW(Instance);
    i->kind = kind;
    i->id = prog.instances.len;
    i->span = sp;
    vec_push(prog.instances, i);
    return i;
}

static void set_sig(Instance *i, Substs *inputs, Type *ret) {
    i->nparams = inputs->len;
    i->params = inputs->t;
    i->ret = ret;
}

Instance *mono_fn_instance(DefId def, Substs *substs, Span sp) {
    substs = mono_substs(substs, sp);
    Def *d = def_get(def);
    uint64_t key = ((uint64_t)(d->is_foreign ? INST_EXTERN : INST_FN) << 32) | def;
    Instance *i = hmap_get2(&inst_tab, key, substs->id);
    if (i)
        return i;
    if (substs->flags & (TYF_HAS_PARAM | TYF_HAS_INFER))
        fatal(sp, "internal: instance of `%s` with non-closed substs %s", def_path_str(def),
              substs_to_str(substs));
    i = inst_new(d->is_foreign ? INST_EXTERN : INST_FN, sp);
    i->def = def;
    i->substs = substs;
    hmap_put2(&inst_tab, key, substs->id, i);
    FnSigT *sig = fn_sig(def);
    Substs *in = mono_substs(substs_subst(sig->inputs, substs), sp);
    set_sig(i, in, mono_ty(sig->output, substs, sp));
    i->variadic = sig->variadic;
    if (d->is_foreign) {
        const char *link = attrs_str_value(&d->item->attrs, "link_name");
        i->name = link ? link : sym_str(d->name);
        hmap_put(&used_names, sym_intern_cstr(i->name), &used_names);
    } else {
        if (!d->item || !d->item->fn_.body)
            fatal(sp, "internal: instance of bodiless fn `%s`", def_path_str(def));
        StrBuf sb = {0};
        mangle_def(&sb, def);
        i->name = unique_name(sb_str(&sb), i->id, substs->len != 0);
    }
    return i;
}

static Instance *closure_like(InstanceKind kind, Type *cty, int mode, Span sp) {
    uint64_t key = ((uint64_t)kind << 32) | (uint32_t)(mode + 1);
    Instance *i = hmap_get2(&inst_tab, key, cty->id);
    if (i)
        return i;
    i = inst_new(kind, sp);
    i->ty = cty;
    i->shim_self = (uint8_t)mode;
    hmap_put2(&inst_tab, key, cty->id, i);
    Substs *inputs;
    Type *output;
    if (cty->kind == T_CLOSURE) {
        ClosureInfo *ci = def_get(cty->def)->closure;
        /* make sure the owner (and hence the closure) is type-checked */
        inputs = mono_substs(substs_subst(ci->inputs, cty->substs), sp);
        output = mono_ty(ci->output, cty->substs, sp);
        i->def = cty->def;
        i->substs = cty->substs;
    } else if (cty->kind == T_FNDEF) {
        FnSigT *sig = fn_sig(cty->def);
        inputs = mono_substs(substs_subst(sig->inputs, cty->substs), sp);
        output = mono_ty(sig->output, cty->substs, sp);
    } else {
        inputs = substs_slice(cty->substs, 0, cty->substs->len - 1);
        output = fnptr_ret(cty);
    }
    Type *tuple = ty_tuple(inputs);
    const char *base = "shim";
    switch (kind) {
    case INST_CLOSURE: {
        set_sig(i, substs_2(closure_env_param_ty(cty), tuple), output);
        StrBuf sb = {0};
        mangle_def(&sb, cty->def);
        base = sb_str(&sb);
        break;
    }
    case INST_CLOSURE_ONCE:
        set_sig(i, substs_2(cty, tuple), output);
        base = "closure_once_shim";
        break;
    case INST_FNPTR_SHIM:
        set_sig(i, inputs, output);
        base = "closure_fnptr_shim";
        break;
    case INST_BOX_ONCE_SHIM:
        set_sig(i, substs_2(ty_adt(lang.box, substs_1(cty)), tuple), output);
        base = "box_call_once_shim";
        break;
    case INST_FN_TRAIT_SHIM: {
        Type *self = mode == 0 ? ty_ref(cty, false) : mode == 1 ? ty_ref(cty, true) : cty;
        set_sig(i, substs_2(self, tuple), output);
        base = "fn_trait_shim";
        break;
    }
    default:
        break;
    }
    i->name = unique_name(base, i->id, true);
    return i;
}

Instance *mono_closure_instance(Type *cty, Span sp) { return closure_like(INST_CLOSURE, cty, 0, sp); }
Instance *mono_closure_once_shim(Type *cty, Span sp) {
    return closure_like(INST_CLOSURE_ONCE, cty, 0, sp);
}
Instance *mono_fnptr_shim(Type *cty, Span sp) { return closure_like(INST_FNPTR_SHIM, cty, 0, sp); }
Instance *mono_box_once_shim(Type *callee_ty, Span sp) {
    return closure_like(INST_BOX_ONCE_SHIM, callee_ty, 0, sp);
}
Instance *mono_fn_trait_shim(Type *fn_ty, int self_mode, Span sp) {
    return closure_like(INST_FN_TRAIT_SHIM, fn_ty, self_mode, sp);
}

/* ------------------------------------------------------------------ */
/* Globals                                                             */
/* ------------------------------------------------------------------ */

static Global *global_new(GlobalKind kind, Type *ty, Span sp) {
    Global *g = NEW(Global);
    g->kind = kind;
    g->id = prog.globals.len;
    g->ty = ty;
    vec_push(prog.globals, g);
    (void)sp;
    return g;
}

static void global_add_init(Global *g, Span sp) {
    Instance *i = inst_new(INST_GLOBAL_INIT, sp);
    i->global = g;
    set_sig(i, substs_empty(), g->ty);
    i->name = arena_printf("%s__init", g->name);
    g->init = i;
}

Global *mono_static(DefId def, Span sp) {
    uint64_t key = ((uint64_t)GLOBAL_STATIC << 32) | def;
    Global *g = hmap_get2(&global_tab, key, 1);
    if (g)
        return g;
    Def *d = def_get(def);
    g = global_new(GLOBAL_STATIC, mono_ty(type_of(def), NULL, sp), sp);
    g->def = def;
    g->substs = substs_empty();
    hmap_put2(&global_tab, key, 1, g);
    if (d->is_foreign) {
        g->is_foreign = true;
        g->name = sym_str(d->name);
    } else {
        StrBuf sb = {0};
        mangle_def(&sb, def);
        g->name = unique_name(sb_str(&sb), g->id + 1000000, false);
        global_add_init(g, sp);
    }
    return g;
}

Global *mono_const_global(DefId def, Substs *substs, Span sp) {
    substs = mono_substs(substs, sp);
    uint64_t key = ((uint64_t)GLOBAL_CONST << 32) | def;
    Global *g = hmap_get2(&global_tab, key, substs->id);
    if (g)
        return g;
    g = global_new(GLOBAL_CONST, mono_ty(type_of(def), substs, sp), sp);
    g->def = def;
    g->substs = substs;
    hmap_put2(&global_tab, key, substs->id, g);
    StrBuf sb = {0};
    mangle_def(&sb, def);
    g->name = unique_name(sb_str(&sb), g->id + 1000000, substs->len != 0);
    global_add_init(g, sp);
    return g;
}

Global *mono_promoted(const TExpr *e, Body *body, Substs *substs, DefId closure, Type *ty, Span sp) {
    Global *g = hmap_get2(&global_tab, PTRKEY(e), substs ? substs->id : 0);
    if (g)
        return g;
    g = global_new(GLOBAL_PROMOTED, ty, sp);
    g->expr = e;
    g->body = body;
    g->body_substs = substs;
    g->closure = closure;
    hmap_put2(&global_tab, PTRKEY(e), substs ? substs->id : 0, g);
    g->name = arena_printf("promoted_%u", g->id);
    global_add_init(g, sp);
    return g;
}

Global *mono_hidden_global(const void *key, Substs *substs, Type *ty) {
    uint64_t sid = substs ? substs->id : 0;
    Global *g = hmap_get2(&global_tab, PTRKEY(key), sid | ((uint64_t)1 << 40));
    if (g)
        return g;
    g = global_new(GLOBAL_PROMOTED, ty, (Span){0});
    hmap_put2(&global_tab, PTRKEY(key), sid | ((uint64_t)1 << 40), g);
    g->name = arena_printf("hidden_%u", g->id);
    return g;
}

/* ------------------------------------------------------------------ */
/* Vtables                                                             */
/* ------------------------------------------------------------------ */

static bool is_object_method(DefId m) {
    Def *d = def_get(m);
    if (d->kind != DEF_FN)
        return false;
    return fn_sig(m)->has_self && generics_of(m)->nown == 0;
}

typedef struct ObjMethod {
    DefId method;
    Substs *trait_substs; /* with the given Self */
} ObjMethod;
typedef VEC(ObjMethod) ObjMethodList;

static void object_methods(Type *dyn, Type *self, ObjMethodList *out) {
    PredList sup = {0};
    TraitRef principal = {dyn->def, substs_concat(substs_1(self), dyn->substs)};
    supertraits_of(principal, &sup);
    for (uint32_t i = 0; i < sup.len; i++) {
        if (sup.data[i].kind != PRED_TRAIT)
            continue;
        Def *td = def_get(sup.data[i].tr.trait);
        for (uint32_t k = 0; k < td->children.len; k++)
            if (is_object_method(td->children.data[k])) {
                ObjMethod om = {td->children.data[k], sup.data[i].tr.substs};
                vec_push(*out, om);
            }
    }
}

uint32_t mono_vtable_slot(Type *dyn, DefId method, Span sp) {
    ObjMethodList ms = {0};
    object_methods(dyn, dyn, &ms);
    for (uint32_t i = 0; i < ms.len; i++)
        if (ms.data[i].method == method)
            return i;
    fatal(sp, "method `%s` cannot be called on a trait object of type `%s`",
          sym_str(def_get(method)->name), ty_to_str(dyn));
}

Vtable *mono_vtable(Type *concrete, Type *dyn, Span sp) {
    Vtable *vt = hmap_get2(&vtable_tab, concrete->id, dyn->id);
    if (vt)
        return vt;
    vt = NEW(Vtable);
    vt->id = prog.vtables.len;
    vt->concrete = concrete;
    vt->dyn = dyn;
    vt->name = arena_printf("vtable_%u", vt->id);
    hmap_put2(&vtable_tab, concrete->id, dyn->id, vt);
    vec_push(prog.vtables, vt);
    ObjMethodList ms = {0};
    object_methods(dyn, concrete, &ms);
    vt->nslots = ms.len;
    vt->slots = NEW_ARRAY(Instance *, ms.len ? ms.len : 1);
    for (uint32_t i = 0; i < ms.len; i++) {
        DefId m = ms.data[i].method;
        Type *self_in = fn_sig(m)->inputs->t[0];
        bool by_box = self_in->kind == T_ADT && self_in->def == lang.box;
        if (self_in->kind == T_PARAM && def_get(m)->parent == lang.fn_once && lang.fn_once &&
            (concrete->kind == T_CLOSURE || concrete->kind == T_FNDEF || concrete->kind == T_FNPTR)) {
            /* FnOnce::call_once(self): callable through Box<dyn FnOnce> */
            vt->slots[i] = mono_box_once_shim(concrete, sp);
            continue;
        }
        if (self_in->kind != T_REF && !by_box)
            continue; /* by-value self: not callable through this vtable */
        /* `where Self: Sized` methods are not part of the object. */
        CalleeRes cr = mono_resolve_callee(m, ms.data[i].trait_substs, sp);
        switch (cr.kind) {
        case CR_INSTANCE:
            vt->slots[i] = cr.inst;
            break;
        case CR_CLOSURE: {
            ClosureInfo *ci = def_get(concrete->def)->closure;
            if (ci->kind != CK_FN_ONCE && !by_box)
                vt->slots[i] = mono_closure_instance(concrete, sp);
            break;
        }
        case CR_FN_UNTUPLED:
            if (!by_box)
                vt->slots[i] = mono_fn_trait_shim(concrete, self_in->is_mut ? 1 : 0, sp);
            break;
        default:
            break;
        }
    }
    mono_require_drop_glue(concrete, sp);
    return vt;
}

/* ------------------------------------------------------------------ */
/* Call resolution                                                     */
/* ------------------------------------------------------------------ */

static bool abi_is_intrinsic(Sym abi) {
    const char *s = sym_str(abi);
    return strcmp(s, "rcc-intrinsic") == 0 || strcmp(s, "rust-intrinsic") == 0;
}

CalleeRes mono_resolve_callee(DefId def, Substs *substs, Span sp) {
    CalleeRes r;
    memset(&r, 0, sizeof r);
    substs = mono_substs(substs, sp);
    r.substs = substs;
    Def *d = def_get(def);
    if (d->kind == DEF_STRUCT || d->kind == DEF_VARIANT) {
        r.kind = CR_CTOR;
        return r;
    }
    if (d->is_foreign && abi_is_intrinsic(d->abi)) {
        r.kind = CR_INTRINSIC;
        r.intrinsic = d->name;
        return r;
    }
    Def *p = d->parent ? def_get(d->parent) : NULL;
    if (p && p->kind == DEF_TRAIT) {
        uint32_t tg = generics_of(d->parent)->count;
        TraitRef tr = {d->parent, substs_slice(substs, 0, tg)};
        Type *self = substs->t[0];
        r.self_ty = self;
        Selection sel = select_closed(tr, sp);
        switch (sel.kind) {
        case SEL_IMPL: {
            DefId it = impl_item(sel.impl, d->name);
            if (it) {
                r.inst = mono_fn_instance(
                    it, substs_concat(sel.substs, substs_slice(substs, tg, substs->len)), sp);
            } else {
                if (!d->item->fn_.body)
                    fatal(sp, "impl `%s` is missing method `%s`", def_path_str(sel.impl),
                          sym_str(d->name));
                r.inst = mono_fn_instance(def, substs, sp);
            }
            r.kind = CR_INSTANCE;
            return r;
        }
        case SEL_OBJECT:
            r.kind = CR_VIRTUAL;
            r.slot = mono_vtable_slot(self, def, sp);
            return r;
        case SEL_CLOSURE:
            r.kind = CR_CLOSURE;
            return r;
        case SEL_FNPTR:
            r.kind = CR_FN_UNTUPLED;
            return r;
        case SEL_BUILTIN:
            if (d->parent == lang.clone && strcmp(sym_str(d->name), "clone") == 0) {
                r.kind = CR_CLONE_COPY;
                return r;
            }
            /* fallthrough */
        default:
            /* A provided method of a trait with only builtin impls. */
            if (d->item->fn_.body) {
                r.kind = CR_INSTANCE;
                r.inst = mono_fn_instance(def, substs, sp);
                return r;
            }
            fatal(sp, "cannot resolve call to `%s` for `%s`", def_path_str(def), ty_to_str(self));
        }
    }
    r.kind = CR_INSTANCE;
    r.inst = mono_fn_instance(def, substs, sp);
    return r;
}

/* ------------------------------------------------------------------ */
/* Driver                                                              */
/* ------------------------------------------------------------------ */

typedef struct RefScan {
    uint32_t *seen; /* per instance: epoch */
    uint32_t epoch;
} RefScan;

static void visit_global(Global *g, RefScan *rs);

static void scan_const(const MirConst *c, RefScan *rs);
static void scan_instance(Instance *inst, RefScan *rs);

static void scan_operand(const Operand *o, RefScan *rs) {
    if (o->kind == OP_CONST)
        scan_const(&o->c, rs);
}

static void scan_const(const MirConst *c, RefScan *rs) {
    if (c->kind == MC_FN && c->inst)
        scan_instance(c->inst, rs);
}

static void scan_instance(Instance *inst, RefScan *rs) {
    if (!inst || !inst->mir || rs->seen[inst->id] == rs->epoch)
        return;
    rs->seen[inst->id] = rs->epoch;
    MirBody *m = inst->mir;
    for (uint32_t b = 0; b < m->nblocks; b++) {
        MirBlock *blk = &m->blocks[b];
        for (uint32_t s = 0; s < blk->nstmts; s++) {
            MirStmt *st = &blk->stmts[s];
            if (st->kind == ST_ASSIGN) {
                Rvalue *rv = &st->rv;
                if (rv->kind == RV_GLOBAL_ADDR)
                    visit_global(rv->global, rs);
                scan_operand(&rv->a, rs);
                scan_operand(&rv->b, rs);
                for (uint32_t i = 0; i < rv->nops; i++)
                    scan_operand(&rv->ops[i], rs);
                if (rv->vtable)
                    for (uint32_t i = 0; i < rv->vtable->nslots; i++)
                        scan_instance(rv->vtable->slots[i], rs);
            } else if (st->kind == ST_CALL) {
                if (st->callee == CALLEE_DIRECT)
                    scan_instance(st->inst, rs);
                scan_operand(&st->rv.a, rs);
                for (uint32_t i = 0; i < st->nargs; i++)
                    scan_operand(&st->args[i], rs);
            }
        }
    }
}

static void visit_global(Global *g, RefScan *rs) {
    if (g->visit || !g->init)
        return;
    g->visit = 1;
    /* Each global gets its own traversal epoch so that shared callees are
       re-examined for globals not yet initialized. */
    uint32_t saved = rs->epoch;
    static uint32_t next_epoch = 1;
    rs->epoch = ++next_epoch;
    scan_instance(g->init, rs);
    rs->epoch = saved;
    g->visit = 2;
    vec_push(prog.init_order, g);
}

Program *mono_program(DefId main_fn) {
    Span sp = def_get(main_fn)->span;
    prog.user_main = mono_fn_instance(main_fn, substs_empty(), sp);
    DefId start = lang_item("start");
    if (start)
        prog.start = mono_fn_instance(start, substs_1(prog.user_main->ret), sp);
    while (next_build < prog.instances.len) {
        Instance *inst = prog.instances.data[next_build++];
        if (inst->kind == INST_EXTERN)
            continue;
        mir_build_instance(inst);
        mir_elaborate_drops(inst->mir);
    }
    RefScan rs;
    rs.seen = NEW_ARRAY(uint32_t, prog.instances.len + 1);
    rs.epoch = 0;
    for (uint32_t i = 0; i < prog.globals.len; i++)
        visit_global(prog.globals.data[i], &rs);
    return &prog;
}
