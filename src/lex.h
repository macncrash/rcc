/*
 * lex.h - Rust tokens and the lexer.
 *
 * The lexer turns a whole source file into a flat array of Tokens terminated
 * by a TK_EOF token. Delimiters are guaranteed balanced (the lexer fatals on
 * mismatched ( [ { ). Comments, including doc comments, are dropped.
 *
 * Keywords are NOT separate token kinds: they are TK_IDENT tokens whose `sym`
 * is one of the KW_* values below (and whose TF_RAW flag is clear; `r#fn` is a
 * plain identifier named "fn"). Use tok_is_kw() to test. This keeps macro
 * token trees uniform: `$x:ident` style matching just looks at TK_IDENT.
 */
#ifndef RCC_LEX_H
#define RCC_LEX_H

#include "util.h"

/*
 * Pre-interned symbols. lex_init() interns these first, in this order, so the
 * enum value IS the Sym. KW_STRICT_FIRST..KW_STRICT_LAST are reserved words
 * that can never be identifiers; the rest are contextual ("weak") keywords or
 * just frequently used names, which are ordinary identifiers.
 */
#define RCC_KEYWORDS(X)                                                        \
    /* strict keywords */                                                      \
    X(AS, "as") X(BREAK, "break") X(CONST, "const") X(CONTINUE, "continue")    \
    X(CRATE, "crate") X(ELSE, "else") X(ENUM, "enum") X(EXTERN, "extern")      \
    X(FALSE, "false") X(FN, "fn") X(FOR, "for") X(IF, "if") X(IMPL, "impl")    \
    X(IN, "in") X(LET, "let") X(LOOP, "loop") X(MATCH, "match")                \
    X(MOD, "mod") X(MOVE, "move") X(MUT, "mut") X(PUB, "pub") X(REF, "ref")    \
    X(RETURN, "return") X(SELF_VALUE, "self") X(SELF_TYPE, "Self")             \
    X(STATIC, "static") X(STRUCT, "struct") X(SUPER, "super")                  \
    X(TRAIT, "trait") X(TRUE, "true") X(TYPE, "type") X(UNSAFE, "unsafe")      \
    X(USE, "use") X(WHERE, "where") X(WHILE, "while") X(ASYNC, "async")        \
    X(AWAIT, "await") X(DYN, "dyn") X(ABSTRACT, "abstract")                    \
    X(BECOME, "become") X(BOX, "box") X(DO, "do") X(FINAL, "final")            \
    X(MACRO, "macro") X(OVERRIDE, "override") X(PRIV, "priv")                  \
    X(TYPEOF, "typeof") X(UNSIZED, "unsized") X(VIRTUAL, "virtual")            \
    X(YIELD, "yield") X(TRY, "try") X(UNDERSCORE, "_")                         \
    /* weak keywords and common names (ordinary identifiers) */                \
    X(UNION, "union") X(AUTO, "auto") X(DEFAULT, "default")                    \
    X(MACRO_RULES, "macro_rules") X(RAW, "raw") X(SAFE, "safe")

typedef enum Keyword {
    KW_NONE = 0,
#define X(name, str) KW_##name,
    RCC_KEYWORDS(X)
#undef X
    KW_COUNT_PLUS_ONE
} Keyword;

#define KW_STRICT_FIRST KW_AS
#define KW_STRICT_LAST KW_UNDERSCORE

/*
 * Token kinds. Punctuation is maximally munched by the lexer ("a>>b" gives
 * TK_SHR); the parser splits compound tokens when needed (closing generics,
 * `&&x`, `||` closures, etc.).
 */
#define RCC_TOKENS(X)                                                          \
    X(EOF, "<eof>")                                                            \
    X(IDENT, "identifier")    /* sym; includes keywords and `_` */             \
    X(LIFETIME, "lifetime")   /* sym = name WITHOUT the quote: 'a -> "a" */    \
    X(LITERAL, "literal")     /* lit */                                        \
    X(INTERP, "interpolated") /* nt: expander-inserted parsed fragment */      \
    X(LPAREN, "(") X(RPAREN, ")") X(LBRACKET, "[") X(RBRACKET, "]")            \
    X(LBRACE, "{") X(RBRACE, "}")                                              \
    X(PLUS, "+") X(MINUS, "-") X(STAR, "*") X(SLASH, "/") X(PERCENT, "%")      \
    X(CARET, "^") X(NOT, "!") X(AND, "&") X(OR, "|") X(ANDAND, "&&")           \
    X(OROR, "||") X(SHL, "<<") X(SHR, ">>") X(PLUSEQ, "+=") X(MINUSEQ, "-=")   \
    X(STAREQ, "*=") X(SLASHEQ, "/=") X(PERCENTEQ, "%=") X(CARETEQ, "^=")       \
    X(ANDEQ, "&=") X(OREQ, "|=") X(SHLEQ, "<<=") X(SHREQ, ">>=")               \
    X(EQ, "=") X(EQEQ, "==") X(NE, "!=") X(GT, ">") X(LT, "<") X(GE, ">=")     \
    X(LE, "<=") X(AT, "@") X(DOT, ".") X(DOTDOT, "..") X(DOTDOTDOT, "...")     \
    X(DOTDOTEQ, "..=") X(COMMA, ",") X(SEMI, ";") X(COLON, ":")                \
    X(PATHSEP, "::") X(RARROW, "->") X(FATARROW, "=>") X(POUND, "#")           \
    X(DOLLAR, "$") X(QUESTION, "?") X(TILDE, "~")

