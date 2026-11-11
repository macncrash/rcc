/*
 * expand_mbe.c - macro_rules! matcher and transcriber.
 *
 * A definition is compiled once into a list of rules (matcher tree,
 * transcriber tree). Matching is greedy and left to right, like rustc's
 * matcher for unambiguous macros: a repetition takes as many iterations as
 * match (an iteration that fails, including its separator, is undone), and
 * each rule is tried in order. A fragment ($e:expr ...) is only attempted
 * when its first token can begin that fragment; once started, a parse error
 * inside it is fatal, as in rustc.
 *
 * Fragments are bound either as raw tokens (ident, lifetime, tt, literal,
 * vis, meta) or as parsed nodes (expr, ty, pat, pat_param, stmt, item,
 * block, path), which are pasted back as TK_INTERP tokens (cloned on every
 * paste so each copy has fresh NodeIds).
 */
#include <stdio.h>

#include "expand_int.h"

typedef enum FragKind {
    FR_EXPR, FR_IDENT, FR_TY, FR_TT, FR_PAT, FR_PAT_PARAM, FR_LITERAL,
    FR_BLOCK, FR_STMT, FR_ITEM, FR_PATH, FR_LIFETIME, FR_VIS, FR_META,
} FragKind;

static const char *frag_names[] = {
    "expr", "ident", "ty", "tt", "pat", "pat_param", "literal", "block",
    "stmt", "item", "path", "lifetime", "vis", "meta", NULL};

typedef struct MNode MNode;
typedef VEC(MNode *) MSeq;
typedef VEC(Sym) SymVec;

typedef enum MNKind { MN_TOK, MN_GROUP, MN_VAR, MN_REP } MNKind;

struct MNode {
    MNKind k;
    Token tok;   /* MN_TOK; MN_GROUP: the open token */
    Token close; /* MN_GROUP */
    MSeq sub;    /* MN_GROUP, MN_REP */
    Sym name;    /* MN_VAR */
    FragKind frag;
    bool has_frag; /* matcher vars have one */
    bool has_sep;  /* MN_REP */
    Token sep;
    char op;     /* '*', '+', '?' */
    SymVec vars; /* MN_REP: metavariables inside, any depth */
    Span span;
};

typedef struct Rule {
    MSeq pat;
    MSeq body;
    Span span;
} Rule;

struct MacroDef {
    Sym name;
    VEC(Rule) rules;
    Span span;
};

Sym mbe_name(const MacroDef *d) { return d->name; }

/* ------------------------------------------------------------------ */
/* Compiling a definition                                              */
/* ------------------------------------------------------------------ */

static void collect_vars(const MSeq *s, SymVec *out) {
    for (uint32_t i = 0; i < s->len; i++) {
        MNode *n = s->data[i];
        if (n->k == MN_VAR) {
            bool dup = false;
            for (uint32_t j = 0; j < out->len; j++)
                if (out->data[j] == n->name)
                    dup = true;
            if (!dup)
                vec_push(*out, n->name);
        } else if (n->k == MN_GROUP || n->k == MN_REP) {
            collect_vars(&n->sub, out);
        }
    }
}

static bool is_rep_op(const Token *t) {
    return t->kind == TK_STAR || t->kind == TK_PLUS || t->kind == TK_QUESTION;
}

