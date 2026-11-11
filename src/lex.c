/* lex.c - the Rust lexer. */
#include "lex.h"

#include <stdio.h>
#include <stdlib.h>

static const char *kw_strings[] = {
    "",
#define X(name, str) str,
    RCC_KEYWORDS(X)
#undef X
};

static const char *tok_strings[] = {
#define X(name, str) str,
    RCC_TOKENS(X)
#undef X
};

void lex_init(void) {
    static bool done;
    if (done)
        return;
    done = true;
    for (uint32_t i = 1; i < sizeof kw_strings / sizeof kw_strings[0]; i++) {
        Sym s = sym_intern_cstr(kw_strings[i]);
        if (s != i) {
            Span none = {0};
            fatal(none, "internal: lex_init must run before any interning");
        }
    }
}

const char *tok_kind_str(int kind) {
    if (kind < 0 || kind >= TK_COUNT)
        return "?";
    return tok_strings[kind];
}

bool tok_is_plain_ident(const Token *t) {
    if (t->kind != TK_IDENT)
        return false;
    if (t->flags & TF_RAW)
        return true;
    return !sym_is_strict_kw(t->v.sym);
}

uint32_t tt_end(const Token *toks, uint32_t i, uint32_t n) {
    if (i >= n || !tok_is_open(toks[i].kind))
        return i;
    int depth = 0;
    for (uint32_t j = i; j < n; j++) {
        if (tok_is_open(toks[j].kind))
            depth++;
        else if (tok_is_close(toks[j].kind)) {
            depth--;
            if (depth == 0)
                return j;
        }
    }
    fatal(toks[i].span, "unbalanced delimiter");
}

/* ------------------------------------------------------------------ */
/* Lexer state                                                         */
/* ------------------------------------------------------------------ */

typedef struct Lexer {
    uint32_t file;
    const char *src;
    uint32_t len;
    uint32_t pos;
    uint32_t line, col; /* of pos */
    VEC(Token) toks;
    uint8_t pending_flags;
    int last_kind; /* kind of the previously emitted token */
    /* delimiter stack */
    VEC(uint32_t) delims; /* indices into toks */
} Lexer;

static inline int peekc(Lexer *L, uint32_t off) {
    uint32_t p = L->pos + off;
    return p < L->len ? (unsigned char)L->src[p] : 0;
}

static void advance(Lexer *L, uint32_t n) {
    while (n-- && L->pos < L->len) {
        if (L->src[L->pos] == '\n') {
            L->line++;
            L->col = 1;
        } else {
            L->col++;
        }
        L->pos++;
    }
}

static Span here(Lexer *L) {
    Span s = {L->file, L->pos, L->pos + 1, L->line, L->col};
    return s;
}

_Noreturn static void lex_error(Lexer *L, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    vfatal(here(L), fmt, ap);
}

static bool is_ident_start(int c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_' ||
           c >= 0x80;
}
static bool is_ident_cont(int c) {
    return is_ident_start(c) || (c >= '0' && c <= '9');
}
static bool is_digit(int c) { return c >= '0' && c <= '9'; }
static int hexval(int c) {
    if (c >= '0' && c <= '9')
        return c - '0';
    if (c >= 'a' && c <= 'f')
        return c - 'a' + 10;
    if (c >= 'A' && c <= 'F')
        return c - 'A' + 10;
    return -1;
}

static Token *push_tok(Lexer *L, int kind, Span sp) {
    Token t;
    memset(&t, 0, sizeof t);
    t.kind = (uint8_t)kind;
    t.flags = L->pending_flags;
    t.span = sp;
    L->pending_flags = 0;
    vec_push(L->toks, t);
    L->last_kind = kind;
    return &vec_last(L->toks);
}

