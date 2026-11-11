/*
 * astprint.c - print the AST as Rust source.
 *
 * The output must re-parse to the same tree (it is how the expander is
 * tested against rustc), so parentheses are inserted from operator
 * precedence, and the places where Rust's grammar is context sensitive are
 * handled explicitly:
 *   - struct literals in `if`/`while`/`match`/`for` heads are wrapped in ( ),
 *   - expression statements that start with a block-like expression
 *     (`match x {}.len();`) are wrapped in ( ),
 *   - `x as T < y` puts the cast in ( ),
 *   - a field used as a callee is wrapped: `(self.f)(x)`,
 *   - `&(mut x)` / `&(a..=b)` / `x @ (A | B)` patterns,
 *   - `&(dyn A + B)` types, 1-tuples `(a,)`.
 */
#include <stdio.h>

#include "expand_int.h"

typedef struct Printer {
    CharVec out;
    int indent;
} Printer;

static void put(Printer *p, const char *s) {
    for (; *s; s++)
        vec_push(p->out, *s);
}
static void putc_(Printer *p, char c) { vec_push(p->out, c); }
static void nl(Printer *p) {
    putc_(p, '\n');
    for (int i = 0; i < p->indent; i++)
        put(p, "    ");
}
static char last_char(Printer *p) { return p->out.len ? p->out.data[p->out.len - 1] : 0; }

static const char *pstr(Printer *p) {
    vec_push(p->out, 0);
    p->out.len--;
    return p->out.data ? p->out.data : "";
}

static void p_expr(Printer *p, const Expr *e, int min);
static void p_ty(Printer *p, const Ty *t);
static void p_pat(Printer *p, const Pat *x);
static void p_block(Printer *p, const Block *b);
static void p_item(Printer *p, const Item *it);
static void p_path(Printer *p, const Path *path, bool type_mode);
static void p_bounds(Printer *p, const BoundList *bl);
static void p_tokens(Printer *p, TokenList tl, bool exact_ws);
static void p_stmt(Printer *p, const Stmt *s, bool is_last);

static void p_ident(Printer *p, Sym s) {
    if (sym_is_strict_kw(s) && s != KW_SELF_VALUE && s != KW_SELF_TYPE &&
        s != KW_CRATE && s != KW_SUPER && s != KW_UNDERSCORE)
        put(p, "r#");
    put(p, sym_str(s));
}

static void p_lit(Printer *p, const Lit *l) {
    if (l->kind == LIT_FLOAT) {
        const char *t = l->float_text;
        put(p, t);
        bool has_dot = strchr(t, '.') || strchr(t, 'e') || strchr(t, 'E');
        size_t n = strlen(t);
        if (n && t[n - 1] == '.')
            put(p, "0");
        else if (!has_dot && !l->suffix)
            put(p, ".0");
        if (l->suffix)
            put(p, sym_str(l->suffix));
        return;
    }
    put(p, lit_to_str(l));
}

/* ------------------------------------------------------------------ */
/* Tokens                                                              */
/* ------------------------------------------------------------------ */

static bool tok_wordlike(int k) {
    return k == TK_IDENT || k == TK_LITERAL || k == TK_LIFETIME || k == TK_INTERP;
}

static void p_interp(Printer *p, const Token *t) {
    switch (t->nt_kind) {
    case NT_EXPR:
    case NT_LIT: p_expr(p, (const Expr *)t->v.node, 0); break;
    case NT_TY: p_ty(p, (const Ty *)t->v.node); break;
    case NT_PAT: p_pat(p, (const Pat *)t->v.node); break;
    case NT_STMT: p_stmt(p, (const Stmt *)t->v.node, true); break;
    case NT_ITEM: p_item(p, (const Item *)t->v.node); break;
    case NT_BLOCK: p_block(p, (const Block *)t->v.node); break;
    case NT_PATH: p_path(p, (const Path *)t->v.node, true); break;
    case NT_VIS: {
        const Visibility *v = t->v.node;
        switch (v->kind) {
        case VIS_INHERITED: break;
        case VIS_PUB: put(p, "pub"); break;
        case VIS_CRATE: put(p, "pub(crate)"); break;
        case VIS_SUPER: put(p, "pub(super)"); break;
        case VIS_SELF: put(p, "pub(self)"); break;
        case VIS_IN: put(p, "pub(in "); p_path(p, v->path, true); put(p, ")"); break;
        }
        break;
    }
    }
}

/* exact_ws: follow the source spacing (stringify!), adding a space only
   where two tokens would otherwise fuse. Otherwise always separate. */
static void p_tokens(Printer *p, TokenList tl, bool exact_ws) {
    for (uint32_t i = 0; i < tl.len; i++) {
        const Token *t = &tl.toks[i];
        if (i > 0) {
            const Token *pv = &tl.toks[i - 1];
            bool space;
            bool fuse = (tok_wordlike(pv->kind) && tok_wordlike(t->kind)) ||
                        (!tok_wordlike(pv->kind) && !tok_wordlike(t->kind) &&
                         !tok_is_open(pv->kind) && !tok_is_close(pv->kind) &&
                         !tok_is_open(t->kind) && !tok_is_close(t->kind) &&
                         pv->kind != TK_COMMA && pv->kind != TK_SEMI &&
                         t->kind != TK_COMMA && t->kind != TK_SEMI);
            if (exact_ws)
                space = (t->flags & TF_WS_BEFORE) || fuse;
            else
                space = !(t->kind == TK_COMMA || t->kind == TK_SEMI || tok_is_close(t->kind) ||
                          tok_is_open(pv->kind)) || fuse;
            if (pv->kind == TK_DOLLAR && !exact_ws)
                space = false;
            if (space)
                putc_(p, ' ');
        }
        if (t->kind == TK_INTERP)
            p_interp(p, t);
        else
            put(p, tok_to_str(t));
    }
}

static void p_delim_open(Printer *p, Delim d) {
    put(p, d == DELIM_PAREN ? "(" : d == DELIM_BRACKET ? "[" : "{");
}
static void p_delim_close(Printer *p, Delim d) {
    put(p, d == DELIM_PAREN ? ")" : d == DELIM_BRACKET ? "]" : "}");
}

static void p_mac(Printer *p, const MacCall *m) {
    p_path(p, m->path, false);
    put(p, "!");
    if (m->delim == DELIM_BRACE)
        put(p, " ");
    p_delim_open(p, m->delim);
    if (m->delim == DELIM_BRACE)
        put(p, " ");
    p_tokens(p, m->tokens, false);
    if (m->delim == DELIM_BRACE)
        put(p, " ");
    p_delim_close(p, m->delim);
}