static MSeq parse_mseq(const Token *t, uint32_t i, uint32_t end, bool matcher) {
    MSeq seq = {0};
    while (i < end) {
        const Token *tk = &t[i];
        MNode *n = NEW(MNode);
        n->span = tk->span;
        if (tk->kind == TK_DOLLAR && i + 1 < end) {
            const Token *nx = &t[i + 1];
            if (nx->kind == TK_LPAREN) {
                uint32_t close = tt_end(t, i + 1, end);
                n->k = MN_REP;
                n->sub = parse_mseq(t, i + 2, close, matcher);
                uint32_t j = close + 1;
                if (j < end && is_rep_op(&t[j])) {
                    n->op = t[j].kind == TK_STAR ? '*' : t[j].kind == TK_PLUS ? '+' : '?';
                    j++;
                } else if (j + 1 < end && !tok_is_open(t[j].kind) &&
                           !tok_is_close(t[j].kind) && is_rep_op(&t[j + 1])) {
                    n->has_sep = true;
                    n->sep = t[j];
                    n->op = t[j + 1].kind == TK_STAR ? '*'
                            : t[j + 1].kind == TK_PLUS ? '+' : '?';
                    j += 2;
                } else {
                    fatal(tk->span, "macro_rules: expected repetition operator after `$(...)`");
                }
                collect_vars(&n->sub, &n->vars);
                vec_push(seq, n);
                i = j;
                continue;
            }
            if (nx->kind == TK_IDENT) {
                if (!matcher && tok_is_kw(nx, KW_CRATE)) {
                    n->k = MN_TOK;
                    n->tok = *nx;
                    n->tok.flags = tk->flags;
                    vec_push(seq, n);
                    i += 2;
                    continue;
                }
                n->k = MN_VAR;
                n->name = nx->v.sym;
                i += 2;
                if (matcher) {
                    if (i + 1 < end && t[i].kind == TK_COLON && t[i + 1].kind == TK_IDENT) {
                        const char *fs = sym_str(t[i + 1].v.sym);
                        int f = -1;
                        for (int k = 0; frag_names[k]; k++)
                            if (strcmp(fs, frag_names[k]) == 0)
                                f = k;
                        if (strcmp(fs, "expr_2021") == 0)
                            f = FR_EXPR;
                        if (f < 0)
                            fatal(t[i + 1].span, "macro_rules: unknown fragment specifier `%s`", fs);
                        n->frag = (FragKind)f;
                        n->has_frag = true;
                        i += 2;
                    } else {
                        fatal(nx->span, "macro_rules: missing fragment specifier for `$%s`",
                              sym_str(n->name));
                    }
                }
                vec_push(seq, n);
                continue;
            }
        }
        if (tok_is_open(tk->kind)) {
            uint32_t close = tt_end(t, i, end);
            n->k = MN_GROUP;
            n->tok = *tk;
            n->close = t[close];
            n->sub = parse_mseq(t, i + 1, close, matcher);
            vec_push(seq, n);
            i = close + 1;
            continue;
        }
        n->k = MN_TOK;
        n->tok = *tk;
        vec_push(seq, n);
        i++;
    }
    return seq;
}

MacroDef *mbe_compile(Item *mr) {
    MacroDef *d = NEW(MacroDef);
    d->name = mr->name;
    d->span = mr->span;
    const Token *t = mr->macro_rules.body.toks;
    uint32_t n = mr->macro_rules.body.len;
    uint32_t i = 0;
    while (i < n) {
        if (!tok_is_open(t[i].kind))
            fatal(t[i].span, "macro_rules `%s`: expected `(` to start a rule", sym_str(d->name));
        uint32_t pc = tt_end(t, i, n);
        Rule r;
        memset(&r, 0, sizeof r);
        r.span = t[i].span;
        r.pat = parse_mseq(t, i + 1, pc, true);
        i = pc + 1;
        if (i >= n || t[i].kind != TK_FATARROW)
            fatal(i < n ? t[i].span : mr->span, "macro_rules `%s`: expected `=>`", sym_str(d->name));
        i++;
        if (i >= n || !tok_is_open(t[i].kind))
            fatal(i < n ? t[i].span : mr->span, "macro_rules `%s`: expected transcriber", sym_str(d->name));
        uint32_t bc = tt_end(t, i, n);
        r.body = parse_mseq(t, i + 1, bc, false);
        vec_push(d->rules, r);
        i = bc + 1;
        if (i < n && t[i].kind == TK_SEMI)
            i++;
    }
    return d;
}

/* ------------------------------------------------------------------ */
/* Matching                                                            */
/* ------------------------------------------------------------------ */

typedef struct Match Match;
typedef VEC(Match *) MatchList;
struct Match {
    bool seq;
    MatchList items; /* seq */
    bool is_nt;
    NtKind nt;
    void *node;
    TokenList toks;
};

typedef struct Bind {
    Sym name;
    Match *m;
} Bind;
typedef VEC(Bind) BindVec;

typedef struct MIn {
    const Token *t;
    Span eof;
} MIn;

static bool tok_eq(const Token *a, const Token *b) {
    if (a->kind != b->kind)
        return false;
    switch (a->kind) {
    case TK_IDENT:
        return a->v.sym == b->v.sym;
    case TK_LIFETIME:
        return a->v.sym == b->v.sym;
    case TK_LITERAL:
        return a->v.lit->kind == b->v.lit->kind &&
               strcmp(lit_to_str(a->v.lit), lit_to_str(b->v.lit)) == 0;
    case TK_INTERP:
        return false;
    default:
        return true;
    }
}