/* Skip whitespace and comments. Returns false at EOF. */
static void skip_trivia(Lexer *L) {
    for (;;) {
        int c = peekc(L, 0);
        if (c == ' ' || c == '\t' || c == '\r' || c == '\f' || c == '\v') {
            L->pending_flags |= TF_WS_BEFORE;
            advance(L, 1);
        } else if (c == '\n') {
            L->pending_flags |= TF_WS_BEFORE | TF_NL_BEFORE;
            advance(L, 1);
        } else if (c == '/' && peekc(L, 1) == '/') {
            L->pending_flags |= TF_WS_BEFORE;
            while (L->pos < L->len && peekc(L, 0) != '\n')
                advance(L, 1);
        } else if (c == '/' && peekc(L, 1) == '*') {
            Span start = here(L);
            L->pending_flags |= TF_WS_BEFORE;
            advance(L, 2);
            int depth = 1;
            while (depth > 0) {
                if (L->pos >= L->len)
                    fatal(start, "unterminated block comment");
                if (peekc(L, 0) == '/' && peekc(L, 1) == '*') {
                    depth++;
                    advance(L, 2);
                } else if (peekc(L, 0) == '*' && peekc(L, 1) == '/') {
                    depth--;
                    advance(L, 2);
                } else {
                    advance(L, 1);
                }
            }
        } else {
            return;
        }
    }
}

/* ------------------------------------------------------------------ */
/* Escapes                                                             */
/* ------------------------------------------------------------------ */

static void put_utf8(CharVec *out, uint32_t c) {
    if (c < 0x80) {
        vec_push(*out, (char)c);
    } else if (c < 0x800) {
        vec_push(*out, (char)(0xC0 | (c >> 6)));
        vec_push(*out, (char)(0x80 | (c & 0x3F)));
    } else if (c < 0x10000) {
        vec_push(*out, (char)(0xE0 | (c >> 12)));
        vec_push(*out, (char)(0x80 | ((c >> 6) & 0x3F)));
        vec_push(*out, (char)(0x80 | (c & 0x3F)));
    } else {
        vec_push(*out, (char)(0xF0 | (c >> 18)));
        vec_push(*out, (char)(0x80 | ((c >> 12) & 0x3F)));
        vec_push(*out, (char)(0x80 | ((c >> 6) & 0x3F)));
        vec_push(*out, (char)(0x80 | (c & 0x3F)));
    }
}

/* Decode one UTF-8 scalar at L->pos, advancing past it. */
static uint32_t read_utf8(Lexer *L) {
    int c = peekc(L, 0);
    uint32_t cp;
    int extra;
    if (c < 0x80) {
        cp = (uint32_t)c;
        extra = 0;
    } else if ((c & 0xE0) == 0xC0) {
        cp = c & 0x1F;
        extra = 1;
    } else if ((c & 0xF0) == 0xE0) {
        cp = c & 0x0F;
        extra = 2;
    } else if ((c & 0xF8) == 0xF0) {
        cp = c & 0x07;
        extra = 3;
    } else {
        lex_error(L, "invalid UTF-8");
    }
    advance(L, 1);
    for (int i = 0; i < extra; i++) {
        int d = peekc(L, 0);
        if ((d & 0xC0) != 0x80)
            lex_error(L, "invalid UTF-8");
        cp = (cp << 6) | (uint32_t)(d & 0x3F);
        advance(L, 1);
    }
    return cp;
}

/*
 * Read an escape sequence after the backslash has been consumed.
 * Returns the scalar value; *is_byte is set for \x escapes in byte mode.
 * Returns UINT32_MAX for a line continuation (string only).
 */
