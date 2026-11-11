/*
 * astdump.c - readable S-expression dump of the AST (--dump-ast).
 *
 * Format: items, statements, match arms, fields etc. go on their own
 * indented lines; expressions and patterns are inline S-expressions; types
 * and paths are printed in Rust-like syntax. Parentheses in the source are not
 * kept, so the nesting shows exactly how precedence was resolved:
 *   a + b * c   =>  (+ a (* b c))
 */
#include <stdio.h>

#include "ast.h"

typedef struct D {
    FILE *out;
    int indent;
} D;

static void nl(D *d) {
    fputc('\n', d->out);
    for (int i = 0; i < d->indent; i++)
        fputs("  ", d->out);
}
static void put(D *d, const char *s) { fputs(s, d->out); }
static void putsym(D *d, Sym s) { fputs(sym_str(s), d->out); }

static void dump_expr(D *d, const Expr *e);
static void dump_ty(D *d, const Ty *t);
static void dump_pat(D *d, const Pat *p);
static void dump_block(D *d, const Block *b);
static void dump_item(D *d, const Item *it);
static void dump_stmt(D *d, const Stmt *s);
static void dump_path(D *d, const QSelf *qs, const Path *p);
static void dump_bounds(D *d, const BoundList *bl);

static void dump_tokens_inline(D *d, TokenList tl) {
    for (uint32_t i = 0; i < tl.len; i++) {
        const Token *t = &tl.toks[i];
        if (i > 0 && (t->flags & TF_WS_BEFORE))
            put(d, " ");
        put(d, tok_to_str(t));
    }
}

static void dump_delimited(D *d, Delim delim, TokenList tl) {
    const char *open = delim == DELIM_PAREN ? "(" : delim == DELIM_BRACKET ? "[" : "{";
    const char *close = delim == DELIM_PAREN ? ")" : delim == DELIM_BRACKET ? "]" : "}";
    put(d, open);
    dump_tokens_inline(d, tl);
    put(d, close);
}

static void dump_attrs(D *d, const AttrList *attrs) {
    for (uint32_t i = 0; i < attrs->len; i++) {
        const Attr *a = attrs->data[i];
        put(d, a->inner ? "#![" : "#[");
        dump_path(d, NULL, a->path);
        if (a->args_kind == ATTR_ARGS_DELIMITED) {
            dump_delimited(d, a->delim, a->tokens);
        } else if (a->args_kind == ATTR_ARGS_EQ) {
            put(d, " = ");
            dump_expr(d, a->value);
        }
        put(d, "]");
        nl(d);
    }
}

/* attrs inline (for expressions etc.) */
static void dump_attrs_inline(D *d, const AttrList *attrs) {
    for (uint32_t i = 0; i < attrs->len; i++) {
        const Attr *a = attrs->data[i];
        put(d, "#[");
        dump_path(d, NULL, a->path);
        if (a->args_kind == ATTR_ARGS_DELIMITED)
            dump_delimited(d, a->delim, a->tokens);
        else if (a->args_kind == ATTR_ARGS_EQ) {
            put(d, " = ");
            dump_expr(d, a->value);
        }
        put(d, "] ");
    }
}

static void dump_lifetime(D *d, Sym s) {
    put(d, "'");
    putsym(d, s);
}

static void dump_generic_args(D *d, const GenericArgs *ga) {
    if (ga->kind == GENERIC_ARGS_PAREN) {
        put(d, "(");
        for (uint32_t i = 0; i < ga->inputs.len; i++) {
            if (i)
                put(d, ", ");
            dump_ty(d, ga->inputs.data[i]);
        }
        put(d, ")");
        if (ga->output) {
            put(d, " -> ");
            dump_ty(d, ga->output);
        }
        return;
    }
    put(d, "<");
    for (uint32_t i = 0; i < ga->args.len; i++) {
        const GenericArg *a = ga->args.data[i];
        if (i)
            put(d, ", ");
        switch (a->kind) {
        case GA_LIFETIME: dump_lifetime(d, a->lifetime); break;
        case GA_TYPE: dump_ty(d, a->ty); break;
        case GA_CONST: put(d, "const "); dump_expr(d, a->expr); break;
        case GA_BINDING:
            putsym(d, a->name);
            if (a->name_args)
                dump_generic_args(d, a->name_args);
            put(d, " = ");
            dump_ty(d, a->ty);
            break;
        case GA_CONSTRAINT:
            putsym(d, a->name);
            put(d, ": ");
            dump_bounds(d, &a->bounds);
            break;
        }
    }
    put(d, ">");
}

