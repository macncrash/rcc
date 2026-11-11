/*
 * expand_derive.c - #[derive(..)] for Clone, Copy, Debug, PartialEq, Eq,
 * Hash, Default, PartialOrd, Ord.
 *
 * Each derive is generated as Rust source text (generics and where clauses
 * are re-printed with astprint), lexed with every token given the derive
 * attribute's span, and parsed as items. Traits and helpers are named by
 * absolute path (::std::clone::Clone, ...). As rustc does, every type
 * parameter gets a `T: ::std::<trait>` bound added in the impl.
 * Shapes are documented in docs/STATUS.md ("Expander").
 */
#include <stdarg.h>
#include <stdio.h>

#include "expand_int.h"

typedef struct SB {
    CharVec v;
} SB;

static void sb_puts(SB *b, const char *s) {
    for (; *s; s++)
        vec_push(b->v, *s);
}

static void sb_printf(SB *b, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    char buf[512];
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    if (n >= (int)sizeof buf) {
        char *big = arena_alloc((size_t)n + 1);
        va_start(ap, fmt);
        vsnprintf(big, (size_t)n + 1, fmt, ap);
        va_end(ap);
        sb_puts(b, big);
        return;
    }
    sb_puts(b, buf);
}

static const char *sb_str(SB *b) {
    return arena_strndup(b->v.data ? b->v.data : "", b->v.len);
}

/* An identifier as source text (raw if it is a keyword). */
static const char *id(Sym s) {
    if (sym_is_strict_kw(s) && s != KW_SELF_VALUE && s != KW_SELF_TYPE &&
        s != KW_CRATE && s != KW_SUPER && s != KW_UNDERSCORE)
        return arena_printf("r#%s", sym_str(s));
    return sym_str(s);
}

typedef struct DCtx {
    Item *it;
    const char *name;
    const Generics *g;
    const char *trait_path;
    Span sp;
} DCtx;

static const Generics *item_generics(Item *it) {
    return it->kind == ITEM_ENUM ? &it->enum_.generics : &it->adt.generics;
}

/* impl<...> TRAIT for NAME<...> where ... */
static void impl_header(SB *b, DCtx *c, const char *trait_bound) {
    const Generics *g = c->g;
    sb_puts(b, "impl");
    if (g->params.len) {
        sb_puts(b, "<");
        for (uint32_t i = 0; i < g->params.len; i++) {
            GenericParam *p = g->params.data[i];
            if (i)
                sb_puts(b, ", ");
            switch (p->kind) {
            case GP_LIFETIME:
                sb_printf(b, "'%s", sym_str(p->name));
                if (p->bounds.len)
                    sb_printf(b, ": %s", ast_bounds_to_str(&p->bounds));
                break;
            case GP_TYPE:
                sb_printf(b, "%s: ", id(p->name));
                if (p->bounds.len)
                    sb_printf(b, "%s + ", ast_bounds_to_str(&p->bounds));
                sb_puts(b, trait_bound);
                break;
            case GP_CONST:
                sb_printf(b, "const %s: %s", id(p->name), ast_ty_to_str(p->const_ty));
                break;
            }
        }
        sb_puts(b, ">");
    }
    sb_printf(b, " %s for %s", c->trait_path, id(c->it->name));
    if (g->params.len) {
        sb_puts(b, "<");
        for (uint32_t i = 0; i < g->params.len; i++) {
            GenericParam *p = g->params.data[i];
            if (i)
                sb_puts(b, ", ");
            if (p->kind == GP_LIFETIME)
                sb_printf(b, "'%s", sym_str(p->name));
            else
                sb_puts(b, id(p->name));
        }
        sb_puts(b, ">");
    }
    sb_printf(b, " %s {\n", ast_where_to_str(g));
}

static bool has_type_params(const Generics *g) {
    for (uint32_t i = 0; i < g->params.len; i++)
        if (g->params.data[i]->kind == GP_TYPE)
            return true;
    return false;
}

/* Field `i` of the struct as an access path on `base` (self / other). */
static const char *field_access(const char *base, FieldDef *f, uint32_t i) {
    if (f->name)
        return arena_printf("%s.%s", base, id(f->name));
    return arena_printf("%s.%u", base, i);
}

