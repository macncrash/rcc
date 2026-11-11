/* parse.c - recursive-descent / precedence-climbing Rust parser. */
#include "parse.h"

#include <stdio.h>
#include <stdlib.h>

/* ------------------------------------------------------------------ */
/* AST helpers                                                         */
/* ------------------------------------------------------------------ */

static NodeId next_node_id = 1;

NodeId ast_fresh_id(void) { return next_node_id++; }

Expr *ast_new_expr(ExprKind kind, Span sp) {
    Expr *e = NEW(Expr);
    e->kind = kind;
    e->span = sp;
    e->id = ast_fresh_id();
    return e;
}
Ty *ast_new_ty(TyKind kind, Span sp) {
    Ty *t = NEW(Ty);
    t->kind = kind;
    t->span = sp;
    t->id = ast_fresh_id();
    return t;
}
Pat *ast_new_pat(PatKind kind, Span sp) {
    Pat *t = NEW(Pat);
    t->kind = kind;
    t->span = sp;
    t->id = ast_fresh_id();
    return t;
}
Stmt *ast_new_stmt(StmtKind kind, Span sp) {
    Stmt *t = NEW(Stmt);
    t->kind = kind;
    t->span = sp;
    t->id = ast_fresh_id();
    return t;
}
Item *ast_new_item(ItemKind kind, Span sp) {
    Item *t = NEW(Item);
    t->kind = kind;
    t->span = sp;
    t->id = ast_fresh_id();
    t->vis.span = sp;
    return t;
}
Block *ast_new_block(Span sp) {
    Block *b = NEW(Block);
    b->span = sp;
    b->id = ast_fresh_id();
    return b;
}
Path *ast_new_path(Span sp) {
    Path *p = NEW(Path);
    p->span = sp;
    p->id = ast_fresh_id();
    return p;
}
Path *ast_path_from_sym(Sym name, Span sp) {
    Path *p = ast_new_path(sp);
    PathSegment *s = NEW(PathSegment);
    s->span = sp;
    s->id = ast_fresh_id();
    s->name = name;
    vec_push(p->segs, s);
    return p;
}

Expr *block_tail(const Block *b) {
    if (!b || b->stmts.len == 0)
        return NULL;
    Stmt *s = vec_last(b->stmts);
    if (s->kind == STMT_EXPR && !s->expr.semi)
        return s->expr.expr;
    return NULL;
}

const char *binop_str(BinOp op) {
    static const char *s[] = {"+", "-", "*", "/", "%", "&&", "||", "^", "&",
                              "|", "<<", ">>", "==", "<", "<=", "!=", ">=", ">"};
    return (unsigned)op < sizeof s / sizeof s[0] ? s[op] : "?";
}
const char *unop_str(UnOp op) {
    return op == UN_NEG ? "-" : op == UN_NOT ? "!" : "*";
}

/* ------------------------------------------------------------------ */
/* Parser state                                                        */
/* ------------------------------------------------------------------ */

struct Parser {
    const Token *toks;
    uint32_t n;
    uint32_t pos;
    bool has_split;
    Token split; /* remainder of toks[pos] after splitting off a prefix */
    Token eof;
    Span prev; /* span of the last consumed token */
};

/* Expression restrictions */
enum {
    R_NO_STRUCT = 1, /* no struct literal (if/while/match/for heads) */
};

/* Precedence levels (higher binds tighter) */
enum {
    PREC_ASSIGN = 1,
    PREC_RANGE = 2,
    PREC_OR = 3,
    PREC_AND = 4,
    PREC_CMP = 5,
    PREC_BITOR = 6,
    PREC_BITXOR = 7,
    PREC_BITAND = 8,
    PREC_SHIFT = 9,
    PREC_ADD = 10,
    PREC_MUL = 11,
    PREC_CAST = 12,
};

static inline const Token *tok(const Parser *p) {
    if (p->has_split)
        return &p->split;
    if (p->pos < p->n)
        return &p->toks[p->pos];
    return &p->eof;
}

static inline const Token *peek(const Parser *p, uint32_t k) {
    if (k == 0)
        return tok(p);
    uint32_t i = p->pos + k;
    if (i < p->n)
        return &p->toks[i];
    return &p->eof;
}

static inline int kind(const Parser *p) { return tok(p)->kind; }
static inline int peek_kind(const Parser *p, uint32_t k) {
    return peek(p, k)->kind;
}
static inline bool at(const Parser *p, int k) { return kind(p) == k; }
static inline bool at_kw(const Parser *p, Keyword kw) {
    return tok_is_kw(tok(p), kw);
}
static inline bool peek_kw(const Parser *p, uint32_t k, Keyword kw) {
    return tok_is_kw(peek(p, k), kw);
}
static inline bool at_eof(const Parser *p) { return at(p, TK_EOF); }

static void bump(Parser *p) {
    p->prev = tok(p)->span;
    if (p->has_split) {
        p->has_split = false;
        p->pos++;
    } else if (p->pos < p->n) {
        p->pos++;
    }
}

static bool eat(Parser *p, int k) {
    if (at(p, k)) {
        bump(p);
        return true;
    }
    return false;
}

static bool eat_kw(Parser *p, Keyword kw) {
    if (at_kw(p, kw)) {
        bump(p);
        return true;
    }
    return false;
}

static const char *describe(const Token *t) {
    if (t->kind == TK_EOF)
        return "end of input";
    return arena_printf("`%s`", tok_to_str(t));
}

_Noreturn static void error_here(Parser *p, const char *fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    fatal(tok(p)->span, "%s, found %s", buf, describe(tok(p)));
}

static void expect(Parser *p, int k) {
    if (!eat(p, k))
        error_here(p, "expected `%s`", tok_kind_str(k));
}

static void expect_kw(Parser *p, Keyword kw) {
    if (!eat_kw(p, kw))
        error_here(p, "expected `%s`", sym_str((Sym)kw));
}

/* Consume the first character of a compound token, leaving `rest`. */
static void split_first(Parser *p, int rest) {
    Token t = *tok(p);
    p->prev = t.span;
    p->prev.hi = t.span.lo + 1;
    t.kind = (uint8_t)rest;
    t.span.lo++;
    t.span.col++;
    t.flags &= (uint8_t)~(TF_WS_BEFORE | TF_NL_BEFORE);
    p->split = t;
    p->has_split = true;
}

/* Eat `>`, splitting `>>`, `>=`, `>>=`. */
static bool eat_gt(Parser *p) {
    switch (kind(p)) {
    case TK_GT: bump(p); return true;
    case TK_SHR: split_first(p, TK_GT); return true;
    case TK_GE: split_first(p, TK_EQ); return true;
    case TK_SHREQ: split_first(p, TK_GE); return true;
    default: return false;
    }
}
static void expect_gt(Parser *p) {
    if (!eat_gt(p))
        error_here(p, "expected `>`");
}
/* Eat `<`, splitting `<<`, `<=`, `<<=`. */
static bool eat_lt(Parser *p) {
    switch (kind(p)) {
    case TK_LT: bump(p); return true;
    case TK_SHL: split_first(p, TK_LT); return true;
    case TK_LE: split_first(p, TK_EQ); return true;
    case TK_SHLEQ: split_first(p, TK_LE); return true;
    default: return false;
    }
}
/* Eat `&`, splitting `&&`. */
static bool eat_amp(Parser *p) {
    if (at(p, TK_AND)) {
        bump(p);
        return true;
    }
    if (at(p, TK_ANDAND)) {
        split_first(p, TK_AND);
        return true;
    }
    return false;
}
/* Eat `|`, splitting `||`. */
static bool eat_pipe(Parser *p) {
    if (at(p, TK_OR)) {
        bump(p);
        return true;
    }
    if (at(p, TK_OROR)) {
        split_first(p, TK_OR);
        return true;
    }
    return false;
}

static Span start_span(const Parser *p) { return tok(p)->span; }
static Span end_span(const Parser *p, Span start) {
    Span s = start;
    if (p->prev.file == s.file && p->prev.hi >= s.lo)
        s.hi = p->prev.hi;
    return s;
}

static bool at_interp(const Parser *p, NtKind k) {
    return at(p, TK_INTERP) && tok(p)->nt_kind == k;
}

/* Plain identifier (not a reserved word), e.g. a binding or item name. */
static bool at_ident(const Parser *p) { return tok_is_plain_ident(tok(p)); }

static Sym expect_ident(Parser *p) {
    if (!at_ident(p))
        error_here(p, "expected identifier");
    Sym s = tok(p)->v.sym;
    bump(p);
    return s;
}

/* Tokens that can start a path segment. */
static bool is_path_seg_tok(const Token *t) {
    if (t->kind != TK_IDENT)
        return false;
    if (tok_is_plain_ident(t))
        return true;
    return tok_is_kw(t, KW_SELF_VALUE) || tok_is_kw(t, KW_SELF_TYPE) ||
           tok_is_kw(t, KW_SUPER) || tok_is_kw(t, KW_CRATE);
}
static bool is_path_start_tok(const Token *t) {
    return t->kind == TK_PATHSEP || is_path_seg_tok(t) ||
           (t->kind == TK_INTERP && t->nt_kind == NT_PATH);
}

/* Record the token tree at the current open delimiter; leaves pos after it. */
static TokenList take_delimited(Parser *p, Delim *delim) {
    if (p->has_split || !tok_is_open(kind(p)))
        error_here(p, "expected one of `(`, `[`, `{`");
    int k = kind(p);
    *delim = k == TK_LPAREN ? DELIM_PAREN : k == TK_LBRACKET ? DELIM_BRACKET
                                                             : DELIM_BRACE;
    uint32_t start = p->pos;
    uint32_t end = tt_end(p->toks, start, p->n);
    TokenList tl = {(Token *)p->toks + start + 1, end - start - 1};
    p->prev = p->toks[end].span;
    p->pos = end + 1;
    return tl;
}

/* Is the parser at `path ! (delim)` (a macro invocation)? */
static bool at_macro_call(const Parser *p) {
    uint32_t k = 0;
    if (peek_kind(p, 0) == TK_INTERP && peek(p, 0)->nt_kind == NT_PATH) {
        k = 1;
    } else {
        if (peek_kind(p, k) == TK_PATHSEP)
            k++;
        for (;;) {
            if (!is_path_seg_tok(peek(p, k)))
                return false;
            k++;
            if (peek_kind(p, k) == TK_PATHSEP)
                k++;
            else
                break;
        }
    }
    if (peek_kind(p, k) != TK_NOT)
        return false;
    return tok_is_open(peek_kind(p, k + 1));
}

/* ------------------------------------------------------------------ */
/* Forward declarations                                                */
/* ------------------------------------------------------------------ */

static Expr *parse_expr(Parser *p);
static Expr *parse_expr_res(Parser *p, int r);
static Expr *parse_assoc(Parser *p, int min_prec, int r);
static Expr *parse_assoc_rest(Parser *p, Expr *lhs, int min_prec, int r);
static Expr *parse_postfix(Parser *p, Expr *e);
static Expr *parse_unary(Parser *p, int r);
static Ty *parse_ty(Parser *p);
static Ty *parse_ty_no_plus(Parser *p);
static Pat *parse_pat_top(Parser *p);
static Pat *parse_pat_no_top_alt(Parser *p);
static Block *parse_block(Parser *p);
static Item *parse_item_opt(Parser *p, AttrList attrs);
static Stmt *parse_stmt_opt(Parser *p);
static Path *parse_path(Parser *p, bool type_mode);
static GenericArgs *parse_generic_args(Parser *p);
static BoundList parse_bounds(Parser *p, bool allow_plus);
static void parse_generics(Parser *p, Generics *g);
static void parse_where(Parser *p, Generics *g);
static GenericParamList parse_for_lifetimes(Parser *p);
static MacCall *parse_mac_call(Parser *p, Path *path);
static bool can_begin_expr(const Parser *p, int r);
static bool can_begin_ty(const Parser *p);
static Expr *parse_lit_expr(Parser *p);
static Expr *parse_primary(Parser *p, int r);
static Expr *parse_stmt_like_expr(Parser *p, bool *complete);

/* ------------------------------------------------------------------ */
/* Attributes and visibility                                           */
/* ------------------------------------------------------------------ */

static Attr *parse_attr_body(Parser *p, bool inner, Span start) {
    Attr *a = NEW(Attr);
    a->id = ast_fresh_id();
    a->inner = inner;
    a->path = parse_path(p, false);
    if (tok_is_open(kind(p))) {
        a->args_kind = ATTR_ARGS_DELIMITED;
        a->tokens = take_delimited(p, &a->delim);
    } else if (at(p, TK_EQ)) {
        bump(p);
        a->args_kind = ATTR_ARGS_EQ;
        uint32_t s = p->pos;
        a->value = parse_expr(p);
        a->tokens.toks = (Token *)p->toks + s;
        a->tokens.len = p->pos - s;
    }
    a->span = end_span(p, start);
    return a;
}

/* Outer attributes `#[...]`. */
static void parse_outer_attrs(Parser *p, AttrList *out) {
    while (at(p, TK_POUND) && peek_kind(p, 1) == TK_LBRACKET) {
        Span start = start_span(p);
        bump(p);
        bump(p);
        Attr *a = parse_attr_body(p, false, start);
        expect(p, TK_RBRACKET);
        a->span = end_span(p, start);
        vec_push(*out, a);
    }
}