static void dump_path(D *d, const QSelf *qs, const Path *p) {
    uint32_t i = 0;
    if (qs) {
        put(d, "<");
        dump_ty(d, qs->ty);
        if (qs->position > 0) {
            put(d, " as ");
            if (p->global)
                put(d, "::");
            for (; i < qs->position; i++) {
                if (i)
                    put(d, "::");
                putsym(d, p->segs.data[i]->name);
                if (p->segs.data[i]->args)
                    dump_generic_args(d, p->segs.data[i]->args);
            }
        }
        put(d, ">");
        for (; i < p->segs.len; i++) {
            put(d, "::");
            putsym(d, p->segs.data[i]->name);
            if (p->segs.data[i]->args) {
                put(d, "::");
                dump_generic_args(d, p->segs.data[i]->args);
            }
        }
        return;
    }
    if (p->global)
        put(d, "::");
    for (i = 0; i < p->segs.len; i++) {
        if (i)
            put(d, "::");
        putsym(d, p->segs.data[i]->name);
        if (p->segs.data[i]->args) {
            if (p->segs.data[i]->args->kind == GENERIC_ARGS_ANGLE)
                put(d, "::");
            dump_generic_args(d, p->segs.data[i]->args);
        }
    }
}

static void dump_for_lifetimes(D *d, const GenericParamList *l) {
    if (!l->len)
        return;
    put(d, "for<");
    for (uint32_t i = 0; i < l->len; i++) {
        if (i)
            put(d, ", ");
        dump_lifetime(d, l->data[i]->name);
    }
    put(d, "> ");
}

static void dump_bounds(D *d, const BoundList *bl) {
    for (uint32_t i = 0; i < bl->len; i++) {
        const GenericBound *b = bl->data[i];
        if (i)
            put(d, " + ");
        if (b->kind == BOUND_LIFETIME) {
            dump_lifetime(d, b->lifetime);
        } else {
            if (b->maybe)
                put(d, "?");
            dump_for_lifetimes(d, &b->for_lifetimes);
            dump_path(d, NULL, b->path);
        }
    }
}

static void dump_ty(D *d, const Ty *t) {
    switch (t->kind) {
    case TY_PATH: dump_path(d, t->path.qself, t->path.path); break;
    case TY_REF:
        put(d, "&");
        if (t->ref.lifetime) {
            dump_lifetime(d, t->ref.lifetime);
            put(d, " ");
        }
        if (t->ref.is_mut)
            put(d, "mut ");
        dump_ty(d, t->ref.elem);
        break;
    case TY_PTR:
        put(d, t->ptr.is_mut ? "*mut " : "*const ");
        dump_ty(d, t->ptr.elem);
        break;
    case TY_SLICE:
        put(d, "[");
        dump_ty(d, t->slice.elem);
        put(d, "]");
        break;
    case TY_ARRAY:
        put(d, "[");
        dump_ty(d, t->array.elem);
        put(d, "; ");
        dump_expr(d, t->array.len);
        put(d, "]");
        break;
    case TY_TUPLE:
        put(d, "(");
        for (uint32_t i = 0; i < t->tuple.elems.len; i++) {
            if (i)
                put(d, ", ");
            dump_ty(d, t->tuple.elems.data[i]);
        }
        if (t->tuple.elems.len == 1)
            put(d, ",");
        put(d, ")");
        break;
    case TY_NEVER: put(d, "!"); break;
    case TY_FN:
        dump_for_lifetimes(d, &t->fn_.for_lifetimes);
        if (t->fn_.is_unsafe)
            put(d, "unsafe ");
        if (t->fn_.abi)
            fprintf(d->out, "extern \"%s\" ", sym_str(t->fn_.abi));
        put(d, "fn(");
        for (uint32_t i = 0; i < t->fn_.params.len; i++) {
            if (i)
                put(d, ", ");
            if (t->fn_.params.data[i]->name) {
                putsym(d, t->fn_.params.data[i]->name);
                put(d, ": ");
            }
            dump_ty(d, t->fn_.params.data[i]->ty);
        }
        if (t->fn_.variadic)
            put(d, t->fn_.params.len ? ", ..." : "...");
        put(d, ")");
        if (t->fn_.ret) {
            put(d, " -> ");
            dump_ty(d, t->fn_.ret);
        }
        break;
    case TY_DYN:
        put(d, "(dyn ");
        dump_bounds(d, &t->bounds);
        put(d, ")");
        break;
    case TY_IMPL:
        put(d, "(impl ");
        dump_bounds(d, &t->bounds);
        put(d, ")");
        break;
    case TY_INFER: put(d, "_"); break;
    case TY_MAC:
        dump_path(d, NULL, t->mac->path);
        put(d, "!");
        dump_delimited(d, t->mac->delim, t->mac->tokens);
        break;
    }
}

