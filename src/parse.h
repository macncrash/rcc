/*
 * parse.h - the Rust parser.
 *
 * The parser works over a flat token array (lex.h), so the macro expander can
 * re-parse macro input/output token streams with the same code. All errors are
 * fatal (file:line:col: message, exit 1).
 */
#ifndef RCC_PARSE_H
#define RCC_PARSE_H

#include "ast.h"

/* Parse a whole lexed source file (tokens must end in TK_EOF). */
Crate *parse_crate(TokenList toks, uint32_t file);

/* Convenience: load, lex and parse a file. */
Crate *parse_file(const char *path);

/*
 * Whole-stream entry points for the expander. `toks` need NOT be
 * EOF-terminated (e.g. MacCall.tokens); `eof_span` is used for errors at the
 * end of input (pass the macro call's span). Each fatals unless the entire
 * stream is consumed.
 */
Expr *parse_expr_from_tokens(TokenList toks, Span eof_span);
Ty *parse_ty_from_tokens(TokenList toks, Span eof_span);
/* Top-level pattern: or-patterns allowed, as for `$p:pat` in edition 2021. */
Pat *parse_pat_from_tokens(TokenList toks, Span eof_span);
/* Statements as in a block body (no braces). */
StmtList parse_stmts_from_tokens(TokenList toks, Span eof_span);
/* Items as in a module body (no braces); inner attrs are rejected. */
ItemList parse_items_from_tokens(TokenList toks, Span eof_span);
/* Associated items as in an impl or trait body. */
ItemList parse_assoc_items_from_tokens(TokenList toks, Span eof_span);

/*
 * Incremental interface, for the macro_rules matcher: parse one fragment
 * from the current position and report how far it got.
 */
typedef struct Parser Parser;

Parser *parser_new(TokenList toks, Span eof_span);
uint32_t parser_pos(const Parser *p);   /* index of the next token */
void parser_set_pos(Parser *p, uint32_t pos);
bool parser_at_eof(const Parser *p);
const Token *parser_peek(const Parser *p, uint32_t ahead);

Expr *parser_expr(Parser *p);   /* $e:expr  */
Ty *parser_ty(Parser *p);       /* $t:ty    */
Pat *parser_pat(Parser *p);     /* $p:pat (top-level, or-patterns allowed) */
Pat *parser_pat_no_top_alt(Parser *p); /* $p:pat_param */
Stmt *parser_stmt(Parser *p);   /* $s:stmt (without trailing `;`) */
Item *parser_item(Parser *p);   /* $i:item; NULL if no item starts here */
Block *parser_block(Parser *p); /* $b:block */
Path *parser_path(Parser *p);   /* $p:path (type-style path) */
Visibility parser_vis(Parser *p); /* $v:vis (may be empty) */
Expr *parser_literal(Parser *p);  /* $l:literal (incl. -lit) */
Attr *parser_meta(Parser *p);     /* $m:meta : path + args, no #[ ] */

/* Predicates for matcher decisions (does the next token start an X?). */
bool parser_can_begin_expr(const Parser *p);
bool parser_can_begin_ty(const Parser *p);

#endif