typedef enum TokKind {
#define X(name, str) TK_##name,
    RCC_TOKENS(X)
#undef X
    TK_COUNT
} TokKind;

/* Literal kinds. Shared by tokens and AST literal expressions. */
typedef enum LitKind {
    LIT_INT,      /* 123, 0xff_u8: value in int_lo/int_hi */
    LIT_FLOAT,    /* 1.5e3f32, also 1f32: text in float_text */
    LIT_STR,      /* "..." / r#"..."#: decoded UTF-8 bytes */
    LIT_BYTE_STR, /* b"..." / br"...": decoded bytes */
    LIT_C_STR,    /* c"..." / cr"...": decoded bytes, WITHOUT trailing NUL */
    LIT_CHAR,     /* 'x': Unicode scalar in char_val */
    LIT_BYTE,     /* b'x': byte in char_val */
    LIT_BOOL,     /* true/false (only in AST; lexer emits IDENT) */
} LitKind;

typedef struct Lit {
    LitKind kind;
    Sym suffix; /* type suffix (u8, i64, f32, usize...), or 0 if none */
    /* LIT_INT: 128-bit value split into two halves. `overflow` is set if the
       literal does not fit in 128 bits. base is 2, 8, 10 or 16. */
    uint64_t int_lo, int_hi;
    uint8_t base;
    bool overflow;
    bool raw; /* string was a raw string (informational only) */
    bool bool_val;
    /* LIT_FLOAT: digits with '_' removed and no suffix, e.g. "1.5e3". */
    const char *float_text;
    /* LIT_STR/BYTE_STR/C_STR: decoded bytes (NUL-terminated for
       convenience; may also contain interior NULs, use str_len). */
    const char *str;
    uint32_t str_len;
    /* LIT_CHAR / LIT_BYTE */
    uint32_t char_val;
} Lit;

/*
 * Interpolated fragments: the macro expander may splice an already-parsed AST
 * node into a token stream as a single TK_INTERP token. The parser accepts it
 * wherever a node of that kind may appear (an NT_EXPR acts like a
 * parenthesised expression, so `$e.foo()` and `$a * $b` behave as in rustc).
 * `$x:ident`, `$l:lifetime` and `$t:tt` should instead be pasted as the
 * original tokens.
 */
typedef enum NtKind {
    NT_EXPR,  /* node: Expr* */
    NT_TY,    /* node: Ty* */
    NT_PAT,   /* node: Pat* */
    NT_STMT,  /* node: Stmt* */
    NT_ITEM,  /* node: Item* */
    NT_BLOCK, /* node: Block* */
    NT_PATH,  /* node: Path* (usable in type, expr and pattern paths) */
    NT_VIS,   /* node: Visibility* */
    NT_LIT,   /* node: Expr* of kind EXPR_LIT (or negated literal) */
} NtKind;

/* Token flags */
enum {
    TF_WS_BEFORE = 1, /* whitespace or comment precedes this token */
    TF_NL_BEFORE = 2, /* a newline precedes this token */
    TF_RAW = 4,       /* raw identifier r#foo (never a keyword) */
};

typedef struct Token {
    uint8_t kind; /* TokKind */
    uint8_t flags;
    uint8_t nt_kind; /* NtKind, for TK_INTERP */
    Span span;
    union {
        Sym sym;    /* TK_IDENT, TK_LIFETIME */
        Lit *lit;   /* TK_LITERAL */
        void *node; /* TK_INTERP */
    } v;
} Token;

typedef struct TokenList {
    Token *toks;
    uint32_t len;
} TokenList;

/* Intern keywords etc. Must be called once before anything else. */
void lex_init(void);

/* Lex an entire source file (already loaded via source_load). The returned
   list's last token (toks[len-1]) is TK_EOF and is counted in len. */
TokenList lex_file(uint32_t file);

/* True if t is the (non-raw) keyword kw. Works for weak keywords too. */
static inline bool tok_is_kw(const Token *t, Keyword kw) {
    return t->kind == TK_IDENT && t->v.sym == (Sym)kw && !(t->flags & TF_RAW);
}
/* True if t is an identifier usable as a name (not a strict keyword). */
bool tok_is_plain_ident(const Token *t);
/* True if sym is a strict keyword. */
static inline bool sym_is_strict_kw(Sym s) {
    return s >= KW_STRICT_FIRST && s <= KW_STRICT_LAST;
}

/* Token-tree helpers for flat token arrays.
   If toks[i] is an opening delimiter, returns the index of its matching close;
   otherwise returns i. Token trees therefore span [i, tt_end(...)] inclusive. */
uint32_t tt_end(const Token *toks, uint32_t i, uint32_t n);
static inline bool tok_is_open(int k) {
    return k == TK_LPAREN || k == TK_LBRACKET || k == TK_LBRACE;
}
static inline bool tok_is_close(int k) {
    return k == TK_RPAREN || k == TK_RBRACKET || k == TK_RBRACE;
}

const char *tok_kind_str(int kind);
/* Human-readable rendering of one token (arena string), e.g. for stringify!
   and error messages. Literals render in source-like form. */
const char *tok_to_str(const Token *t);
/* Source-like rendering of a literal ("a\n" is re-escaped). */
const char *lit_to_str(const Lit *lit);

/* Write a token dump of `tl` to stdout (for --tokens). */
void dump_tokens(TokenList tl);

#endif