static void dump_mac(D *d, const MacCall *m) {
    put(d, "(mac ");
    dump_path(d, NULL, m->path);
    put(d, "! ");
    dump_delimited(d, m->delim, m->tokens);
    put(d, ")");
}

static void dump_pat(D *d, const Pat *p) {
    switch (p->kind) {
    case PAT_WILD: put(d, "_"); break;
    case PAT_IDENT:
        if (!p->ident.by_ref && !p->ident.is_mut && !p->ident.sub) {
            putsym(d, p->ident.name);
            break;
        }
        put(d, "(bind ");
        if (p->ident.by_ref)
            put(d, "ref ");
        if (p->ident.is_mut)
            put(d, "mut ");
        putsym(d, p->ident.name);
        if (p->ident.sub) {
            put(d, " @ ");
            dump_pat(d, p->ident.sub);
        }
        put(d, ")");
        break;
    case PAT_LIT:
        dump_expr(d, p->lit);
        break;
    case PAT_RANGE:
        put(d, "(range ");
        if (p->range.lo)
            dump_expr(d, p->range.lo);
        else
            put(d, "_");
        put(d, p->range.end == RANGE_INCLUSIVE ? " ..= " : " .. ");
        if (p->range.hi)
            dump_expr(d, p->range.hi);
        else
            put(d, "_");
        put(d, ")");
        break;
    case PAT_PATH:
        put(d, "(path ");
        dump_path(d, p->path.qself, p->path.path);
        put(d, ")");
        break;
    case PAT_TUPLE:
    case PAT_SLICE:
    case PAT_OR:
        put(d, p->kind == PAT_TUPLE ? "(tuple" : p->kind == PAT_SLICE ? "(slice" : "(|");
        for (uint32_t i = 0; i < p->elems.len; i++) {
            put(d, " ");
            dump_pat(d, p->elems.data[i]);
        }
        put(d, ")");
        break;
    case PAT_TUPLE_STRUCT:
        put(d, "(tstruct ");
        dump_path(d, p->tuple_struct.qself, p->tuple_struct.path);
        for (uint32_t i = 0; i < p->tuple_struct.elems.len; i++) {
            put(d, " ");
            dump_pat(d, p->tuple_struct.elems.data[i]);
        }
        put(d, ")");
        break;
    case PAT_STRUCT:
        put(d, "(struct ");
        dump_path(d, p->struct_.qself, p->struct_.path);
        for (uint32_t i = 0; i < p->struct_.fields.len; i++) {
            const PatField *f = p->struct_.fields.data[i];
            put(d, " ");
            if (f->shorthand) {
                dump_pat(d, f->pat);
            } else {
                put(d, "(");
                putsym(d, f->name);
                put(d, ": ");
                dump_pat(d, f->pat);
                put(d, ")");
            }
        }
        if (p->struct_.has_rest)
            put(d, " ..");
        put(d, ")");
        break;
    case PAT_REST: put(d, ".."); break;
    case PAT_REF:
        put(d, p->ref.is_mut ? "(&mut " : "(& ");
        dump_pat(d, p->ref.pat);
        put(d, ")");
        break;
    case PAT_MAC: dump_mac(d, p->mac); break;
    }
}

static void dump_label(D *d, Sym label) {
    if (label) {
        put(d, " '");
        putsym(d, label);
    }
}

static void dump_lit(D *d, const Lit *l) { put(d, lit_to_str(l)); }

static void dump_expr_list(D *d, const ExprList *l) {
    for (uint32_t i = 0; i < l->len; i++) {
        put(d, " ");
        dump_expr(d, l->data[i]);
    }
}