/* Inner attributes `#![...]`. */
static void parse_inner_attrs(Parser *p, AttrList *out) {
    while (at(p, TK_POUND) && peek_kind(p, 1) == TK_NOT &&
           peek_kind(p, 2) == TK_LBRACKET) {
        Span start = start_span(p);
        bump(p);
        bump(p);
        bump(p);
        Attr *a = parse_attr_body(p, true, start);
        expect(p, TK_RBRACKET);
        a->span = end_span(p, start);
        vec_push(*out, a);
    }
}

static Visibility parse_vis(Parser *p) {
    Visibility v;
    memset(&v, 0, sizeof v);
    v.span = start_span(p);
    if (at_interp(p, NT_VIS)) {
        v = *(Visibility *)tok(p)->v.node;
        bump(p);
        return v;
    }
    if (!at_kw(p, KW_PUB)) {
        v.kind = VIS_INHERITED;
        v.span.hi = v.span.lo;
        return v;
    }
    Span start = start_span(p);
    bump(p);
    v.kind = VIS_PUB;
    if (at(p, TK_LPAREN)) {
        const Token *t1 = peek(p, 1);
        if (peek_kind(p, 2) == TK_RPAREN &&
            (tok_is_kw(t1, KW_CRATE) || tok_is_kw(t1, KW_SUPER) ||
             tok_is_kw(t1, KW_SELF_VALUE))) {
            v.kind = tok_is_kw(t1, KW_CRATE)   ? VIS_CRATE
                     : tok_is_kw(t1, KW_SUPER) ? VIS_SUPER
                                               : VIS_SELF;
            bump(p);
            bump(p);
            bump(p);
        } else if (tok_is_kw(t1, KW_IN)) {
            bump(p);
            bump(p);
            v.kind = VIS_IN;
            v.path = parse_path(p, false);
            expect(p, TK_RPAREN);
        }
    }
    v.span = end_span(p, start);
    return v;
}

/* ------------------------------------------------------------------ */
/* Paths                                                               */
/* ------------------------------------------------------------------ */

static PathSegment *parse_path_segment_name(Parser *p) {
    const Token *t = tok(p);
    if (!is_path_seg_tok(t))
        error_here(p, "expected path segment");
    PathSegment *seg = NEW(PathSegment);
    seg->id = ast_fresh_id();
    seg->span = t->span;
    seg->name = t->v.sym;
    bump(p);
    return seg;
}

/* Parenthesised Fn sugar: (A, B) -> C */
static GenericArgs *parse_paren_args(Parser *p) {
    GenericArgs *ga = NEW(GenericArgs);
    Span start = start_span(p);
    ga->kind = GENERIC_ARGS_PAREN;
    expect(p, TK_LPAREN);
    while (!at(p, TK_RPAREN)) {
        vec_push(ga->inputs, parse_ty(p));
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RPAREN);
    if (eat(p, TK_RARROW))
        ga->output = parse_ty_no_plus(p);
    ga->span = end_span(p, start);
    return ga;
}

/* Continue a path after its first segments (used by qualified paths too). */
static void parse_path_segments(Parser *p, Path *path, bool type_mode) {
    for (;;) {
        PathSegment *seg = parse_path_segment_name(p);
        vec_push(path->segs, seg);
        if (type_mode) {
            if (at(p, TK_LT) || at(p, TK_SHL) ||
                (at(p, TK_PATHSEP) &&
                 (peek_kind(p, 1) == TK_LT || peek_kind(p, 1) == TK_SHL))) {
                eat(p, TK_PATHSEP);
                seg->args = parse_generic_args(p);
            } else if (at(p, TK_LPAREN)) {
                seg->args = parse_paren_args(p);
            }
        } else if (at(p, TK_PATHSEP) &&
                   (peek_kind(p, 1) == TK_LT || peek_kind(p, 1) == TK_SHL)) {
            bump(p);
            seg->args = parse_generic_args(p);
        }
        seg->span = end_span(p, seg->span);
        if (at(p, TK_PATHSEP) && is_path_seg_tok(peek(p, 1))) {
            bump(p);
            continue;
        }
        break;
    }
}

/* Path. type_mode: generic args via `<` and Fn() sugar; otherwise only
   turbofish `::<`. */
static Path *parse_path(Parser *p, bool type_mode) {
    if (at_interp(p, NT_PATH)) {
        Path *src = (Path *)tok(p)->v.node;
        bump(p);
        /* copy so that appending segments does not mutate the fragment */
        Path *path = ast_new_path(src->span);
        path->global = src->global;
        for (uint32_t i = 0; i < src->segs.len; i++)
            vec_push(path->segs, src->segs.data[i]);
        if (at(p, TK_PATHSEP) && is_path_seg_tok(peek(p, 1))) {
            bump(p);
            parse_path_segments(p, path, type_mode);
        }
        path->span = end_span(p, path->span);
        return path;
    }
    Span start = start_span(p);
    Path *path = ast_new_path(start);
    if (eat(p, TK_PATHSEP))
        path->global = true;
    parse_path_segments(p, path, type_mode);
    path->span = end_span(p, start);
    return path;
}

/* Qualified path `<T as Trait>::rest` / `<T>::rest`; at `<` or `<<`. */
static Path *parse_qpath(Parser *p, QSelf **qself_out, bool type_mode) {
    Span start = start_span(p);
    if (!eat_lt(p))
        error_here(p, "expected `<`");
    QSelf *qs = NEW(QSelf);
    qs->span = start;
    qs->ty = parse_ty(p);
    Path *path = ast_new_path(start);
    if (eat_kw(p, KW_AS)) {
        Path *tp = parse_path(p, true);
        path->global = tp->global;
        for (uint32_t i = 0; i < tp->segs.len; i++)
            vec_push(path->segs, tp->segs.data[i]);
    }
    qs->position = path->segs.len;
    expect_gt(p);
    qs->span = end_span(p, start);
    expect(p, TK_PATHSEP);
    parse_path_segments(p, path, type_mode);
    path->span = end_span(p, start);
    *qself_out = qs;
    return path;
}

/* Generic arguments `<...>`; at `<` (or `<<`). */
static GenericArgs *parse_generic_args(Parser *p) {
    GenericArgs *ga = NEW(GenericArgs);
    Span start = start_span(p);
    ga->kind = GENERIC_ARGS_ANGLE;
    if (!eat_lt(p))
        error_here(p, "expected `<`");
    while (!eat_gt(p)) {
        GenericArg *a = NEW(GenericArg);
        a->span = start_span(p);
        const Token *t = tok(p);
        if (t->kind == TK_LIFETIME) {
            a->kind = GA_LIFETIME;
            a->lifetime = t->v.sym;
            bump(p);
        } else if (t->kind == TK_LBRACE) {
            a->kind = GA_CONST;
            Span bs = start_span(p);
            Block *b = parse_block(p);
            a->expr = ast_new_expr(EXPR_BLOCK, end_span(p, bs));
            a->expr->block.block = b;
        } else if (t->kind == TK_LITERAL || tok_is_kw(t, KW_TRUE) ||
                   tok_is_kw(t, KW_FALSE) ||
                   (t->kind == TK_MINUS && peek_kind(p, 1) == TK_LITERAL) ||
                   (t->kind == TK_INTERP &&
                    (t->nt_kind == NT_EXPR || t->nt_kind == NT_LIT))) {
            a->kind = GA_CONST;
            a->expr = parse_lit_expr(p);
        } else if (tok_is_plain_ident(t) && peek_kind(p, 1) == TK_EQ) {
            a->kind = GA_BINDING;
            a->name = t->v.sym;
            bump(p);
            bump(p);
            a->ty = parse_ty(p);
        } else if (tok_is_plain_ident(t) && peek_kind(p, 1) == TK_COLON) {
            a->kind = GA_CONSTRAINT;
            a->name = t->v.sym;
            bump(p);
            bump(p);
            a->bounds = parse_bounds(p, true);
        } else if (tok_is_plain_ident(t) && peek_kind(p, 1) == TK_LT) {
            /* Could be `Item<'a> = T` (GAT binding) or a generic type.
               Parse as type; if `=` follows, convert. */
            Ty *ty = parse_ty(p);
            if (at(p, TK_EQ) && ty->kind == TY_PATH && !ty->path.qself &&
                ty->path.path->segs.len == 1) {
                bump(p);
                a->kind = GA_BINDING;
                a->name = ty->path.path->segs.data[0]->name;
                a->name_args = ty->path.path->segs.data[0]->args;
                a->ty = parse_ty(p);
            } else {
                a->kind = GA_TYPE;
                a->ty = ty;
            }
        } else {
            a->kind = GA_TYPE;
            a->ty = parse_ty(p);
        }
        a->span = end_span(p, a->span);
        vec_push(ga->args, a);
        if (!eat(p, TK_COMMA)) {
            expect_gt(p);
            break;
        }
    }
    ga->span = end_span(p, start);
    return ga;
}

/* ------------------------------------------------------------------ */
/* Generics and bounds                                                 */
/* ------------------------------------------------------------------ */

static GenericParamList parse_for_lifetimes(Parser *p) {
    GenericParamList list = {0};
    if (!at_kw(p, KW_FOR))
        return list;
    bump(p);
    if (!eat_lt(p))
        error_here(p, "expected `<` after `for`");
    while (!eat_gt(p)) {
        GenericParam *gp = NEW(GenericParam);
        gp->id = ast_fresh_id();
        gp->span = start_span(p);
        parse_outer_attrs(p, &gp->attrs);
        gp->kind = GP_LIFETIME;
        if (!at(p, TK_LIFETIME))
            error_here(p, "expected lifetime in for<>");
        gp->name = tok(p)->v.sym;
        bump(p);
        vec_push(list, gp);
        if (!eat(p, TK_COMMA)) {
            expect_gt(p);
            break;
        }
    }
    return list;
}

static GenericBound *parse_bound(Parser *p) {
    GenericBound *b = NEW(GenericBound);
    Span start = start_span(p);
    b->span = start;
    if (at(p, TK_LIFETIME)) {
        b->kind = BOUND_LIFETIME;
        b->lifetime = tok(p)->v.sym;
        bump(p);
        return b;
    }
    bool paren = eat(p, TK_LPAREN);
    b->kind = BOUND_TRAIT;
    if (eat(p, TK_QUESTION))
        b->maybe = true;
    if (at(p, TK_TILDE)) /* ~const: accepted and ignored */
        fatal(tok(p)->span, "unsupported: `~const` bounds");
    b->for_lifetimes = parse_for_lifetimes(p);
    b->path = parse_path(p, true);
    if (paren)
        expect(p, TK_RPAREN);
    b->span = end_span(p, start);
    return b;
}

static bool can_begin_bound(const Parser *p) {
    const Token *t = tok(p);
    return t->kind == TK_LIFETIME || t->kind == TK_QUESTION ||
           t->kind == TK_LPAREN || tok_is_kw(t, KW_FOR) ||
           is_path_start_tok(t);
}

static BoundList parse_bounds(Parser *p, bool allow_plus) {
    BoundList list = {0};
    for (;;) {
        if (!can_begin_bound(p))
            break;
        vec_push(list, parse_bound(p));
        if (!allow_plus || !at(p, TK_PLUS))
            break;
        bump(p);
    }
    return list;
}

static void parse_generics(Parser *p, Generics *g) {
    g->span = start_span(p);
    if (!at(p, TK_LT)) {
        g->span.hi = g->span.lo;
        return;
    }
    bump(p);
    while (!eat_gt(p)) {
        GenericParam *gp = NEW(GenericParam);
        gp->id = ast_fresh_id();
        gp->span = start_span(p);
        parse_outer_attrs(p, &gp->attrs);
        if (at(p, TK_LIFETIME)) {
            gp->kind = GP_LIFETIME;
            gp->name = tok(p)->v.sym;
            bump(p);
            if (eat(p, TK_COLON)) {
                while (at(p, TK_LIFETIME)) {
                    GenericBound *b = NEW(GenericBound);
                    b->span = tok(p)->span;
                    b->kind = BOUND_LIFETIME;
                    b->lifetime = tok(p)->v.sym;
                    bump(p);
                    vec_push(gp->bounds, b);
                    if (!eat(p, TK_PLUS))
                        break;
                }
            }
        } else if (at_kw(p, KW_CONST)) {
            bump(p);
            gp->kind = GP_CONST;
            gp->name = expect_ident(p);
            expect(p, TK_COLON);
            gp->const_ty = parse_ty(p);
            if (eat(p, TK_EQ)) {
                if (at(p, TK_LBRACE)) {
                    Span bs = start_span(p);
                    Block *b = parse_block(p);
                    gp->const_default = ast_new_expr(EXPR_BLOCK, end_span(p, bs));
                    gp->const_default->block.block = b;
                } else {
                    gp->const_default = parse_unary(p, 0);
                }
            }
        } else {
            gp->kind = GP_TYPE;
            gp->name = expect_ident(p);
            if (eat(p, TK_COLON))
                gp->bounds = parse_bounds(p, true);
            if (eat(p, TK_EQ))
                gp->default_ty = parse_ty(p);
        }
        gp->span = end_span(p, gp->span);
        vec_push(g->params, gp);
        if (!eat(p, TK_COMMA)) {
            expect_gt(p);
            break;
        }
    }
    g->span = end_span(p, g->span);
}