static uint32_t read_escape(Lexer *L, bool byte_mode) {
    int c = peekc(L, 0);
    advance(L, 1);
    switch (c) {
    case 'n': return '\n';
    case 'r': return '\r';
    case 't': return '\t';
    case '\\': return '\\';
    case '0': return 0;
    case '\'': return '\'';
    case '"': return '"';
    case 'x': {
        int h1 = hexval(peekc(L, 0)), h2 = hexval(peekc(L, 1));
        if (h1 < 0 || h2 < 0)
            lex_error(L, "invalid \\x escape");
        advance(L, 2);
        uint32_t v = (uint32_t)(h1 * 16 + h2);
        if (!byte_mode && v > 0x7F)
            lex_error(L, "\\x escape out of range (must be <= 0x7f)");
        return v;
    }
    case 'u': {
        if (peekc(L, 0) != '{')
            lex_error(L, "expected { after \\u");
        advance(L, 1);
        uint32_t v = 0;
        int nd = 0;
        while (peekc(L, 0) != '}') {
            int c2 = peekc(L, 0);
            if (c2 == '_') {
                advance(L, 1);
                continue;
            }
            int h = hexval(c2);
            if (h < 0 || nd >= 6)
                lex_error(L, "invalid \\u escape");
            v = v * 16 + (uint32_t)h;
            nd++;
            advance(L, 1);
        }
        advance(L, 1);
        if (v > 0x10FFFF || (v >= 0xD800 && v <= 0xDFFF))
            lex_error(L, "invalid unicode scalar in \\u escape");
        return v;
    }
    case '\n':
        /* line continuation: skip following whitespace */
        while (peekc(L, 0) == ' ' || peekc(L, 0) == '\t' ||
               peekc(L, 0) == '\n' || peekc(L, 0) == '\r')
            advance(L, 1);
        return UINT32_MAX;
    case '\r':
        if (peekc(L, 0) == '\n') {
            advance(L, 1);
            while (peekc(L, 0) == ' ' || peekc(L, 0) == '\t' ||
                   peekc(L, 0) == '\n' || peekc(L, 0) == '\r')
                advance(L, 1);
            return UINT32_MAX;
        }
        /* fallthrough */
    default:
        lex_error(L, "unknown escape \\%c", c);
    }
}

/* ------------------------------------------------------------------ */
/* Literals                                                            */
/* ------------------------------------------------------------------ */

static Sym read_suffix(Lexer *L) {
    if (!is_ident_start(peekc(L, 0)))
        return 0;
    uint32_t start = L->pos;
    while (is_ident_cont(peekc(L, 0)))
        advance(L, 1);
    return sym_intern(L->src + start, L->pos - start);
}

/* Quoted string body after the opening quote. */
static void lex_quoted(Lexer *L, Lit *lit, bool byte_mode) {
    CharVec out = {0};
    for (;;) {
        if (L->pos >= L->len)
            lex_error(L, "unterminated string literal");
        int c = peekc(L, 0);
        if (c == '"') {
            advance(L, 1);
            break;
        }
        if (c == '\\') {
            advance(L, 1);
            uint32_t v = read_escape(L, byte_mode);
            if (v == UINT32_MAX)
                continue;
            if (byte_mode)
                vec_push(out, (char)v);
            else
                put_utf8(&out, v);
        } else if (c == '\r' && peekc(L, 1) == '\n') {
            advance(L, 1); /* CRLF -> LF */
        } else {
            vec_push(out, (char)c);
            advance(L, 1);
        }
    }
    lit->str_len = out.len;
    vec_push(out, 0);
    lit->str = out.data;
}

/* Raw string after the 'r': #* " ... " #* */
static void lex_raw(Lexer *L, Lit *lit) {
    uint32_t hashes = 0;
    while (peekc(L, 0) == '#') {
        hashes++;
        advance(L, 1);
    }
    if (peekc(L, 0) != '"')
        lex_error(L, "expected \" in raw string");
    advance(L, 1);
    CharVec out = {0};
    for (;;) {
        if (L->pos >= L->len)
            lex_error(L, "unterminated raw string");
        int c = peekc(L, 0);
        if (c == '"') {
            uint32_t k = 0;
            while (k < hashes && peekc(L, 1 + k) == '#')
                k++;
            if (k == hashes) {
                advance(L, 1 + hashes);
                break;
            }
        }
        if (!(c == '\r' && peekc(L, 1) == '\n'))
            vec_push(out, (char)c);
        advance(L, 1);
    }
    lit->raw = true;
    lit->str_len = out.len;
    vec_push(out, 0);
    lit->str = out.data;
}

