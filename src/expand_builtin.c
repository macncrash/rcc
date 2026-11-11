/*
 * expand_builtin.c - builtin macros (format family, panic family, assert
 * family, vec!, matches!, thread_local!, cfg!, env!, concat!, ...) and the
 * std::fmt format-string parser.
 *
 * Generated code is built as a token stream (template text lexed once and
 * cached, plus TK_INTERP tokens carrying the user's parsed argument
 * expressions) and handed to the ordinary parser. The caller walks the
 * result, so macros nested in user arguments are expanded afterwards
 * (outside-in). Arguments needed eagerly (format strings, concat!/env!
 * arguments) are expanded on the spot with ex_expand_expr_now().
 *
 * Lowering of the format family (see docs/STATUS.md "Expander"):
 *   ARGS := R::fmt::Arguments { pieces: &[PIECE, ...] }
 *   PIECE := R::fmt::Piece::Lit("text")
 *          | R::fmt::Piece::Arg(R::fmt::Argument::TRAIT(REF), SPEC)
 * where R is ex_fmt_root() ("::std"). If every explicit argument is used
 * exactly once, as a value, in argument order, REF is `&(arg)` inline and
 * the result is CONSUMER(ARGS). Otherwise all arguments are evaluated once
 * up front:
 *   match (&a0, &a1, ..) { (__rcc_a0, __rcc_a1, ..) => CONSUMER(ARGS) }
 * with REF = __rcc_aN and counts `Some(*__rcc_aN)`. CONSUMER is e.g.
 * `R::fmt::format(..)`, `R::io::_print(..)`, `d.write_fmt(..)`; plain
 * format_args! has no consumer.
 */
#include <stdio.h>
#include <stdlib.h>

#include "expand_int.h"

/* ------------------------------------------------------------------ */
/* Argument parsing helpers                                            */
/* ------------------------------------------------------------------ */

static Parser *args_parser(MacCall *m) { return parser_new(m->tokens, m->span); }

static int pk(Parser *p) { return parser_peek(p, 0)->kind; }
static void padv(Parser *p) { parser_set_pos(p, parser_pos(p) + 1); }

static bool eat_comma(Parser *p) {
    if (pk(p) == TK_COMMA) {
        padv(p);
        return true;
    }
    return false;
}

static void expect_comma(Parser *p, const char *mac) {
    if (!eat_comma(p))
        fatal(parser_peek(p, 0)->span, "%s!: expected `,`", mac);
}

static void expect_end(Parser *p, const char *mac) {
    eat_comma(p);
    if (!parser_at_eof(p))
        fatal(parser_peek(p, 0)->span, "%s!: unexpected tokens", mac);
}

static const char *mname(MacCall *m) { return sym_str(vec_last(m->path->segs)->name); }

static Expr *parse_tokens_expr(TokBuf *tb) {
    return parse_expr_from_tokens(tb_list(tb), tb->sp);
}

static MacOut out_expr(Expr *e) {
    MacOut o;
    memset(&o, 0, sizeof o);
    o.kind = MO_EXPR;
    o.expr = e;
    return o;
}

static MacOut out_defer(void) {
    MacOut o;
    memset(&o, 0, sizeof o);
    o.kind = MO_DEFER;
    return o;
}

/* Eagerly expand an expression that must become a literal. Returns NULL
   if an inner macro is unresolved (defer). */
static Expr *eager_lit(Expr *e) {
    if (!ex_expand_expr_now(&e))
        return NULL;
    return e;
}

static bool is_str_lit(const Expr *e) {
    return e && e->kind == EXPR_LIT && e->lit->kind == LIT_STR;
}

/* ------------------------------------------------------------------ */
/* Format strings                                                      */
/* ------------------------------------------------------------------ */

enum {
    TR_DISPLAY, TR_DEBUG, TR_LOWER_HEX, TR_UPPER_HEX, TR_OCTAL, TR_BINARY,
    TR_LOWER_EXP, TR_UPPER_EXP, TR_POINTER, TR_DEBUG_LOWER_HEX, TR_DEBUG_UPPER_HEX,
};
static const char *trait_names[] = {
    "Display", "Debug", "LowerHex", "UpperHex", "Octal", "Binary",
    "LowerExp", "UpperExp", "Pointer", "DebugLowerHex", "DebugUpperHex"};

enum { AL_UNKNOWN, AL_LEFT, AL_CENTER, AL_RIGHT };
static const char *align_names[] = {"Unknown", "Left", "Center", "Right"};

enum { CNT_NONE, CNT_LIT, CNT_ARG };

typedef struct FSpec {
    int arg;
    int trait;
    uint32_t fill;
    int align;
    bool plus, minus, alt, zero;
    int wkind, pkind;
    uint64_t wval, pval;
    int warg, parg;
} FSpec;

typedef struct FPiece {
    bool is_lit;
    CharVec text;
    FSpec spec;
} FPiece;

typedef struct FArg {
    Expr *e;
    Sym name;       /* named or captured */
    bool captured;  /* implicit capture of a variable */
    int value_uses;
    int count_uses;
} FArg;

typedef struct Fmt {
    VEC(FPiece) pieces;
    VEC(FArg) args;
    uint32_t nexplicit;
    VEC(int) value_order; /* arg index of each value use, in piece order */
    Span sp;
} Fmt;

static void lit_append(Fmt *f, const char *s, size_t n) {
    if (!n)
        return;
    if (!f->pieces.len || !vec_last(f->pieces).is_lit) {
        FPiece p;
        memset(&p, 0, sizeof p);
        p.is_lit = true;
        vec_push(f->pieces, p);
    }
    FPiece *p = &f->pieces.data[f->pieces.len - 1];
    for (size_t i = 0; i < n; i++)
        vec_push(p->text, s[i]);
}

static uint32_t utf8_decode(const char *s, uint32_t n, uint32_t *len) {
    unsigned char c = (unsigned char)s[0];
    if (c < 0x80 || n < 2) {
        *len = 1;
        return c;
    }
    if ((c & 0xE0) == 0xC0) {
        *len = 2;
        return ((c & 0x1F) << 6) | (s[1] & 0x3F);
    }
    if ((c & 0xF0) == 0xE0 && n >= 3) {
        *len = 3;
        return ((c & 0x0F) << 12) | ((s[1] & 0x3F) << 6) | (s[2] & 0x3F);
    }
    if (n >= 4) {
        *len = 4;
        return ((c & 0x07) << 18) | ((s[1] & 0x3F) << 12) | ((s[2] & 0x3F) << 6) |
               (s[3] & 0x3F);
    }
    *len = 1;
    return c;
}