/* ------------------------------------------------------------------ */
/* Attributes, paths, generics                                         */
/* ------------------------------------------------------------------ */

static void p_attr(Printer *p, const Attr *a) {
    put(p, a->inner ? "#![" : "#[");
    p_path(p, a->path, false);
    if (a->args_kind == ATTR_ARGS_DELIMITED) {
        p_delim_open(p, a->delim);
        p_tokens(p, a->tokens, false);
        p_delim_close(p, a->delim);
    } else if (a->args_kind == ATTR_ARGS_EQ) {
        put(p, " = ");
        if (a->value)
            p_expr(p, a->value, 0);
        else
            p_tokens(p, a->tokens, false);
    }
    put(p, "]");
}

static void p_outer_attrs(Printer *p, const AttrList *l, bool newline) {
    for (uint32_t i = 0; i < l->len; i++) {
        if (l->data[i]->inner)
            continue;
        p_attr(p, l->data[i]);
        if (newline)
            nl(p);
        else
            put(p, " ");
    }
}

static void p_inner_attrs(Printer *p, const AttrList *l) {
    for (uint32_t i = 0; i < l->len; i++) {
        if (!l->data[i]->inner)
            continue;
        nl(p);
        p_attr(p, l->data[i]);
    }
}

static void p_gargs(Printer *p, const GenericArgs *ga, bool type_mode) {
    if (ga->kind == GENERIC_ARGS_PAREN) {
        put(p, "(");
        for (uint32_t i = 0; i < ga->inputs.len; i++) {
            if (i)
                put(p, ", ");
            p_ty(p, ga->inputs.data[i]);
        }
        put(p, ")");
        if (ga->output) {
            put(p, " -> ");
            p_ty(p, ga->output);
        }
        return;
    }
    put(p, type_mode ? "<" : "::<");
    for (uint32_t i = 0; i < ga->args.len; i++) {
        const GenericArg *a = ga->args.data[i];
        if (i)
            put(p, ", ");
        switch (a->kind) {
        case GA_LIFETIME:
            put(p, "'");
            put(p, sym_str(a->lifetime));
            break;
        case GA_TYPE: p_ty(p, a->ty); break;
        case GA_CONST:
            if (a->expr->kind == EXPR_LIT ||
                (a->expr->kind == EXPR_UNARY && a->expr->unary.op == UN_NEG &&
                 a->expr->unary.operand->kind == EXPR_LIT) ||
                a->expr->kind == EXPR_BLOCK)
                p_expr(p, a->expr, 0);
            else {
                put(p, "{ ");
                p_expr(p, a->expr, 0);
                put(p, " }");
            }
            break;
        case GA_BINDING:
        case GA_CONSTRAINT:
            p_ident(p, a->name);
            if (a->name_args)
                p_gargs(p, a->name_args, true);
            if (a->kind == GA_BINDING) {
                put(p, " = ");
                p_ty(p, a->ty);
            } else {
                put(p, ": ");
                p_bounds(p, &a->bounds);
            }
            break;
        }
    }
    put(p, ">");
}

static void p_path_segs(Printer *p, const Path *path, uint32_t from, uint32_t to, bool type_mode,
                        bool leading_sep) {
    for (uint32_t i = from; i < to; i++) {
        if (i > from || leading_sep)
            put(p, "::");
        const PathSegment *s = path->segs.data[i];
        p_ident(p, s->name);
        if (s->args)
            p_gargs(p, s->args, type_mode);
    }
}

static void p_path(Printer *p, const Path *path, bool type_mode) {
    if (path->global)
        put(p, "::");
    p_path_segs(p, path, 0, path->segs.len, type_mode, false);
}

static void p_qpath(Printer *p, const QSelf *q, const Path *path, bool type_mode) {
    if (!q) {
        p_path(p, path, type_mode);
        return;
    }
    put(p, "<");
    p_ty(p, q->ty);
    if (q->position > 0) {
        put(p, " as ");
        if (path->global)
            put(p, "::");
        p_path_segs(p, path, 0, q->position, true, false);
    }
    put(p, ">");
    p_path_segs(p, path, q->position, path->segs.len, type_mode, true);
}

static void p_for_lifetimes(Printer *p, const GenericParamList *l);

static void p_bound(Printer *p, const GenericBound *b) {
    if (b->kind == BOUND_LIFETIME) {
        put(p, "'");
        put(p, sym_str(b->lifetime));
        return;
    }
    p_for_lifetimes(p, &b->for_lifetimes);
    if (b->maybe)
        put(p, "?");
    p_path(p, b->path, true);
}

static void p_bounds(Printer *p, const BoundList *bl) {
    for (uint32_t i = 0; i < bl->len; i++) {
        if (i)
            put(p, " + ");
        p_bound(p, bl->data[i]);
    }
}

static void p_gparam(Printer *p, const GenericParam *g) {
    p_outer_attrs(p, &g->attrs, false);
    switch (g->kind) {
    case GP_LIFETIME:
        put(p, "'");
        put(p, sym_str(g->name));
        if (g->bounds.len) {
            put(p, ": ");
            p_bounds(p, &g->bounds);
        }
        break;
    case GP_TYPE:
        p_ident(p, g->name);
        if (g->bounds.len) {
            put(p, ": ");
            p_bounds(p, &g->bounds);
        }
        if (g->default_ty) {
            put(p, " = ");
            p_ty(p, g->default_ty);
        }
        break;
    case GP_CONST:
        put(p, "const ");
        p_ident(p, g->name);
        put(p, ": ");
        p_ty(p, g->const_ty);
        if (g->const_default) {
            put(p, " = ");
            if (g->const_default->kind == EXPR_LIT || g->const_default->kind == EXPR_BLOCK)
                p_expr(p, g->const_default, 0);
            else {
                put(p, "{ ");
                p_expr(p, g->const_default, 0);
                put(p, " }");
            }
        }
        break;
    }
}

static void p_gparams(Printer *p, const GenericParamList *l) {
    if (!l->len)
        return;
    put(p, "<");
    for (uint32_t i = 0; i < l->len; i++) {
        if (i)
            put(p, ", ");
        p_gparam(p, l->data[i]);
    }
    put(p, ">");
}

static void p_for_lifetimes(Printer *p, const GenericParamList *l) {
    if (!l->len)
        return;
    put(p, "for");
    p_gparams(p, l);
    put(p, " ");
}