static bool can_begin_frag(FragKind f, Parser *p) {
    const Token *t = parser_peek(p, 0);
    if (t->kind == TK_EOF)
        return f == FR_VIS;
    if (t->kind == TK_INTERP) {
        switch (f) {
        case FR_EXPR: return t->nt_kind == NT_EXPR || t->nt_kind == NT_LIT ||
                             t->nt_kind == NT_PATH || t->nt_kind == NT_BLOCK;
        case FR_TY: return t->nt_kind == NT_TY || t->nt_kind == NT_PATH;
        case FR_PAT:
        case FR_PAT_PARAM: return t->nt_kind == NT_PAT || t->nt_kind == NT_EXPR ||
                                  t->nt_kind == NT_LIT || t->nt_kind == NT_PATH;
        case FR_STMT: return t->nt_kind != NT_VIS || true;
        case FR_ITEM: return t->nt_kind == NT_ITEM || t->nt_kind == NT_VIS;
        case FR_BLOCK: return t->nt_kind == NT_BLOCK;
        case FR_PATH: return t->nt_kind == NT_PATH;
        case FR_LITERAL: return t->nt_kind == NT_LIT;
        case FR_VIS: return true;
        case FR_TT: return true;
        default: return false;
        }
    }
    switch (f) {
    case FR_EXPR:
        return parser_can_begin_expr(p) && !tok_is_kw(t, KW_LET);
    case FR_TY:
        return parser_can_begin_ty(p);
    case FR_PAT:
    case FR_PAT_PARAM:
        if (tok_is_close(t->kind))
            return false;
        switch (t->kind) {
        case TK_IDENT: case TK_LITERAL: case TK_LPAREN: case TK_LBRACKET:
        case TK_AND: case TK_ANDAND: case TK_MINUS: case TK_DOTDOT:
        case TK_DOTDOTEQ: case TK_LT: case TK_SHL: case TK_PATHSEP:
            return true;
        case TK_OR:
            return f == FR_PAT;
        default:
            return false;
        }
    case FR_BLOCK:
        return t->kind == TK_LBRACE;
    case FR_STMT:
        return !tok_is_close(t->kind) && t->kind != TK_SEMI;
    case FR_ITEM:
        return true;
    case FR_PATH:
        return t->kind == TK_PATHSEP || t->kind == TK_LT ||
               (t->kind == TK_IDENT && (tok_is_plain_ident(t) ||
                                        tok_is_kw(t, KW_SELF_VALUE) ||
                                        tok_is_kw(t, KW_SELF_TYPE) ||
                                        tok_is_kw(t, KW_SUPER) ||
                                        tok_is_kw(t, KW_CRATE)));
    case FR_LITERAL:
        return t->kind == TK_LITERAL || tok_is_kw(t, KW_TRUE) ||
               tok_is_kw(t, KW_FALSE) ||
               (t->kind == TK_MINUS && parser_peek(p, 1)->kind == TK_LITERAL);
    case FR_META:
        return t->kind == TK_IDENT || t->kind == TK_PATHSEP;
    case FR_VIS:
        return true;
    default:
        return false;
    }
}

/* Match one fragment at in[*pos .. end). */
static bool match_frag(MIn *in, FragKind f, uint32_t *pos, uint32_t end, Match *m) {
    uint32_t start = *pos;
    const Token *t = &in->t[start];
    if (f == FR_TT) {
        if (start >= end || tok_is_close(t->kind))
            return false;
        uint32_t e = tt_end(in->t, start, end);
        m->toks = (TokenList){(Token *)t, e - start + 1};
        *pos = e + 1;
        return true;
    }
    if (f == FR_IDENT) {
        if (start >= end || t->kind != TK_IDENT ||
            (t->v.sym == KW_UNDERSCORE && !(t->flags & TF_RAW)))
            return false;
        m->toks = (TokenList){(Token *)t, 1};
        *pos = start + 1;
        return true;
    }
    if (f == FR_LIFETIME) {
        if (start >= end || t->kind != TK_LIFETIME)
            return false;
        m->toks = (TokenList){(Token *)t, 1};
        *pos = start + 1;
        return true;
    }
    Span eof = start < end ? in->t[end - 1].span : in->eof;
    if (start >= end && f != FR_VIS)
        return false;
    Parser *p = parser_new((TokenList){(Token *)in->t + start, end - start}, eof);
    if (!can_begin_frag(f, p))
        return false;
    switch (f) {
    case FR_EXPR:
        m->is_nt = true; m->nt = NT_EXPR; m->node = parser_expr(p); break;
    case FR_TY:
        m->is_nt = true; m->nt = NT_TY; m->node = parser_ty(p); break;
    case FR_PAT:
        m->is_nt = true; m->nt = NT_PAT; m->node = parser_pat(p); break;
    case FR_PAT_PARAM:
        m->is_nt = true; m->nt = NT_PAT; m->node = parser_pat_no_top_alt(p); break;
    case FR_STMT:
        m->is_nt = true; m->nt = NT_STMT; m->node = parser_stmt(p); break;
    case FR_ITEM:
        m->is_nt = true; m->nt = NT_ITEM; m->node = parser_item(p);
        if (!m->node)
            return false;
        break;
    case FR_BLOCK:
        m->is_nt = true; m->nt = NT_BLOCK; m->node = parser_block(p); break;
    case FR_PATH:
        m->is_nt = true; m->nt = NT_PATH; m->node = parser_path(p); break;
    case FR_LITERAL:
        (void)parser_literal(p);
        break;
    case FR_VIS:
        (void)parser_vis(p);
        break;
    case FR_META:
        (void)parser_meta(p);
        break;
    default:
        return false;
    }
    uint32_t used = parser_pos(p);
    if (!m->is_nt)
        m->toks = (TokenList){(Token *)in->t + start, used};
    *pos = start + used;
    return true;
}