/* Pattern for a variant binding fields to PREFIX_0, PREFIX_1, ... */
static const char *variant_pat(DCtx *c, Variant *v, const char *prefix) {
    SB b = {0};
    sb_printf(&b, "%s::%s", id(c->it->name), id(v->name));
    if (v->data.kind == VD_TUPLE) {
        sb_puts(&b, "(");
        for (uint32_t i = 0; i < v->data.fields.len; i++)
            sb_printf(&b, "%s%s_%u", i ? ", " : "", prefix, i);
        sb_puts(&b, ")");
    } else if (v->data.kind == VD_STRUCT) {
        sb_puts(&b, " { ");
        for (uint32_t i = 0; i < v->data.fields.len; i++)
            sb_printf(&b, "%s%s: %s_%u", i ? ", " : "", id(v->data.fields.data[i]->name), prefix, i);
        sb_puts(&b, " }");
    }
    return sb_str(&b);
}

/* Wildcard pattern for a variant. */
static const char *variant_wild(DCtx *c, Variant *v) {
    const char *base = arena_printf("%s::%s", id(c->it->name), id(v->name));
    if (v->data.kind == VD_TUPLE)
        return arena_printf("%s(..)", base);
    if (v->data.kind == VD_STRUCT)
        return arena_printf("%s { .. }", base);
    return base;
}

/* Discriminant type: #[repr(intty)] or isize. */
static const char *repr_ty(Item *it) {
    static const char *ints[] = {"u8", "u16", "u32", "u64", "u128", "usize",
                                 "i8", "i16", "i32", "i64", "i128", "isize", NULL};
    for (uint32_t i = 0; i < it->attrs.len; i++) {
        Attr *a = it->attrs.data[i];
        if (a->path->segs.len != 1 || strcmp(sym_str(a->path->segs.data[0]->name), "repr") != 0)
            continue;
        for (uint32_t k = 0; k < a->tokens.len; k++) {
            Token *t = &a->tokens.toks[k];
            if (t->kind != TK_IDENT)
                continue;
            for (int j = 0; ints[j]; j++)
                if (strcmp(sym_str(t->v.sym), ints[j]) == 0)
                    return ints[j];
        }
    }
    return "isize";
}

/* `match SUBJ { V => d, ... }` computing each variant's discriminant. */
static const char *discr_expr(DCtx *c, const char *subj) {
    Item *it = c->it;
    const char *rt = repr_ty(it);
    SB b = {0};
    sb_printf(&b, "match %s { ", subj);
    const char *base = NULL; /* last explicit discriminant text */
    int64_t lit_base = 0;
    bool lit = true;
    uint64_t k = 0;
    for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
        Variant *v = it->enum_.variants.data[i];
        if (v->disc) {
            Expr *d = v->disc;
            bool neg = false;
            if (d->kind == EXPR_UNARY && d->unary.op == UN_NEG) {
                neg = true;
                d = d->unary.operand;
            }
            if (d->kind == EXPR_LIT && d->lit->kind == LIT_INT && !d->lit->int_hi) {
                lit = true;
                lit_base = neg ? -(int64_t)d->lit->int_lo : (int64_t)d->lit->int_lo;
            } else {
                lit = false;
                base = ast_expr_to_str(v->disc);
            }
            k = 0;
        }
        sb_printf(&b, "%s => ", variant_wild(c, v));
        if (lit) {
            int64_t val = lit_base + (int64_t)k;
            if (val < 0)
                sb_printf(&b, "-%llu%s, ", (unsigned long long)(-(val + 1)) + 1ULL, rt);
            else
                sb_printf(&b, "%llu%s, ", (unsigned long long)val, rt);
        } else {
            sb_printf(&b, "((%s) as %s + %llu%s), ", base, rt, (unsigned long long)k, rt);
        }
        k++;
    }
    sb_puts(&b, "}");
    return sb_str(&b);
}

/* ---- per trait ---- */