/* Char or byte literal body after the opening quote. */
static uint32_t lex_char_body(Lexer *L, bool byte_mode) {
    uint32_t v;
    if (peekc(L, 0) == '\\') {
        advance(L, 1);
        v = read_escape(L, byte_mode);
        if (v == UINT32_MAX)
            lex_error(L, "invalid escape in char literal");
    } else if (byte_mode) {
        v = (uint32_t)peekc(L, 0);
        advance(L, 1);
    } else {
        v = read_utf8(L);
    }
    if (peekc(L, 0) != '\'')
        lex_error(L, "unterminated character literal");
    advance(L, 1);
    return v;
}

/* 128-bit accumulate: (hi:lo) = (hi:lo) * base + d. Returns overflow. */
static bool mul_add_128(uint64_t *lo, uint64_t *hi, uint32_t base, uint32_t d) {
    /* split into 32-bit limbs */
    uint64_t limbs[4] = {*lo & 0xFFFFFFFFu, *lo >> 32, *hi & 0xFFFFFFFFu,
                         *hi >> 32};
    uint64_t carry = d;
    for (int i = 0; i < 4; i++) {
        uint64_t v = limbs[i] * base + carry;
        limbs[i] = v & 0xFFFFFFFFu;
        carry = v >> 32;
    }
    *lo = limbs[0] | (limbs[1] << 32);
    *hi = limbs[2] | (limbs[3] << 32);
    return carry != 0;
}

static void lex_number(Lexer *L, Span sp) {
    Lit *lit = NEW(Lit);
    uint32_t base = 10;
    if (peekc(L, 0) == '0' && (peekc(L, 1) == 'x' || peekc(L, 1) == 'o' ||
                               peekc(L, 1) == 'b')) {
        int b = peekc(L, 1);
        base = b == 'x' ? 16 : b == 'o' ? 8 : 2;
        advance(L, 2);
    }
    CharVec text = {0};
    bool is_float = false;
    if (base == 16) {
        while (hexval(peekc(L, 0)) >= 0 || peekc(L, 0) == '_') {
            if (peekc(L, 0) != '_')
                vec_push(text, (char)peekc(L, 0));
            advance(L, 1);
        }
    } else {
        while (is_digit(peekc(L, 0)) || peekc(L, 0) == '_') {
            if (peekc(L, 0) != '_')
                vec_push(text, (char)peekc(L, 0));
            advance(L, 1);
        }
        if (base == 10) {
            /* Fraction: "1.5", or "1." not followed by '.', ident, or digit
               context. After a '.' token (tuple index like x.0.1) we never
               take a fraction. */
            if (L->last_kind != TK_DOT && peekc(L, 0) == '.' &&
                peekc(L, 1) != '.' && !is_ident_start(peekc(L, 1))) {
                is_float = true;
                vec_push(text, '.');
                advance(L, 1);
                if (is_digit(peekc(L, 0))) {
                    while (is_digit(peekc(L, 0)) || peekc(L, 0) == '_') {
                        if (peekc(L, 0) != '_')
                            vec_push(text, (char)peekc(L, 0));
                        advance(L, 1);
                    }
                }
            }
            /* Exponent */
            if (L->last_kind != TK_DOT &&
                (peekc(L, 0) == 'e' || peekc(L, 0) == 'E')) {
                uint32_t k = 1;
                if (peekc(L, k) == '+' || peekc(L, k) == '-')
                    k++;
                while (peekc(L, k) == '_')
                    k++;
                if (is_digit(peekc(L, k))) {
                    is_float = true;
                    vec_push(text, 'e');
                    advance(L, 1);
                    if (peekc(L, 0) == '+' || peekc(L, 0) == '-') {
                        vec_push(text, (char)peekc(L, 0));
                        advance(L, 1);
                    }
                    while (is_digit(peekc(L, 0)) || peekc(L, 0) == '_') {
                        if (peekc(L, 0) != '_')
                            vec_push(text, (char)peekc(L, 0));
                        advance(L, 1);
                    }
                }
            }
        }
    }
    lit->suffix = read_suffix(L);
    const char *sfx = sym_str(lit->suffix);
    if (!is_float && (strcmp(sfx, "f32") == 0 || strcmp(sfx, "f64") == 0)) {
        if (base != 10)
            lex_error(L, "float suffix on non-decimal literal");
        is_float = true;
    }
    if (text.len == 0)
        lex_error(L, "missing digits in number literal");
    vec_push(text, 0);
    if (is_float) {
        lit->kind = LIT_FLOAT;
        lit->float_text = text.data;
    } else {
        lit->kind = LIT_INT;
        lit->base = (uint8_t)base;
        for (const char *p = text.data; *p; p++) {
            int d = hexval(*p);
            if (d < 0 || (uint32_t)d >= base)
                lex_error(L, "invalid digit '%c' in base-%u literal", *p, base);
            if (mul_add_128(&lit->int_lo, &lit->int_hi, base, (uint32_t)d))
                lit->overflow = true;
        }
    }
    sp.hi = L->pos;
    Token *t = push_tok(L, TK_LITERAL, sp);
    t->v.lit = lit;
}