static void parse_where(Parser *p, Generics *g) {
    if (!eat_kw(p, KW_WHERE))
        return;
    for (;;) {
        WherePredicate *wp = NEW(WherePredicate);
        wp->span = start_span(p);
        if (at(p, TK_LIFETIME)) {
            wp->kind = WP_LIFETIME;
            wp->lifetime = tok(p)->v.sym;
            bump(p);
            expect(p, TK_COLON);
            wp->bounds = parse_bounds(p, true);
        } else if (at_kw(p, KW_FOR) || can_begin_ty(p)) {
            wp->kind = WP_BOUND;
            wp->for_lifetimes = parse_for_lifetimes(p);
            wp->ty = parse_ty(p);
            expect(p, TK_COLON);
            wp->bounds = parse_bounds(p, true);
        } else {
            break;
        }
        wp->span = end_span(p, wp->span);
        vec_push(g->where, wp);
        if (!eat(p, TK_COMMA))
            break;
    }
}

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

static bool can_begin_ty(const Parser *p) {
    const Token *t = tok(p);
    switch (t->kind) {
    case TK_LPAREN: case TK_NOT: case TK_STAR: case TK_AND: case TK_ANDAND:
    case TK_LBRACKET: case TK_LT: case TK_SHL: case TK_PATHSEP:
        return true;
    case TK_INTERP:
        return t->nt_kind == NT_TY || t->nt_kind == NT_PATH;
    case TK_IDENT:
        if (tok_is_kw(t, KW_UNDERSCORE) || tok_is_kw(t, KW_FN) ||
            tok_is_kw(t, KW_UNSAFE) || tok_is_kw(t, KW_EXTERN) ||
            tok_is_kw(t, KW_FOR) || tok_is_kw(t, KW_DYN) ||
            tok_is_kw(t, KW_IMPL))
            return true;
        return is_path_seg_tok(t);
    default:
        return false;
    }
}

static Ty *parse_fn_ptr_ty(Parser *p, Span start) {
    Ty *t = ast_new_ty(TY_FN, start);
    t->fn_.for_lifetimes = parse_for_lifetimes(p);
    if (eat_kw(p, KW_UNSAFE))
        t->fn_.is_unsafe = true;
    if (eat_kw(p, KW_EXTERN)) {
        if (at(p, TK_LITERAL) && tok(p)->v.lit->kind == LIT_STR) {
            t->fn_.abi = sym_intern(tok(p)->v.lit->str, tok(p)->v.lit->str_len);
            bump(p);
        } else {
            t->fn_.abi = sym_intern_cstr("C");
        }
    }
    expect_kw(p, KW_FN);
    expect(p, TK_LPAREN);
    while (!at(p, TK_RPAREN)) {
        AttrList ignored = {0};
        parse_outer_attrs(p, &ignored);
        if (eat(p, TK_DOTDOTDOT)) {
            t->fn_.variadic = true;
            break;
        }
        FnPtrParam *fp = NEW(FnPtrParam);
        fp->span = start_span(p);
        if ((at_ident(p) || at_kw(p, KW_UNDERSCORE)) &&
            peek_kind(p, 1) == TK_COLON) {
            fp->name = tok(p)->v.sym;
            bump(p);
            bump(p);
        }
        fp->ty = parse_ty(p);
        fp->span = end_span(p, fp->span);
        vec_push(t->fn_.params, fp);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RPAREN);
    if (eat(p, TK_RARROW))
        t->fn_.ret = parse_ty_no_plus(p);
    t->span = end_span(p, start);
    return t;
}

static Ty *parse_ty_common(Parser *p, bool allow_plus) {
    Span start = start_span(p);
    const Token *t = tok(p);
    Ty *ty;
    if (t->kind == TK_INTERP && t->nt_kind == NT_TY) {
        bump(p);
        return (Ty *)t->v.node;
    }
    switch (t->kind) {
    case TK_LPAREN: {
        bump(p);
        if (eat(p, TK_RPAREN))
            return ast_new_ty(TY_TUPLE, end_span(p, start));
        Ty *first = parse_ty(p);
        if (eat(p, TK_RPAREN)) {
            /* parenthesised type */
            if (allow_plus && at(p, TK_PLUS) &&
                (first->kind == TY_PATH)) {
                fatal(tok(p)->span, "unsupported: bare trait object `(Trait) + ...`");
            }
            return first;
        }
        ty = ast_new_ty(TY_TUPLE, start);
        vec_push(ty->tuple.elems, first);
        while (eat(p, TK_COMMA)) {
            if (at(p, TK_RPAREN))
                break;
            vec_push(ty->tuple.elems, parse_ty(p));
        }
        expect(p, TK_RPAREN);
        break;
    }
    case TK_NOT:
        bump(p);
        ty = ast_new_ty(TY_NEVER, start);
        break;
    case TK_STAR:
        bump(p);
        ty = ast_new_ty(TY_PTR, start);
        if (eat_kw(p, KW_MUT))
            ty->ptr.is_mut = true;
        else
            expect_kw(p, KW_CONST);
        ty->ptr.elem = parse_ty_no_plus(p);
        break;
    case TK_AND:
    case TK_ANDAND:
        eat_amp(p);
        ty = ast_new_ty(TY_REF, start);
        if (at(p, TK_LIFETIME)) {
            ty->ref.lifetime = tok(p)->v.sym;
            bump(p);
        }
        if (eat_kw(p, KW_MUT))
            ty->ref.is_mut = true;
        ty->ref.elem = parse_ty_no_plus(p);
        break;
    case TK_LBRACKET: {
        bump(p);
        Ty *elem = parse_ty(p);
        if (eat(p, TK_SEMI)) {
            ty = ast_new_ty(TY_ARRAY, start);
            ty->array.elem = elem;
            ty->array.len = parse_expr(p);
        } else {
            ty = ast_new_ty(TY_SLICE, start);
            ty->slice.elem = elem;
        }
        expect(p, TK_RBRACKET);
        break;
    }
    case TK_LT:
    case TK_SHL: {
        ty = ast_new_ty(TY_PATH, start);
        ty->path.path = parse_qpath(p, &ty->path.qself, true);
        break;
    }
    default:
        if (tok_is_kw(t, KW_UNDERSCORE)) {
            bump(p);
            ty = ast_new_ty(TY_INFER, start);
        } else if (tok_is_kw(t, KW_FN) || tok_is_kw(t, KW_UNSAFE) ||
                   tok_is_kw(t, KW_EXTERN)) {
            return parse_fn_ptr_ty(p, start);
        } else if (tok_is_kw(t, KW_FOR)) {
            /* for<'a> fn(...)  or  for<'a> Trait (bare, rejected) */
            uint32_t save = p->pos;
            bool save_split = p->has_split;
            Token save_tok = p->split;
            Span save_prev = p->prev;
            (void)parse_for_lifetimes(p);
            bool is_fn = at_kw(p, KW_FN) || at_kw(p, KW_UNSAFE) ||
                         at_kw(p, KW_EXTERN);
            p->pos = save;
            p->has_split = save_split;
            p->split = save_tok;
            p->prev = save_prev;
            if (!is_fn)
                fatal(start, "unsupported: bare `for<'a> Trait` type without `dyn`");
            return parse_fn_ptr_ty(p, start);
        } else if (tok_is_kw(t, KW_DYN)) {
            bump(p);
            ty = ast_new_ty(TY_DYN, start);
            ty->bounds = parse_bounds(p, allow_plus);
        } else if (tok_is_kw(t, KW_IMPL)) {
            bump(p);
            ty = ast_new_ty(TY_IMPL, start);
            ty->bounds = parse_bounds(p, allow_plus);
        } else if (is_path_start_tok(t)) {
            if (at_macro_call(p)) {
                Path *path = parse_path(p, false);
                ty = ast_new_ty(TY_MAC, start);
                ty->mac = parse_mac_call(p, path);
            } else {
                ty = ast_new_ty(TY_PATH, start);
                ty->path.path = parse_path(p, true);
            }
        } else {
            error_here(p, "expected type");
        }
    }
    ty->span = end_span(p, start);
    return ty;
}

static Ty *parse_ty(Parser *p) { return parse_ty_common(p, true); }
static Ty *parse_ty_no_plus(Parser *p) { return parse_ty_common(p, false); }

/* ------------------------------------------------------------------ */
/* Macro calls                                                         */
/* ------------------------------------------------------------------ */

/* At `!` after a path. */
static MacCall *parse_mac_call(Parser *p, Path *path) {
    MacCall *m = NEW(MacCall);
    m->id = ast_fresh_id();
    m->path = path;
    expect(p, TK_NOT);
    m->tokens = take_delimited(p, &m->delim);
    m->span = end_span(p, path->span);
    return m;
}

/* ------------------------------------------------------------------ */
/* Patterns                                                            */
/* ------------------------------------------------------------------ */

static Expr *new_lit_expr(Lit *lit, Span sp) {
    Expr *e = ast_new_expr(EXPR_LIT, sp);
    e->lit = lit;
    return e;
}

static Lit *bool_lit(bool v) {
    Lit *l = NEW(Lit);
    l->kind = LIT_BOOL;
    l->bool_val = v;
    return l;
}

/* Literal (or `-` literal) as an expression; for patterns, const args,
   and $l:literal. */
static Expr *parse_lit_expr(Parser *p) {
    Span start = start_span(p);
    const Token *t = tok(p);
    if (t->kind == TK_INTERP && (t->nt_kind == NT_EXPR || t->nt_kind == NT_LIT)) {
        bump(p);
        return (Expr *)t->v.node;
    }
    if (t->kind == TK_MINUS) {
        bump(p);
        Expr *inner = parse_lit_expr(p);
        Expr *e = ast_new_expr(EXPR_UNARY, end_span(p, start));
        e->unary.op = UN_NEG;
        e->unary.operand = inner;
        return e;
    }
    if (t->kind == TK_LITERAL) {
        bump(p);
        return new_lit_expr(t->v.lit, t->span);
    }
    if (tok_is_kw(t, KW_TRUE) || tok_is_kw(t, KW_FALSE)) {
        bump(p);
        return new_lit_expr(bool_lit(tok_is_kw(t, KW_TRUE)), t->span);
    }
    error_here(p, "expected literal");
}

static bool at_lit_start(const Parser *p) {
    const Token *t = tok(p);
    return t->kind == TK_LITERAL || tok_is_kw(t, KW_TRUE) ||
           tok_is_kw(t, KW_FALSE) ||
           (t->kind == TK_MINUS && peek_kind(p, 1) == TK_LITERAL) ||
           (t->kind == TK_INTERP && t->nt_kind == NT_LIT);
}

/* Range pattern bound: literal, -literal, or path. */
static Expr *parse_range_bound(Parser *p) {
    if (at_lit_start(p))
        return parse_lit_expr(p);
    Span start = start_span(p);
    Expr *e = ast_new_expr(EXPR_PATH, start);
    if (at(p, TK_LT) || at(p, TK_SHL))
        e->path.path = parse_qpath(p, &e->path.qself, false);
    else
        e->path.path = parse_path(p, false);
    e->span = end_span(p, start);
    return e;
}

static bool can_begin_range_bound(const Parser *p) {
    return at_lit_start(p) || is_path_start_tok(tok(p)) || at(p, TK_LT);
}

/* After a lower bound: if at a range operator, build the range pattern. */
static Pat *maybe_range_pat(Parser *p, Expr *lo, Span start, Pat *fallback) {
    if (at(p, TK_DOTDOTEQ) || at(p, TK_DOTDOTDOT)) {
        bump(p);
        Pat *r = ast_new_pat(PAT_RANGE, start);
        r->range.lo = lo;
        r->range.end = RANGE_INCLUSIVE;
        r->range.hi = parse_range_bound(p);
        r->span = end_span(p, start);
        return r;
    }
    if (at(p, TK_DOTDOT)) {
        bump(p);
        Pat *r = ast_new_pat(PAT_RANGE, start);
        r->range.lo = lo;
        r->range.end = RANGE_HALF_OPEN;
        if (can_begin_range_bound(p))
            r->range.hi = parse_range_bound(p);
        r->span = end_span(p, start);
        return r;
    }
    return fallback;
}

/* Comma-separated patterns until `close`; returns list (close consumed). */
static PatList parse_pat_list(Parser *p, int close) {
    PatList list = {0};
    while (!at(p, close)) {
        vec_push(list, parse_pat_top(p));
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, close);
    return list;
}