static bool is_id_start(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_';
}
static bool is_id_char(char c) { return is_id_start(c) || (c >= '0' && c <= '9'); }
static bool is_digit(char c) { return c >= '0' && c <= '9'; }

static int find_named(Fmt *f, Sym name) {
    for (uint32_t i = 0; i < f->args.len; i++)
        if (f->args.data[i].name == name)
            return (int)i;
    return -1;
}

/* Resolve a named reference: an explicit named argument or a capture. */
static int ref_name(Fmt *f, const char *s, size_t n) {
    Sym name = sym_intern(s, n);
    int i = find_named(f, name);
    if (i >= 0)
        return i;
    FArg a;
    memset(&a, 0, sizeof a);
    a.name = name;
    a.captured = true;
    vec_push(f->args, a);
    return (int)f->args.len - 1;
}

static int ref_index(Fmt *f, uint64_t idx) {
    if (idx >= f->nexplicit)
        fatal(f->sp, "format string refers to argument %llu, but there are %u arguments",
              (unsigned long long)idx, f->nexplicit);
    return (int)idx;
}

/* Parse a count (`5`, `2$`, `name$`) at s[*i]. Returns false if none. */
static bool parse_count(Fmt *f, const char *s, size_t n, size_t *i, int *kind,
                        uint64_t *val, int *arg) {
    size_t j = *i;
    if (j < n && is_digit(s[j])) {
        uint64_t v = 0;
        while (j < n && is_digit(s[j]))
            v = v * 10 + (uint64_t)(s[j++] - '0');
        if (j < n && s[j] == '$') {
            *kind = CNT_ARG;
            *arg = ref_index(f, v);
            *i = j + 1;
            return true;
        }
        *kind = CNT_LIT;
        *val = v;
        *i = j;
        return true;
    }
    if (j < n && is_id_start(s[j])) {
        size_t st = j;
        while (j < n && is_id_char(s[j]))
            j++;
        if (j < n && s[j] == '$') {
            *kind = CNT_ARG;
            *arg = ref_name(f, s + st, j - st);
            *i = j + 1;
            return true;
        }
    }
    return false;
}

static void parse_placeholder(Fmt *f, const char *s, size_t n, uint32_t *next_implicit) {
    FSpec sp;
    memset(&sp, 0, sizeof sp);
    sp.fill = ' ';
    sp.arg = -1;
    size_t i = 0;
    /* argument */
    if (i < n && is_digit(s[i])) {
        uint64_t v = 0;
        while (i < n && is_digit(s[i]))
            v = v * 10 + (uint64_t)(s[i++] - '0');
        sp.arg = ref_index(f, v);
    } else if (i < n && is_id_start(s[i])) {
        size_t st = i;
        while (i < n && is_id_char(s[i]))
            i++;
        sp.arg = ref_name(f, s + st, i - st);
    }
    while (i < n && s[i] == ' ')
        i++;
    bool star = false;
    if (i < n) {
        if (s[i] != ':')
            fatal(f->sp, "invalid format string: expected `:` or `}` in `{%.*s}`", (int)n, s);
        i++;
        /* fill and align */
        uint32_t clen = 0;
        if (i < n) {
            uint32_t c = utf8_decode(s + i, (uint32_t)(n - i), &clen);
            if (i + clen < n && (s[i + clen] == '<' || s[i + clen] == '^' || s[i + clen] == '>')) {
                sp.fill = c;
                i += clen;
            }
        }
        if (i < n && (s[i] == '<' || s[i] == '^' || s[i] == '>')) {
            sp.align = s[i] == '<' ? AL_LEFT : s[i] == '^' ? AL_CENTER : AL_RIGHT;
            i++;
        }
        if (i < n && (s[i] == '+' || s[i] == '-')) {
            sp.plus = s[i] == '+';
            sp.minus = s[i] == '-';
            i++;
        }
        if (i < n && s[i] == '#') {
            sp.alt = true;
            i++;
        }
        if (i < n && s[i] == '0' && !(i + 1 < n && s[i + 1] == '$')) {
            sp.zero = true;
            i++;
        }
        parse_count(f, s, n, &i, &sp.wkind, &sp.wval, &sp.warg);
        if (i < n && s[i] == '.') {
            i++;
            if (i < n && s[i] == '*') {
                star = true;
                i++;
            } else if (!parse_count(f, s, n, &i, &sp.pkind, &sp.pval, &sp.parg)) {
                fatal(f->sp, "invalid format string: expected precision after `.`");
            }
        }
        /* type */
        const char *ty = s + i;
        size_t tl = n - i;
        if (tl == 0) sp.trait = TR_DISPLAY;
        else if (tl == 1 && ty[0] == '?') sp.trait = TR_DEBUG;
        else if (tl == 2 && ty[0] == 'x' && ty[1] == '?') sp.trait = TR_DEBUG_LOWER_HEX;
        else if (tl == 2 && ty[0] == 'X' && ty[1] == '?') sp.trait = TR_DEBUG_UPPER_HEX;
        else if (tl == 1 && ty[0] == 'x') sp.trait = TR_LOWER_HEX;
        else if (tl == 1 && ty[0] == 'X') sp.trait = TR_UPPER_HEX;
        else if (tl == 1 && ty[0] == 'o') sp.trait = TR_OCTAL;
        else if (tl == 1 && ty[0] == 'b') sp.trait = TR_BINARY;
        else if (tl == 1 && ty[0] == 'e') sp.trait = TR_LOWER_EXP;
        else if (tl == 1 && ty[0] == 'E') sp.trait = TR_UPPER_EXP;
        else if (tl == 1 && ty[0] == 'p') sp.trait = TR_POINTER;
        else
            fatal(f->sp, "invalid format string: unknown format trait `%.*s`", (int)tl, ty);
    }
    if (star) {
        sp.pkind = CNT_ARG;
        sp.parg = ref_index(f, (*next_implicit)++);
    }
    if (sp.arg < 0)
        sp.arg = ref_index(f, (*next_implicit)++);
    f->args.data[sp.arg].value_uses++;
    vec_push(f->value_order, sp.arg);
    if (sp.wkind == CNT_ARG)
        f->args.data[sp.warg].count_uses++;
    if (sp.pkind == CNT_ARG)
        f->args.data[sp.parg].count_uses++;
    FPiece p;
    memset(&p, 0, sizeof p);
    p.spec = sp;
    vec_push(f->pieces, p);
}