/* ------------------------------------------------------------------ */
/* Main loop                                                           */
/* ------------------------------------------------------------------ */

static void open_delim(Lexer *L, int kind, Span sp) {
    vec_push(L->delims, L->toks.len);
    push_tok(L, kind, sp);
}

static void close_delim(Lexer *L, int kind, Span sp) {
    int want = kind == TK_RPAREN     ? TK_LPAREN
               : kind == TK_RBRACKET ? TK_LBRACKET
                                     : TK_LBRACE;
    if (L->delims.len == 0)
        fatal(sp, "unexpected closing delimiter `%s`", tok_kind_str(kind));
    uint32_t oi = vec_pop(L->delims);
    if (L->toks.data[oi].kind != want)
        fatal(sp, "mismatched closing delimiter `%s` (opened at %u:%u)",
              tok_kind_str(kind), L->toks.data[oi].span.line,
              L->toks.data[oi].span.col);
    push_tok(L, kind, sp);
}

static void lex_string_like(Lexer *L, Span sp, LitKind kind, bool raw) {
    Lit *lit = NEW(Lit);
    lit->kind = kind;
    if (raw)
        lex_raw(L, lit);
    else
        lex_quoted(L, lit, kind == LIT_BYTE_STR);
    lit->suffix = read_suffix(L);
    sp.hi = L->pos;
    Token *t = push_tok(L, TK_LITERAL, sp);
    t->v.lit = lit;
}