static Pat *parse_struct_pat_body(Parser *p, Pat *pat) {
    expect(p, TK_LBRACE);
    while (!at(p, TK_RBRACE)) {
        AttrList attrs = {0};
        parse_outer_attrs(p, &attrs);
        if (at(p, TK_DOTDOT)) {
            bump(p);
            pat->struct_.has_rest = true;
            break;
        }
        PatField *f = NEW(PatField);
        f->id = ast_fresh_id();
        f->span = start_span(p);
        f->attrs = attrs;
        if (at_kw(p, KW_REF) || at_kw(p, KW_MUT) || at_kw(p, KW_BOX)) {
            if (at_kw(p, KW_BOX))
                fatal(tok(p)->span, "unsupported: box patterns");
            Pat *b = ast_new_pat(PAT_IDENT, f->span);
            if (eat_kw(p, KW_REF))
                b->ident.by_ref = true;
            if (eat_kw(p, KW_MUT))
                b->ident.is_mut = true;
            b->ident.name = expect_ident(p);
            b->span = end_span(p, b->span);
            f->name = b->ident.name;
            f->pat = b;
            f->shorthand = true;
        } else if (at(p, TK_LITERAL) && tok(p)->v.lit->kind == LIT_INT) {
            f->name = sym_intern_cstr(arena_printf(
                "%llu", (unsigned long long)tok(p)->v.lit->int_lo));
            bump(p);
            expect(p, TK_COLON);
            f->pat = parse_pat_top(p);
        } else {
            Span ns = start_span(p);
            f->name = expect_ident(p);
            if (eat(p, TK_COLON)) {
                f->pat = parse_pat_top(p);
            } else {
                f->shorthand = true;
                f->pat = ast_new_pat(PAT_IDENT, ns);
                f->pat->ident.name = f->name;
            }
        }
        f->span = end_span(p, f->span);
        vec_push(pat->struct_.fields, f);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RBRACE);
    return pat;
}

static Pat *parse_pat_atom(Parser *p) {
    Span start = start_span(p);
    const Token *t = tok(p);
    Pat *pat;
    if (t->kind == TK_INTERP) {
        if (t->nt_kind == NT_PAT) {
            bump(p);
            return (Pat *)t->v.node;
        }
        if (t->nt_kind == NT_EXPR || t->nt_kind == NT_LIT) {
            Expr *e = parse_lit_expr(p);
            pat = ast_new_pat(PAT_LIT, start);
            pat->lit = e;
            return maybe_range_pat(p, e, start, pat);
        }
    }
    if (tok_is_kw(t, KW_UNDERSCORE)) {
        bump(p);
        return ast_new_pat(PAT_WILD, start);
    }
    if (t->kind == TK_DOTDOT) {
        /* `..` rest, or `..hi` half-open range */
        bump(p);
        if (can_begin_range_bound(p)) {
            pat = ast_new_pat(PAT_RANGE, start);
            pat->range.end = RANGE_HALF_OPEN;
            pat->range.hi = parse_range_bound(p);
            pat->span = end_span(p, start);
            return pat;
        }
        return ast_new_pat(PAT_REST, start);
    }
    if (t->kind == TK_DOTDOTEQ) {
        bump(p);
        pat = ast_new_pat(PAT_RANGE, start);
        pat->range.end = RANGE_INCLUSIVE;
        pat->range.hi = parse_range_bound(p);
        pat->span = end_span(p, start);
        return pat;
    }
    if (t->kind == TK_AND || t->kind == TK_ANDAND) {
        eat_amp(p);
        pat = ast_new_pat(PAT_REF, start);
        if (eat_kw(p, KW_MUT))
            pat->ref.is_mut = true;
        pat->ref.pat = parse_pat_atom(p);
        pat->span = end_span(p, start);
        return pat;
    }
    if (t->kind == TK_LPAREN) {
        bump(p);
        if (eat(p, TK_RPAREN))
            return ast_new_pat(PAT_TUPLE, end_span(p, start));
        Pat *first = parse_pat_top(p);
        if (eat(p, TK_RPAREN)) {
            /* parenthesised pattern (a 1-tuple needs a trailing comma) */
            return first;
        }
        pat = ast_new_pat(PAT_TUPLE, start);
        vec_push(pat->elems, first);
        expect(p, TK_COMMA);
        PatList rest = parse_pat_list(p, TK_RPAREN);
        for (uint32_t i = 0; i < rest.len; i++)
            vec_push(pat->elems, rest.data[i]);
        pat->span = end_span(p, start);
        return pat;
    }
    if (t->kind == TK_LBRACKET) {
        bump(p);
        pat = ast_new_pat(PAT_SLICE, start);
        pat->elems = parse_pat_list(p, TK_RBRACKET);
        pat->span = end_span(p, start);
        return pat;
    }
    if (at_lit_start(p)) {
        Expr *e = parse_lit_expr(p);
        pat = ast_new_pat(PAT_LIT, end_span(p, start));
        pat->lit = e;
        return maybe_range_pat(p, e, start, pat);
    }
    if (tok_is_kw(t, KW_BOX))
        fatal(t->span, "unsupported: box patterns");
    if (tok_is_kw(t, KW_REF) || tok_is_kw(t, KW_MUT)) {
        pat = ast_new_pat(PAT_IDENT, start);
        if (eat_kw(p, KW_REF))
            pat->ident.by_ref = true;
        if (eat_kw(p, KW_MUT))
            pat->ident.is_mut = true;
        pat->ident.name = expect_ident(p);
        if (eat(p, TK_AT))
            pat->ident.sub = parse_pat_no_top_alt(p);
        pat->span = end_span(p, start);
        return pat;
    }
    /* Single identifier: binding (or unit path, decided by resolve). */
    if (tok_is_plain_ident(t) && peek_kind(p, 1) != TK_PATHSEP &&
        peek_kind(p, 1) != TK_LPAREN && peek_kind(p, 1) != TK_LBRACE &&
        peek_kind(p, 1) != TK_NOT && peek_kind(p, 1) != TK_DOTDOT &&
        peek_kind(p, 1) != TK_DOTDOTEQ && peek_kind(p, 1) != TK_DOTDOTDOT) {
        bump(p);
        pat = ast_new_pat(PAT_IDENT, start);
        pat->ident.name = t->v.sym;
        if (eat(p, TK_AT))
            pat->ident.sub = parse_pat_no_top_alt(p);
        pat->span = end_span(p, start);
        return pat;
    }
    if (is_path_start_tok(t) || t->kind == TK_LT || t->kind == TK_SHL) {
        QSelf *qself = NULL;
        Path *path;
        if (t->kind == TK_LT || t->kind == TK_SHL) {
            path = parse_qpath(p, &qself, false);
        } else if (at_macro_call(p)) {
            path = parse_path(p, false);
            pat = ast_new_pat(PAT_MAC, start);
            pat->mac = parse_mac_call(p, path);
            pat->span = end_span(p, start);
            return pat;
        } else {
            path = parse_path(p, false);
        }
        if (at(p, TK_LPAREN)) {
            bump(p);
            pat = ast_new_pat(PAT_TUPLE_STRUCT, start);
            pat->tuple_struct.qself = qself;
            pat->tuple_struct.path = path;
            pat->tuple_struct.elems = parse_pat_list(p, TK_RPAREN);
        } else if (at(p, TK_LBRACE)) {
            pat = ast_new_pat(PAT_STRUCT, start);
            pat->struct_.qself = qself;
            pat->struct_.path = path;
            parse_struct_pat_body(p, pat);
        } else {
            pat = ast_new_pat(PAT_PATH, start);
            pat->path.qself = qself;
            pat->path.path = path;
            pat->span = end_span(p, start);
            if (at(p, TK_DOTDOT) || at(p, TK_DOTDOTEQ) || at(p, TK_DOTDOTDOT)) {
                /* const path as range bound, e.g. A..=B */
                Expr *pe = ast_new_expr(EXPR_PATH, pat->span);
                pe->path.qself = qself;
                pe->path.path = path;
                return maybe_range_pat(p, pe, start, NULL);
            }
        }
        pat->span = end_span(p, start);
        return pat;
    }
    error_here(p, "expected pattern");
}

static Pat *parse_pat_no_top_alt(Parser *p) { return parse_pat_atom(p); }

static Pat *parse_pat_top(Parser *p) {
    Span start = start_span(p);
    eat(p, TK_OR); /* leading vert */
    Pat *first = parse_pat_no_top_alt(p);
    if (!at(p, TK_OR))
        return first;
    Pat *orp = ast_new_pat(PAT_OR, start);
    vec_push(orp->elems, first);
    while (eat(p, TK_OR))
        vec_push(orp->elems, parse_pat_no_top_alt(p));
    orp->span = end_span(p, start);
    return orp;
}

/* ------------------------------------------------------------------ */
/* Expressions                                                         */
/* ------------------------------------------------------------------ */

static bool can_begin_expr(const Parser *p, int r) {
    const Token *t = tok(p);
    switch (t->kind) {
    case TK_LITERAL: case TK_LPAREN: case TK_LBRACKET: case TK_NOT:
    case TK_MINUS: case TK_STAR: case TK_AND: case TK_ANDAND: case TK_OR:
    case TK_OROR: case TK_DOTDOT: case TK_DOTDOTEQ: case TK_LT: case TK_SHL:
    case TK_PATHSEP: case TK_POUND: case TK_LIFETIME: case TK_INTERP:
        return true;
    case TK_LBRACE:
        return !(r & R_NO_STRUCT);
    case TK_IDENT:
        if (tok_is_plain_ident(t))
            return true;
        switch (t->v.sym) {
        case KW_SELF_VALUE: case KW_SELF_TYPE: case KW_SUPER: case KW_CRATE:
        case KW_TRUE: case KW_FALSE: case KW_IF: case KW_MATCH: case KW_LOOP:
        case KW_WHILE: case KW_FOR: case KW_UNSAFE: case KW_MOVE:
        case KW_RETURN: case KW_BREAK: case KW_CONTINUE: case KW_LET:
        case KW_UNDERSCORE: case KW_STATIC:
            return true;
        default:
            return false;
        }
    default:
        return false;
    }
}

/* Binary operator info for the current token. */
typedef enum { OPK_NONE, OPK_BIN, OPK_ASSIGN, OPK_ASSIGN_OP, OPK_RANGE, OPK_CAST } OpKind;

static OpKind op_info(const Parser *p, int *prec, BinOp *op) {
    const Token *t = tok(p);
    switch (t->kind) {
    case TK_EQ: *prec = PREC_ASSIGN; return OPK_ASSIGN;
#define AOP(tk, bop) case tk: *prec = PREC_ASSIGN; *op = bop; return OPK_ASSIGN_OP;
    AOP(TK_PLUSEQ, BIN_ADD) AOP(TK_MINUSEQ, BIN_SUB) AOP(TK_STAREQ, BIN_MUL)
    AOP(TK_SLASHEQ, BIN_DIV) AOP(TK_PERCENTEQ, BIN_REM)
    AOP(TK_CARETEQ, BIN_BITXOR) AOP(TK_ANDEQ, BIN_BITAND)
    AOP(TK_OREQ, BIN_BITOR) AOP(TK_SHLEQ, BIN_SHL) AOP(TK_SHREQ, BIN_SHR)
#undef AOP
    case TK_DOTDOT: case TK_DOTDOTEQ: *prec = PREC_RANGE; return OPK_RANGE;
#define BOP(tk, pr, bop) case tk: *prec = pr; *op = bop; return OPK_BIN;
    BOP(TK_OROR, PREC_OR, BIN_OR) BOP(TK_ANDAND, PREC_AND, BIN_AND)
    BOP(TK_EQEQ, PREC_CMP, BIN_EQ) BOP(TK_NE, PREC_CMP, BIN_NE)
    BOP(TK_LT, PREC_CMP, BIN_LT) BOP(TK_GT, PREC_CMP, BIN_GT)
    BOP(TK_LE, PREC_CMP, BIN_LE) BOP(TK_GE, PREC_CMP, BIN_GE)
    BOP(TK_OR, PREC_BITOR, BIN_BITOR) BOP(TK_CARET, PREC_BITXOR, BIN_BITXOR)
    BOP(TK_AND, PREC_BITAND, BIN_BITAND) BOP(TK_SHL, PREC_SHIFT, BIN_SHL)
    BOP(TK_SHR, PREC_SHIFT, BIN_SHR) BOP(TK_PLUS, PREC_ADD, BIN_ADD)
    BOP(TK_MINUS, PREC_ADD, BIN_SUB) BOP(TK_STAR, PREC_MUL, BIN_MUL)
    BOP(TK_SLASH, PREC_MUL, BIN_DIV) BOP(TK_PERCENT, PREC_MUL, BIN_REM)
#undef BOP
    case TK_IDENT:
        if (tok_is_kw(t, KW_AS)) {
            *prec = PREC_CAST;
            return OPK_CAST;
        }
        return OPK_NONE;
    default:
        return OPK_NONE;
    }
}

static Expr *parse_expr(Parser *p) { return parse_assoc(p, 0, 0); }
static Expr *parse_expr_res(Parser *p, int r) { return parse_assoc(p, 0, r); }

static Expr *parse_assoc(Parser *p, int min_prec, int r) {
    Span start = start_span(p);
    if ((at(p, TK_DOTDOT) || at(p, TK_DOTDOTEQ)) && min_prec <= PREC_RANGE) {
        bool incl = at(p, TK_DOTDOTEQ);
        bump(p);
        Expr *e = ast_new_expr(EXPR_RANGE, start);
        e->range.inclusive = incl;
        if (can_begin_expr(p, r))
            e->range.hi = parse_assoc(p, PREC_RANGE + 1, r);
        e->span = end_span(p, start);
        return parse_assoc_rest(p, e, min_prec, r);
    }
    Expr *lhs = parse_unary(p, r);
    return parse_assoc_rest(p, lhs, min_prec, r);
}