static void fmt_parse_string(Fmt *f, const char *s, uint32_t n) {
    uint32_t next_implicit = 0;
    uint32_t i = 0;
    while (i < n) {
        char c = s[i];
        if (c == '{') {
            if (i + 1 < n && s[i + 1] == '{') {
                lit_append(f, "{", 1);
                i += 2;
                continue;
            }
            uint32_t j = i + 1;
            while (j < n && s[j] != '}')
                j++;
            if (j >= n)
                fatal(f->sp, "invalid format string: expected `}` but string was terminated");
            parse_placeholder(f, s + i + 1, j - i - 1, &next_implicit);
            i = j + 1;
            continue;
        }
        if (c == '}') {
            if (i + 1 < n && s[i + 1] == '}') {
                lit_append(f, "}", 1);
                i += 2;
                continue;
            }
            fatal(f->sp, "invalid format string: unmatched `}` found");
        }
        uint32_t j = i;
        while (j < n && s[j] != '{' && s[j] != '}')
            j++;
        lit_append(f, s + i, j - i);
        i = j;
    }
    for (uint32_t k = 0; k < f->nexplicit; k++) {
        FArg *a = &f->args.data[k];
        if (!a->value_uses && !a->count_uses)
            fatal(a->e ? a->e->span : f->sp, "format argument never used");
    }
}

/* Parse the remaining macro arguments as format arguments into f (after
   the format string). Named args are `name = expr`. */
static void fmt_parse_args(Fmt *f, Parser *p, const char *mac) {
    while (eat_comma(p)) {
        if (parser_at_eof(p))
            break;
        FArg a;
        memset(&a, 0, sizeof a);
        const Token *t = parser_peek(p, 0);
        if (t->kind == TK_IDENT && parser_peek(p, 1)->kind == TK_EQ) {
            a.name = t->v.sym;
            padv(p);
            padv(p);
        }
        a.e = parser_expr(p);
        vec_push(f->args, a);
    }
    if (!parser_at_eof(p))
        fatal(parser_peek(p, 0)->span, "%s!: expected `,`", mac);
    f->nexplicit = f->args.len;
}

/* Parse `"fmt", args...` from p. *ok=false means defer. */
static Fmt *fmt_from_parser(Parser *p, MacCall *m, bool *ok, Expr **fmt_expr_out) {
    *ok = true;
    Fmt *f = NEW(Fmt);
    f->sp = m->span;
    Expr *fe = parser_expr(p);
    fe = eager_lit(fe);
    if (!fe) {
        *ok = false;
        return NULL;
    }
    if (!is_str_lit(fe))
        fatal(fe->span, "%s!: format argument must be a string literal", mname(m));
    if (fmt_expr_out)
        *fmt_expr_out = fe;
    fmt_parse_args(f, p, mname(m));
    fmt_parse_string(f, fe->lit->str, fe->lit->str_len);
    return f;
}

/* A format with just a literal message. */
static Fmt *fmt_literal(const char *msg, Span sp) {
    Fmt *f = NEW(Fmt);
    f->sp = sp;
    lit_append(f, msg, strlen(msg));
    return f;
}

static void fmt_prepend(Fmt *f, const char *msg) {
    if (f->pieces.len && f->pieces.data[0].is_lit) {
        CharVec nv = {0};
        for (const char *c = msg; *c; c++)
            vec_push(nv, *c);
        for (uint32_t i = 0; i < f->pieces.data[0].text.len; i++)
            vec_push(nv, f->pieces.data[0].text.data[i]);
        f->pieces.data[0].text = nv;
        return;
    }
    FPiece p;
    memset(&p, 0, sizeof p);
    p.is_lit = true;
    for (const char *c = msg; *c; c++)
        vec_push(p.text, *c);
    vec_push(f->pieces, p);
    for (uint32_t i = f->pieces.len - 1; i > 0; i--) {
        FPiece t = f->pieces.data[i];
        f->pieces.data[i] = f->pieces.data[i - 1];
        f->pieces.data[i - 1] = t;
    }
}

static void fmt_newline(Fmt *f) { lit_append(f, "\n", 1); }

/* ---- lowering ---- */

typedef void (*ConsumerFn)(TokBuf *out, TokBuf *args, void *ctx);

typedef struct WrapCtx {
    TokBuf pre, post;
} WrapCtx;

static void wrap_consumer(TokBuf *out, TokBuf *args, void *vctx) {
    WrapCtx *w = vctx;
    tb_toks(out, tb_list(&w->pre));
    tb_toks(out, tb_list(args));
    tb_toks(out, tb_list(&w->post));
}

static void emit_option_count(TokBuf *tb, Fmt *f, int kind, uint64_t val, int arg, bool inline_) {
    if (kind == CNT_NONE) {
        tb_text(tb, "::std::option::Option::None");
        return;
    }
    tb_text(tb, "::std::option::Option::Some(");
    if (kind == CNT_LIT) {
        tb_uint(tb, val, "usize");
    } else if (inline_) {
        /* only captured variables are used as counts in inline form */
        tb_ident(tb, f->args.data[arg].name);
    } else {
        tb_text(tb, "*");
        tb_ident(tb, sym_intern_cstr(arena_printf("__rcc_a%d", arg)));
    }
    tb_text(tb, ")");
}

static void emit_arg_ref(TokBuf *tb, Fmt *f, int arg, bool inline_) {
    FArg *a = &f->args.data[arg];
    if (!inline_) {
        tb_ident(tb, sym_intern_cstr(arena_printf("__rcc_a%d", arg)));
        return;
    }
    tb_text(tb, "&");
    if (a->captured)
        tb_ident(tb, a->name);
    else
        tb_nt(tb, NT_EXPR, a->e, a->e->span);
}

static void emit_arguments(TokBuf *tb, Fmt *f, bool inline_) {
    const char *R = ex_fmt_root();
    tb_fmt(tb, "%s::fmt::Arguments { pieces: &[", R);
    for (uint32_t i = 0; i < f->pieces.len; i++) {
        FPiece *p = &f->pieces.data[i];
        if (i)
            tb_text(tb, ",");
        if (p->is_lit) {
            tb_fmt(tb, "%s::fmt::Piece::Lit(", R);
            tb_str(tb, p->text.data ? p->text.data : "", p->text.len);
            tb_text(tb, ")");
            continue;
        }
        FSpec *s = &p->spec;
        tb_fmt(tb, "%s::fmt::Piece::Arg(%s::fmt::Argument::%s(", R, R, trait_names[s->trait]);
        emit_arg_ref(tb, f, s->arg, inline_);
        tb_fmt(tb, "), %s::fmt::Spec { fill:", R);
        tb_char(tb, s->fill);
        tb_fmt(tb, ", align: %s::fmt::Align::%s, sign_plus: %s, sign_minus: %s, alternate: %s, "
                   "zero_pad: %s, width:",
               R, align_names[s->align], s->plus ? "true" : "false", s->minus ? "true" : "false",
               s->alt ? "true" : "false", s->zero ? "true" : "false");
        emit_option_count(tb, f, s->wkind, s->wval, s->warg, inline_);
        tb_text(tb, ", precision:");
        emit_option_count(tb, f, s->pkind, s->pval, s->parg, inline_);
        tb_text(tb, "})");
    }
    tb_text(tb, "] }");
}