TokenList lex_file(uint32_t file) {
    lex_init();
    const SourceFile *sf = source_get(file);
    Lexer L;
    memset(&L, 0, sizeof L);
    L.file = file;
    L.src = sf->text;
    L.len = sf->len;
    L.line = 1;
    L.col = 1;
    L.last_kind = -1;
    /* BOM */
    if (L.len >= 3 && (unsigned char)L.src[0] == 0xEF &&
        (unsigned char)L.src[1] == 0xBB && (unsigned char)L.src[2] == 0xBF)
        L.pos = 3;
    /* Shebang (but not inner attribute #![) */
    if (peekc(&L, 0) == '#' && peekc(&L, 1) == '!' && peekc(&L, 2) != '[') {
        while (L.pos < L.len && peekc(&L, 0) != '\n')
            advance(&L, 1);
    }

    for (;;) {
        skip_trivia(&L);
        Span sp = here(&L);
        if (L.pos >= L.len) {
            sp.hi = sp.lo;
            if (L.delims.len)
                fatal(L.toks.data[vec_last(L.delims)].span,
                      "unclosed delimiter");
            push_tok(&L, TK_EOF, sp);
            break;
        }
        int c = peekc(&L, 0);
        int c1 = peekc(&L, 1);

        /* --- prefixed literals and raw identifiers --- */
        if (c == 'b' && c1 == '"') {
            advance(&L, 2);
            lex_string_like(&L, sp, LIT_BYTE_STR, false);
            continue;
        }
        if (c == 'b' && c1 == 'r' && (peekc(&L, 2) == '"' || peekc(&L, 2) == '#')) {
            advance(&L, 2);
            lex_string_like(&L, sp, LIT_BYTE_STR, true);
            continue;
        }
        if (c == 'b' && c1 == '\'') {
            advance(&L, 2);
            Lit *lit = NEW(Lit);
            lit->kind = LIT_BYTE;
            lit->char_val = lex_char_body(&L, true);
            lit->suffix = read_suffix(&L);
            sp.hi = L.pos;
            push_tok(&L, TK_LITERAL, sp)->v.lit = lit;
            continue;
        }
        if (c == 'c' && c1 == '"') {
            advance(&L, 2);
            lex_string_like(&L, sp, LIT_C_STR, false);
            continue;
        }
        if (c == 'c' && c1 == 'r' && (peekc(&L, 2) == '"' || peekc(&L, 2) == '#')) {
            advance(&L, 2);
            lex_string_like(&L, sp, LIT_C_STR, true);
            continue;
        }
        if (c == 'r' && (c1 == '"' || (c1 == '#' && (peekc(&L, 2) == '"' ||
                                                     peekc(&L, 2) == '#')))) {
            advance(&L, 1);
            lex_string_like(&L, sp, LIT_STR, true);
            continue;
        }
        if (c == 'r' && c1 == '#' && is_ident_start(peekc(&L, 2))) {
            advance(&L, 2);
            uint32_t start = L.pos;
            while (is_ident_cont(peekc(&L, 0)))
                advance(&L, 1);
            sp.hi = L.pos;
            Token *t = push_tok(&L, TK_IDENT, sp);
            t->v.sym = sym_intern(L.src + start, L.pos - start);
            t->flags |= TF_RAW;
            continue;
        }

        /* --- identifiers / keywords --- */
        if (is_ident_start(c)) {
            uint32_t start = L.pos;
            while (is_ident_cont(peekc(&L, 0)))
                advance(&L, 1);
            sp.hi = L.pos;
            Token *t = push_tok(&L, TK_IDENT, sp);
            t->v.sym = sym_intern(L.src + start, L.pos - start);
            continue;
        }

        /* --- numbers --- */
        if (is_digit(c)) {
            lex_number(&L, sp);
            continue;
        }

        /* --- strings --- */
        if (c == '"') {
            advance(&L, 1);
            lex_string_like(&L, sp, LIT_STR, false);
            continue;
        }

        /* --- char literal vs lifetime --- */
        if (c == '\'') {
            /* 'x' or '\n' is a char; 'abc (no closing quote right after
               the first char) is a lifetime/label. */
            bool is_char = false;
            if (c1 == '\\') {
                is_char = true;
            } else {
                /* find length of the first UTF-8 char after quote */
                uint32_t k = 1;
                int b = peekc(&L, 1);
                uint32_t clen = b < 0x80 ? 1 : (b & 0xE0) == 0xC0 ? 2
                                         : (b & 0xF0) == 0xE0 ? 3 : 4;
                k += clen;
                if (peekc(&L, k) == '\'')
                    is_char = true;
            }
            if (is_char) {
                advance(&L, 1);
                Lit *lit = NEW(Lit);
                lit->kind = LIT_CHAR;
                lit->char_val = lex_char_body(&L, false);
                lit->suffix = read_suffix(&L);
                sp.hi = L.pos;
                push_tok(&L, TK_LITERAL, sp)->v.lit = lit;
                continue;
            }
            advance(&L, 1);
            bool raw = false;
            if (peekc(&L, 0) == 'r' && peekc(&L, 1) == '#') {
                raw = true;
                advance(&L, 2);
            }
            if (!is_ident_start(peekc(&L, 0)))
                lex_error(&L, "invalid lifetime or char literal");
            uint32_t start = L.pos;
            while (is_ident_cont(peekc(&L, 0)))
                advance(&L, 1);
            sp.hi = L.pos;
            Token *t = push_tok(&L, TK_LIFETIME, sp);
            t->v.sym = sym_intern(L.src + start, L.pos - start);
            if (raw)
                t->flags |= TF_RAW;
            continue;
        }

        /* --- punctuation --- */
        int kind = -1;
        uint32_t n = 1;
        int c2 = peekc(&L, 2);
        switch (c) {
        case '(': advance(&L, 1); sp.hi = L.pos; open_delim(&L, TK_LPAREN, sp); continue;
        case '[': advance(&L, 1); sp.hi = L.pos; open_delim(&L, TK_LBRACKET, sp); continue;
        case '{': advance(&L, 1); sp.hi = L.pos; open_delim(&L, TK_LBRACE, sp); continue;
        case ')': advance(&L, 1); sp.hi = L.pos; close_delim(&L, TK_RPAREN, sp); continue;
        case ']': advance(&L, 1); sp.hi = L.pos; close_delim(&L, TK_RBRACKET, sp); continue;
        case '}': advance(&L, 1); sp.hi = L.pos; close_delim(&L, TK_RBRACE, sp); continue;
        case '+': kind = c1 == '=' ? (n = 2, TK_PLUSEQ) : TK_PLUS; break;
        case '-':
            kind = c1 == '=' ? (n = 2, TK_MINUSEQ)
                   : c1 == '>' ? (n = 2, TK_RARROW) : TK_MINUS;
            break;
        case '*': kind = c1 == '=' ? (n = 2, TK_STAREQ) : TK_STAR; break;
        case '/': kind = c1 == '=' ? (n = 2, TK_SLASHEQ) : TK_SLASH; break;
        case '%': kind = c1 == '=' ? (n = 2, TK_PERCENTEQ) : TK_PERCENT; break;
        case '^': kind = c1 == '=' ? (n = 2, TK_CARETEQ) : TK_CARET; break;
        case '!': kind = c1 == '=' ? (n = 2, TK_NE) : TK_NOT; break;
        case '&':
            kind = c1 == '&' ? (n = 2, TK_ANDAND)
                   : c1 == '=' ? (n = 2, TK_ANDEQ) : TK_AND;
            break;
        case '|':
            kind = c1 == '|' ? (n = 2, TK_OROR)
                   : c1 == '=' ? (n = 2, TK_OREQ) : TK_OR;
            break;
        case '<':
            if (c1 == '<')
                kind = c2 == '=' ? (n = 3, TK_SHLEQ) : (n = 2, TK_SHL);
            else
                kind = c1 == '=' ? (n = 2, TK_LE) : TK_LT;
            break;
        case '>':
            if (c1 == '>')
                kind = c2 == '=' ? (n = 3, TK_SHREQ) : (n = 2, TK_SHR);
            else
                kind = c1 == '=' ? (n = 2, TK_GE) : TK_GT;
            break;
        case '=':
            kind = c1 == '=' ? (n = 2, TK_EQEQ)
                   : c1 == '>' ? (n = 2, TK_FATARROW) : TK_EQ;
            break;
        case '@': kind = TK_AT; break;
        case '.':
            if (c1 == '.') {
                kind = c2 == '.'   ? (n = 3, TK_DOTDOTDOT)
                       : c2 == '=' ? (n = 3, TK_DOTDOTEQ) : (n = 2, TK_DOTDOT);
            } else {
                kind = TK_DOT;
            }
            break;
        case ',': kind = TK_COMMA; break;
        case ';': kind = TK_SEMI; break;
        case ':': kind = c1 == ':' ? (n = 2, TK_PATHSEP) : TK_COLON; break;
        case '#': kind = TK_POUND; break;
        case '$': kind = TK_DOLLAR; break;
        case '?': kind = TK_QUESTION; break;
        case '~': kind = TK_TILDE; break;
        default:
            lex_error(&L, "unexpected character '%c' (0x%02x)",
                      c >= 32 && c < 127 ? c : '?', c);
        }
        advance(&L, n);
        sp.hi = L.pos;
        push_tok(&L, kind, sp);
    }
    TokenList tl = {L.toks.data, L.toks.len};
    return tl;
}