static void p_where(Printer *p, const Generics *g) {
    if (!g->where.len)
        return;
    put(p, " where ");
    for (uint32_t i = 0; i < g->where.len; i++) {
        const WherePredicate *w = g->where.data[i];
        if (i)
            put(p, ", ");
        if (w->kind == WP_LIFETIME) {
            put(p, "'");
            put(p, sym_str(w->lifetime));
        } else {
            p_for_lifetimes(p, &w->for_lifetimes);
            p_ty(p, w->ty);
        }
        put(p, ": ");
        p_bounds(p, &w->bounds);
    }
}

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

static void p_ty_elem(Printer *p, const Ty *t) {
    bool paren = (t->kind == TY_DYN || t->kind == TY_IMPL) && t->bounds.len > 1;
    if (paren)
        put(p, "(");
    p_ty(p, t);
    if (paren)
        put(p, ")");
}

static void p_ty(Printer *p, const Ty *t) {
    switch (t->kind) {
    case TY_PATH: p_qpath(p, t->path.qself, t->path.path, true); break;
    case TY_REF:
        put(p, "&");
        if (t->ref.lifetime) {
            put(p, "'");
            put(p, sym_str(t->ref.lifetime));
            put(p, " ");
        }
        if (t->ref.is_mut)
            put(p, "mut ");
        p_ty_elem(p, t->ref.elem);
        break;
    case TY_PTR:
        put(p, t->ptr.is_mut ? "*mut " : "*const ");
        p_ty_elem(p, t->ptr.elem);
        break;
    case TY_SLICE:
        put(p, "[");
        p_ty(p, t->slice.elem);
        put(p, "]");
        break;
    case TY_ARRAY:
        put(p, "[");
        p_ty(p, t->array.elem);
        put(p, "; ");
        p_expr(p, t->array.len, 0);
        put(p, "]");
        break;
    case TY_TUPLE:
        put(p, "(");
        for (uint32_t i = 0; i < t->tuple.elems.len; i++) {
            if (i)
                put(p, ", ");
            p_ty(p, t->tuple.elems.data[i]);
        }
        if (t->tuple.elems.len == 1)
            put(p, ",");
        put(p, ")");
        break;
    case TY_NEVER: put(p, "!"); break;
    case TY_FN:
        p_for_lifetimes(p, &t->fn_.for_lifetimes);
        if (t->fn_.is_unsafe)
            put(p, "unsafe ");
        if (t->fn_.abi) {
            put(p, "extern \"");
            put(p, sym_str(t->fn_.abi));
            put(p, "\" ");
        }
        put(p, "fn(");
        for (uint32_t i = 0; i < t->fn_.params.len; i++) {
            const FnPtrParam *fp = t->fn_.params.data[i];
            if (i)
                put(p, ", ");
            if (fp->name) {
                p_ident(p, fp->name);
                put(p, ": ");
            }
            p_ty(p, fp->ty);
        }
        if (t->fn_.variadic)
            put(p, t->fn_.params.len ? ", ..." : "...");
        put(p, ")");
        if (t->fn_.ret) {
            put(p, " -> ");
            p_ty_elem(p, t->fn_.ret);
        }
        break;
    case TY_DYN:
        put(p, "dyn ");
        p_bounds(p, &t->bounds);
        break;
    case TY_IMPL:
        put(p, "impl ");
        p_bounds(p, &t->bounds);
        break;
    case TY_INFER: put(p, "_"); break;
    case TY_MAC: p_mac(p, t->mac); break;
    }
}

/* ------------------------------------------------------------------ */
/* Patterns                                                            */
/* ------------------------------------------------------------------ */

static void p_pat_list(Printer *p, const PatList *l, bool tuple) {
    for (uint32_t i = 0; i < l->len; i++) {
        if (i)
            put(p, ", ");
        p_pat(p, l->data[i]);
    }
    if (tuple && l->len == 1 && l->data[0]->kind != PAT_REST)
        put(p, ",");
}

static void p_pat_paren_if(Printer *p, const Pat *x, bool paren) {
    if (paren)
        put(p, "(");
    p_pat(p, x);
    if (paren)
        put(p, ")");
}

#define P_PRIMARY_ 15
static void p_range_end(Printer *p, const Expr *e) {
    bool plain = e->kind == EXPR_LIT || e->kind == EXPR_PATH ||
                 (e->kind == EXPR_UNARY && e->unary.op == UN_NEG &&
                  e->unary.operand->kind == EXPR_LIT);
    p_expr(p, e, plain ? 0 : P_PRIMARY_);
}

static void p_pat(Printer *p, const Pat *x) {
    switch (x->kind) {
    case PAT_WILD: put(p, "_"); break;
    case PAT_IDENT:
        if (x->ident.by_ref)
            put(p, "ref ");
        if (x->ident.is_mut)
            put(p, "mut ");
        p_ident(p, x->ident.name);
        if (x->ident.sub) {
            put(p, " @ ");
            p_pat_paren_if(p, x->ident.sub, x->ident.sub->kind == PAT_OR ||
                                                x->ident.sub->kind == PAT_RANGE);
        }
        break;
    case PAT_LIT: p_expr(p, x->lit, 0); break;
    case PAT_RANGE:
        if (x->range.lo)
            p_range_end(p, x->range.lo);
        put(p, x->range.end == RANGE_INCLUSIVE ? "..=" : "..");
        if (x->range.hi)
            p_range_end(p, x->range.hi);
        break;
    case PAT_PATH: p_qpath(p, x->path.qself, x->path.path, false); break;
    case PAT_TUPLE:
        put(p, "(");
        p_pat_list(p, &x->elems, true);
        put(p, ")");
        break;
    case PAT_TUPLE_STRUCT:
        p_qpath(p, x->tuple_struct.qself, x->tuple_struct.path, false);
        put(p, "(");
        p_pat_list(p, &x->tuple_struct.elems, false);
        put(p, ")");
        break;
    case PAT_STRUCT:
        p_qpath(p, x->struct_.qself, x->struct_.path, false);
        put(p, " {");
        for (uint32_t i = 0; i < x->struct_.fields.len; i++) {
            const PatField *f = x->struct_.fields.data[i];
            put(p, i ? ", " : " ");
            p_outer_attrs(p, &f->attrs, false);
            if (f->shorthand) {
                p_pat(p, f->pat);
            } else {
                p_ident(p, f->name);
                put(p, ": ");
                p_pat(p, f->pat);
            }
        }
        if (x->struct_.has_rest)
            put(p, x->struct_.fields.len ? ", .." : " ..");
        put(p, " }");
        break;
    case PAT_SLICE:
        put(p, "[");
        p_pat_list(p, &x->elems, false);
        put(p, "]");
        break;
    case PAT_REST: put(p, ".."); break;
    case PAT_REF: {
        put(p, x->ref.is_mut ? "&mut " : "&");
        const Pat *s = x->ref.pat;
        bool paren = s->kind == PAT_OR || s->kind == PAT_RANGE ||
                     (s->kind == PAT_IDENT && (s->ident.is_mut || s->ident.by_ref) && !x->ref.is_mut) ||
                     (s->kind == PAT_LIT && s->lit->kind == EXPR_UNARY);
        p_pat_paren_if(p, s, paren);
        break;
    }
    case PAT_OR:
        for (uint32_t i = 0; i < x->elems.len; i++) {
            if (i)
                put(p, " | ");
            p_pat(p, x->elems.data[i]);
        }
        break;
    case PAT_MAC: p_mac(p, x->mac); break;
    }
}