static void dump_expr(D *d, const Expr *e) {
    if (e->attrs.len)
        dump_attrs_inline(d, &e->attrs);
    switch (e->kind) {
    case EXPR_LIT: dump_lit(d, e->lit); break;
    case EXPR_PATH: dump_path(d, e->path.qself, e->path.path); break;
    case EXPR_UNARY:
        fprintf(d->out, "(%s ", e->unary.op == UN_NEG ? "neg" : e->unary.op == UN_NOT ? "!" : "deref");
        dump_expr(d, e->unary.operand);
        put(d, ")");
        break;
    case EXPR_ADDR_OF:
        put(d, e->addr_of.raw ? (e->addr_of.is_mut ? "(&raw-mut " : "(&raw-const ")
                              : (e->addr_of.is_mut ? "(&mut " : "(& "));
        dump_expr(d, e->addr_of.operand);
        put(d, ")");
        break;
    case EXPR_BINARY:
        fprintf(d->out, "(%s ", binop_str(e->binary.op));
        dump_expr(d, e->binary.lhs);
        put(d, " ");
        dump_expr(d, e->binary.rhs);
        put(d, ")");
        break;
    case EXPR_ASSIGN:
    case EXPR_ASSIGN_OP:
        if (e->kind == EXPR_ASSIGN)
            put(d, "(= ");
        else
            fprintf(d->out, "(%s= ", binop_str(e->assign.op));
        dump_expr(d, e->assign.lhs);
        put(d, " ");
        dump_expr(d, e->assign.rhs);
        put(d, ")");
        break;
    case EXPR_CAST:
        put(d, "(as ");
        dump_expr(d, e->cast.expr);
        put(d, " ");
        dump_ty(d, e->cast.ty);
        put(d, ")");
        break;
    case EXPR_CALL:
        put(d, "(call ");
        dump_expr(d, e->call.callee);
        dump_expr_list(d, &e->call.args);
        put(d, ")");
        break;
    case EXPR_METHOD_CALL:
        put(d, "(method ");
        dump_expr(d, e->method_call.receiver);
        put(d, " ");
        putsym(d, e->method_call.method->name);
        if (e->method_call.method->args) {
            put(d, "::");
            dump_generic_args(d, e->method_call.method->args);
        }
        dump_expr_list(d, &e->method_call.args);
        put(d, ")");
        break;
    case EXPR_FIELD:
        put(d, "(field ");
        dump_expr(d, e->field.base);
        put(d, " ");
        putsym(d, e->field.name);
        put(d, ")");
        break;
    case EXPR_INDEX:
        put(d, "(index ");
        dump_expr(d, e->index.base);
        put(d, " ");
        dump_expr(d, e->index.index);
        put(d, ")");
        break;
    case EXPR_STRUCT:
        put(d, "(struct ");
        dump_path(d, e->struct_.qself, e->struct_.path);
        for (uint32_t i = 0; i < e->struct_.fields.len; i++) {
            const ExprField *f = e->struct_.fields.data[i];
            put(d, " (");
            putsym(d, f->name);
            if (!f->shorthand) {
                put(d, ": ");
                dump_expr(d, f->expr);
            }
            put(d, ")");
        }
        if (e->struct_.base) {
            put(d, " (.. ");
            dump_expr(d, e->struct_.base);
            put(d, ")");
        } else if (e->struct_.has_rest) {
            put(d, " ..");
        }
        put(d, ")");
        break;
    case EXPR_TUPLE:
        put(d, "(tuple");
        dump_expr_list(d, &e->elems);
        put(d, ")");
        break;
    case EXPR_ARRAY:
        put(d, "(array");
        dump_expr_list(d, &e->elems);
        put(d, ")");
        break;
    case EXPR_REPEAT:
        put(d, "(repeat ");
        dump_expr(d, e->repeat.elem);
        put(d, " ");
        dump_expr(d, e->repeat.count);
        put(d, ")");
        break;
    case EXPR_RANGE:
        put(d, e->range.inclusive ? "(..= " : "(.. ");
        if (e->range.lo)
            dump_expr(d, e->range.lo);
        else
            put(d, "_");
        put(d, " ");
        if (e->range.hi)
            dump_expr(d, e->range.hi);
        else
            put(d, "_");
        put(d, ")");
        break;
    case EXPR_CLOSURE:
        put(d, e->closure.is_move ? "(closure move (" : "(closure (");
        for (uint32_t i = 0; i < e->closure.params.len; i++) {
            const Param *pa = e->closure.params.data[i];
            if (i)
                put(d, " ");
            dump_pat(d, pa->pat);
            if (pa->ty) {
                put(d, ": ");
                dump_ty(d, pa->ty);
            }
        }
        put(d, ")");
        if (e->closure.ret) {
            put(d, " -> ");
            dump_ty(d, e->closure.ret);
        }
        put(d, " ");
        dump_expr(d, e->closure.body);
        put(d, ")");
        break;
    case EXPR_BLOCK:
        if (e->block.label) {
            put(d, "(labeled");
            dump_label(d, e->block.label);
            put(d, " ");
            dump_block(d, e->block.block);
            put(d, ")");
        } else {
            dump_block(d, e->block.block);
        }
        break;
    case EXPR_IF:
        put(d, "(if ");
        dump_expr(d, e->if_.cond);
        put(d, " ");
        dump_block(d, e->if_.then);
        if (e->if_.els) {
            put(d, " else ");
            dump_expr(d, e->if_.els);
        }
        put(d, ")");
        break;
    case EXPR_LET:
        put(d, "(let ");
        dump_pat(d, e->let.pat);
        put(d, " ");
        dump_expr(d, e->let.scrutinee);
        put(d, ")");
        break;
    case EXPR_MATCH:
        put(d, "(match ");
        dump_expr(d, e->match.scrutinee);
        d->indent++;
        for (uint32_t i = 0; i < e->match.arms.len; i++) {
            const Arm *a = e->match.arms.data[i];
            nl(d);
            dump_attrs_inline(d, &a->attrs);
            put(d, "(arm ");
            dump_pat(d, a->pat);
            if (a->guard) {
                put(d, " if ");
                dump_expr(d, a->guard);
            }
            put(d, " => ");
            dump_expr(d, a->body);
            put(d, ")");
        }
        d->indent--;
        put(d, ")");
        break;
    case EXPR_WHILE:
        put(d, "(while");
        dump_label(d, e->while_.label);
        put(d, " ");
        dump_expr(d, e->while_.cond);
        put(d, " ");
        dump_block(d, e->while_.body);
        put(d, ")");
        break;
    case EXPR_FOR:
        put(d, "(for");
        dump_label(d, e->for_.label);
        put(d, " ");
        dump_pat(d, e->for_.pat);
        put(d, " in ");
        dump_expr(d, e->for_.iter);
        put(d, " ");
        dump_block(d, e->for_.body);
        put(d, ")");
        break;
    case EXPR_LOOP:
        put(d, "(loop");
        dump_label(d, e->loop.label);
        put(d, " ");
        dump_block(d, e->loop.body);
        put(d, ")");
        break;
    case EXPR_BREAK:
        put(d, "(break");
        dump_label(d, e->break_.label);
        if (e->break_.value) {
            put(d, " ");
            dump_expr(d, e->break_.value);
        }
        put(d, ")");
        break;
    case EXPR_CONTINUE:
        put(d, "(continue");
        dump_label(d, e->continue_.label);
        put(d, ")");
        break;
    case EXPR_RETURN:
        put(d, "(return");
        if (e->ret) {
            put(d, " ");
            dump_expr(d, e->ret);
        }
        put(d, ")");
        break;
    case EXPR_TRY:
        put(d, "(? ");
        dump_expr(d, e->try_);
        put(d, ")");
        break;
    case EXPR_MAC: dump_mac(d, e->mac); break;
    case EXPR_UNDERSCORE: put(d, "_"); break;
    }
}