static Expr *parse_assoc_rest(Parser *p, Expr *lhs, int min_prec, int r) {
    Span start = lhs->span;
    for (;;) {
        int prec = 0;
        BinOp op = BIN_ADD;
        OpKind k = op_info(p, &prec, &op);
        if (k == OPK_NONE || prec < min_prec)
            break;
        bool incl = at(p, TK_DOTDOTEQ);
        bump(p);
        Expr *e;
        switch (k) {
        case OPK_CAST:
            e = ast_new_expr(EXPR_CAST, start);
            e->cast.expr = lhs;
            e->cast.ty = parse_ty_no_plus(p);
            break;
        case OPK_RANGE:
            e = ast_new_expr(EXPR_RANGE, start);
            e->range.lo = lhs;
            e->range.inclusive = incl;
            if (can_begin_expr(p, r))
                e->range.hi = parse_assoc(p, PREC_RANGE + 1, r);
            break;
        case OPK_ASSIGN:
            e = ast_new_expr(EXPR_ASSIGN, start);
            e->assign.lhs = lhs;
            e->assign.rhs = parse_assoc(p, PREC_ASSIGN, r);
            break;
        case OPK_ASSIGN_OP:
            e = ast_new_expr(EXPR_ASSIGN_OP, start);
            e->assign.op = op;
            e->assign.lhs = lhs;
            e->assign.rhs = parse_assoc(p, PREC_ASSIGN, r);
            break;
        default:
            e = ast_new_expr(EXPR_BINARY, start);
            e->binary.op = op;
            e->binary.lhs = lhs;
            e->binary.rhs = parse_assoc(p, prec + 1, r);
            break;
        }
        e->span = end_span(p, start);
        lhs = e;
    }
    return lhs;
}

static Expr *parse_unary(Parser *p, int r) {
    Span start = start_span(p);
    if (at(p, TK_POUND) && peek_kind(p, 1) == TK_LBRACKET) {
        AttrList attrs = {0};
        parse_outer_attrs(p, &attrs);
        Expr *e = parse_unary(p, r);
        for (uint32_t i = 0; i < attrs.len; i++)
            vec_push(e->attrs, attrs.data[i]);
        return e;
    }
    UnOp uop;
    switch (kind(p)) {
    case TK_MINUS: uop = UN_NEG; goto unary;
    case TK_NOT: uop = UN_NOT; goto unary;
    case TK_STAR: uop = UN_DEREF; goto unary;
    unary: {
        bump(p);
        Expr *operand = parse_unary(p, r);
        Expr *e = ast_new_expr(EXPR_UNARY, end_span(p, start));
        e->unary.op = uop;
        e->unary.operand = operand;
        return e;
    }
    case TK_AND:
    case TK_ANDAND: {
        eat_amp(p);
        Expr *e = ast_new_expr(EXPR_ADDR_OF, start);
        if (at_kw(p, KW_RAW) && (peek_kw(p, 1, KW_CONST) || peek_kw(p, 1, KW_MUT))) {
            bump(p);
            e->addr_of.raw = true;
            if (eat_kw(p, KW_MUT))
                e->addr_of.is_mut = true;
            else
                expect_kw(p, KW_CONST);
        } else if (eat_kw(p, KW_MUT)) {
            e->addr_of.is_mut = true;
        }
        e->addr_of.operand = parse_unary(p, r);
        e->span = end_span(p, start);
        return e;
    }
    default:
        break;
    }
    Expr *e = parse_primary(p, r);
    return parse_postfix(p, e);
}

static ExprList parse_call_args(Parser *p) {
    ExprList args = {0};
    expect(p, TK_LPAREN);
    while (!at(p, TK_RPAREN)) {
        vec_push(args, parse_expr(p));
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RPAREN);
    return args;
}

static Expr *parse_postfix(Parser *p, Expr *e) {
    Span start = e->span;
    for (;;) {
        if (at(p, TK_QUESTION)) {
            bump(p);
            Expr *t = ast_new_expr(EXPR_TRY, end_span(p, start));
            t->try_ = e;
            e = t;
        } else if (at(p, TK_DOT)) {
            bump(p);
            const Token *t = tok(p);
            if (t->kind == TK_IDENT) {
                if (tok_is_kw(t, KW_AWAIT))
                    fatal(t->span, "unsupported: .await");
                PathSegment *seg = NEW(PathSegment);
                seg->id = ast_fresh_id();
                seg->span = t->span;
                seg->name = t->v.sym;
                bump(p);
                if (at(p, TK_PATHSEP)) {
                    bump(p);
                    seg->args = parse_generic_args(p);
                    seg->span = end_span(p, seg->span);
                    if (!at(p, TK_LPAREN))
                        error_here(p, "expected `(` after turbofish method");
                }
                if (at(p, TK_LPAREN)) {
                    Expr *m = ast_new_expr(EXPR_METHOD_CALL, start);
                    m->method_call.receiver = e;
                    m->method_call.method = seg;
                    m->method_call.args = parse_call_args(p);
                    m->span = end_span(p, start);
                    e = m;
                } else {
                    Expr *f = ast_new_expr(EXPR_FIELD, end_span(p, start));
                    f->field.base = e;
                    f->field.name = seg->name;
                    e = f;
                }
            } else if (t->kind == TK_LITERAL && t->v.lit->kind == LIT_INT &&
                       !t->v.lit->suffix) {
                bump(p);
                Expr *f = ast_new_expr(EXPR_FIELD, end_span(p, start));
                f->field.base = e;
                f->field.is_index = true;
                f->field.index = (uint32_t)t->v.lit->int_lo;
                f->field.name = sym_intern_cstr(
                    arena_printf("%u", f->field.index));
                e = f;
            } else {
                error_here(p, "expected field name or method after `.`");
            }
        } else if (at(p, TK_LPAREN)) {
            Expr *c = ast_new_expr(EXPR_CALL, start);
            c->call.callee = e;
            c->call.args = parse_call_args(p);
            c->span = end_span(p, start);
            e = c;
        } else if (at(p, TK_LBRACKET)) {
            bump(p);
            Expr *ix = ast_new_expr(EXPR_INDEX, start);
            ix->index.base = e;
            ix->index.index = parse_expr(p);
            expect(p, TK_RBRACKET);
            ix->span = end_span(p, start);
            e = ix;
        } else {
            break;
        }
    }
    return e;
}

/* Wrap a block in an EXPR_BLOCK. */
static Expr *block_expr(Block *b, Sym label, Span sp) {
    Expr *e = ast_new_expr(EXPR_BLOCK, sp);
    e->block.block = b;
    e->block.label = label;
    return e;
}

static Expr *parse_if(Parser *p) {
    Span start = start_span(p);
    expect_kw(p, KW_IF);
    Expr *e = ast_new_expr(EXPR_IF, start);
    e->if_.cond = parse_expr_res(p, R_NO_STRUCT);
    e->if_.then = parse_block(p);
    if (eat_kw(p, KW_ELSE)) {
        if (at_kw(p, KW_IF)) {
            e->if_.els = parse_if(p);
        } else {
            Span bs = start_span(p);
            Block *b = parse_block(p);
            e->if_.els = block_expr(b, 0, end_span(p, bs));
        }
    }
    e->span = end_span(p, start);
    return e;
}

static bool expr_is_blocklike(const Expr *e) {
    switch (e->kind) {
    case EXPR_BLOCK: case EXPR_IF: case EXPR_MATCH: case EXPR_WHILE:
    case EXPR_FOR: case EXPR_LOOP:
        return true;
    case EXPR_MAC:
        return e->mac->delim == DELIM_BRACE;
    default:
        return false;
    }
}

static Expr *parse_match(Parser *p) {
    Span start = start_span(p);
    expect_kw(p, KW_MATCH);
    Expr *e = ast_new_expr(EXPR_MATCH, start);
    e->match.scrutinee = parse_expr_res(p, R_NO_STRUCT);
    expect(p, TK_LBRACE);
    AttrList inner = {0};
    parse_inner_attrs(p, &inner);
    while (!at(p, TK_RBRACE)) {
        Arm *arm = NEW(Arm);
        arm->id = ast_fresh_id();
        arm->span = start_span(p);
        parse_outer_attrs(p, &arm->attrs);
        arm->pat = parse_pat_top(p);
        if (eat_kw(p, KW_IF))
            arm->guard = parse_expr(p);
        expect(p, TK_FATARROW);
        /* Arm body: a block-like body ends the arm (comma optional). */
        bool complete = false;
        arm->body = parse_stmt_like_expr(p, &complete);
        arm->span = end_span(p, arm->span);
        vec_push(e->match.arms, arm);
        if (eat(p, TK_COMMA))
            continue;
        if (at(p, TK_RBRACE))
            break;
        if (!complete)
            error_here(p, "expected `,` after match arm");
    }
    expect(p, TK_RBRACE);
    e->span = end_span(p, start);
    return e;
}

static Expr *parse_closure(Parser *p, int r) {
    Span start = start_span(p);
    Expr *e = ast_new_expr(EXPR_CLOSURE, start);
    if (eat_kw(p, KW_MOVE))
        e->closure.is_move = true;
    if (eat(p, TK_OROR)) {
        /* no params */
    } else {
        if (!eat_pipe(p))
            error_here(p, "expected `|`");
        while (!at(p, TK_OR)) {
            Param *pa = NEW(Param);
            pa->id = ast_fresh_id();
            pa->span = start_span(p);
            parse_outer_attrs(p, &pa->attrs);
            pa->pat = parse_pat_no_top_alt(p);
            if (eat(p, TK_COLON))
                pa->ty = parse_ty_no_plus(p);
            pa->span = end_span(p, pa->span);
            vec_push(e->closure.params, pa);
            if (!eat(p, TK_COMMA))
                break;
        }
        expect(p, TK_OR);
    }
    if (eat(p, TK_RARROW)) {
        e->closure.ret = parse_ty_no_plus(p);
        Span bs = start_span(p);
        Block *b = parse_block(p);
        e->closure.body = block_expr(b, 0, end_span(p, bs));
    } else {
        e->closure.body = parse_assoc(p, 0, r);
    }
    e->span = end_span(p, start);
    return e;
}

static Expr *parse_struct_lit(Parser *p, QSelf *qself, Path *path, Span start) {
    Expr *e = ast_new_expr(EXPR_STRUCT, start);
    e->struct_.qself = qself;
    e->struct_.path = path;
    expect(p, TK_LBRACE);
    while (!at(p, TK_RBRACE)) {
        AttrList attrs = {0};
        parse_outer_attrs(p, &attrs);
        if (at(p, TK_DOTDOT)) {
            bump(p);
            if (at(p, TK_RBRACE))
                e->struct_.has_rest = true;
            else
                e->struct_.base = parse_expr(p);
            break;
        }
        ExprField *f = NEW(ExprField);
        f->id = ast_fresh_id();
        f->span = start_span(p);
        f->attrs = attrs;
        if (at(p, TK_LITERAL) && tok(p)->v.lit->kind == LIT_INT) {
            f->name = sym_intern_cstr(arena_printf(
                "%llu", (unsigned long long)tok(p)->v.lit->int_lo));
            bump(p);
            expect(p, TK_COLON);
            f->expr = parse_expr(p);
        } else {
            Span ns = start_span(p);
            f->name = expect_ident(p);
            if (eat(p, TK_COLON)) {
                f->expr = parse_expr(p);
            } else {
                f->shorthand = true;
                f->expr = ast_new_expr(EXPR_PATH, ns);
                f->expr->path.path = ast_path_from_sym(f->name, ns);
            }
        }
        f->span = end_span(p, f->span);
        vec_push(e->struct_.fields, f);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RBRACE);
    e->span = end_span(p, start);
    return e;
}

/* Expression starting with a path (or qualified path). */
static Expr *parse_path_expr(Parser *p, int r) {
    Span start = start_span(p);
    QSelf *qself = NULL;
    Path *path;
    if (at(p, TK_LT) || at(p, TK_SHL)) {
        path = parse_qpath(p, &qself, false);
    } else {
        if (at_macro_call(p)) {
            path = parse_path(p, false);
            Expr *e = ast_new_expr(EXPR_MAC, start);
            e->mac = parse_mac_call(p, path);
            e->span = end_span(p, start);
            return e;
        }
        path = parse_path(p, false);
    }
    if (at(p, TK_LBRACE) && !(r & R_NO_STRUCT))
        return parse_struct_lit(p, qself, path, start);
    Expr *e = ast_new_expr(EXPR_PATH, end_span(p, start));
    e->path.qself = qself;
    e->path.path = path;
    return e;
}

/* Loop-ish expressions with an optional label already parsed. */
static Expr *parse_labeled(Parser *p, Sym label, Span start) {
    Expr *e;
    if (eat_kw(p, KW_LOOP)) {
        e = ast_new_expr(EXPR_LOOP, start);
        e->loop.label = label;
        e->loop.body = parse_block(p);
    } else if (eat_kw(p, KW_WHILE)) {
        e = ast_new_expr(EXPR_WHILE, start);
        e->while_.label = label;
        e->while_.cond = parse_expr_res(p, R_NO_STRUCT);
        e->while_.body = parse_block(p);
    } else if (eat_kw(p, KW_FOR)) {
        e = ast_new_expr(EXPR_FOR, start);
        e->for_.label = label;
        e->for_.pat = parse_pat_top(p);
        expect_kw(p, KW_IN);
        e->for_.iter = parse_expr_res(p, R_NO_STRUCT);
        e->for_.body = parse_block(p);
    } else if (at(p, TK_LBRACE) || (at_kw(p, KW_UNSAFE) && peek_kind(p, 1) == TK_LBRACE)) {
        bool uns = eat_kw(p, KW_UNSAFE);
        Block *b = parse_block(p);
        b->is_unsafe = uns;
        e = block_expr(b, label, start);
    } else {
        error_here(p, "expected `loop`, `while`, `for` or block after label");
    }
    e->span = end_span(p, start);
    return e;
}