static bool fmt_can_inline(Fmt *f) {
    int last = -1;
    for (uint32_t i = 0; i < f->nexplicit; i++) {
        FArg *a = &f->args.data[i];
        if (a->count_uses || a->value_uses != 1)
            return false;
    }
    for (uint32_t i = 0; i < f->value_order.len; i++) {
        int a = f->value_order.data[i];
        if ((uint32_t)a >= f->nexplicit)
            continue;
        if (a <= last)
            return false;
        last = a;
    }
    return true;
}

/* Lower to tokens: CONSUMER(ARGS), possibly inside the binding match. */
static void fmt_lower_tokens(TokBuf *out, Fmt *f, ConsumerFn cons, void *ctx) {
    bool inline_ = fmt_can_inline(f);
    TokBuf args;
    tb_init(&args, out->sp);
    emit_arguments(&args, f, inline_);
    if (inline_) {
        if (cons)
            cons(out, &args, ctx);
        else
            tb_toks(out, tb_list(&args));
        return;
    }
    tb_text(out, "match (");
    for (uint32_t i = 0; i < f->args.len; i++) {
        FArg *a = &f->args.data[i];
        tb_text(out, "&");
        if (a->captured)
            tb_ident(out, a->name);
        else
            tb_nt(out, NT_EXPR, a->e, a->e->span);
        tb_text(out, ",");
    }
    tb_text(out, ") { (");
    for (uint32_t i = 0; i < f->args.len; i++) {
        tb_ident(out, sym_intern_cstr(arena_printf("__rcc_a%u", i)));
        tb_text(out, ",");
    }
    tb_text(out, ") =>");
    if (cons)
        cons(out, &args, ctx);
    else
        tb_toks(out, tb_list(&args));
    tb_text(out, "}");
}

static Expr *fmt_lower(Fmt *f, Span sp, ConsumerFn cons, void *ctx) {
    TokBuf out;
    tb_init(&out, sp);
    fmt_lower_tokens(&out, f, cons, ctx);
    return parse_tokens_expr(&out);
}

static Expr *fmt_lower_wrap(Fmt *f, Span sp, const char *pre, TokBuf *post_tb, const char *post) {
    WrapCtx w;
    tb_init(&w.pre, sp);
    tb_init(&w.post, sp);
    tb_text(&w.pre, pre);
    if (post_tb)
        tb_toks(&w.post, tb_list(post_tb));
    if (post)
        tb_text(&w.post, post);
    return fmt_lower(f, sp, wrap_consumer, &w);
}

/* `, "file", LINEu32, COLu32` */
static void emit_location(TokBuf *tb, Span call) {
    Span site = ex_site(call);
    const char *file = ex_file_of_span(site);
    tb_text(tb, ",");
    tb_str(tb, file, (uint32_t)strlen(file));
    tb_text(tb, ",");
    tb_uint(tb, site.line, "u32");
    tb_text(tb, ",");
    tb_uint(tb, site.col, "u32");
}

static Expr *panic_expr(Fmt *f, Span sp) {
    TokBuf post;
    tb_init(&post, sp);
    emit_location(&post, sp);
    tb_text(&post, ")");
    return fmt_lower_wrap(f, sp, arena_printf("%s::rt::panic_fmt(", ex_fmt_root()), &post, NULL);
}

/* ------------------------------------------------------------------ */
/* Compat mode: re-emit the call with expanded arguments               */
/* ------------------------------------------------------------------ */

/* Expand `n_lead` leading expressions, then (if present) a format string
   and its arguments, and return a frozen copy of the call. */
static MacOut compat_call(MacCall *m, int n_lead, bool has_fmt) {
    Parser *p = args_parser(m);
    TokBuf tb;
    tb_init(&tb, m->span);
    for (int i = 0; i < n_lead; i++) {
        if (parser_at_eof(p))
            break;
        if (i)
            expect_comma(p, mname(m));
        Expr *e = parser_expr(p);
        if (!ex_expand_expr_now(&e))
            return out_defer();
        if (i)
            tb_text(&tb, ",");
        tb_nt(&tb, NT_EXPR, e, e->span);
    }
    if (has_fmt) {
        if (n_lead && !parser_at_eof(p))
            expect_comma(p, mname(m));
        if (!parser_at_eof(p)) {
            Expr *fe = eager_lit(parser_expr(p));
            if (!fe)
                return out_defer();
            if (tb.v.len)
                tb_text(&tb, ",");
            if (is_str_lit(fe))
                tb_str(&tb, fe->lit->str, fe->lit->str_len);
            else
                tb_nt(&tb, NT_EXPR, fe, fe->span);
            while (eat_comma(p)) {
                if (parser_at_eof(p))
                    break;
                tb_text(&tb, ",");
                const Token *t = parser_peek(p, 0);
                if (t->kind == TK_IDENT && parser_peek(p, 1)->kind == TK_EQ) {
                    tb_ident(&tb, t->v.sym);
                    tb_text(&tb, "=");
                    padv(p);
                    padv(p);
                }
                Expr *e = parser_expr(p);
                if (!ex_expand_expr_now(&e))
                    return out_defer();
                tb_nt(&tb, NT_EXPR, e, e->span);
            }
        }
    }
    expect_end(p, mname(m));
    MacCall *nm = NEW(MacCall);
    *nm = *m;
    nm->id = ast_fresh_id();
    nm->tokens = tb_list(&tb);
    ex_freeze(nm);
    Expr *e = ast_new_expr(EXPR_MAC, m->span);
    e->mac = nm;
    return out_expr(e);
}

/* ------------------------------------------------------------------ */
/* The format family                                                   */
/* ------------------------------------------------------------------ */

static void freeze_format_args(Expr *e) {
    if (!e)
        return;
    switch (e->kind) {
    case EXPR_MAC: ex_freeze(e->mac); break;
    case EXPR_MATCH:
        for (uint32_t i = 0; i < e->match.arms.len; i++)
            freeze_format_args(e->match.arms.data[i]->body);
        break;
    case EXPR_METHOD_CALL:
        for (uint32_t i = 0; i < e->method_call.args.len; i++)
            freeze_format_args(e->method_call.args.data[i]);
        break;
    default: break;
    }
}

typedef struct MockWriteCtx {
    Expr *dest;
    bool failed;
} MockWriteCtx;