static bool match_seq(MIn *in, const MSeq *pat, uint32_t *pos, uint32_t end, BindVec *bs);

static Match *find_bind(BindVec *bs, Sym name) {
    for (uint32_t i = bs->len; i-- > 0;)
        if (bs->data[i].name == name)
            return bs->data[i].m;
    return NULL;
}

static bool match_node(MIn *in, MNode *n, uint32_t *pos, uint32_t end, BindVec *bs) {
    switch (n->k) {
    case MN_TOK:
        if (*pos >= end || !tok_eq(&n->tok, &in->t[*pos]))
            return false;
        (*pos)++;
        return true;
    case MN_GROUP: {
        if (*pos >= end || in->t[*pos].kind != n->tok.kind)
            return false;
        uint32_t close = tt_end(in->t, *pos, end);
        uint32_t ip = *pos + 1;
        if (!match_seq(in, &n->sub, &ip, close, bs) || ip != close)
            return false;
        *pos = close + 1;
        return true;
    }
    case MN_VAR: {
        Match *m = NEW(Match);
        if (!match_frag(in, n->frag, pos, end, m))
            return false;
        Bind b = {n->name, m};
        vec_push(*bs, b);
        return true;
    }
    case MN_REP: {
        VEC(BindVec) iters = {0};
        uint32_t p = *pos;
        for (;;) {
            if (n->op == '?' && iters.len == 1)
                break;
            uint32_t q = p;
            if (iters.len > 0 && n->has_sep) {
                if (q >= end || !tok_eq(&n->sep, &in->t[q]))
                    break;
                q++;
            }
            BindVec ib = {0};
            uint32_t q0 = q;
            if (!match_seq(in, &n->sub, &q, end, &ib))
                break;
            if (q == q0 && n->sub.len > 0) {
                /* matched nothing: stop to avoid looping forever */
                vec_push(iters, ib);
                p = q;
                break;
            }
            vec_push(iters, ib);
            p = q;
            if (n->sub.len == 0)
                break;
        }
        if (n->op == '+' && iters.len == 0)
            return false;
        for (uint32_t v = 0; v < n->vars.len; v++) {
            Match *m = NEW(Match);
            m->seq = true;
            for (uint32_t k = 0; k < iters.len; k++) {
                Match *im = find_bind(&iters.data[k], n->vars.data[v]);
                if (!im) {
                    im = NEW(Match);
                    im->seq = true; /* bound in an alternative not taken */
                }
                vec_push(m->items, im);
            }
            Bind b = {n->vars.data[v], m};
            vec_push(*bs, b);
        }
        *pos = p;
        return true;
    }
    }
    return false;
}

static bool match_seq(MIn *in, const MSeq *pat, uint32_t *pos, uint32_t end, BindVec *bs) {
    uint32_t save_len = bs->len;
    uint32_t p = *pos;
    for (uint32_t i = 0; i < pat->len; i++) {
        if (!match_node(in, pat->data[i], &p, end, bs)) {
            bs->len = save_len;
            return false;
        }
    }
    *pos = p;
    return true;
}

/* ------------------------------------------------------------------ */
/* Transcription                                                       */
/* ------------------------------------------------------------------ */

typedef struct TCtx {
    BindVec *bs;
    uint32_t idx[64];
    MacroDef *def;
    Span call;
    uint32_t exp_id;
    PtrMap marks; /* orig sym -> marked sym, for this expansion */
    VEC(uint32_t) body_lits; /* output indices of string literals from the body */
} TCtx;