static Expr *parse_primary(Parser *p, int r) {
    Span start = start_span(p);
    const Token *t = tok(p);
    Expr *e;
    switch (t->kind) {
    case TK_INTERP:
        switch (t->nt_kind) {
        case NT_EXPR:
        case NT_LIT:
            bump(p);
            return (Expr *)t->v.node;
        case NT_BLOCK:
            bump(p);
            return block_expr((Block *)t->v.node, 0, t->span);
        case NT_PATH:
            return parse_path_expr(p, r);
        default:
            error_here(p, "expected expression");
        }
    case TK_LITERAL:
        bump(p);
        return new_lit_expr(t->v.lit, t->span);
    case TK_LPAREN: {
        bump(p);
        if (eat(p, TK_RPAREN))
            return ast_new_expr(EXPR_TUPLE, end_span(p, start));
        Expr *first = parse_expr(p);
        if (eat(p, TK_RPAREN))
            return first; /* parenthesised */
        e = ast_new_expr(EXPR_TUPLE, start);
        vec_push(e->elems, first);
        while (eat(p, TK_COMMA)) {
            if (at(p, TK_RPAREN))
                break;
            vec_push(e->elems, parse_expr(p));
        }
        expect(p, TK_RPAREN);
        e->span = end_span(p, start);
        return e;
    }
    case TK_LBRACKET: {
        bump(p);
        if (eat(p, TK_RBRACKET))
            return ast_new_expr(EXPR_ARRAY, end_span(p, start));
        Expr *first = parse_expr(p);
        if (eat(p, TK_SEMI)) {
            e = ast_new_expr(EXPR_REPEAT, start);
            e->repeat.elem = first;
            e->repeat.count = parse_expr(p);
            expect(p, TK_RBRACKET);
        } else {
            e = ast_new_expr(EXPR_ARRAY, start);
            vec_push(e->elems, first);
            while (eat(p, TK_COMMA)) {
                if (at(p, TK_RBRACKET))
                    break;
                vec_push(e->elems, parse_expr(p));
            }
            expect(p, TK_RBRACKET);
        }
        e->span = end_span(p, start);
        return e;
    }
    case TK_LBRACE: {
        Block *b = parse_block(p);
        return block_expr(b, 0, end_span(p, start));
    }
    case TK_OR:
    case TK_OROR:
        return parse_closure(p, r);
    case TK_LT:
    case TK_SHL:
    case TK_PATHSEP:
        return parse_path_expr(p, r);
    case TK_LIFETIME:
        if (peek_kind(p, 1) == TK_COLON) {
            Sym label = t->v.sym;
            bump(p);
            bump(p);
            return parse_labeled(p, label, start);
        }
        error_here(p, "expected expression");
    case TK_IDENT:
        break;
    default:
        error_here(p, "expected expression");
    }

    /* identifiers and keywords */
    if (t->flags & TF_RAW)
        return parse_path_expr(p, r);
    switch (t->v.sym) {
    case KW_TRUE:
    case KW_FALSE:
        bump(p);
        return new_lit_expr(bool_lit(t->v.sym == KW_TRUE), t->span);
    case KW_MOVE:
        return parse_closure(p, r);
    case KW_STATIC:
        fatal(t->span, "unsupported: static closures / coroutines");
    case KW_ASYNC:
        fatal(t->span, "unsupported: async");
    case KW_UNSAFE: {
        bump(p);
        Block *b = parse_block(p);
        b->is_unsafe = true;
        return block_expr(b, 0, end_span(p, start));
    }
    case KW_CONST:
        if (peek_kind(p, 1) == TK_LBRACE)
            fatal(t->span, "unsupported: inline const blocks");
        break;
    case KW_IF:
        return parse_if(p);
    case KW_MATCH:
        return parse_match(p);
    case KW_LOOP:
    case KW_WHILE:
    case KW_FOR:
        return parse_labeled(p, 0, start);
    case KW_RETURN:
        bump(p);
        e = ast_new_expr(EXPR_RETURN, start);
        if (can_begin_expr(p, r))
            e->ret = parse_assoc(p, 0, r);
        e->span = end_span(p, start);
        return e;
    case KW_BREAK:
        bump(p);
        e = ast_new_expr(EXPR_BREAK, start);
        if (at(p, TK_LIFETIME)) {
            e->break_.label = tok(p)->v.sym;
            bump(p);
        }
        if (can_begin_expr(p, r))
            e->break_.value = parse_assoc(p, 0, r);
        e->span = end_span(p, start);
        return e;
    case KW_CONTINUE:
        bump(p);
        e = ast_new_expr(EXPR_CONTINUE, start);
        if (at(p, TK_LIFETIME)) {
            e->continue_.label = tok(p)->v.sym;
            bump(p);
        }
        e->span = end_span(p, start);
        return e;
    case KW_LET:
        bump(p);
        e = ast_new_expr(EXPR_LET, start);
        e->let.pat = parse_pat_top(p);
        expect(p, TK_EQ);
        e->let.scrutinee = parse_assoc(p, PREC_AND + 1, r | R_NO_STRUCT);
        e->span = end_span(p, start);
        return e;
    case KW_UNDERSCORE:
        bump(p);
        return ast_new_expr(EXPR_UNDERSCORE, start);
    default:
        break;
    }
    if (is_path_seg_tok(t))
        return parse_path_expr(p, r);
    error_here(p, "expected expression");
}

/* Does the current token start a block-like expression statement? */
static bool at_blocklike_start(const Parser *p) {
    const Token *t = tok(p);
    if (t->kind == TK_LBRACE)
        return true;
    if (t->kind == TK_LIFETIME && peek_kind(p, 1) == TK_COLON)
        return true;
    if (t->kind == TK_INTERP && t->nt_kind == NT_BLOCK)
        return true;
    if (t->kind != TK_IDENT || (t->flags & TF_RAW))
        return false;
    switch (t->v.sym) {
    case KW_IF: case KW_MATCH: case KW_LOOP: case KW_WHILE: case KW_FOR:
        return true;
    case KW_UNSAFE:
        return peek_kind(p, 1) == TK_LBRACE;
    default:
        return false;
    }
}

/*
 * Expression in statement (or match-arm body) position. If it is a
 * block-like expression (if/match/loop/while/for/block) not followed by
 * `.` or `?`, it ends there and *complete is set: it needs no `;`/`,`.
 */
static Expr *parse_stmt_like_expr(Parser *p, bool *complete) {
    *complete = false;
    AttrList attrs = {0};
    parse_outer_attrs(p, &attrs);
    Expr *e;
    if (at_blocklike_start(p)) {
        e = parse_primary(p, 0);
        if (at(p, TK_DOT) || at(p, TK_QUESTION)) {
            e = parse_postfix(p, e);
            e = parse_assoc_rest(p, e, 0, 0);
        } else {
            *complete = true;
        }
    } else {
        e = parse_expr(p);
        if (expr_is_blocklike(e) && e->kind == EXPR_MAC)
            *complete = true;
    }
    for (uint32_t i = 0; i < attrs.len; i++)
        vec_push(e->attrs, attrs.data[i]);
    return e;
}

/* ------------------------------------------------------------------ */
/* Statements and blocks                                               */
/* ------------------------------------------------------------------ */

static bool at_item_start(const Parser *p) {
    const Token *t = tok(p);
    if (t->kind == TK_INTERP)
        return t->nt_kind == NT_ITEM || t->nt_kind == NT_VIS;
    if (t->kind != TK_IDENT || (t->flags & TF_RAW))
        return false;
    switch (t->v.sym) {
    case KW_FN: case KW_STRUCT: case KW_ENUM: case KW_TRAIT: case KW_IMPL:
    case KW_MOD: case KW_USE: case KW_TYPE: case KW_EXTERN: case KW_PUB:
        return true;
    case KW_STATIC:
        /* `static ||` closures are not supported anyway */
        return true;
    case KW_CONST:
        return peek_kind(p, 1) != TK_LBRACE;
    case KW_UNSAFE:
        return peek_kw(p, 1, KW_FN) || peek_kw(p, 1, KW_IMPL) ||
               peek_kw(p, 1, KW_TRAIT) || peek_kw(p, 1, KW_EXTERN) ||
               peek_kw(p, 1, KW_MOD) || peek_kw(p, 1, KW_AUTO);
    case KW_UNION:
        return tok_is_plain_ident(peek(p, 1));
    case KW_AUTO:
        return peek_kw(p, 1, KW_TRAIT);
    case KW_MACRO_RULES:
        return peek_kind(p, 1) == TK_NOT && peek_kind(p, 2) == TK_IDENT;
    default:
        return false;
    }
}

static Stmt *parse_let_stmt(Parser *p, AttrList attrs, Span start) {
    expect_kw(p, KW_LET);
    Stmt *s = ast_new_stmt(STMT_LET, start);
    s->attrs = attrs;
    s->let.pat = parse_pat_top(p);
    if (eat(p, TK_COLON))
        s->let.ty = parse_ty(p);
    if (eat(p, TK_EQ)) {
        s->let.init = parse_expr(p);
        if (eat_kw(p, KW_ELSE))
            s->let.els = parse_block(p);
    }
    expect(p, TK_SEMI);
    s->span = end_span(p, start);
    return s;
}

static bool at_block_end(const Parser *p) {
    return at(p, TK_RBRACE) || at_eof(p);
}

/* Parse one statement; NULL at end of block. */
static Stmt *parse_stmt_opt(Parser *p) {
    if (at_block_end(p))
        return NULL;
    Span start = start_span(p);
    if (at(p, TK_SEMI)) {
        bump(p);
        return ast_new_stmt(STMT_EMPTY, start);
    }
    AttrList attrs = {0};
    parse_outer_attrs(p, &attrs);
    if (at_interp(p, NT_STMT)) {
        Stmt *s = (Stmt *)tok(p)->v.node;
        bump(p);
        for (uint32_t i = 0; i < attrs.len; i++)
            vec_push(s->attrs, attrs.data[i]);
        if (s->kind == STMT_EXPR && !s->expr.semi && eat(p, TK_SEMI))
            s->expr.semi = true;
        return s;
    }
    if (at_kw(p, KW_LET))
        return parse_let_stmt(p, attrs, start);
    if (at_item_start(p)) {
        Item *it = parse_item_opt(p, attrs);
        if (!it)
            error_here(p, "expected item");
        Stmt *s = ast_new_stmt(STMT_ITEM, it->span);
        s->item = it;
        return s;
    }
    if (at_macro_call(p)) {
        Path *path = parse_path(p, false);
        MacCall *mac = parse_mac_call(p, path);
        if (mac->delim == DELIM_BRACE || at(p, TK_SEMI) || at_block_end(p)) {
            Stmt *s = ast_new_stmt(STMT_MAC, start);
            s->attrs = attrs;
            s->mac.mac = mac;
            s->mac.semi = eat(p, TK_SEMI);
            s->span = end_span(p, start);
            return s;
        }
        Expr *e = ast_new_expr(EXPR_MAC, mac->span);
        e->mac = mac;
        e = parse_postfix(p, e);
        e = parse_assoc_rest(p, e, 0, 0);
        Stmt *s = ast_new_stmt(STMT_EXPR, start);
        s->attrs = attrs;
        s->expr.expr = e;
        if (eat(p, TK_SEMI))
            s->expr.semi = true;
        else if (!at_block_end(p))
            error_here(p, "expected `;`");
        s->span = end_span(p, start);
        return s;
    }
    bool complete = false;
    Expr *e = parse_stmt_like_expr(p, &complete);
    Stmt *s = ast_new_stmt(STMT_EXPR, start);
    s->attrs = attrs;
    s->expr.expr = e;
    if (eat(p, TK_SEMI))
        s->expr.semi = true;
    else if (!complete && !at_block_end(p))
        error_here(p, "expected `;` or `}` after expression");
    s->span = end_span(p, start);
    return s;
}

static Block *parse_block(Parser *p) {
    Span start = start_span(p);
    if (at_interp(p, NT_BLOCK)) {
        Block *b = (Block *)tok(p)->v.node;
        bump(p);
        return b;
    }
    expect(p, TK_LBRACE);
    Block *b = ast_new_block(start);
    parse_inner_attrs(p, &b->attrs);
    Stmt *s;
    while ((s = parse_stmt_opt(p)) != NULL)
        vec_push(b->stmts, s);
    expect(p, TK_RBRACE);
    b->span = end_span(p, start);
    return b;
}

/* ------------------------------------------------------------------ */
/* Items                                                               */
/* ------------------------------------------------------------------ */

static Sym parse_abi_opt(Parser *p) {
    if (at(p, TK_LITERAL) && tok(p)->v.lit->kind == LIT_STR) {
        Sym s = sym_intern(tok(p)->v.lit->str, tok(p)->v.lit->str_len);
        bump(p);
        return s;
    }
    return sym_intern_cstr("C");
}