/* ------------------------------------------------------------------ */
/* Printing                                                            */
/* ------------------------------------------------------------------ */

static void escape_into(CharVec *out, const char *s, uint32_t n, char quote,
                        bool escape_high) {
    for (uint32_t i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        char buf[8];
        if (c == '\\' || c == (unsigned char)quote) {
            vec_push(*out, '\\');
            vec_push(*out, (char)c);
        } else if (c == '\n') {
            vec_push(*out, '\\');
            vec_push(*out, 'n');
        } else if (c == '\t') {
            vec_push(*out, '\\');
            vec_push(*out, 't');
        } else if (c == '\r') {
            vec_push(*out, '\\');
            vec_push(*out, 'r');
        } else if (c < 32 || c == 127 || (escape_high && c >= 0x80)) {
            snprintf(buf, sizeof buf, "\\x%02x", c);
            for (char *p = buf; *p; p++)
                vec_push(*out, *p);
        } else {
            vec_push(*out, (char)c);
        }
    }
}

static void push_str(CharVec *out, const char *s) {
    while (*s)
        vec_push(*out, *s++);
}

const char *lit_to_str(const Lit *lit) {
    CharVec out = {0};
    char buf[64];
    switch (lit->kind) {
    case LIT_INT:
        if (lit->int_hi)
            snprintf(buf, sizeof buf, "0x%llx%016llx",
                     (unsigned long long)lit->int_hi,
                     (unsigned long long)lit->int_lo);
        else
            snprintf(buf, sizeof buf, "%llu", (unsigned long long)lit->int_lo);
        push_str(&out, buf);
        break;
    case LIT_FLOAT:
        push_str(&out, lit->float_text);
        break;
    case LIT_STR:
    case LIT_BYTE_STR:
    case LIT_C_STR:
        if (lit->kind == LIT_BYTE_STR)
            vec_push(out, 'b');
        if (lit->kind == LIT_C_STR)
            vec_push(out, 'c');
        vec_push(out, '"');
        escape_into(&out, lit->str, lit->str_len, '"', lit->kind != LIT_STR);
        vec_push(out, '"');
        break;
    case LIT_CHAR:
    case LIT_BYTE:
        if (lit->kind == LIT_BYTE)
            vec_push(out, 'b');
        vec_push(out, '\'');
        if (lit->char_val < 0x80) {
            char ch = (char)lit->char_val;
            escape_into(&out, &ch, 1, '\'', false);
        } else if (lit->kind == LIT_BYTE) {
            snprintf(buf, sizeof buf, "\\x%02x", lit->char_val);
            push_str(&out, buf);
        } else {
            put_utf8(&out, lit->char_val);
        }
        vec_push(out, '\'');
        break;
    case LIT_BOOL:
        push_str(&out, lit->bool_val ? "true" : "false");
        break;
    }
    if (lit->suffix)
        push_str(&out, sym_str(lit->suffix));
    vec_push(out, 0);
    return out.data;
}