/* Or-patterns in parameter position need parentheses. */
static void p_param_pat(Printer *p, const Pat *x) { p_pat_paren_if(p, x, x->kind == PAT_OR); }

/* ------------------------------------------------------------------ */
/* Expressions                                                         */
/* ------------------------------------------------------------------ */

enum {
    P_MIN = 0,
    P_ASSIGN = 1,
    P_RANGE = 2,
    P_OR = 3,
    P_AND = 4,
    P_CMP = 5,
    P_BITOR = 6,
    P_BITXOR = 7,
    P_BITAND = 8,
    P_SHIFT = 9,
    P_ADD = 10,
    P_MUL = 11,
    P_CAST = 12,
    P_UNARY = 13,
    P_POSTFIX = 14,
    P_PRIMARY = 15,
};

static int binop_prec(BinOp op) {
    switch (op) {
    case BIN_ADD: case BIN_SUB: return P_ADD;
    case BIN_MUL: case BIN_DIV: case BIN_REM: return P_MUL;
    case BIN_AND: return P_AND;
    case BIN_OR: return P_OR;
    case BIN_BITXOR: return P_BITXOR;
    case BIN_BITAND: return P_BITAND;
    case BIN_BITOR: return P_BITOR;
    case BIN_SHL: case BIN_SHR: return P_SHIFT;
    default: return P_CMP;
    }
}

static int expr_prec(const Expr *e) {
    switch (e->kind) {
    case EXPR_BINARY: return binop_prec(e->binary.op);
    case EXPR_ASSIGN:
    case EXPR_ASSIGN_OP: return P_ASSIGN;
    case EXPR_RANGE: return P_RANGE;
    case EXPR_CAST: return P_CAST;
    case EXPR_UNARY:
    case EXPR_ADDR_OF: return P_UNARY;
    case EXPR_CALL:
    case EXPR_METHOD_CALL:
    case EXPR_FIELD:
    case EXPR_INDEX:
    case EXPR_TRY: return P_POSTFIX;
    case EXPR_CLOSURE:
    case EXPR_RETURN:
    case EXPR_BREAK:
    case EXPR_LET: return P_MIN;
    default: return P_PRIMARY;
    }
}

static bool is_blocklike(const Expr *e) {
    switch (e->kind) {
    case EXPR_BLOCK: case EXPR_IF: case EXPR_MATCH: case EXPR_LOOP:
    case EXPR_WHILE: case EXPR_FOR:
        return true;
    case EXPR_MAC:
        return e->mac->delim == DELIM_BRACE;
    default:
        return false;
    }
}

/* Leftmost sub-expression printed first without its own parentheses. */
static const Expr *leftmost(const Expr *e) {
    for (;;) {
        const Expr *n = NULL;
        switch (e->kind) {
        case EXPR_BINARY: n = e->binary.lhs; break;
        case EXPR_ASSIGN:
        case EXPR_ASSIGN_OP: n = e->assign.lhs; break;
        case EXPR_CAST: n = e->cast.expr; break;
        case EXPR_CALL: n = e->call.callee; break;
        case EXPR_METHOD_CALL: n = e->method_call.receiver; break;
        case EXPR_FIELD: n = e->field.base; break;
        case EXPR_INDEX: n = e->index.base; break;
        case EXPR_TRY: n = e->try_; break;
        case EXPR_RANGE: n = e->range.lo; break;
        default: break;
        }
        if (!n)
            return e;
        /* if the child gets parenthesised by precedence, stop */
        int need = P_POSTFIX;
        if (e->kind == EXPR_BINARY)
            need = binop_prec(e->binary.op);
        else if (e->kind == EXPR_ASSIGN || e->kind == EXPR_ASSIGN_OP)
            need = P_ASSIGN + 1;
        else if (e->kind == EXPR_CAST)
            need = P_CAST;
        else if (e->kind == EXPR_RANGE)
            need = P_RANGE + 1;
        if (expr_prec(n) < need)
            return e;
        e = n;
    }
}

static bool starts_blocklike(const Expr *e) {
    const Expr *l = leftmost(e);
    return l != e && is_blocklike(l);
}

/* Does an unparenthesised struct literal appear at the top of e? */
static bool has_struct_head(const Expr *e) {
    if (!e)
        return false;
    switch (e->kind) {
    case EXPR_STRUCT: return true;
    case EXPR_BINARY: return has_struct_head(e->binary.lhs) || has_struct_head(e->binary.rhs);
    case EXPR_ASSIGN:
    case EXPR_ASSIGN_OP: return has_struct_head(e->assign.lhs) || has_struct_head(e->assign.rhs);
    case EXPR_UNARY: return has_struct_head(e->unary.operand);
    case EXPR_ADDR_OF: return has_struct_head(e->addr_of.operand);
    case EXPR_CAST: return has_struct_head(e->cast.expr);
    case EXPR_CALL: return has_struct_head(e->call.callee);
    case EXPR_METHOD_CALL: return has_struct_head(e->method_call.receiver);
    case EXPR_FIELD: return has_struct_head(e->field.base);
    case EXPR_INDEX: return has_struct_head(e->index.base);
    case EXPR_TRY: return has_struct_head(e->try_);
    case EXPR_RANGE: return has_struct_head(e->range.lo) || has_struct_head(e->range.hi);
    case EXPR_LET: return has_struct_head(e->let.scrutinee);
    case EXPR_RETURN: return has_struct_head(e->ret);
    case EXPR_BREAK: return has_struct_head(e->break_.value);
    case EXPR_CLOSURE: return has_struct_head(e->closure.body);
    default: return false;
    }
}