static void gen_clone(SB *b, DCtx *c, bool also_copy) {
    Item *it = c->it;
    sb_puts(b, "#[inline] fn clone(&self) -> Self { ");
    if (it->kind == ITEM_UNION || (also_copy && !has_type_params(c->g))) {
        sb_puts(b, "*self");
    } else if (it->kind == ITEM_STRUCT) {
        VariantData *d = &it->adt.data;
        if (d->kind == VD_UNIT) {
            sb_puts(b, id(it->name));
        } else {
            sb_printf(b, "%s%s", id(it->name), d->kind == VD_TUPLE ? "(" : " { ");
            for (uint32_t i = 0; i < d->fields.len; i++) {
                FieldDef *f = d->fields.data[i];
                if (i)
                    sb_puts(b, ", ");
                if (f->name)
                    sb_printf(b, "%s: ", id(f->name));
                sb_printf(b, "::std::clone::Clone::clone(&%s)", field_access("self", f, i));
            }
            sb_puts(b, d->kind == VD_TUPLE ? ")" : " }");
        }
    } else {
        if (!it->enum_.variants.len) {
            sb_puts(b, "match *self {}");
        } else {
            sb_puts(b, "match self { ");
            for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
                Variant *v = it->enum_.variants.data[i];
                sb_printf(b, "%s => %s::%s", variant_pat(c, v, "__self"), id(it->name), id(v->name));
                if (v->data.kind == VD_TUPLE) {
                    sb_puts(b, "(");
                    for (uint32_t k = 0; k < v->data.fields.len; k++)
                        sb_printf(b, "%s::std::clone::Clone::clone(__self_%u)", k ? ", " : "", k);
                    sb_puts(b, ")");
                } else if (v->data.kind == VD_STRUCT) {
                    sb_puts(b, " { ");
                    for (uint32_t k = 0; k < v->data.fields.len; k++)
                        sb_printf(b, "%s%s: ::std::clone::Clone::clone(__self_%u)", k ? ", " : "",
                                  id(v->data.fields.data[k]->name), k);
                    sb_puts(b, " }");
                }
                sb_puts(b, ", ");
            }
            sb_puts(b, "}");
        }
    }
    sb_puts(b, " }\n");
}

static void debug_fields(SB *b, VariantData *d, const char *name, bool on_self) {
    if (d->kind == VD_UNIT || (d->kind == VD_STRUCT && d->fields.len == 0)) {
        sb_printf(b, "f.write_str(\"%s\")", name);
        return;
    }
    if (d->kind == VD_STRUCT)
        sb_printf(b, "f.debug_struct(\"%s\")", name);
    else
        sb_printf(b, "f.debug_tuple(\"%s\")", name);
    for (uint32_t i = 0; i < d->fields.len; i++) {
        FieldDef *fd = d->fields.data[i];
        const char *val = on_self ? arena_printf("&%s", field_access("self", fd, i))
                                  : arena_printf("__self_%u", i);
        if (d->kind == VD_STRUCT)
            sb_printf(b, ".field(\"%s\", %s)", sym_str(fd->name), val);
        else
            sb_printf(b, ".field(%s)", val);
    }
    sb_puts(b, ".finish()");
}

static void gen_debug(SB *b, DCtx *c) {
    Item *it = c->it;
    sb_puts(b, "fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result { ");
    if (it->kind == ITEM_STRUCT) {
        debug_fields(b, &it->adt.data, sym_str(it->name), true);
    } else if (it->kind == ITEM_ENUM) {
        if (!it->enum_.variants.len) {
            sb_puts(b, "match *self {}");
        } else {
            sb_puts(b, "match self { ");
            for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
                Variant *v = it->enum_.variants.data[i];
                sb_printf(b, "%s => ", variant_pat(c, v, "__self"));
                debug_fields(b, &v->data, sym_str(v->name), false);
                sb_puts(b, ", ");
            }
            sb_puts(b, "}");
        }
    } else {
        fatal(c->sp, "derive(Debug) on a union is not supported");
    }
    sb_puts(b, " }\n");
}

static void gen_partial_eq(SB *b, DCtx *c) {
    Item *it = c->it;
    sb_puts(b, "#[inline] fn eq(&self, other: &Self) -> bool { ");
    if (it->kind == ITEM_STRUCT) {
        VariantData *d = &it->adt.data;
        if (!d->fields.len)
            sb_puts(b, "true");
        for (uint32_t i = 0; i < d->fields.len; i++)
            sb_printf(b, "%s%s == %s", i ? " && " : "", field_access("self", d->fields.data[i], i),
                      field_access("other", d->fields.data[i], i));
    } else if (it->kind == ITEM_ENUM) {
        if (!it->enum_.variants.len) {
            sb_puts(b, "match *self {}");
        } else {
            sb_puts(b, "match (self, other) { ");
            for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
                Variant *v = it->enum_.variants.data[i];
                sb_printf(b, "(%s, %s) => ", variant_pat(c, v, "__self"), variant_pat(c, v, "__arg1"));
                if (!v->data.fields.len)
                    sb_puts(b, "true");
                for (uint32_t k = 0; k < v->data.fields.len; k++)
                    sb_printf(b, "%s*__self_%u == *__arg1_%u", k ? " && " : "", k, k);
                sb_puts(b, ", ");
            }
            if (it->enum_.variants.len > 1)
                sb_puts(b, "_ => false, ");
            sb_puts(b, "}");
        }
    } else {
        fatal(c->sp, "derive(PartialEq) on a union is not supported");
    }
    sb_puts(b, " }\n");
}