/* write! under a mock fmt root: d.write_fmt(::std::format_args!("{}", ARGS)) */
static void mock_write_consumer(TokBuf *out, TokBuf *args, void *vctx) {
    MockWriteCtx *c = vctx;
    Expr *a = parse_expr_from_tokens(tb_list(args), out->sp);
    if (!ex_expand_expr_now(&a))
        c->failed = true;
    tb_nt(out, NT_EXPR, c->dest, c->dest->span);
    tb_text(out, ".write_fmt(::std::format_args!(\"{}\",");
    tb_nt(out, NT_EXPR, a, a->span);
    tb_text(out, "))");
}

static MacOut mac_format_family(MacCall *m, const char *name) {
    bool is_write = strcmp(name, "write") == 0 || strcmp(name, "writeln") == 0;
    bool ln = strcmp(name, "println") == 0 || strcmp(name, "eprintln") == 0 ||
              strcmp(name, "writeln") == 0;
    if (ex_opts()->compat)
        return compat_call(m, is_write ? 1 : 0, true);
    Parser *p = args_parser(m);
    Expr *dest = NULL;
    Fmt *f = NULL;
    if (is_write) {
        dest = parser_expr(p);
        if (!eat_comma(p) || parser_at_eof(p)) {
            if (!ln)
                fatal(m->span, "%s! requires a format string argument", name);
            expect_end(p, name);
            f = fmt_literal("", m->span);
        }
    }
    if (!f) {
        if (parser_at_eof(p)) {
            if (!ln)
                fatal(m->span, "%s! requires at least a format string argument", name);
            f = fmt_literal("", m->span);
        } else {
            bool ok;
            f = fmt_from_parser(p, m, &ok, NULL);
            if (!ok)
                return out_defer();
        }
    }
    if (ln)
        fmt_newline(f);
    const char *R = ex_fmt_root();
    if (strcmp(name, "format_args") == 0)
        return out_expr(fmt_lower(f, m->span, NULL, NULL));
    if (strcmp(name, "format") == 0)
        return out_expr(fmt_lower_wrap(f, m->span, arena_printf("%s::fmt::format(", R), NULL, ")"));
    if (strcmp(name, "print") == 0 || strcmp(name, "println") == 0)
        return out_expr(fmt_lower_wrap(f, m->span, arena_printf("%s::io::_print(", R), NULL, ")"));
    if (strcmp(name, "eprint") == 0 || strcmp(name, "eprintln") == 0)
        return out_expr(fmt_lower_wrap(f, m->span, arena_printf("%s::io::_eprint(", R), NULL, ")"));
    /* write! / writeln! */
    if (strcmp(R, "::std") != 0) {
        MockWriteCtx c = {dest, false};
        Expr *e = fmt_lower(f, m->span, mock_write_consumer, &c);
        if (c.failed)
            return out_defer();
        freeze_format_args(e);
        return out_expr(e);
    }
    WrapCtx w;
    tb_init(&w.pre, m->span);
    tb_init(&w.post, m->span);
    tb_nt(&w.pre, NT_EXPR, dest, dest->span);
    tb_text(&w.pre, ".write_fmt(");
    tb_text(&w.post, ")");
    return out_expr(fmt_lower(f, m->span, wrap_consumer, &w));
}

/* ------------------------------------------------------------------ */
/* panic family and asserts                                            */
/* ------------------------------------------------------------------ */

static MacOut mac_panic_family(MacCall *m, const char *name) {
    if (ex_opts()->compat)
        return compat_call(m, 0, true);
    const char *base = NULL;
    if (strcmp(name, "unreachable") == 0)
        base = "internal error: entered unreachable code";
    else if (strcmp(name, "todo") == 0)
        base = "not yet implemented";
    else if (strcmp(name, "unimplemented") == 0)
        base = "not implemented";
    Parser *p = args_parser(m);
    Fmt *f;
    if (parser_at_eof(p)) {
        f = fmt_literal(base ? base : "explicit panic", m->span);
    } else {
        bool ok;
        f = fmt_from_parser(p, m, &ok, NULL);
        if (!ok)
            return out_defer();
        if (base)
            fmt_prepend(f, arena_printf("%s: ", base));
    }
    return out_expr(panic_expr(f, m->span));
}

static MacOut mac_assert(MacCall *m) {
    if (ex_opts()->compat)
        return compat_call(m, 1, true);
    Parser *p = args_parser(m);
    Expr *cond = parser_expr(p);
    Fmt *f;
    if (eat_comma(p) && !parser_at_eof(p)) {
        bool ok;
        f = fmt_from_parser(p, m, &ok, NULL);
        if (!ok)
            return out_defer();
    } else {
        expect_end(p, "assert");
        const char *txt = ast_expr_to_str(cond);
        f = fmt_literal(arena_printf("assertion failed: %s", txt), m->span);
    }
    TokBuf tb;
    tb_init(&tb, m->span);
    tb_text(&tb, "if !");
    tb_nt(&tb, NT_EXPR, cond, cond->span);
    tb_text(&tb, "{");
    Expr *pe = panic_expr(f, m->span);
    tb_nt(&tb, NT_EXPR, pe, m->span);
    tb_text(&tb, ";}");
    return out_expr(parse_tokens_expr(&tb));
}

static MacOut mac_assert_eq(MacCall *m, bool eq) {
    if (ex_opts()->compat)
        return compat_call(m, 2, true);
    Parser *p = args_parser(m);
    Expr *l = parser_expr(p);
    expect_comma(p, eq ? "assert_eq" : "assert_ne");
    Expr *r = parser_expr(p);
    Fmt *f = NULL;
    if (eat_comma(p) && !parser_at_eof(p)) {
        bool ok;
        f = fmt_from_parser(p, m, &ok, NULL);
        if (!ok)
            return out_defer();
    } else {
        expect_end(p, eq ? "assert_eq" : "assert_ne");
    }
    const char *R = ex_fmt_root();
    const char *op = eq ? "==" : "!=";
    TokBuf tb;
    tb_init(&tb, m->span);
    tb_text(&tb, "match (&");
    tb_nt(&tb, NT_EXPR, l, l->span);
    tb_text(&tb, ", &");
    tb_nt(&tb, NT_EXPR, r, r->span);
    tb_fmt(&tb, ") { (__rcc_l, __rcc_r) => { if %s(*__rcc_l == *__rcc_r) {", eq ? "!" : "");
    TokBuf loc;
    tb_init(&loc, m->span);
    emit_location(&loc, m->span);
    if (!f) {
        tb_fmt(&tb, "%s::rt::assert_failed(\"%s\", __rcc_l, __rcc_r, ::std::option::Option::None",
               R, op);
        tb_toks(&tb, tb_list(&loc));
        tb_text(&tb, ");");
    } else {
        TokBuf post;
        tb_init(&post, m->span);
        tb_text(&post, ")");
        tb_toks(&post, tb_list(&loc));
        tb_text(&post, ")");
        Expr *call = fmt_lower_wrap(
            f, m->span,
            arena_printf("%s::rt::assert_failed(\"%s\", __rcc_l, __rcc_r, ::std::option::Option::Some(", R, op),
            &post, NULL);
        tb_nt(&tb, NT_EXPR, call, m->span);
        tb_text(&tb, ";");
    }
    tb_text(&tb, "} } }");
    return out_expr(parse_tokens_expr(&tb));
}