/* Expression in a no-struct-literal head position. */
static void p_head(Printer *p, const Expr *e) {
    if (e->kind == EXPR_LET) {
        put(p, "let ");
        p_pat(p, e->let.pat);
        put(p, " = ");
        const Expr *s = e->let.scrutinee;
        bool paren = has_struct_head(s) || expr_prec(s) <= P_AND;
        if (paren)
            put(p, "(");
        p_expr(p, s, 0);
        if (paren)
            put(p, ")");
        return;
    }
    if (e->kind == EXPR_BINARY && e->binary.op == BIN_AND &&
        (e->binary.lhs->kind == EXPR_LET || e->binary.rhs->kind == EXPR_LET)) {
        /* let chain */
        p_head(p, e->binary.lhs);
        put(p, " && ");
        p_head(p, e->binary.rhs);
        return;
    }
    bool paren = has_struct_head(e);
    if (paren)
        put(p, "(");
    p_expr(p, e, 0);
    if (paren)
        put(p, ")");
}

static void p_label(Printer *p, Sym label) {
    if (label) {
        put(p, "'");
        put(p, sym_str(label));
        put(p, ": ");
    }
}

static void p_args(Printer *p, const ExprList *l) {
    put(p, "(");
    for (uint32_t i = 0; i < l->len; i++) {
        if (i)
            put(p, ", ");
        p_outer_attrs(p, &l->data[i]->attrs, false);
        p_expr(p, l->data[i], 0);
    }
    put(p, ")");
}

static void p_arm_body(Printer *p, const Expr *e) {
    bool paren = starts_blocklike(e);
    if (paren)
        put(p, "(");
    p_expr(p, e, 0);
    if (paren)
        put(p, ")");
}

static void p_expr_inner(Printer *p, const Expr *e) {
    switch (e->kind) {
    case EXPR_LIT: p_lit(p, e->lit); break;
    case EXPR_PATH: p_qpath(p, e->path.qself, e->path.path, false); break;
    case EXPR_UNARY:
        put(p, e->unary.op == UN_NEG ? "-" : e->unary.op == UN_NOT ? "!" : "*");
        p_expr(p, e->unary.operand, P_UNARY);
        break;
    case EXPR_ADDR_OF:
        put(p, "&");
        if (e->addr_of.raw)
            put(p, e->addr_of.is_mut ? "raw mut " : "raw const ");
        else if (e->addr_of.is_mut)
            put(p, "mut ");
        p_expr(p, e->addr_of.operand, P_UNARY);
        break;
    case EXPR_BINARY: {
        int pr = binop_prec(e->binary.op);
        const Expr *l = e->binary.lhs;
        bool lparen = expr_prec(l) < (pr == P_CMP ? pr + 1 : pr);
        if (l->kind == EXPR_CAST && (pr == P_CMP || pr == P_SHIFT))
            lparen = true;
        if (lparen)
            put(p, "(");
        p_expr(p, l, 0);
        if (lparen)
            put(p, ")");
        put(p, " ");
        put(p, binop_str(e->binary.op));
        put(p, " ");
        p_expr(p, e->binary.rhs, pr + 1);
        break;
    }
    case EXPR_ASSIGN:
    case EXPR_ASSIGN_OP:
        p_expr(p, e->assign.lhs, P_ASSIGN + 1);
        if (e->kind == EXPR_ASSIGN)
            put(p, " = ");
        else {
            put(p, " ");
            put(p, binop_str(e->assign.op));
            put(p, "= ");
        }
        p_expr(p, e->assign.rhs, P_ASSIGN);
        break;
    case EXPR_CAST:
        p_expr(p, e->cast.expr, P_CAST);
        put(p, " as ");
        p_ty(p, e->cast.ty);
        break;
    case EXPR_CALL: {
        const Expr *c = e->call.callee;
        bool paren = c->kind == EXPR_FIELD || expr_prec(c) < P_POSTFIX;
        if (paren)
            put(p, "(");
        p_expr(p, c, 0);
        if (paren)
            put(p, ")");
        p_args(p, &e->call.args);
        break;
    }
    case EXPR_METHOD_CALL: {
        p_expr(p, e->method_call.receiver, P_POSTFIX);
        put(p, ".");
        const PathSegment *m = e->method_call.method;
        p_ident(p, m->name);
        if (m->args)
            p_gargs(p, m->args, false);
        p_args(p, &e->method_call.args);
        break;
    }
    case EXPR_FIELD: {
        const Expr *b = e->field.base;
        bool paren = expr_prec(b) < P_POSTFIX ||
                     (e->field.is_index && b->kind == EXPR_FIELD && b->field.is_index) ||
                     (b->kind == EXPR_LIT && (b->lit->kind == LIT_INT || b->lit->kind == LIT_FLOAT));
        if (paren)
            put(p, "(");
        p_expr(p, b, 0);
        if (paren)
            put(p, ")");
        put(p, ".");
        if (e->field.is_index) {
            char buf[16];
            snprintf(buf, sizeof buf, "%u", e->field.index);
            put(p, buf);
        } else {
            p_ident(p, e->field.name);
        }
        break;
    }
    case EXPR_INDEX:
        p_expr(p, e->index.base, P_POSTFIX);
        put(p, "[");
        p_expr(p, e->index.index, 0);
        put(p, "]");
        break;
    case EXPR_STRUCT:
        p_qpath(p, e->struct_.qself, e->struct_.path, false);
        put(p, " {");
        for (uint32_t i = 0; i < e->struct_.fields.len; i++) {
            const ExprField *f = e->struct_.fields.data[i];
            put(p, i ? ", " : " ");
            p_outer_attrs(p, &f->attrs, false);
            bool sh = f->shorthand && f->expr->kind == EXPR_PATH && !f->expr->path.qself &&
                      f->expr->path.path->segs.len == 1 &&
                      f->expr->path.path->segs.data[0]->name == f->name &&
                      !f->expr->path.path->segs.data[0]->args;
            p_ident(p, f->name);
            if (!sh) {
                put(p, ": ");
                p_expr(p, f->expr, 0);
            }
        }
        if (e->struct_.base) {
            put(p, e->struct_.fields.len ? ", .." : " ..");
            p_expr(p, e->struct_.base, 0);
        } else if (e->struct_.has_rest) {
            put(p, e->struct_.fields.len ? ", .." : " ..");
        }
        put(p, " }");
        break;
    case EXPR_TUPLE:
        put(p, "(");
        for (uint32_t i = 0; i < e->elems.len; i++) {
            if (i)
                put(p, ", ");
            p_outer_attrs(p, &e->elems.data[i]->attrs, false);
            p_expr(p, e->elems.data[i], 0);
        }
        if (e->elems.len == 1)
            put(p, ",");
        put(p, ")");
        break;
    case EXPR_ARRAY:
        put(p, "[");
        for (uint32_t i = 0; i < e->elems.len; i++) {
            if (i)
                put(p, ", ");
            p_outer_attrs(p, &e->elems.data[i]->attrs, false);
            p_expr(p, e->elems.data[i], 0);
        }
        put(p, "]");
        break;
    case EXPR_REPEAT:
        put(p, "[");
        p_expr(p, e->repeat.elem, 0);
        put(p, "; ");
        p_expr(p, e->repeat.count, 0);
        put(p, "]");
        break;
    case EXPR_RANGE:
        if (e->range.lo)
            p_expr(p, e->range.lo, P_RANGE + 1);
        put(p, e->range.inclusive ? "..=" : "..");
        if (e->range.hi)
            p_expr(p, e->range.hi, P_RANGE + 1);
        break;
    case EXPR_CLOSURE:
        if (e->closure.is_move)
            put(p, "move ");
        put(p, "|");
        for (uint32_t i = 0; i < e->closure.params.len; i++) {
            const Param *pa = e->closure.params.data[i];
            if (i)
                put(p, ", ");
            p_outer_attrs(p, &pa->attrs, false);
            p_param_pat(p, pa->pat);
            if (pa->ty) {
                put(p, ": ");
                p_ty(p, pa->ty);
            }
        }
        put(p, "| ");
        if (e->closure.ret) {
            put(p, "-> ");
            p_ty(p, e->closure.ret);
            put(p, " ");
            if (e->closure.body->kind != EXPR_BLOCK || e->closure.body->block.label) {
                put(p, "{ ");
                p_expr(p, e->closure.body, 0);
                put(p, " }");
                break;
            }
        }
        p_expr(p, e->closure.body, 0);
        break;
    case EXPR_BLOCK:
        p_label(p, e->block.label);
        p_block(p, e->block.block);
        break;
    case EXPR_IF:
        put(p, "if ");
        p_head(p, e->if_.cond);
        put(p, " ");
        p_block(p, e->if_.then);
        if (e->if_.els) {
            put(p, " else ");
            const Expr *el = e->if_.els;
            if (el->kind == EXPR_IF || (el->kind == EXPR_BLOCK && !el->block.label))
                p_expr_inner(p, el);
            else {
                put(p, "{ ");
                p_expr(p, el, 0);
                put(p, " }");
            }
        }
        break;
    case EXPR_LET:
        p_head(p, e);
        break;
    case EXPR_MATCH:
        put(p, "match ");
        p_head(p, e->match.scrutinee);
        put(p, " {");
        p->indent++;
        for (uint32_t i = 0; i < e->match.arms.len; i++) {
            const Arm *a = e->match.arms.data[i];
            nl(p);
            p_outer_attrs(p, &a->attrs, false);
            p_pat(p, a->pat);
            if (a->guard) {
                put(p, " if ");
                p_expr(p, a->guard, 0);
            }
            put(p, " => ");
            p_arm_body(p, a->body);
            put(p, ",");
        }
        p->indent--;
        nl(p);
        put(p, "}");
        break;
    case EXPR_WHILE:
        p_label(p, e->while_.label);
        put(p, "while ");
        p_head(p, e->while_.cond);
        put(p, " ");
        p_block(p, e->while_.body);
        break;
    case EXPR_FOR:
        p_label(p, e->for_.label);
        put(p, "for ");
        p_pat(p, e->for_.pat);
        put(p, " in ");
        p_head(p, e->for_.iter);
        put(p, " ");
        p_block(p, e->for_.body);
        break;
    case EXPR_LOOP:
        p_label(p, e->loop.label);
        put(p, "loop ");
        p_block(p, e->loop.body);
        break;
    case EXPR_BREAK:
        put(p, "break");
        if (e->break_.label) {
            put(p, " '");
            put(p, sym_str(e->break_.label));
        }
        if (e->break_.value) {
            put(p, " ");
            bool paren = !e->break_.label && e->break_.value->kind == EXPR_BLOCK &&
                         e->break_.value->block.label;
            if (paren)
                put(p, "(");
            p_expr(p, e->break_.value, 0);
            if (paren)
                put(p, ")");
        }
        break;
    case EXPR_CONTINUE:
        put(p, "continue");
        if (e->continue_.label) {
            put(p, " '");
            put(p, sym_str(e->continue_.label));
        }
        break;
    case EXPR_RETURN:
        put(p, "return");
        if (e->ret) {
            put(p, " ");
            p_expr(p, e->ret, 0);
        }
        break;
    case EXPR_TRY:
        p_expr(p, e->try_, P_POSTFIX);
        put(p, "?");
        break;
    case EXPR_MAC: p_mac(p, e->mac); break;
    case EXPR_UNDERSCORE: put(p, "_"); break;
    }
}