/* ------------------------------------------------------------------ */
/* Hygiene for local variables                                         */
/*                                                                     */
/* macro_rules! is def-site hygienic for local variables: a `let x`     */
/* written in the macro body does not capture or shadow the caller's    */
/* `x`. Identifiers that come from the macro body (not from fragments)  */
/* are first renamed to `__rcc_h<N>_<name>` (N = expansion number). The */
/* output is then parsed once to find which of those names are bound by */
/* a pattern in the expansion; every other renamed identifier gets its  */
/* original name back. Renamed names used as fields/methods/path parts  */
/* (`.x`, `a::x`, `x::`, `x!`) are restored too; struct literal and     */
/* pattern field names are restored by the expansion walk              */
/* (ex_unmark_sym).                                                     */
/* ------------------------------------------------------------------ */

static PtrMap unmark_map; /* marked sym -> orig sym */
static uint32_t exp_counter;

static Sym hyg_mark(TCtx *c, Sym s) {
    Sym m = (Sym)(uintptr_t)ptrmap_get(&c->marks, (void *)(uintptr_t)s);
    if (m)
        return m;
    m = sym_intern_cstr(arena_printf("__rcc_h%u_%s", c->exp_id, sym_str(s)));
    ptrmap_put(&c->marks, (void *)(uintptr_t)s, (void *)(uintptr_t)m);
    ptrmap_put(&unmark_map, (void *)(uintptr_t)m, (void *)(uintptr_t)s);
    return m;
}

Sym ex_unmark_sym(Sym s) {
    Sym o = (Sym)(uintptr_t)ptrmap_get(&unmark_map, (void *)(uintptr_t)s);
    return o ? o : s;
}

static bool is_marked(Sym s) { return ptrmap_get(&unmark_map, (void *)(uintptr_t)s) != NULL; }

typedef struct BoundSet {
    PtrMap set;
} BoundSet;

static void hb_pat(BoundSet *b, Pat *p);
static void hb_expr(BoundSet *b, Expr *e);
static void hb_block(BoundSet *b, Block *bl);
static void hb_item(BoundSet *b, Item *it);

static void hb_bind(BoundSet *b, Sym s) {
    if (!is_marked(s))
        return;
    const char *o = sym_str(ex_unmark_sym(s));
    if ((o[0] >= 'a' && o[0] <= 'z') || (o[0] == '_' && o[1]))
        ptrmap_put(&b->set, (void *)(uintptr_t)s, (void *)1);
}

static void hb_pat(BoundSet *b, Pat *p) {
    if (!p)
        return;
    switch (p->kind) {
    case PAT_IDENT:
        hb_bind(b, p->ident.name);
        hb_pat(b, p->ident.sub);
        break;
    case PAT_TUPLE: case PAT_SLICE: case PAT_OR:
        for (uint32_t i = 0; i < p->elems.len; i++)
            hb_pat(b, p->elems.data[i]);
        break;
    case PAT_TUPLE_STRUCT:
        for (uint32_t i = 0; i < p->tuple_struct.elems.len; i++)
            hb_pat(b, p->tuple_struct.elems.data[i]);
        break;
    case PAT_STRUCT:
        for (uint32_t i = 0; i < p->struct_.fields.len; i++)
            hb_pat(b, p->struct_.fields.data[i]->pat);
        break;
    case PAT_REF: hb_pat(b, p->ref.pat); break;
    default: break;
    }
}

static void hb_exprs(BoundSet *b, ExprList *l) {
    for (uint32_t i = 0; i < l->len; i++)
        hb_expr(b, l->data[i]);
}