const char *tok_to_str(const Token *t) {
    switch (t->kind) {
    case TK_IDENT:
        if (t->flags & TF_RAW)
            return arena_printf("r#%s", sym_str(t->v.sym));
        return sym_str(t->v.sym);
    case TK_LIFETIME:
        return arena_printf("'%s", sym_str(t->v.sym));
    case TK_LITERAL:
        return lit_to_str(t->v.lit);
    case TK_INTERP:
        return "<interpolated>";
    default:
        return tok_kind_str(t->kind);
    }
}

void dump_tokens(TokenList tl) {
    for (uint32_t i = 0; i < tl.len; i++) {
        const Token *t = &tl.toks[i];
        const char *kind = t->kind == TK_IDENT
                               ? (sym_is_strict_kw(t->v.sym) &&
                                          !(t->flags & TF_RAW)
                                      ? "KEYWORD"
                                      : "IDENT")
                           : t->kind == TK_LIFETIME ? "LIFETIME"
                           : t->kind == TK_LITERAL  ? "LITERAL"
                           : t->kind == TK_EOF      ? "EOF"
                                                    : "PUNCT";
        printf("%u:%u\t%s\t%s%s\n", t->span.line, t->span.col, kind,
               tok_to_str(t), (t->flags & TF_WS_BEFORE) ? "" : "\t(no-space-before)");
    }
}