static void p_expr(Printer *p, const Expr *e, int min) {
    bool paren = expr_prec(e) < min;
    if (paren)
        put(p, "(");
    if (e->attrs.len && !paren && min > 0) {
        /* attributes on a nested operand: keep them attached */
        put(p, "(");
        p_outer_attrs(p, &e->attrs, false);
        p_expr_inner(p, e);
        put(p, ")");
        return;
    }
    p_expr_inner(p, e);
    if (paren)
        put(p, ")");
}

/* ------------------------------------------------------------------ */
/* Statements and blocks                                               */
/* ------------------------------------------------------------------ */

static void p_stmt(Printer *p, const Stmt *s, bool is_last) {
    switch (s->kind) {
    case STMT_LET: {
        p_outer_attrs(p, &s->attrs, true);
        put(p, "let ");
        p_param_pat(p, s->let.pat);
        if (s->let.ty) {
            put(p, ": ");
            p_ty(p, s->let.ty);
        }
        if (s->let.init) {
            put(p, " = ");
            if (s->let.els) {
                Printer tmp = {{0}, p->indent};
                p_expr(&tmp, s->let.init, 0);
                bool paren = last_char(&tmp) == '}';
                if (paren)
                    put(p, "(");
                put(p, pstr(&tmp));
                if (paren)
                    put(p, ")");
                put(p, " else ");
                p_block(p, s->let.els);
            } else {
                p_expr(p, s->let.init, 0);
            }
        }
        put(p, ";");
        break;
    }
    case STMT_ITEM: p_item(p, s->item); break;
    case STMT_EXPR: {
        p_outer_attrs(p, &s->attrs, true);
        const Expr *e = s->expr.expr;
        p_outer_attrs(p, &e->attrs, true);
        bool paren = starts_blocklike(e);
        if (paren)
            put(p, "(");
        p_expr_inner(p, e);
        if (paren)
            put(p, ")");
        if (s->expr.semi || (!is_last && !is_blocklike(e)))
            put(p, ";");
        break;
    }
    case STMT_MAC:
        p_outer_attrs(p, &s->attrs, true);
        p_mac(p, s->mac.mac);
        if (s->mac.semi || (!is_last && s->mac.mac->delim != DELIM_BRACE))
            put(p, ";");
        break;
    case STMT_EMPTY: put(p, ";"); break;
    }
}