static void hb_expr(BoundSet *b, Expr *e) {
    if (!e)
        return;
    switch (e->kind) {
    case EXPR_UNARY: hb_expr(b, e->unary.operand); break;
    case EXPR_ADDR_OF: hb_expr(b, e->addr_of.operand); break;
    case EXPR_BINARY: hb_expr(b, e->binary.lhs); hb_expr(b, e->binary.rhs); break;
    case EXPR_ASSIGN: case EXPR_ASSIGN_OP:
        hb_expr(b, e->assign.lhs); hb_expr(b, e->assign.rhs); break;
    case EXPR_CAST: hb_expr(b, e->cast.expr); break;
    case EXPR_CALL: hb_expr(b, e->call.callee); hb_exprs(b, &e->call.args); break;
    case EXPR_METHOD_CALL:
        hb_expr(b, e->method_call.receiver); hb_exprs(b, &e->method_call.args); break;
    case EXPR_FIELD: hb_expr(b, e->field.base); break;
    case EXPR_INDEX: hb_expr(b, e->index.base); hb_expr(b, e->index.index); break;
    case EXPR_STRUCT:
        for (uint32_t i = 0; i < e->struct_.fields.len; i++)
            hb_expr(b, e->struct_.fields.data[i]->expr);
        hb_expr(b, e->struct_.base);
        break;
    case EXPR_TUPLE: case EXPR_ARRAY: hb_exprs(b, &e->elems); break;
    case EXPR_REPEAT: hb_expr(b, e->repeat.elem); hb_expr(b, e->repeat.count); break;
    case EXPR_RANGE: hb_expr(b, e->range.lo); hb_expr(b, e->range.hi); break;
    case EXPR_CLOSURE:
        for (uint32_t i = 0; i < e->closure.params.len; i++)
            hb_pat(b, e->closure.params.data[i]->pat);
        hb_expr(b, e->closure.body);
        break;
    case EXPR_BLOCK: hb_block(b, e->block.block); break;
    case EXPR_IF: hb_expr(b, e->if_.cond); hb_block(b, e->if_.then); hb_expr(b, e->if_.els); break;
    case EXPR_LET: hb_pat(b, e->let.pat); hb_expr(b, e->let.scrutinee); break;
    case EXPR_MATCH:
        hb_expr(b, e->match.scrutinee);
        for (uint32_t i = 0; i < e->match.arms.len; i++) {
            Arm *a = e->match.arms.data[i];
            hb_pat(b, a->pat);
            hb_expr(b, a->guard);
            hb_expr(b, a->body);
        }
        break;
    case EXPR_WHILE: hb_expr(b, e->while_.cond); hb_block(b, e->while_.body); break;
    case EXPR_FOR:
        hb_pat(b, e->for_.pat); hb_expr(b, e->for_.iter); hb_block(b, e->for_.body); break;
    case EXPR_LOOP: hb_block(b, e->loop.body); break;
    case EXPR_BREAK: hb_expr(b, e->break_.value); break;
    case EXPR_RETURN: hb_expr(b, e->ret); break;
    case EXPR_TRY: hb_expr(b, e->try_); break;
    default: break;
    }
}

static void hb_stmt(BoundSet *b, Stmt *s) {
    switch (s->kind) {
    case STMT_LET:
        hb_pat(b, s->let.pat);
        hb_expr(b, s->let.init);
        if (s->let.els)
            hb_block(b, s->let.els);
        break;
    case STMT_EXPR: hb_expr(b, s->expr.expr); break;
    case STMT_ITEM: hb_item(b, s->item); break;
    default: break;
    }
}

static void hb_block(BoundSet *b, Block *bl) {
    if (!bl)
        return;
    for (uint32_t i = 0; i < bl->stmts.len; i++)
        hb_stmt(b, bl->stmts.data[i]);
}

static void hb_item(BoundSet *b, Item *it) {
    switch (it->kind) {
    case ITEM_FN:
        for (uint32_t i = 0; i < it->fn_.sig.params.len; i++)
            hb_pat(b, it->fn_.sig.params.data[i]->pat);
        hb_block(b, it->fn_.body);
        break;
    case ITEM_IMPL:
        for (uint32_t i = 0; i < it->impl.items.len; i++)
            hb_item(b, it->impl.items.data[i]);
        break;
    case ITEM_CONST: hb_expr(b, it->const_.expr); break;
    case ITEM_STATIC: hb_expr(b, it->static_.expr); break;
    default: break;
    }
}

static bool is_fmt_macro_name(Sym s) {
    static const char *names[] = {"format", "format_args", "print", "println", "eprint",
                                  "eprintln", "panic", "unreachable", "todo",
                                  "unimplemented", NULL};
    const char *n = sym_str(ex_unmark_sym(s));
    for (int i = 0; names[i]; i++)
        if (strcmp(n, names[i]) == 0)
            return true;
    return false;
}

/* Rewrite implicit captures `{name}` / `{name:..}` in a format string from
   the macro body whose `name` is a hygienic local of this expansion. */