static void dump_block(D *d, const Block *b) {
    put(d, b->is_unsafe ? "(unsafe-block" : "(block");
    d->indent++;
    for (uint32_t i = 0; i < b->attrs.len; i++) {
        nl(d);
        AttrList one = {0};
        vec_push(one, b->attrs.data[i]);
        dump_attrs_inline(d, &one);
    }
    for (uint32_t i = 0; i < b->stmts.len; i++) {
        nl(d);
        dump_stmt(d, b->stmts.data[i]);
    }
    d->indent--;
    put(d, ")");
}

static void dump_stmt(D *d, const Stmt *s) {
    if (s->attrs.len)
        dump_attrs_inline(d, &s->attrs);
    switch (s->kind) {
    case STMT_LET:
        put(d, "(let ");
        dump_pat(d, s->let.pat);
        if (s->let.ty) {
            put(d, " : ");
            dump_ty(d, s->let.ty);
        }
        if (s->let.init) {
            put(d, " = ");
            dump_expr(d, s->let.init);
        }
        if (s->let.els) {
            put(d, " else ");
            dump_block(d, s->let.els);
        }
        put(d, ")");
        break;
    case STMT_ITEM:
        dump_item(d, s->item);
        break;
    case STMT_EXPR:
        put(d, s->expr.semi ? "(semi " : "(expr ");
        dump_expr(d, s->expr.expr);
        put(d, ")");
        break;
    case STMT_MAC:
        put(d, s->mac.semi ? "(semi " : "(expr ");
        dump_mac(d, s->mac.mac);
        put(d, ")");
        break;
    case STMT_EMPTY:
        put(d, "(empty)");
        break;
    }
}

