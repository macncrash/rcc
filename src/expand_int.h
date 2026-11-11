/*
 * expand_int.h - internals shared by expand*.c (not a public interface).
 */
#ifndef RCC_EXPAND_INT_H
#define RCC_EXPAND_INT_H

#include "expand.h"
#include "parse.h"

/* ------------------------------------------------------------------ */
/* Pointer -> pointer hash map (arena-backed, never shrinks)           */
/* ------------------------------------------------------------------ */

typedef struct PtrMap {
    const void **keys;
    void **vals;
    uint32_t cap, len;
} PtrMap;

void *ptrmap_get(const PtrMap *m, const void *key);
void ptrmap_put(PtrMap *m, const void *key, void *val);

/* ------------------------------------------------------------------ */
/* Token building (expand.c)                                           */
/* ------------------------------------------------------------------ */

typedef VEC(Token) TokVec;

typedef struct TokBuf {
    TokVec v;
    Span sp; /* span given to tokens created from text */
} TokBuf;

void tb_init(TokBuf *tb, Span sp);
TokenList tb_list(const TokBuf *tb);
void tb_tok(TokBuf *tb, Token t);
void tb_toks(TokBuf *tb, TokenList tl);
/* Lex `text` (cached) and append its tokens, each with span tb->sp. */
void tb_text(TokBuf *tb, const char *text);
/* printf-style tb_text. */
void tb_fmt(TokBuf *tb, const char *fmt, ...);
void tb_ident(TokBuf *tb, Sym s); /* raw ident if s is a keyword-ish name? no: plain */
void tb_str(TokBuf *tb, const char *s, uint32_t len); /* string literal */
void tb_char(TokBuf *tb, uint32_t c);                 /* char literal */
void tb_uint(TokBuf *tb, uint64_t v, const char *suffix);
void tb_bool(TokBuf *tb, bool v);
void tb_punct(TokBuf *tb, int kind);
/* Interpolated node; the node is used as is (callers clone if needed). */
void tb_nt(TokBuf *tb, NtKind k, void *node, Span sp);

Lit *mk_str_lit(const char *s, uint32_t len);
Expr *mk_str_expr(const char *s, uint32_t len, Span sp);

/* ------------------------------------------------------------------ */
/* Deep copies with fresh NodeIds (expand_clone.c)                     */
/* ------------------------------------------------------------------ */

Expr *clone_expr(const Expr *e);
Ty *clone_ty(const Ty *t);
Pat *clone_pat(const Pat *p);
Stmt *clone_stmt(const Stmt *s);
Item *clone_item(const Item *it);
Block *clone_block(const Block *b);
Path *clone_path(const Path *p);
Visibility clone_vis(Visibility v);
Generics clone_generics(const Generics *g);
Attr *clone_attr(const Attr *a);
TokenList clone_tokens(TokenList tl);
/* Clone the node of an interpolated token of kind k. */
void *clone_nt(NtKind k, const void *node);

/* ------------------------------------------------------------------ */
/* Driver services (expand.c)                                          */
/* ------------------------------------------------------------------ */

typedef enum MacPos {
    POS_EXPR,
    POS_STMT,
    POS_ITEM,
    POS_ASSOC,
    POS_FOREIGN,
    POS_PAT,
    POS_TY,
} MacPos;

typedef enum MacOutKind {
    MO_DEFER,  /* could not expand yet (an inner macro is unresolved) */
    MO_TOKENS, /* re-parse according to the position */
    MO_EXPR,
    MO_STMTS,
    MO_ITEMS,
    MO_NONE, /* expands to nothing (debug_assert!) */
} MacOutKind;

typedef struct MacOut {
    MacOutKind kind;
    TokenList toks;
    Expr *expr;
    StmtList stmts;
    ItemList items;
} MacOut;

typedef struct ExpandState ExpandState;
extern ExpandState *EX;

const ExpandOptions *ex_opts(void);
/* "::std" or the --fmt-root override. */
const char *ex_fmt_root(void);
/* Span to report for builtins: the outermost macro call site. */
Span ex_site(Span inner);
/* Expand macros in *e right now (eagerly). False if something in it had to
   be deferred (unresolved macro). */
bool ex_expand_expr_now(Expr **e);
/* Mark a macro call as final: it is printed as is and never expanded
   (compat mode). */
void ex_freeze(MacCall *m);
bool ex_is_frozen(const MacCall *m);
/* #[cfg]/#[cfg_attr] processing of an attribute list; false = removed. */
bool ex_attrs_keep(AttrList *l);
/* cfg predicate evaluation over the tokens inside `cfg( ... )`. */
bool ex_cfg_eval(TokenList tl, Span sp);
/* env!/option_env! lookup; NULL if unset. */
const char *ex_getenv(const char *name);
/* module_path!() */
const char *ex_module_path(void);
/* Directory of the source file of span sp (for include_str!). */
const char *ex_dir_of_span(Span sp);
/* Path of a source file as written (for file!()). */
const char *ex_file_of_span(Span sp);
/* Allocate a fresh `__rcc_*` name with the given stem. */
Sym ex_fresh_name(const char *stem);

/* ------------------------------------------------------------------ */
/* macro_rules! (expand_mbe.c)                                         */
/* ------------------------------------------------------------------ */

typedef struct MacroDef MacroDef;
MacroDef *mbe_compile(Item *mr);
Sym mbe_name(const MacroDef *d);
/* Expand `mac` with `d`; fatal if no rule matches. */
TokenList mbe_expand(MacroDef *d, MacCall *mac, MacPos pos);
/* Original name of a hygienically renamed identifier (else s). */
Sym ex_unmark_sym(Sym s);

/* ------------------------------------------------------------------ */
/* builtin macros (expand_builtin.c)                                   */
/* ------------------------------------------------------------------ */

bool builtin_exists(Sym name);
MacOut builtin_expand(Sym name, MacCall *mac, MacPos pos);
/* Render tokens like stringify! does. */
const char *tokens_stringify(TokenList tl);

/* ------------------------------------------------------------------ */
/* #[derive] (expand_derive.c)                                         */
/* ------------------------------------------------------------------ */

/* Generate the impls for one derive trait name on an ADT item. */
ItemList derive_generate(Item *adt, Sym trait, Span sp, bool also_copy);

/* astprint.c helpers for derive */
const char *ast_bounds_to_str(const BoundList *bl);
const char *ast_where_to_str(const Generics *g); /* "" or "where ..." */

#endif