static void p_block(Printer *p, const Block *b) {
    if (b->is_unsafe)
        put(p, "unsafe ");
    put(p, "{");
    p->indent++;
    p_inner_attrs(p, &b->attrs);
    for (uint32_t i = 0; i < b->stmts.len; i++) {
        nl(p);
        p_stmt(p, b->stmts.data[i], i + 1 == b->stmts.len);
    }
    p->indent--;
    if (b->stmts.len || b->attrs.len)
        nl(p);
    put(p, "}");
}

/* ------------------------------------------------------------------ */
/* Items                                                               */
/* ------------------------------------------------------------------ */

static void p_vis(Printer *p, const Visibility *v) {
    switch (v->kind) {
    case VIS_INHERITED: break;
    case VIS_PUB: put(p, "pub "); break;
    case VIS_CRATE: put(p, "pub(crate) "); break;
    case VIS_SUPER: put(p, "pub(super) "); break;
    case VIS_SELF: put(p, "pub(self) "); break;
    case VIS_IN:
        put(p, "pub(in ");
        p_path(p, v->path, true);
        put(p, ") ");
        break;
    }
}

static void p_fields(Printer *p, const VariantData *d) {
    if (d->kind == VD_UNIT)
        return;
    if (d->kind == VD_TUPLE) {
        put(p, "(");
        for (uint32_t i = 0; i < d->fields.len; i++) {
            const FieldDef *f = d->fields.data[i];
            if (i)
                put(p, ", ");
            p_outer_attrs(p, &f->attrs, false);
            p_vis(p, &f->vis);
            p_ty(p, f->ty);
        }
        put(p, ")");
        return;
    }
    put(p, " {");
    p->indent++;
    for (uint32_t i = 0; i < d->fields.len; i++) {
        const FieldDef *f = d->fields.data[i];
        nl(p);
        p_outer_attrs(p, &f->attrs, true);
        p_vis(p, &f->vis);
        p_ident(p, f->name);
        put(p, ": ");
        p_ty(p, f->ty);
        put(p, ",");
    }
    p->indent--;
    if (d->fields.len)
        nl(p);
    put(p, "}");
}

static void p_use_tree(Printer *p, const UseTree *u) {
    if (u->prefix) {
        if (u->prefix->global)
            put(p, "::");
        for (uint32_t i = 0; i < u->prefix->segs.len; i++) {
            if (i)
                put(p, "::");
            p_ident(p, u->prefix->segs.data[i]->name);
        }
    }
    bool has_prefix = u->prefix && u->prefix->segs.len;
    switch (u->kind) {
    case USE_SIMPLE:
        if (u->rename) {
            put(p, " as ");
            p_ident(p, u->rename);
        }
        break;
    case USE_GLOB: put(p, has_prefix ? "::*" : "*"); break;
    case USE_NESTED:
        put(p, has_prefix ? "::{" : "{");
        for (uint32_t i = 0; i < u->items.len; i++) {
            if (i)
                put(p, ", ");
            p_use_tree(p, u->items.data[i]);
        }
        put(p, "}");
        break;
    }
}

static void p_items_body(Printer *p, const AttrList *inner, const ItemList *items) {
    put(p, "{");
    p->indent++;
    if (inner)
        p_inner_attrs(p, inner);
    for (uint32_t i = 0; i < items->len; i++) {
        nl(p);
        p_item(p, items->data[i]);
    }
    p->indent--;
    nl(p);
    put(p, "}");
}

static void p_fn(Printer *p, const Item *it) {
    const FnSig *s = &it->fn_.sig;
    if (s->is_const)
        put(p, "const ");
    if (s->is_unsafe)
        put(p, "unsafe ");
    if (s->is_extern) {
        put(p, "extern \"");
        put(p, sym_str(s->abi));
        put(p, "\" ");
    }
    put(p, "fn ");
    p_ident(p, it->name);
    p_gparams(p, &it->fn_.generics.params);
    put(p, "(");
    bool first = true;
    if (s->self_param) {
        const SelfParam *sp = s->self_param;
        p_outer_attrs(p, &sp->attrs, false);
        switch (sp->kind) {
        case SELF_VALUE:
            put(p, sp->is_mut ? "mut self" : "self");
            break;
        case SELF_REF:
            put(p, "&");
            if (sp->lifetime) {
                put(p, "'");
                put(p, sym_str(sp->lifetime));
                put(p, " ");
            }
            put(p, sp->is_mut ? "mut self" : "self");
            break;
        case SELF_EXPLICIT:
            put(p, sp->is_mut ? "mut self: " : "self: ");
            p_ty(p, sp->ty);
            break;
        }
        first = false;
    }
    for (uint32_t i = 0; i < s->params.len; i++) {
        const Param *pa = s->params.data[i];
        if (!first)
            put(p, ", ");
        first = false;
        p_outer_attrs(p, &pa->attrs, false);
        p_param_pat(p, pa->pat);
        put(p, ": ");
        p_ty(p, pa->ty);
    }
    if (s->variadic)
        put(p, first ? "..." : ", ...");
    put(p, ")");
    if (s->ret) {
        put(p, " -> ");
        p_ty(p, s->ret);
    }
    p_where(p, &it->fn_.generics);
    if (it->fn_.body) {
        put(p, " ");
        p_block(p, it->fn_.body);
    } else {
        put(p, ";");
    }
}