/* ------------------------------------------------------------------ */
/* vec!, matches!, dbg!                                                */
/* ------------------------------------------------------------------ */

static MacOut mac_vec(MacCall *m) {
    TokBuf tb;
    tb_init(&tb, m->span);
    Parser *p = args_parser(m);
    if (parser_at_eof(p)) {
        tb_text(&tb, "::std::vec::Vec::new()");
        return out_expr(parse_tokens_expr(&tb));
    }
    Expr *first = parser_expr(p);
    if (pk(p) == TK_SEMI) {
        padv(p);
        Expr *n = parser_expr(p);
        expect_end(p, "vec");
        tb_text(&tb, "::std::vec::from_elem(");
        tb_nt(&tb, NT_EXPR, first, first->span);
        tb_text(&tb, ",");
        tb_nt(&tb, NT_EXPR, n, n->span);
        tb_text(&tb, ")");
        return out_expr(parse_tokens_expr(&tb));
    }
    ExprList all = {0}, elems = {0};
    vec_push(all, first);
    while (eat_comma(p)) {
        if (parser_at_eof(p))
            break;
        vec_push(all, parser_expr(p));
    }
    expect_end(p, "vec");
    for (uint32_t i = 0; i < all.len; i++)
        if (ex_attrs_keep(&all.data[i]->attrs))
            vec_push(elems, all.data[i]);
    if (ex_opts()->compat) {
        /* rustc's own lowering, so that element coercions to an expected
           type (Vec<Box<dyn T>>) keep working under real rustc */
        tb_text(&tb, "<[_]>::into_vec(::std::boxed::Box::new([");
        for (uint32_t i = 0; i < elems.len; i++) {
            tb_nt(&tb, NT_EXPR, elems.data[i], elems.data[i]->span);
            tb_text(&tb, ",");
        }
        tb_text(&tb, "]))");
        return out_expr(parse_tokens_expr(&tb));
    }
    /* `::std::vec::from_array([a, b, ..])`: unlike the push sequence of
       the original contract, the array literal is checked against the
       expected element type, so `let v: Vec<Box<dyn T>> = vec![..]`
       coerces each element as rustc does. */
    tb_fmt(&tb, "%s::vec::from_array([", ex_fmt_root());
    for (uint32_t i = 0; i < elems.len; i++) {
        tb_nt(&tb, NT_EXPR, elems.data[i], elems.data[i]->span);
        tb_text(&tb, ",");
    }
    tb_text(&tb, "])");
    return out_expr(parse_tokens_expr(&tb));
}

static MacOut mac_matches(MacCall *m) {
    Parser *p = args_parser(m);
    Expr *e = parser_expr(p);
    expect_comma(p, "matches");
    Pat *pat = parser_pat(p);
    Expr *guard = NULL;
    if (tok_is_kw(parser_peek(p, 0), KW_IF)) {
        padv(p);
        guard = parser_expr(p);
    }
    expect_end(p, "matches");
    TokBuf tb;
    tb_init(&tb, m->span);
    tb_text(&tb, "match");
    tb_nt(&tb, NT_EXPR, e, e->span);
    tb_text(&tb, "{");
    tb_nt(&tb, NT_PAT, pat, pat->span);
    if (guard) {
        tb_text(&tb, "if");
        tb_nt(&tb, NT_EXPR, guard, guard->span);
    }
    tb_text(&tb, "=> true, _ => false }");
    return out_expr(parse_tokens_expr(&tb));
}

static MacOut mac_dbg(MacCall *m) {
    Parser *p = args_parser(m);
    ExprList es = {0};
    while (!parser_at_eof(p)) {
        vec_push(es, parser_expr(p));
        if (!eat_comma(p))
            break;
    }
    expect_end(p, "dbg");
    Span site = ex_site(m->span);
    const char *file = ex_file_of_span(site);
    TokBuf tb;
    tb_init(&tb, m->span);
    if (es.len == 0) {
        tb_text(&tb, "::std::eprintln!(\"[{}:{}:{}]\",");
        tb_str(&tb, file, (uint32_t)strlen(file));
        tb_fmt(&tb, ", %uu32, %uu32)", site.line, site.col);
        return out_expr(parse_tokens_expr(&tb));
    }
    if (es.len > 1)
        tb_text(&tb, "(");
    for (uint32_t i = 0; i < es.len; i++) {
        tb_text(&tb, "match");
        tb_nt(&tb, NT_EXPR, es.data[i], es.data[i]->span);
        tb_text(&tb, "{ __rcc_tmp => { ::std::eprintln!(\"[{}:{}:{}] {} = {:#?}\",");
        tb_str(&tb, file, (uint32_t)strlen(file));
        tb_fmt(&tb, ", %uu32, %uu32,", site.line, site.col);
        const char *s = ast_expr_to_str(es.data[i]);
        tb_str(&tb, s, (uint32_t)strlen(s));
        tb_text(&tb, ", &__rcc_tmp); __rcc_tmp } }");
        if (es.len > 1)
            tb_text(&tb, ",");
    }
    if (es.len > 1)
        tb_text(&tb, ")");
    return out_expr(parse_tokens_expr(&tb));
}

/* ------------------------------------------------------------------ */
/* thread_local!                                                       */
/* ------------------------------------------------------------------ */