static void parse_fn_params(Parser *p, FnSig *sig) {
    expect(p, TK_LPAREN);
    bool first = true;
    while (!at(p, TK_RPAREN)) {
        Span ps = start_span(p);
        AttrList attrs = {0};
        parse_outer_attrs(p, &attrs);
        if (first) {
            /* self receivers */
            uint32_t k = 0;
            bool ref = false;
            if (at(p, TK_AND)) {
                ref = true;
                k = 1;
                if (peek_kind(p, k) == TK_LIFETIME)
                    k++;
                if (peek_kw(p, k, KW_MUT))
                    k++;
            } else if (at_kw(p, KW_MUT)) {
                k = 1;
            }
            if (peek_kw(p, k, KW_SELF_VALUE) && peek_kind(p, k + 1) != TK_PATHSEP) {
                SelfParam *sp = NEW(SelfParam);
                sp->id = ast_fresh_id();
                sp->attrs = attrs;
                if (ref) {
                    sp->kind = SELF_REF;
                    bump(p);
                    if (at(p, TK_LIFETIME)) {
                        sp->lifetime = tok(p)->v.sym;
                        bump(p);
                    }
                    if (eat_kw(p, KW_MUT))
                        sp->is_mut = true;
                    bump(p); /* self */
                } else {
                    sp->kind = SELF_VALUE;
                    if (eat_kw(p, KW_MUT))
                        sp->is_mut = true;
                    bump(p); /* self */
                    if (eat(p, TK_COLON)) {
                        sp->kind = SELF_EXPLICIT;
                        sp->ty = parse_ty(p);
                    }
                }
                sp->span = end_span(p, ps);
                sig->self_param = sp;
                first = false;
                if (!eat(p, TK_COMMA))
                    break;
                continue;
            }
        }
        first = false;
        if (eat(p, TK_DOTDOTDOT)) {
            sig->variadic = true;
            eat(p, TK_COMMA);
            break;
        }
        Param *pa = NEW(Param);
        pa->id = ast_fresh_id();
        pa->attrs = attrs;
        pa->pat = parse_pat_no_top_alt(p);
        expect(p, TK_COLON);
        if (eat(p, TK_DOTDOTDOT)) { /* `args: ...` */
            sig->variadic = true;
            eat(p, TK_COMMA);
            break;
        }
        pa->ty = parse_ty(p);
        pa->span = end_span(p, ps);
        vec_push(sig->params, pa);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RPAREN);
}

/* At the fn qualifiers (const/unsafe/extern/safe) or `fn`. */
static Item *parse_fn(Parser *p, Span start) {
    Item *it = ast_new_item(ITEM_FN, start);
    FnSig *sig = &it->fn_.sig;
    sig->span = start_span(p);
    for (;;) {
        if (eat_kw(p, KW_CONST))
            sig->is_const = true;
        else if (eat_kw(p, KW_UNSAFE))
            sig->is_unsafe = true;
        else if (at_kw(p, KW_SAFE) && peek_kw(p, 1, KW_FN))
            bump(p);
        else if (at_kw(p, KW_ASYNC))
            fatal(tok(p)->span, "unsupported: async fn");
        else if (eat_kw(p, KW_EXTERN)) {
            sig->is_extern = true;
            sig->abi = parse_abi_opt(p);
        } else
            break;
    }
    expect_kw(p, KW_FN);
    it->name = expect_ident(p);
    parse_generics(p, &it->fn_.generics);
    parse_fn_params(p, sig);
    if (eat(p, TK_RARROW))
        sig->ret = parse_ty(p);
    sig->span = end_span(p, sig->span);
    parse_where(p, &it->fn_.generics);
    if (!eat(p, TK_SEMI))
        it->fn_.body = parse_block(p);
    it->span = end_span(p, start);
    return it;
}

static FieldDefList parse_named_fields(Parser *p) {
    FieldDefList list = {0};
    expect(p, TK_LBRACE);
    while (!at(p, TK_RBRACE)) {
        FieldDef *f = NEW(FieldDef);
        f->id = ast_fresh_id();
        f->span = start_span(p);
        parse_outer_attrs(p, &f->attrs);
        f->vis = parse_vis(p);
        f->name = expect_ident(p);
        expect(p, TK_COLON);
        f->ty = parse_ty(p);
        f->span = end_span(p, f->span);
        vec_push(list, f);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RBRACE);
    return list;
}

static FieldDefList parse_tuple_fields(Parser *p) {
    FieldDefList list = {0};
    expect(p, TK_LPAREN);
    while (!at(p, TK_RPAREN)) {
        FieldDef *f = NEW(FieldDef);
        f->id = ast_fresh_id();
        f->span = start_span(p);
        parse_outer_attrs(p, &f->attrs);
        f->vis = parse_vis(p);
        f->ty = parse_ty(p);
        f->span = end_span(p, f->span);
        vec_push(list, f);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RPAREN);
    return list;
}

static Item *parse_struct_or_union(Parser *p, Span start, bool is_union) {
    Item *it = ast_new_item(is_union ? ITEM_UNION : ITEM_STRUCT, start);
    bump(p); /* struct / union */
    it->name = expect_ident(p);
    parse_generics(p, &it->adt.generics);
    parse_where(p, &it->adt.generics);
    if (at(p, TK_LBRACE)) {
        it->adt.data.kind = VD_STRUCT;
        it->adt.data.fields = parse_named_fields(p);
    } else if (!is_union && at(p, TK_LPAREN)) {
        it->adt.data.kind = VD_TUPLE;
        it->adt.data.fields = parse_tuple_fields(p);
        parse_where(p, &it->adt.generics);
        expect(p, TK_SEMI);
    } else if (!is_union) {
        it->adt.data.kind = VD_UNIT;
        expect(p, TK_SEMI);
    } else {
        error_here(p, "expected `{` in union");
    }
    it->span = end_span(p, start);
    return it;
}

static Item *parse_enum(Parser *p, Span start) {
    Item *it = ast_new_item(ITEM_ENUM, start);
    expect_kw(p, KW_ENUM);
    it->name = expect_ident(p);
    parse_generics(p, &it->enum_.generics);
    parse_where(p, &it->enum_.generics);
    expect(p, TK_LBRACE);
    while (!at(p, TK_RBRACE)) {
        Variant *v = NEW(Variant);
        v->id = ast_fresh_id();
        v->span = start_span(p);
        parse_outer_attrs(p, &v->attrs);
        v->vis = parse_vis(p);
        v->name = expect_ident(p);
        if (at(p, TK_LBRACE)) {
            v->data.kind = VD_STRUCT;
            v->data.fields = parse_named_fields(p);
        } else if (at(p, TK_LPAREN)) {
            v->data.kind = VD_TUPLE;
            v->data.fields = parse_tuple_fields(p);
        } else {
            v->data.kind = VD_UNIT;
        }
        if (eat(p, TK_EQ))
            v->disc = parse_expr(p);
        v->span = end_span(p, v->span);
        vec_push(it->enum_.variants, v);
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RBRACE);
    it->span = end_span(p, start);
    return it;
}

/* Items inside `{ ... }` of trait / impl / extern block / inline mod. */
static ItemList parse_item_body(Parser *p, AttrList *inner_attrs) {
    ItemList items = {0};
    expect(p, TK_LBRACE);
    parse_inner_attrs(p, inner_attrs);
    while (!at(p, TK_RBRACE)) {
        AttrList attrs = {0};
        parse_outer_attrs(p, &attrs);
        if (at(p, TK_SEMI) && attrs.len == 0) {
            bump(p);
            continue;
        }
        Item *it = parse_item_opt(p, attrs);
        if (!it)
            error_here(p, "expected item");
        vec_push(items, it);
    }
    expect(p, TK_RBRACE);
    return items;
}

static Item *parse_trait(Parser *p, Span start) {
    Item *it = ast_new_item(ITEM_TRAIT, start);
    if (eat_kw(p, KW_UNSAFE))
        it->trait.is_unsafe = true;
    if (eat_kw(p, KW_AUTO))
        it->trait.is_auto = true;
    expect_kw(p, KW_TRAIT);
    it->name = expect_ident(p);
    parse_generics(p, &it->trait.generics);
    if (eat(p, TK_COLON))
        it->trait.supertraits = parse_bounds(p, true);
    parse_where(p, &it->trait.generics);
    if (at(p, TK_EQ))
        fatal(tok(p)->span, "unsupported: trait aliases");
    it->trait.items = parse_item_body(p, &it->attrs);
    it->span = end_span(p, start);
    return it;
}

/* Does `impl <` start generic params (vs. a qualified self type)? */
static bool impl_has_generics(const Parser *p) {
    if (!at(p, TK_LT))
        return false;
    const Token *t1 = peek(p, 1);
    if (t1->kind == TK_GT || t1->kind == TK_LIFETIME || t1->kind == TK_POUND ||
        tok_is_kw(t1, KW_CONST))
        return true;
    if (tok_is_plain_ident(t1)) {
        int k2 = peek_kind(p, 2);
        return k2 == TK_GT || k2 == TK_COMMA || k2 == TK_COLON || k2 == TK_EQ;
    }
    return false;
}

static Item *parse_impl(Parser *p, Span start) {
    Item *it = ast_new_item(ITEM_IMPL, start);
    if (eat_kw(p, KW_UNSAFE))
        it->impl.is_unsafe = true;
    expect_kw(p, KW_IMPL);
    if (impl_has_generics(p))
        parse_generics(p, &it->impl.generics);
    if (at(p, TK_NOT) ) {
        bump(p);
        it->impl.negative = true;
    }
    Ty *first = parse_ty_no_plus(p);
    if (eat_kw(p, KW_FOR)) {
        if (first->kind != TY_PATH || first->path.qself)
            fatal(first->span, "expected trait path in impl");
        it->impl.trait_path = first->path.path;
        it->impl.self_ty = parse_ty_no_plus(p);
    } else {
        it->impl.self_ty = first;
    }
    parse_where(p, &it->impl.generics);
    it->impl.items = parse_item_body(p, &it->attrs);
    it->span = end_span(p, start);
    return it;
}

static void parse_use_tree_into(Parser *p, UseTree *ut);

static UseTree *parse_use_tree(Parser *p) {
    UseTree *ut = NEW(UseTree);
    ut->id = ast_fresh_id();
    parse_use_tree_into(p, ut);
    return ut;
}

static void parse_use_group(Parser *p, UseTree *ut) {
    ut->kind = USE_NESTED;
    expect(p, TK_LBRACE);
    while (!at(p, TK_RBRACE)) {
        vec_push(ut->items, parse_use_tree(p));
        if (!eat(p, TK_COMMA))
            break;
    }
    expect(p, TK_RBRACE);
}

static void parse_use_tree_into(Parser *p, UseTree *ut) {
    Span start = start_span(p);
    ut->span = start;
    Path *prefix = ast_new_path(start);
    ut->prefix = prefix;
    if (eat(p, TK_PATHSEP))
        prefix->global = true;
    if (at(p, TK_STAR)) {
        bump(p);
        ut->kind = USE_GLOB;
        ut->span = end_span(p, start);
        return;
    }
    if (at(p, TK_LBRACE)) {
        parse_use_group(p, ut);
        ut->span = end_span(p, start);
        return;
    }
    for (;;) {
        PathSegment *seg = parse_path_segment_name(p);
        vec_push(prefix->segs, seg);
        if (at(p, TK_PATHSEP)) {
            bump(p);
            if (at(p, TK_STAR)) {
                bump(p);
                ut->kind = USE_GLOB;
                prefix->span = end_span(p, start);
                ut->span = end_span(p, start);
                return;
            }
            if (at(p, TK_LBRACE)) {
                prefix->span = end_span(p, start);
                parse_use_group(p, ut);
                ut->span = end_span(p, start);
                return;
            }
            continue;
        }
        break;
    }
    prefix->span = end_span(p, start);
    ut->kind = USE_SIMPLE;
    if (eat_kw(p, KW_AS)) {
        if (eat_kw(p, KW_UNDERSCORE))
            ut->rename = KW_UNDERSCORE;
        else
            ut->rename = expect_ident(p);
    }
    ut->span = end_span(p, start);
}

/*
 * Parse an item (attrs already parsed). Returns NULL, without consuming
 * anything, if no item starts here.
 */