static void p_item(Printer *p, const Item *it) {
    p_outer_attrs(p, &it->attrs, true);
    if (it->kind != ITEM_MAC && it->kind != ITEM_MACRO_RULES && it->kind != ITEM_IMPL)
        p_vis(p, &it->vis);
    switch (it->kind) {
    case ITEM_USE:
        put(p, "use ");
        p_use_tree(p, it->use_tree);
        put(p, ";");
        break;
    case ITEM_EXTERN_CRATE:
        put(p, "extern crate ");
        p_ident(p, it->extern_crate.orig);
        if (it->name != it->extern_crate.orig) {
            put(p, " as ");
            p_ident(p, it->name);
        }
        put(p, ";");
        break;
    case ITEM_MOD:
        if (it->mod.is_unsafe)
            put(p, "unsafe ");
        put(p, "mod ");
        p_ident(p, it->name);
        if (!it->mod.inline_) {
            put(p, ";");
            break;
        }
        put(p, " ");
        p_items_body(p, &it->attrs, &it->mod.items);
        break;
    case ITEM_FN: p_fn(p, it); break;
    case ITEM_STRUCT:
    case ITEM_UNION:
        put(p, it->kind == ITEM_STRUCT ? "struct " : "union ");
        p_ident(p, it->name);
        p_gparams(p, &it->adt.generics.params);
        if (it->adt.data.kind == VD_STRUCT) {
            p_where(p, &it->adt.generics);
            p_fields(p, &it->adt.data);
        } else {
            p_fields(p, &it->adt.data);
            p_where(p, &it->adt.generics);
            put(p, ";");
        }
        break;
    case ITEM_ENUM:
        put(p, "enum ");
        p_ident(p, it->name);
        p_gparams(p, &it->enum_.generics.params);
        p_where(p, &it->enum_.generics);
        put(p, " {");
        p->indent++;
        for (uint32_t i = 0; i < it->enum_.variants.len; i++) {
            const Variant *v = it->enum_.variants.data[i];
            nl(p);
            p_outer_attrs(p, &v->attrs, true);
            p_ident(p, v->name);
            p_fields(p, &v->data);
            if (v->disc) {
                put(p, " = ");
                p_expr(p, v->disc, 0);
            }
            put(p, ",");
        }
        p->indent--;
        nl(p);
        put(p, "}");
        break;
    case ITEM_TRAIT:
        if (it->trait.is_unsafe)
            put(p, "unsafe ");
        if (it->trait.is_auto)
            put(p, "auto ");
        put(p, "trait ");
        p_ident(p, it->name);
        p_gparams(p, &it->trait.generics.params);
        if (it->trait.supertraits.len) {
            put(p, ": ");
            p_bounds(p, &it->trait.supertraits);
        }
        p_where(p, &it->trait.generics);
        put(p, " ");
        p_items_body(p, NULL, &it->trait.items);
        break;
    case ITEM_IMPL:
        if (it->impl.is_unsafe)
            put(p, "unsafe ");
        put(p, "impl");
        p_gparams(p, &it->impl.generics.params);
        put(p, " ");
        if (it->impl.trait_path) {
            if (it->impl.negative)
                put(p, "!");
            p_path(p, it->impl.trait_path, true);
            put(p, " for ");
        }
        p_ty(p, it->impl.self_ty);
        p_where(p, &it->impl.generics);
        put(p, " ");
        p_items_body(p, NULL, &it->impl.items);
        break;
    case ITEM_TYPE_ALIAS:
        put(p, "type ");
        p_ident(p, it->name);
        p_gparams(p, &it->type_alias.generics.params);
        if (it->type_alias.bounds.len) {
            put(p, ": ");
            p_bounds(p, &it->type_alias.bounds);
        }
        p_where(p, &it->type_alias.generics);
        if (it->type_alias.ty) {
            put(p, " = ");
            p_ty(p, it->type_alias.ty);
        }
        put(p, ";");
        break;
    case ITEM_CONST:
        put(p, "const ");
        p_ident(p, it->name);
        if (it->const_.ty) {
            put(p, ": ");
            p_ty(p, it->const_.ty);
        }
        if (it->const_.expr) {
            put(p, " = ");
            p_expr(p, it->const_.expr, 0);
        }
        put(p, ";");
        break;
    case ITEM_STATIC:
        put(p, it->static_.is_mut ? "static mut " : "static ");
        p_ident(p, it->name);
        put(p, ": ");
        p_ty(p, it->static_.ty);
        if (it->static_.expr) {
            put(p, " = ");
            p_expr(p, it->static_.expr, 0);
        }
        put(p, ";");
        break;
    case ITEM_FOREIGN_MOD:
        if (it->foreign_mod.is_unsafe)
            put(p, "unsafe ");
        put(p, "extern \"");
        put(p, sym_str(it->foreign_mod.abi));
        put(p, "\" ");
        p_items_body(p, &it->attrs, &it->foreign_mod.items);
        break;
    case ITEM_MAC:
        p_mac(p, it->mac);
        if (it->mac->delim != DELIM_BRACE)
            put(p, ";");
        break;
    case ITEM_MACRO_RULES:
        put(p, "macro_rules! ");
        p_ident(p, it->name);
        put(p, " ");
        p_delim_open(p, it->macro_rules.delim);
        put(p, " ");
        p_tokens(p, it->macro_rules.body, false);
        put(p, " ");
        p_delim_close(p, it->macro_rules.delim);
        if (it->macro_rules.delim != DELIM_BRACE)
            put(p, ";");
        break;
    }
}

/* ------------------------------------------------------------------ */
/* Entry points                                                        */
/* ------------------------------------------------------------------ */

void ast_print_crate(FILE *out, const Crate *c) {
    Printer p = {{0}, 0};
    for (uint32_t i = 0; i < c->attrs.len; i++) {
        Attr a = *c->attrs.data[i];
        a.inner = true;
        p_attr(&p, &a);
        put(&p, "\n");
    }
    for (uint32_t i = 0; i < c->items.len; i++) {
        p_item(&p, c->items.data[i]);
        put(&p, "\n");
    }
    fwrite(p.out.data, 1, p.out.len, out);
}

const char *ast_expr_to_str(const Expr *e) {
    Printer p = {{0}, 0};
    p_expr(&p, e, 0);
    return pstr(&p);
}

const char *ast_ty_to_str(const Ty *t) {
    Printer p = {{0}, 0};
    p_ty(&p, t);
    return pstr(&p);
}

const char *ast_pat_to_str(const Pat *x) {
    Printer p = {{0}, 0};
    p_pat(&p, x);
    return pstr(&p);
}

const char *ast_path_to_str(const Path *path) {
    Printer p = {{0}, 0};
    p_path(&p, path, true);
    return pstr(&p);
}

const char *ast_tokens_to_str(TokenList tl) {
    Printer p = {{0}, 0};
    p_tokens(&p, tl, true);
    return pstr(&p);
}

const char *ast_bounds_to_str(const BoundList *bl) {
    Printer p = {{0}, 0};
    p_bounds(&p, bl);
    return pstr(&p);
}

const char *ast_where_to_str(const Generics *g) {
    Printer p = {{0}, 0};
    p_where(&p, g);
    return pstr(&p);
}