static MacOut mac_thread_local(MacCall *m) {
    const Token *t = m->tokens.toks;
    uint32_t n = m->tokens.len, i = 0;
    TokBuf tb;
    tb_init(&tb, m->span);
    bool compat = ex_opts()->compat;
    while (i < n) {
        /* attributes, pasted as tokens */
        uint32_t attr_start = i;
        while (i + 1 < n && t[i].kind == TK_POUND && t[i + 1].kind == TK_LBRACKET)
            i = tt_end(t, i + 1, n) + 1;
        TokenList attrs = {(Token *)t + attr_start, i - attr_start};
        Parser *p = parser_new((TokenList){(Token *)t + i, n - i}, m->span);
        uint32_t vis_start = i;
        (void)parser_vis(p);
        TokenList vis = {(Token *)t + vis_start, parser_pos(p)};
        if (!tok_is_kw(parser_peek(p, 0), KW_STATIC))
            fatal(parser_peek(p, 0)->span, "thread_local!: expected `static`");
        padv(p);
        const Token *nt = parser_peek(p, 0);
        if (nt->kind != TK_IDENT)
            fatal(nt->span, "thread_local!: expected a name");
        Sym name = nt->v.sym;
        padv(p);
        if (pk(p) != TK_COLON)
            fatal(parser_peek(p, 0)->span, "thread_local!: expected `:`");
        padv(p);
        Ty *ty = parser_ty(p);
        if (pk(p) != TK_EQ)
            fatal(parser_peek(p, 0)->span, "thread_local!: expected `=`");
        padv(p);
        bool is_const = false;
        Expr *init;
        if (tok_is_kw(parser_peek(p, 0), KW_CONST) && parser_peek(p, 1)->kind == TK_LBRACE) {
            padv(p);
            is_const = true;
            Block *b = parser_block(p);
            init = ast_new_expr(EXPR_BLOCK, b->span);
            init->block.block = b;
        } else {
            init = parser_expr(p);
        }
        if (pk(p) == TK_SEMI)
            padv(p);
        else if (!parser_at_eof(p))
            fatal(parser_peek(p, 0)->span, "thread_local!: expected `;`");
        i += parser_pos(p);
        if (compat) {
            if (!ex_expand_expr_now(&init))
                return out_defer();
            tb_toks(&tb, attrs);
            tb_toks(&tb, vis);
            tb_text(&tb, "static");
            tb_ident(&tb, name);
            tb_text(&tb, ":");
            tb_nt(&tb, NT_TY, ty, ty->span);
            tb_text(&tb, "=");
            if (is_const)
                tb_text(&tb, "const");
            tb_nt(&tb, NT_EXPR, init, init->span);
            tb_text(&tb, ";");
            continue;
        }
        Sym fn = sym_intern_cstr(arena_printf("__rcc_tls_init_%s", sym_str(name)));
        tb_toks(&tb, attrs);
        tb_toks(&tb, vis);
        tb_text(&tb, "static");
        tb_ident(&tb, name);
        tb_fmt(&tb, ": %s::thread::LocalKey<", ex_fmt_root());
        tb_nt(&tb, NT_TY, ty, ty->span);
        tb_fmt(&tb, "> = %s::thread::LocalKey::new(", ex_fmt_root());
        tb_ident(&tb, fn);
        tb_text(&tb, "); fn");
        tb_ident(&tb, fn);
        tb_text(&tb, "() ->");
        tb_nt(&tb, NT_TY, clone_ty(ty), ty->span);
        tb_text(&tb, "{");
        tb_nt(&tb, NT_EXPR, init, init->span);
        tb_text(&tb, "}");
    }
    MacOut o;
    memset(&o, 0, sizeof o);
    o.kind = MO_ITEMS;
    if (compat) {
        MacCall *nm = NEW(MacCall);
        *nm = *m;
        nm->id = ast_fresh_id();
        nm->delim = DELIM_BRACE;
        nm->tokens = tb_list(&tb);
        ex_freeze(nm);
        Item *it = ast_new_item(ITEM_MAC, m->span);
        it->mac = nm;
        vec_push(o.items, it);
        return o;
    }
    o.items = parse_items_from_tokens(tb_list(&tb), m->span);
    return o;
}

/* ------------------------------------------------------------------ */
/* Compile-time values                                                 */
/* ------------------------------------------------------------------ */

static Expr *mk_bool(bool v, Span sp) {
    Expr *e = ast_new_expr(EXPR_LIT, sp);
    Lit *l = NEW(Lit);
    l->kind = LIT_BOOL;
    l->bool_val = v;
    e->lit = l;
    return e;
}

static Expr *mk_u32(uint32_t v, Span sp) {
    Expr *e = ast_new_expr(EXPR_LIT, sp);
    Lit *l = NEW(Lit);
    l->kind = LIT_INT;
    l->int_lo = v;
    l->base = 10;
    l->suffix = sym_intern_cstr("u32");
    e->lit = l;
    return e;
}

/* One string-literal argument (after eager expansion). NULL = defer. */
static Expr *str_arg(Parser *p, MacCall *m, bool *defer) {
    *defer = false;
    Expr *e = eager_lit(parser_expr(p));
    if (!e) {
        *defer = true;
        return NULL;
    }
    if (!is_str_lit(e))
        fatal(e->span, "%s!: expected a string literal", mname(m));
    return e;
}

static void append_lit_text(CharVec *cv, Expr *e, MacCall *m) {
    const char *s = NULL;
    uint32_t n = 0;
    char buf[64];
    if (e->kind == EXPR_UNARY && e->unary.op == UN_NEG && e->unary.operand->kind == EXPR_LIT) {
        vec_push(*cv, '-');
        e = e->unary.operand;
    }
    if (e->kind != EXPR_LIT)
        fatal(e->span, "%s!: expected a literal", mname(m));
    Lit *l = e->lit;
    switch (l->kind) {
    case LIT_STR: s = l->str; n = l->str_len; break;
    case LIT_CHAR: {
        uint32_t c = l->char_val;
        if (c < 0x80) { buf[0] = (char)c; n = 1; }
        else if (c < 0x800) { buf[0] = (char)(0xC0 | (c >> 6)); buf[1] = (char)(0x80 | (c & 0x3F)); n = 2; }
        else if (c < 0x10000) { buf[0] = (char)(0xE0 | (c >> 12)); buf[1] = (char)(0x80 | ((c >> 6) & 0x3F)); buf[2] = (char)(0x80 | (c & 0x3F)); n = 3; }
        else { buf[0] = (char)(0xF0 | (c >> 18)); buf[1] = (char)(0x80 | ((c >> 12) & 0x3F)); buf[2] = (char)(0x80 | ((c >> 6) & 0x3F)); buf[3] = (char)(0x80 | (c & 0x3F)); n = 4; }
        s = buf;
        break;
    }
    case LIT_INT:
        if (l->int_hi)
            fatal(e->span, "concat!: integer too large");
        snprintf(buf, sizeof buf, "%llu", (unsigned long long)l->int_lo);
        s = buf;
        n = (uint32_t)strlen(buf);
        break;
    case LIT_FLOAT: s = l->float_text; n = (uint32_t)strlen(s); break;
    case LIT_BOOL: s = l->bool_val ? "true" : "false"; n = (uint32_t)strlen(s); break;
    default: fatal(e->span, "%s!: cannot concatenate this literal", mname(m));
    }
    for (uint32_t i = 0; i < n; i++)
        vec_push(*cv, s[i]);
}