static void gen_hash(SB *b, DCtx *c) {
    Item *it = c->it;
    sb_puts(b, "fn hash<__H: ::std::hash::Hasher>(&self, state: &mut __H) { ");
    if (it->kind == ITEM_STRUCT) {
        VariantData *d = &it->adt.data;
        for (uint32_t i = 0; i < d->fields.len; i++)
            sb_printf(b, "::std::hash::Hash::hash(&%s, state); ", field_access("self", d->fields.data[i], i));
    } else if (it->kind == ITEM_ENUM) {
        if (it->enum_.variants.len > 1)
            sb_printf(b, "let __self_discr = %s; ::std::hash::Hash::hash(&__self_discr, state); ",
                      discr_expr(c, "self"));
        bool any_fields = false;
        for (uint32_t i = 0; i < it->enum_.variants.len; i++)
            if (it->enum_.variants.data[i]->data.fields.len)
                any_fields = true;
        if (any_fields) {
            sb_puts(b, "match self { ");
            for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
                Variant *v = it->enum_.variants.data[i];
                if (!v->data.fields.len)
                    continue;
                sb_printf(b, "%s => { ", variant_pat(c, v, "__self"));
                for (uint32_t k = 0; k < v->data.fields.len; k++)
                    sb_printf(b, "::std::hash::Hash::hash(__self_%u, state); ", k);
                sb_puts(b, "} ");
            }
            sb_puts(b, "_ => {} }");
        }
    } else {
        fatal(c->sp, "derive(Hash) on a union is not supported");
    }
    sb_puts(b, " }\n");
}

static void gen_default(SB *b, DCtx *c) {
    Item *it = c->it;
    sb_puts(b, "#[inline] fn default() -> Self { ");
    if (it->kind == ITEM_STRUCT) {
        VariantData *d = &it->adt.data;
        if (d->kind == VD_UNIT) {
            sb_puts(b, id(it->name));
        } else {
            sb_printf(b, "%s%s", id(it->name), d->kind == VD_TUPLE ? "(" : " { ");
            for (uint32_t i = 0; i < d->fields.len; i++) {
                FieldDef *f = d->fields.data[i];
                if (i)
                    sb_puts(b, ", ");
                if (f->name)
                    sb_printf(b, "%s: ", id(f->name));
                sb_puts(b, "::std::default::Default::default()");
            }
            sb_puts(b, d->kind == VD_TUPLE ? ")" : " }");
        }
    } else if (it->kind == ITEM_ENUM) {
        Variant *dv = NULL;
        for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
            Variant *v = it->enum_.variants.data[i];
            for (uint32_t k = 0; k < v->attrs.len; k++) {
                Attr *a = v->attrs.data[k];
                if (a->path->segs.len == 1 &&
                    strcmp(sym_str(a->path->segs.data[0]->name), "default") == 0)
                    dv = v;
            }
        }
        if (!dv)
            fatal(c->sp, "derive(Default) on an enum needs a #[default] variant");
        if (dv->data.kind != VD_UNIT)
            fatal(dv->span, "the #[default] variant must be a unit variant");
        sb_printf(b, "%s::%s", id(it->name), id(dv->name));
    } else {
        fatal(c->sp, "derive(Default) on a union is not supported");
    }
    sb_puts(b, " }\n");
}

/* Lexicographic comparison chain over n field pairs. */
static void cmp_chain(SB *b, bool ord, uint32_t n, const char **ls, const char **rs) {
    const char *fn = ord ? "::std::cmp::Ord::cmp" : "::std::cmp::PartialOrd::partial_cmp";
    const char *eq = ord ? "::std::cmp::Ordering::Equal"
                         : "::std::option::Option::Some(::std::cmp::Ordering::Equal)";
    if (n == 0) {
        sb_puts(b, eq);
        return;
    }
    for (uint32_t i = 0; i + 1 < n; i++)
        sb_printf(b, "match %s(%s, %s) { %s => ", fn, ls[i], rs[i], eq);
    sb_printf(b, "%s(%s, %s)", fn, ls[n - 1], rs[n - 1]);
    for (uint32_t i = 0; i + 1 < n; i++)
        sb_puts(b, ", cmp => cmp, }");
}