static void dump_vis(D *d, const Visibility *v) {
    switch (v->kind) {
    case VIS_INHERITED: break;
    case VIS_PUB: put(d, "pub "); break;
    case VIS_CRATE: put(d, "pub(crate) "); break;
    case VIS_SUPER: put(d, "pub(super) "); break;
    case VIS_SELF: put(d, "pub(self) "); break;
    case VIS_IN:
        put(d, "pub(in ");
        dump_path(d, NULL, v->path);
        put(d, ") ");
        break;
    }
}

static void dump_generics(D *d, const Generics *g) {
    if (g->params.len) {
        nl(d);
        put(d, "(generics");
        for (uint32_t i = 0; i < g->params.len; i++) {
            const GenericParam *gp = g->params.data[i];
            put(d, " ");
            if (gp->attrs.len)
                dump_attrs_inline(d, &gp->attrs);
            switch (gp->kind) {
            case GP_LIFETIME:
                dump_lifetime(d, gp->name);
                if (gp->bounds.len) {
                    put(d, ": ");
                    dump_bounds(d, &gp->bounds);
                }
                break;
            case GP_TYPE:
                put(d, "(");
                putsym(d, gp->name);
                if (gp->bounds.len) {
                    put(d, ": ");
                    dump_bounds(d, &gp->bounds);
                }
                if (gp->default_ty) {
                    put(d, " = ");
                    dump_ty(d, gp->default_ty);
                }
                put(d, ")");
                break;
            case GP_CONST:
                put(d, "(const ");
                putsym(d, gp->name);
                put(d, ": ");
                dump_ty(d, gp->const_ty);
                if (gp->const_default) {
                    put(d, " = ");
                    dump_expr(d, gp->const_default);
                }
                put(d, ")");
                break;
            }
        }
        put(d, ")");
    }
    if (g->where.len) {
        nl(d);
        put(d, "(where");
        for (uint32_t i = 0; i < g->where.len; i++) {
            const WherePredicate *w = g->where.data[i];
            put(d, " (");
            if (w->kind == WP_LIFETIME) {
                dump_lifetime(d, w->lifetime);
            } else {
                dump_for_lifetimes(d, &w->for_lifetimes);
                dump_ty(d, w->ty);
            }
            put(d, ": ");
            dump_bounds(d, &w->bounds);
            put(d, ")");
        }
        put(d, ")");
    }
}

static void dump_fields(D *d, const VariantData *vd) {
    if (vd->kind == VD_UNIT)
        return;
    for (uint32_t i = 0; i < vd->fields.len; i++) {
        const FieldDef *f = vd->fields.data[i];
        nl(d);
        dump_attrs_inline(d, &f->attrs);
        put(d, "(field ");
        dump_vis(d, &f->vis);
        if (f->name) {
            putsym(d, f->name);
            put(d, ": ");
        } else {
            fprintf(d->out, "%u: ", i);
        }
        dump_ty(d, f->ty);
        put(d, ")");
    }
}

static void dump_use_tree(D *d, const UseTree *u) {
    if (u->prefix->segs.len || u->prefix->global)
        dump_path(d, NULL, u->prefix);
    switch (u->kind) {
    case USE_SIMPLE:
        if (u->rename) {
            put(d, " as ");
            putsym(d, u->rename);
        }
        break;
    case USE_GLOB:
        put(d, u->prefix->segs.len ? "::*" : "*");
        break;
    case USE_NESTED:
        put(d, u->prefix->segs.len ? "::{" : "{");
        for (uint32_t i = 0; i < u->items.len; i++) {
            if (i)
                put(d, ", ");
            dump_use_tree(d, u->items.data[i]);
        }
        put(d, "}");
        break;
    }
}