static Item *parse_item_opt(Parser *p, AttrList attrs) {
    Span start = attrs.len ? attrs.data[0]->span : start_span(p);
    Item *it = NULL;
    if (at_interp(p, NT_ITEM)) {
        it = (Item *)tok(p)->v.node;
        bump(p);
        for (uint32_t i = 0; i < attrs.len; i++)
            vec_push(it->attrs, attrs.data[i]);
        return it;
    }
    uint32_t save_pos = p->pos;
    Visibility vis = parse_vis(p);
    const Token *t = tok(p);
    bool had_vis = vis.kind != VIS_INHERITED;

    if (tok_is_kw(t, KW_MACRO_RULES) && peek_kind(p, 1) == TK_NOT &&
        peek_kind(p, 2) == TK_IDENT) {
        it = ast_new_item(ITEM_MACRO_RULES, start);
        bump(p);
        bump(p);
        it->name = tok(p)->v.sym;
        bump(p);
        it->macro_rules.body = take_delimited(p, &it->macro_rules.delim);
        if (it->macro_rules.delim != DELIM_BRACE)
            expect(p, TK_SEMI);
    } else if (tok_is_kw(t, KW_USE)) {
        it = ast_new_item(ITEM_USE, start);
        bump(p);
        it->use_tree = parse_use_tree(p);
        expect(p, TK_SEMI);
    } else if (tok_is_kw(t, KW_EXTERN) && peek_kw(p, 1, KW_CRATE)) {
        it = ast_new_item(ITEM_EXTERN_CRATE, start);
        bump(p);
        bump(p);
        if (eat_kw(p, KW_SELF_VALUE))
            it->extern_crate.orig = KW_SELF_VALUE;
        else
            it->extern_crate.orig = expect_ident(p);
        it->name = it->extern_crate.orig;
        if (eat_kw(p, KW_AS)) {
            if (eat_kw(p, KW_UNDERSCORE))
                it->name = KW_UNDERSCORE;
            else
                it->name = expect_ident(p);
        }
        expect(p, TK_SEMI);
    } else if ((tok_is_kw(t, KW_EXTERN) &&
                (peek_kind(p, 1) == TK_LBRACE ||
                 (peek_kind(p, 1) == TK_LITERAL && peek_kind(p, 2) == TK_LBRACE))) ||
               (tok_is_kw(t, KW_UNSAFE) && peek_kw(p, 1, KW_EXTERN) &&
                (peek_kind(p, 2) == TK_LBRACE ||
                 (peek_kind(p, 2) == TK_LITERAL && peek_kind(p, 3) == TK_LBRACE)))) {
        it = ast_new_item(ITEM_FOREIGN_MOD, start);
        if (eat_kw(p, KW_UNSAFE))
            it->foreign_mod.is_unsafe = true;
        expect_kw(p, KW_EXTERN);
        it->foreign_mod.abi = parse_abi_opt(p);
        it->foreign_mod.items = parse_item_body(p, &attrs);
    } else if (tok_is_kw(t, KW_FN) ||
               (tok_is_kw(t, KW_CONST) &&
                (peek_kw(p, 1, KW_FN) || peek_kw(p, 1, KW_UNSAFE) ||
                 peek_kw(p, 1, KW_EXTERN) || peek_kw(p, 1, KW_ASYNC))) ||
               (tok_is_kw(t, KW_UNSAFE) &&
                (peek_kw(p, 1, KW_FN) || peek_kw(p, 1, KW_EXTERN))) ||
               (tok_is_kw(t, KW_EXTERN) &&
                (peek_kw(p, 1, KW_FN) ||
                 (peek_kind(p, 1) == TK_LITERAL && peek_kw(p, 2, KW_FN)))) ||
               (tok_is_kw(t, KW_SAFE) &&
                (peek_kw(p, 1, KW_FN) || peek_kw(p, 1, KW_STATIC))) ||
               tok_is_kw(t, KW_ASYNC)) {
        if (tok_is_kw(t, KW_SAFE) && peek_kw(p, 1, KW_STATIC)) {
            bump(p);
            goto static_item;
        }
        it = parse_fn(p, start);
    } else if (tok_is_kw(t, KW_CONST)) {
        it = ast_new_item(ITEM_CONST, start);
        bump(p);
        if (eat_kw(p, KW_UNDERSCORE))
            it->name = KW_UNDERSCORE;
        else
            it->name = expect_ident(p);
        parse_generics(p, &it->const_.generics);
        if (eat(p, TK_COLON))
            it->const_.ty = parse_ty(p);
        if (eat(p, TK_EQ))
            it->const_.expr = parse_expr(p);
        parse_where(p, &it->const_.generics);
        expect(p, TK_SEMI);
    } else if (tok_is_kw(t, KW_STATIC)) {
    static_item:
        it = ast_new_item(ITEM_STATIC, start);
        expect_kw(p, KW_STATIC);
        if (eat_kw(p, KW_MUT))
            it->static_.is_mut = true;
        it->name = expect_ident(p);
        expect(p, TK_COLON);
        it->static_.ty = parse_ty(p);
        if (eat(p, TK_EQ))
            it->static_.expr = parse_expr(p);
        expect(p, TK_SEMI);
    } else if (tok_is_kw(t, KW_STRUCT)) {
        it = parse_struct_or_union(p, start, false);
    } else if (tok_is_kw(t, KW_UNION) && tok_is_plain_ident(peek(p, 1))) {
        it = parse_struct_or_union(p, start, true);
    } else if (tok_is_kw(t, KW_ENUM)) {
        it = parse_enum(p, start);
    } else if (tok_is_kw(t, KW_TRAIT) ||
               (tok_is_kw(t, KW_UNSAFE) &&
                (peek_kw(p, 1, KW_TRAIT) || peek_kw(p, 1, KW_AUTO))) ||
               (tok_is_kw(t, KW_AUTO) && peek_kw(p, 1, KW_TRAIT))) {
        it = parse_trait(p, start);
    } else if (tok_is_kw(t, KW_IMPL) ||
               (tok_is_kw(t, KW_UNSAFE) && peek_kw(p, 1, KW_IMPL))) {
        it = parse_impl(p, start);
    } else if (tok_is_kw(t, KW_TYPE)) {
        it = ast_new_item(ITEM_TYPE_ALIAS, start);
        bump(p);
        it->name = expect_ident(p);
        parse_generics(p, &it->type_alias.generics);
        if (eat(p, TK_COLON))
            it->type_alias.bounds = parse_bounds(p, true);
        parse_where(p, &it->type_alias.generics);
        if (eat(p, TK_EQ))
            it->type_alias.ty = parse_ty(p);
        parse_where(p, &it->type_alias.generics);
        expect(p, TK_SEMI);
    } else if (tok_is_kw(t, KW_MOD) ||
               (tok_is_kw(t, KW_UNSAFE) && peek_kw(p, 1, KW_MOD))) {
        it = ast_new_item(ITEM_MOD, start);
        if (eat_kw(p, KW_UNSAFE))
            it->mod.is_unsafe = true;
        expect_kw(p, KW_MOD);
        it->name = expect_ident(p);
        if (!eat(p, TK_SEMI)) {
            it->mod.inline_ = true;
            it->mod.items = parse_item_body(p, &attrs);
        }
    } else if (at_macro_call(p)) {
        it = ast_new_item(ITEM_MAC, start);
        Path *path = parse_path(p, false);
        it->mac = parse_mac_call(p, path);
        if (it->mac->delim != DELIM_BRACE)
            expect(p, TK_SEMI);
    } else {
        if (had_vis)
            error_here(p, "expected item after visibility");
        p->pos = save_pos;
        return NULL;
    }
    /* Prepend outer attrs to whatever inner attrs the body added. */
    AttrList all = {0};
    for (uint32_t i = 0; i < attrs.len; i++)
        vec_push(all, attrs.data[i]);
    for (uint32_t i = 0; i < it->attrs.len; i++)
        vec_push(all, it->attrs.data[i]);
    it->attrs = all;
    it->vis = vis;
    it->span = end_span(p, start);
    return it;
}

/* ------------------------------------------------------------------ */
/* Entry points                                                        */
/* ------------------------------------------------------------------ */

static void parser_init(Parser *p, TokenList toks, Span eof_span) {
    memset(p, 0, sizeof *p);
    p->toks = toks.toks;
    p->n = toks.len;
    p->eof.kind = TK_EOF;
    p->eof.span = eof_span;
    p->prev = eof_span;
    if (toks.len)
        p->prev = toks.toks[0].span;
}

Parser *parser_new(TokenList toks, Span eof_span) {
    Parser *p = NEW(Parser);
    parser_init(p, toks, eof_span);
    return p;
}

uint32_t parser_pos(const Parser *p) { return p->pos; }
void parser_set_pos(Parser *p, uint32_t pos) {
    p->pos = pos;
    p->has_split = false;
}
bool parser_at_eof(const Parser *p) { return at_eof(p); }
const Token *parser_peek(const Parser *p, uint32_t ahead) { return peek(p, ahead); }

static void check_split(Parser *p) {
    if (p->has_split)
        fatal(tok(p)->span, "fragment ends in the middle of a token");
}

Expr *parser_expr(Parser *p) { Expr *e = parse_expr(p); check_split(p); return e; }
Ty *parser_ty(Parser *p) { Ty *t = parse_ty(p); check_split(p); return t; }
Pat *parser_pat(Parser *p) { Pat *x = parse_pat_top(p); check_split(p); return x; }
Pat *parser_pat_no_top_alt(Parser *p) { Pat *x = parse_pat_no_top_alt(p); check_split(p); return x; }
Block *parser_block(Parser *p) { return parse_block(p); }
Path *parser_path(Parser *p) { Path *x = parse_path(p, true); check_split(p); return x; }
Visibility parser_vis(Parser *p) { return parse_vis(p); }
Expr *parser_literal(Parser *p) { return parse_lit_expr(p); }
bool parser_can_begin_expr(const Parser *p) { return can_begin_expr(p, 0); }
bool parser_can_begin_ty(const Parser *p) { return can_begin_ty(p); }

Item *parser_item(Parser *p) {
    AttrList attrs = {0};
    uint32_t save = p->pos;
    parse_outer_attrs(p, &attrs);
    Item *it = parse_item_opt(p, attrs);
    if (!it)
        p->pos = save;
    return it;
}

Stmt *parser_stmt(Parser *p) {
    Span start = start_span(p);
    AttrList attrs = {0};
    parse_outer_attrs(p, &attrs);
    if (at_kw(p, KW_LET)) {
        /* like parse_let_stmt but without the `;` */
        bump(p);
        Stmt *s = ast_new_stmt(STMT_LET, start);
        s->attrs = attrs;
        s->let.pat = parse_pat_top(p);
        if (eat(p, TK_COLON))
            s->let.ty = parse_ty(p);
        if (eat(p, TK_EQ)) {
            s->let.init = parse_expr(p);
            if (eat_kw(p, KW_ELSE))
                s->let.els = parse_block(p);
        }
        s->span = end_span(p, start);
        return s;
    }
    if (at_item_start(p)) {
        Item *it = parse_item_opt(p, attrs);
        Stmt *s = ast_new_stmt(STMT_ITEM, it->span);
        s->item = it;
        return s;
    }
    Stmt *s = ast_new_stmt(STMT_EXPR, start);
    s->attrs = attrs;
    bool complete;
    s->expr.expr = parse_stmt_like_expr(p, &complete);
    s->span = end_span(p, start);
    return s;
}

Attr *parser_meta(Parser *p) {
    return parse_attr_body(p, false, start_span(p));
}

static void expect_eof(Parser *p) {
    if (!at_eof(p) || p->has_split)
        error_here(p, "unexpected token");
}

Expr *parse_expr_from_tokens(TokenList toks, Span eof_span) {
    Parser p;
    parser_init(&p, toks, eof_span);
    Expr *e = parse_expr(&p);
    expect_eof(&p);
    return e;
}

Ty *parse_ty_from_tokens(TokenList toks, Span eof_span) {
    Parser p;
    parser_init(&p, toks, eof_span);
    Ty *t = parse_ty(&p);
    expect_eof(&p);
    return t;
}

Pat *parse_pat_from_tokens(TokenList toks, Span eof_span) {
    Parser p;
    parser_init(&p, toks, eof_span);
    Pat *x = parse_pat_top(&p);
    expect_eof(&p);
    return x;
}

StmtList parse_stmts_from_tokens(TokenList toks, Span eof_span) {
    Parser p;
    parser_init(&p, toks, eof_span);
    StmtList list = {0};
    Stmt *s;
    while ((s = parse_stmt_opt(&p)) != NULL)
        vec_push(list, s);
    expect_eof(&p);
    return list;
}

static ItemList parse_items_until_eof(Parser *p) {
    ItemList items = {0};
    while (!at_eof(p)) {
        AttrList attrs = {0};
        parse_outer_attrs(p, &attrs);
        if (at(p, TK_SEMI) && attrs.len == 0) {
            bump(p);
            continue;
        }
        Item *it = parse_item_opt(p, attrs);
        if (!it)
            error_here(p, "expected item");
        vec_push(items, it);
    }
    return items;
}

ItemList parse_items_from_tokens(TokenList toks, Span eof_span) {
    Parser p;
    parser_init(&p, toks, eof_span);
    return parse_items_until_eof(&p);
}

ItemList parse_assoc_items_from_tokens(TokenList toks, Span eof_span) {
    return parse_items_from_tokens(toks, eof_span);
}

Crate *parse_crate(TokenList toks, uint32_t file) {
    Parser p;
    Span eof = toks.len ? toks.toks[toks.len - 1].span : (Span){file, 0, 0, 1, 1};
    parser_init(&p, toks, eof);
    Crate *c = NEW(Crate);
    c->id = ast_fresh_id();
    c->file = file;
    c->span = toks.len ? toks.toks[0].span : eof;
    parse_inner_attrs(&p, &c->attrs);
    c->items = parse_items_until_eof(&p);
    c->span = end_span(&p, c->span);
    return c;
}

Crate *parse_file(const char *path) {
    lex_init();
    uint32_t f = source_load(path);
    TokenList tl = lex_file(f);
    return parse_crate(tl, f);
}