static void gen_cmp(SB *b, DCtx *c, bool ord) {
    Item *it = c->it;
    if (ord)
        sb_puts(b, "#[inline] fn cmp(&self, other: &Self) -> ::std::cmp::Ordering { ");
    else
        sb_puts(b, "#[inline] fn partial_cmp(&self, other: &Self) -> "
                   "::std::option::Option<::std::cmp::Ordering> { ");
    if (it->kind == ITEM_STRUCT) {
        VariantData *d = &it->adt.data;
        const char **ls = NEW_ARRAY(const char *, d->fields.len + 1);
        const char **rs = NEW_ARRAY(const char *, d->fields.len + 1);
        for (uint32_t i = 0; i < d->fields.len; i++) {
            ls[i] = arena_printf("&%s", field_access("self", d->fields.data[i], i));
            rs[i] = arena_printf("&%s", field_access("other", d->fields.data[i], i));
        }
        cmp_chain(b, ord, d->fields.len, ls, rs);
    } else if (it->kind == ITEM_ENUM) {
        uint32_t nv = it->enum_.variants.len;
        if (!nv) {
            sb_puts(b, "match *self {}");
        } else {
            if (nv > 1)
                sb_printf(b, "let __self_discr = %s; let __arg1_discr = %s; ",
                          discr_expr(c, "self"), discr_expr(c, "other"));
            sb_puts(b, "match (self, other) { ");
            for (uint32_t i = 0; i < nv; i++) {
                Variant *v = it->enum_.variants.data[i];
                uint32_t n = v->data.fields.len;
                sb_printf(b, "(%s, %s) => ", variant_pat(c, v, "__self"), variant_pat(c, v, "__arg1"));
                const char **ls = NEW_ARRAY(const char *, n + 1);
                const char **rs = NEW_ARRAY(const char *, n + 1);
                for (uint32_t k = 0; k < n; k++) {
                    ls[k] = arena_printf("__self_%u", k);
                    rs[k] = arena_printf("__arg1_%u", k);
                }
                cmp_chain(b, ord, n, ls, rs);
                sb_puts(b, ", ");
            }
            if (nv > 1)
                sb_printf(b, "_ => %s(&__self_discr, &__arg1_discr), ",
                          ord ? "::std::cmp::Ord::cmp" : "::std::cmp::PartialOrd::partial_cmp");
            sb_puts(b, "}");
        }
    } else {
        fatal(c->sp, "derive(PartialOrd/Ord) on a union is not supported");
    }
    sb_puts(b, " }\n");
}

ItemList derive_generate(Item *it, Sym trait, Span sp, bool also_copy) {
    const char *t = sym_str(trait);
    DCtx c;
    c.it = it;
    c.name = sym_str(it->name);
    c.g = item_generics(it);
    c.sp = sp;
    SB b = {0};
    if (strcmp(t, "Clone") == 0) {
        c.trait_path = "::std::clone::Clone";
        impl_header(&b, &c, c.trait_path);
        gen_clone(&b, &c, also_copy);
    } else if (strcmp(t, "Copy") == 0) {
        c.trait_path = "::std::marker::Copy";
        impl_header(&b, &c, c.trait_path);
    } else if (strcmp(t, "Debug") == 0) {
        c.trait_path = "::std::fmt::Debug";
        impl_header(&b, &c, c.trait_path);
        gen_debug(&b, &c);
    } else if (strcmp(t, "PartialEq") == 0) {
        c.trait_path = "::std::cmp::PartialEq";
        impl_header(&b, &c, c.trait_path);
        gen_partial_eq(&b, &c);
    } else if (strcmp(t, "Eq") == 0) {
        c.trait_path = "::std::cmp::Eq";
        impl_header(&b, &c, c.trait_path);
    } else if (strcmp(t, "Hash") == 0) {
        c.trait_path = "::std::hash::Hash";
        impl_header(&b, &c, c.trait_path);
        gen_hash(&b, &c);
    } else if (strcmp(t, "Default") == 0) {
        c.trait_path = "::std::default::Default";
        impl_header(&b, &c, c.trait_path);
        gen_default(&b, &c);
    } else if (strcmp(t, "PartialOrd") == 0) {
        c.trait_path = "::std::cmp::PartialOrd";
        impl_header(&b, &c, c.trait_path);
        gen_cmp(&b, &c, false);
    } else if (strcmp(t, "Ord") == 0) {
        c.trait_path = "::std::cmp::Ord";
        impl_header(&b, &c, c.trait_path);
        gen_cmp(&b, &c, true);
    } else {
        fatal(sp, "unsupported derive `%s`", t);
    }
    sb_puts(&b, "}\n");
    TokBuf tb;
    tb_init(&tb, sp);
    tb_text(&tb, sb_str(&b));
    return parse_items_from_tokens(tb_list(&tb), sp);
}