static Lit *rename_captures(Lit *l, BoundSet *b) {
    CharVec out = {0};
    bool changed = false;
    const char *s = l->str;
    uint32_t n = l->str_len;
    for (uint32_t i = 0; i < n;) {
        if (s[i] == '{' && i + 1 < n && s[i + 1] == '{') {
            vec_push(out, '{');
            vec_push(out, '{');
            i += 2;
            continue;
        }
        if (s[i] == '{') {
            uint32_t j = i + 1;
            while (j < n && (s[j] == '_' || (s[j] >= 'a' && s[j] <= 'z') ||
                             (s[j] >= 'A' && s[j] <= 'Z') || (s[j] >= '0' && s[j] <= '9')))
                j++;
            if (j > i + 1 && j < n && (s[j] == '}' || s[j] == ':') &&
                !(s[i + 1] >= '0' && s[i + 1] <= '9')) {
                Sym orig = sym_intern(s + i + 1, j - i - 1);
                Sym m = 0;
                for (uint32_t k = 0; k < b->set.cap; k++)
                    if (b->set.keys[k] && ex_unmark_sym((Sym)(uintptr_t)b->set.keys[k]) == orig)
                        m = (Sym)(uintptr_t)b->set.keys[k];
                if (m) {
                    vec_push(out, '{');
                    for (const char *c = sym_str(m); *c; c++)
                        vec_push(out, *c);
                    changed = true;
                    i = j;
                    continue;
                }
            }
        }
        vec_push(out, s[i]);
        i++;
    }
    if (!changed)
        return l;
    Lit *nl = NEW(Lit);
    *nl = *l;
    nl->str = arena_strndup(out.data, out.len);
    nl->str_len = out.len;
    return nl;
}

static TokenList hygiene(TokenList toks, MacPos pos, Span sp, TCtx *c) {
    bool any = false;
    for (uint32_t i = 0; i < toks.len && !any; i++)
        if (toks.toks[i].kind == TK_IDENT && is_marked(toks.toks[i].v.sym))
            any = true;
    if (!any)
        return toks;
    BoundSet b;
    memset(&b, 0, sizeof b);
    TokenList copy = clone_tokens(toks);
    switch (pos) {
    case POS_EXPR: {
        TokenList t = copy;
        if (t.len && t.toks[t.len - 1].kind == TK_SEMI)
            t.len--;
        hb_expr(&b, parse_expr_from_tokens(t, sp));
        break;
    }
    case POS_STMT: {
        StmtList sl = parse_stmts_from_tokens(copy, sp);
        for (uint32_t i = 0; i < sl.len; i++)
            hb_stmt(&b, sl.data[i]);
        break;
    }
    case POS_ITEM:
    case POS_ASSOC:
    case POS_FOREIGN: {
        ItemList il = parse_items_from_tokens(copy, sp);
        for (uint32_t i = 0; i < il.len; i++)
            hb_item(&b, il.data[i]);
        break;
    }
    case POS_PAT:
    case POS_TY:
        break;
    }
    for (uint32_t i = 0; i < toks.len; i++) {
        Token *t = &toks.toks[i];
        if (t->kind != TK_IDENT || !is_marked(t->v.sym))
            continue;
        bool keep = ptrmap_get(&b.set, (void *)(uintptr_t)t->v.sym) != NULL;
        if (keep) {
            int pk = i > 0 ? toks.toks[i - 1].kind : -1;
            int nk = i + 1 < toks.len ? toks.toks[i + 1].kind : -1;
            if (pk == TK_DOT || pk == TK_PATHSEP || nk == TK_PATHSEP || nk == TK_NOT)
                keep = false;
            /* the name an item declares (`fn min`) is never a local, even if
               some other fn of the same expansion binds a local `min` */
            if (i > 0 && (tok_is_kw(&toks.toks[i - 1], KW_FN) || tok_is_kw(&toks.toks[i - 1], KW_STRUCT) ||
                          tok_is_kw(&toks.toks[i - 1], KW_ENUM) || tok_is_kw(&toks.toks[i - 1], KW_TRAIT) ||
                          tok_is_kw(&toks.toks[i - 1], KW_TYPE) || tok_is_kw(&toks.toks[i - 1], KW_MOD)))
                keep = false;
        }
        if (!keep)
            t->v.sym = ex_unmark_sym(t->v.sym);
    }
    /* format strings written in the body: `name!(".." ` with name unmarked now */
    for (uint32_t k = 0; k < c->body_lits.len; k++) {
        uint32_t i = c->body_lits.data[k];
        if (i < 3 || toks.toks[i - 1].kind != TK_LPAREN || toks.toks[i - 2].kind != TK_NOT ||
            toks.toks[i - 3].kind != TK_IDENT || !is_fmt_macro_name(toks.toks[i - 3].v.sym))
            continue;
        toks.toks[i].v.lit = rename_captures(toks.toks[i].v.lit, &b);
    }
    return toks;
}

/* The binding of `name` at the current repetition position; NULL if the
   name is not a metavariable of this rule. */
static Match *lookup(TCtx *c, Sym name, int depth) {
    Match *m = find_bind(c->bs, name);
    if (!m)
        return NULL;
    for (int d = 0; d < depth && m->seq; d++) {
        if (c->idx[d] >= m->items.len)
            fatal(c->call, "macro `%s`: metavariable `$%s` repeats a different number of times",
                  sym_str(c->def->name), sym_str(name));
        m = m->items.data[c->idx[d]];
    }
    return m;
}