static void dump_items(D *d, const ItemList *items) {
    for (uint32_t i = 0; i < items->len; i++) {
        nl(d);
        dump_item(d, items->data[i]);
    }
}

static void dump_fn(D *d, const Item *it) {
    const FnSig *s = &it->fn_.sig;
    put(d, "(fn ");
    dump_vis(d, &it->vis);
    if (s->is_const)
        put(d, "const ");
    if (s->is_unsafe)
        put(d, "unsafe ");
    if (s->is_extern)
        fprintf(d->out, "extern \"%s\" ", sym_str(s->abi));
    putsym(d, it->name);
    d->indent++;
    dump_generics(d, &it->fn_.generics);
    nl(d);
    put(d, "(params");
    if (s->self_param) {
        const SelfParam *sp = s->self_param;
        switch (sp->kind) {
        case SELF_VALUE: put(d, sp->is_mut ? " (mut self)" : " (self)"); break;
        case SELF_REF:
            put(d, " (&");
            if (sp->lifetime) {
                dump_lifetime(d, sp->lifetime);
                put(d, " ");
            }
            put(d, sp->is_mut ? "mut self)" : "self)");
            break;
        case SELF_EXPLICIT:
            put(d, sp->is_mut ? " (mut self: " : " (self: ");
            dump_ty(d, sp->ty);
            put(d, ")");
            break;
        }
    }
    for (uint32_t i = 0; i < s->params.len; i++) {
        put(d, " (");
        dump_pat(d, s->params.data[i]->pat);
        put(d, ": ");
        dump_ty(d, s->params.data[i]->ty);
        put(d, ")");
    }
    if (s->variadic)
        put(d, " ...");
    put(d, ")");
    if (s->ret) {
        nl(d);
        put(d, "(ret ");
        dump_ty(d, s->ret);
        put(d, ")");
    }
    if (it->fn_.body) {
        nl(d);
        dump_block(d, it->fn_.body);
    }
    d->indent--;
    put(d, ")");
}