static MacOut mac_concat(MacCall *m) {
    Parser *p = args_parser(m);
    CharVec cv = {0};
    while (!parser_at_eof(p)) {
        Expr *e = eager_lit(parser_expr(p));
        if (!e)
            return out_defer();
        append_lit_text(&cv, e, m);
        if (!eat_comma(p))
            break;
    }
    expect_end(p, "concat");
    return out_expr(mk_str_expr(cv.data ? cv.data : "", cv.len, m->span));
}

static char *read_file_rel(MacCall *m, const char *rel, uint32_t *len) {
    const char *dir = ex_dir_of_span(m->span);
    const char *path = rel[0] == '/' ? rel : arena_printf("%s/%s", dir, rel);
    FILE *f = fopen(path, "rb");
    if (!f)
        fatal(m->span, "%s!: couldn't read `%s`", mname(m), path);
    CharVec cv = {0};
    char buf[4096];
    size_t k;
    while ((k = fread(buf, 1, sizeof buf, f)) > 0)
        for (size_t i = 0; i < k; i++)
            vec_push(cv, buf[i]);
    fclose(f);
    *len = cv.len;
    return arena_strndup(cv.data ? cv.data : "", cv.len);
}

const char *tokens_stringify(TokenList tl) { return ast_tokens_to_str(tl); }

/* ------------------------------------------------------------------ */
/* Dispatch                                                            */
/* ------------------------------------------------------------------ */

static const char *builtin_names[] = {
    "format_args", "format", "print", "println", "eprint", "eprintln", "write",
    "writeln", "panic", "unreachable", "todo", "unimplemented", "assert",
    "assert_eq", "assert_ne", "debug_assert", "debug_assert_eq",
    "debug_assert_ne", "vec", "matches", "thread_local", "cfg", "env",
    "option_env", "concat", "stringify", "line", "column", "file",
    "module_path", "include_str", "include_bytes", "compile_error", "dbg",
    NULL};

bool builtin_exists(Sym name) {
    const char *s = sym_str(name);
    for (int i = 0; builtin_names[i]; i++)
        if (strcmp(s, builtin_names[i]) == 0)
            return true;
    return false;
}

MacOut builtin_expand(Sym sym, MacCall *m, MacPos pos) {
    const char *name = sym_str(sym);
    Span sp = m->span;
    (void)pos;
    if (strcmp(name, "format_args") == 0 || strcmp(name, "format") == 0 ||
        strcmp(name, "print") == 0 || strcmp(name, "println") == 0 ||
        strcmp(name, "eprint") == 0 || strcmp(name, "eprintln") == 0 ||
        strcmp(name, "write") == 0 || strcmp(name, "writeln") == 0)
        return mac_format_family(m, name);
    if (strcmp(name, "panic") == 0 || strcmp(name, "unreachable") == 0 ||
        strcmp(name, "todo") == 0 || strcmp(name, "unimplemented") == 0)
        return mac_panic_family(m, name);
    if (strcmp(name, "assert") == 0)
        return mac_assert(m);
    if (strcmp(name, "assert_eq") == 0)
        return mac_assert_eq(m, true);
    if (strcmp(name, "assert_ne") == 0)
        return mac_assert_eq(m, false);
    if (strncmp(name, "debug_assert", 12) == 0) {
        MacOut o;
        memset(&o, 0, sizeof o);
        o.kind = MO_NONE;
        return o;
    }
    if (strcmp(name, "vec") == 0)
        return mac_vec(m);
    if (strcmp(name, "matches") == 0)
        return mac_matches(m);
    if (strcmp(name, "dbg") == 0)
        return mac_dbg(m);
    if (strcmp(name, "thread_local") == 0)
        return mac_thread_local(m);
    if (strcmp(name, "cfg") == 0)
        return out_expr(mk_bool(ex_cfg_eval(m->tokens, sp), sp));
    if (strcmp(name, "env") == 0 || strcmp(name, "option_env") == 0) {
        Parser *p = args_parser(m);
        bool defer;
        Expr *k = str_arg(p, m, &defer);
        if (defer)
            return out_defer();
        if (eat_comma(p) && !parser_at_eof(p)) {
            (void)str_arg(p, m, &defer); /* custom error message */
            if (defer)
                return out_defer();
        }
        expect_end(p, name);
        const char *v = ex_getenv(k->lit->str);
        if (name[0] == 'e') {
            if (!v)
                fatal(sp, "environment variable `%s` not defined at compile time", k->lit->str);
            return out_expr(mk_str_expr(v, (uint32_t)strlen(v), sp));
        }
        TokBuf tb;
        tb_init(&tb, sp);
        if (v) {
            tb_text(&tb, "::std::option::Option::Some(");
            tb_str(&tb, v, (uint32_t)strlen(v));
            tb_text(&tb, ")");
        } else {
            tb_text(&tb, "::std::option::Option::None::<&'static str>");
        }
        return out_expr(parse_tokens_expr(&tb));
    }
    if (strcmp(name, "concat") == 0)
        return mac_concat(m);
    if (strcmp(name, "stringify") == 0) {
        const char *s = tokens_stringify(m->tokens);
        return out_expr(mk_str_expr(s, (uint32_t)strlen(s), sp));
    }
    if (strcmp(name, "line") == 0)
        return out_expr(mk_u32(ex_site(sp).line, sp));
    if (strcmp(name, "column") == 0)
        return out_expr(mk_u32(ex_site(sp).col, sp));
    if (strcmp(name, "file") == 0) {
        const char *f = ex_file_of_span(ex_site(sp));
        return out_expr(mk_str_expr(f, (uint32_t)strlen(f), sp));
    }
    if (strcmp(name, "module_path") == 0) {
        const char *f = ex_module_path();
        return out_expr(mk_str_expr(f, (uint32_t)strlen(f), sp));
    }
    if (strcmp(name, "include_str") == 0 || strcmp(name, "include_bytes") == 0) {
        Parser *p = args_parser(m);
        bool defer;
        Expr *k = str_arg(p, m, &defer);
        if (defer)
            return out_defer();
        expect_end(p, name);
        uint32_t len;
        char *data = read_file_rel(m, k->lit->str, &len);
        Expr *e = mk_str_expr(data, len, sp);
        if (name[8] == 'b')
            e->lit->kind = LIT_BYTE_STR;
        return out_expr(e);
    }
    if (strcmp(name, "compile_error") == 0) {
        Parser *p = args_parser(m);
        bool defer;
        Expr *k = str_arg(p, m, &defer);
        if (defer)
            return out_defer();
        fatal(sp, "%s", k->lit->str);
    }
    fatal(sp, "internal: builtin macro `%s!` not handled", name);
}