static void transcribe(TCtx *c, const MSeq *s, int depth, TokBuf *out) {
    for (uint32_t i = 0; i < s->len; i++) {
        MNode *n = s->data[i];
        switch (n->k) {
        case MN_TOK: {
            Token t = n->tok;
            if (t.kind == TK_IDENT && t.v.sym >= (Sym)KW_COUNT_PLUS_ONE)
                t.v.sym = hyg_mark(c, t.v.sym);
            if (t.kind == TK_LITERAL && t.v.lit->kind == LIT_STR)
                vec_push(c->body_lits, out->v.len);
            tb_tok(out, t);
            break;
        }
        case MN_GROUP:
            tb_tok(out, n->tok);
            transcribe(c, &n->sub, depth, out);
            tb_tok(out, n->close);
            break;
        case MN_VAR: {
            Match *m = lookup(c, n->name, depth);
            if (!m) {
                /* not a metavariable: emit `$name` literally (macro-defining
                   macros) */
                Token d = n->tok;
                memset(&d, 0, sizeof d);
                d.kind = TK_DOLLAR;
                d.span = n->span;
                tb_tok(out, d);
                Token id;
                memset(&id, 0, sizeof id);
                id.kind = TK_IDENT;
                id.v.sym = n->name;
                id.span = n->span;
                tb_tok(out, id);
                break;
            }
            if (m->seq)
                fatal(n->span, "macro `%s`: variable `$%s` is still repeating at this depth",
                      sym_str(c->def->name), sym_str(n->name));
            if (m->is_nt)
                tb_nt(out, m->nt, clone_nt(m->nt, m->node), n->span);
            else
                tb_toks(out, clone_tokens(m->toks));
            break;
        }
        case MN_REP: {
            int64_t count = -1;
            for (uint32_t v = 0; v < n->vars.len; v++) {
                Match *m = lookup(c, n->vars.data[v], depth);
                if (!m || !m->seq)
                    continue;
                if (count >= 0 && (int64_t)m->items.len != count)
                    fatal(c->call, "macro `%s`: meta-variable repeats %d times, but `$%s` repeats %u times",
                          sym_str(c->def->name), (int)count, sym_str(n->vars.data[v]), m->items.len);
                count = m->items.len;
            }
            if (count < 0) {
                /* no repeating variable: emit literally (inner macro defs) */
                bool any_bound = false;
                for (uint32_t v = 0; v < n->vars.len; v++)
                    if (find_bind(c->bs, n->vars.data[v]))
                        any_bound = true;
                if (any_bound)
                    fatal(n->span, "macro `%s`: repetition contains no variable repeating at this depth",
                          sym_str(c->def->name));
                Token t;
                memset(&t, 0, sizeof t);
                t.span = n->span;
                t.kind = TK_DOLLAR;
                tb_tok(out, t);
                t.kind = TK_LPAREN;
                tb_tok(out, t);
                transcribe(c, &n->sub, depth, out);
                t.kind = TK_RPAREN;
                tb_tok(out, t);
                if (n->has_sep)
                    tb_tok(out, n->sep);
                t.kind = n->op == '*' ? TK_STAR : n->op == '+' ? TK_PLUS : TK_QUESTION;
                tb_tok(out, t);
                break;
            }
            if (depth >= 63)
                fatal(n->span, "macro repetition nested too deeply");
            for (int64_t k = 0; k < count; k++) {
                if (k > 0 && n->has_sep)
                    tb_tok(out, n->sep);
                c->idx[depth] = (uint32_t)k;
                transcribe(c, &n->sub, depth + 1, out);
            }
            break;
        }
        }
    }
}

TokenList mbe_expand(MacroDef *d, MacCall *mac, MacPos mpos) {
    MIn in = {mac->tokens.toks, mac->span};
    for (uint32_t r = 0; r < d->rules.len; r++) {
        Rule *rule = &d->rules.data[r];
        BindVec bs = {0};
        uint32_t pos = 0;
        if (!match_seq(&in, &rule->pat, &pos, mac->tokens.len, &bs) ||
            pos != mac->tokens.len)
            continue;
        TokBuf out;
        tb_init(&out, mac->span);
        TCtx c;
        memset(&c, 0, sizeof c);
        c.bs = &bs;
        c.def = d;
        c.call = mac->span;
        c.exp_id = ++exp_counter;
        transcribe(&c, &rule->body, 0, &out);
        return hygiene(tb_list(&out), mpos, mac->span, &c);
    }
    fatal(mac->span, "no rules of macro `%s` matched this invocation", sym_str(d->name));
}