static void dump_item(D *d, const Item *it) {
    dump_attrs(d, &it->attrs);
    switch (it->kind) {
    case ITEM_USE:
        put(d, "(use ");
        dump_vis(d, &it->vis);
        dump_use_tree(d, it->use_tree);
        put(d, ")");
        break;
    case ITEM_EXTERN_CRATE:
        put(d, "(extern-crate ");
        putsym(d, it->extern_crate.orig);
        put(d, " as ");
        putsym(d, it->name);
        put(d, ")");
        break;
    case ITEM_MOD:
        put(d, "(mod ");
        dump_vis(d, &it->vis);
        putsym(d, it->name);
        if (!it->mod.inline_) {
            put(d, " <file>)");
            break;
        }
        d->indent++;
        dump_items(d, &it->mod.items);
        d->indent--;
        put(d, ")");
        break;
    case ITEM_FN: dump_fn(d, it); break;
    case ITEM_STRUCT:
    case ITEM_UNION:
        put(d, it->kind == ITEM_UNION ? "(union " : it->adt.data.kind == VD_TUPLE ? "(tuple-struct " : it->adt.data.kind == VD_UNIT ? "(unit-struct " : "(struct ");
        dump_vis(d, &it->vis);
        putsym(d, it->name);
        d->indent++;
        dump_generics(d, &it->adt.generics);
        dump_fields(d, &it->adt.data);
        d->indent--;
        put(d, ")");
        break;
    case ITEM_ENUM:
        put(d, "(enum ");
        dump_vis(d, &it->vis);
        putsym(d, it->name);
        d->indent++;
        dump_generics(d, &it->enum_.generics);
        for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
            const Variant *v = it->enum_.variants.data[i];
            nl(d);
            dump_attrs_inline(d, &v->attrs);
            put(d, v->data.kind == VD_TUPLE ? "(tuple-variant " : v->data.kind == VD_STRUCT ? "(struct-variant " : "(variant ");
            putsym(d, v->name);
            if (v->disc) {
                put(d, " = ");
                dump_expr(d, v->disc);
            }
            d->indent++;
            dump_fields(d, &v->data);
            d->indent--;
            put(d, ")");
        }
        d->indent--;
        put(d, ")");
        break;
    case ITEM_TRAIT:
        put(d, "(trait ");
        dump_vis(d, &it->vis);
        if (it->trait.is_unsafe)
            put(d, "unsafe ");
        putsym(d, it->name);
        d->indent++;
        dump_generics(d, &it->trait.generics);
        if (it->trait.supertraits.len) {
            nl(d);
            put(d, "(supertraits ");
            dump_bounds(d, &it->trait.supertraits);
            put(d, ")");
        }
        dump_items(d, &it->trait.items);
        d->indent--;
        put(d, ")");
        break;
    case ITEM_IMPL:
        put(d, "(impl ");
        if (it->impl.is_unsafe)
            put(d, "unsafe ");
        if (it->impl.trait_path) {
            if (it->impl.negative)
                put(d, "!");
            dump_path(d, NULL, it->impl.trait_path);
            put(d, " for ");
        }
        dump_ty(d, it->impl.self_ty);
        d->indent++;
        dump_generics(d, &it->impl.generics);
        dump_items(d, &it->impl.items);
        d->indent--;
        put(d, ")");
        break;
    case ITEM_TYPE_ALIAS:
        put(d, "(type ");
        dump_vis(d, &it->vis);
        putsym(d, it->name);
        if (it->type_alias.bounds.len) {
            put(d, ": ");
            dump_bounds(d, &it->type_alias.bounds);
        }
        if (it->type_alias.ty) {
            put(d, " = ");
            dump_ty(d, it->type_alias.ty);
        }
        d->indent++;
        dump_generics(d, &it->type_alias.generics);
        d->indent--;
        put(d, ")");
        break;
    case ITEM_CONST:
        put(d, "(const ");
        dump_vis(d, &it->vis);
        putsym(d, it->name);
        if (it->const_.ty) {
            put(d, ": ");
            dump_ty(d, it->const_.ty);
        }
        if (it->const_.expr) {
            put(d, " = ");
            dump_expr(d, it->const_.expr);
        }
        put(d, ")");
        break;
    case ITEM_STATIC:
        put(d, "(static ");
        dump_vis(d, &it->vis);
        if (it->static_.is_mut)
            put(d, "mut ");
        putsym(d, it->name);
        put(d, ": ");
        dump_ty(d, it->static_.ty);
        if (it->static_.expr) {
            put(d, " = ");
            dump_expr(d, it->static_.expr);
        }
        put(d, ")");
        break;
    case ITEM_FOREIGN_MOD:
        fprintf(d->out, "(extern \"%s\"", sym_str(it->foreign_mod.abi));
        d->indent++;
        dump_items(d, &it->foreign_mod.items);
        d->indent--;
        put(d, ")");
        break;
    case ITEM_MAC:
        dump_mac(d, it->mac);
        break;
    case ITEM_MACRO_RULES:
        put(d, "(macro_rules ");
        putsym(d, it->name);
        put(d, " ");
        dump_delimited(d, it->macro_rules.delim, it->macro_rules.body);
        put(d, ")");
        break;
    }
}

void ast_dump_crate(FILE *out, const Crate *c) {
    D d = {out, 0};
    put(&d, "(crate");
    d.indent++;
    for (uint32_t i = 0; i < c->attrs.len; i++) {
        nl(&d);
        AttrList one = {0};
        vec_push(one, c->attrs.data[i]);
        dump_attrs_inline(&d, &one);
    }
    dump_items(&d, &c->items);
    d.indent--;
    put(&d, ")\n");
}

void ast_dump_item(FILE *out, const Item *it) {
    D d = {out, 0};
    dump_item(&d, it);
    put(&d, "\n");
}
void ast_dump_expr(FILE *out, const Expr *e) {
    D d = {out, 0};
    dump_expr(&d, e);
    put(&d, "\n");
}
void ast_dump_ty(FILE *out, const Ty *t) {
    D d = {out, 0};
    dump_ty(&d, t);
    put(&d, "\n");
}
void ast_dump_pat(FILE *out, const Pat *p) {
    D d = {out, 0};
    dump_pat(&d, p);
    put(&d, "\n");
}
void ast_dump_stmt(FILE *out, const Stmt *s) {
    D d = {out, 0};
    dump_stmt(&d, s);
    put(&d, "\n");
}
