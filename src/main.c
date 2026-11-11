/* main.c - rcc driver. */
#include <stdio.h>
#include <string.h>

#include "ast.h"
#include "lex.h"
#include "parse.h"
#include "expand.h"
#include "driver.h"

static void usage(void) {
    fprintf(stderr,
            "usage: rcc MODE FILE.rs\n"
            "modes:\n"
            "  --tokens FILE    dump tokens\n"
            "  --parse FILE     parse only; exit 0 on success\n"
            "  --dump-ast FILE  parse and print the AST as S-expressions\n"
            "  --check-macros FILE\n"
            "                   re-parse the arguments of std macro calls\n"
            "                   (vec!, format!, assert_eq!, matches!, ...) with\n"
            "                   the token-stream parser API (front-end test)\n"
            "  --expand ROOT.rs [--expand-compat] [--fmt-root PATH] [--crate-name N]\n"
            "           [--cfg SPEC]... [--env K=V]...\n"
            "                   print the macro-expanded crate as Rust source\n");
}

/*
 * Front-end self test for the re-parsing API: find `name!(...)` calls of
 * std macros whose arguments are comma-separated expressions and parse them
 * with parser_expr() / parser_pat(). Prints the number of calls checked.
 */
static int check_macros(const char *path) {
    uint32_t f = source_load(path);
    TokenList tl = lex_file(f);
    static const char *expr_macros[] = {
        "vec", "format", "print", "println", "eprint", "eprintln", "write",
        "writeln", "assert", "assert_eq", "assert_ne", "debug_assert",
        "debug_assert_eq", "debug_assert_ne", "panic", "unreachable", "todo",
        "unimplemented", "format_args", NULL};
    int checked = 0;
    for (uint32_t i = 0; i + 2 < tl.len; i++) {
        Token *t = &tl.toks[i];
        if (t->kind != TK_IDENT || tl.toks[i + 1].kind != TK_NOT ||
            !tok_is_open(tl.toks[i + 2].kind))
            continue;
        const char *name = sym_str(t->v.sym);
        bool is_matches = strcmp(name, "matches") == 0;
        bool known = is_matches;
        for (int k = 0; expr_macros[k]; k++)
            if (strcmp(name, expr_macros[k]) == 0)
                known = true;
        if (!known || (i > 0 && tl.toks[i - 1].kind == TK_DOLLAR))
            continue;
        uint32_t open = i + 2, close = tt_end(tl.toks, open, tl.len);
        TokenList args = {tl.toks + open + 1, close - open - 1};
        Parser *p = parser_new(args, tl.toks[close].span);
        bool has_dollar = false;
        for (uint32_t k = 0; k < args.len; k++)
            if (args.toks[k].kind == TK_DOLLAR)
                has_dollar = true;
        if (has_dollar) /* inside a macro_rules body */
            continue;
        if (is_matches) {
            (void)parser_expr(p);
            if (parser_peek(p, 0)->kind != TK_COMMA)
                fatal(parser_peek(p, 0)->span, "matches!: expected `,`");
            parser_set_pos(p, parser_pos(p) + 1);
            (void)parser_pat(p);
            if (tok_is_kw(parser_peek(p, 0), KW_IF)) {
                parser_set_pos(p, parser_pos(p) + 1);
                (void)parser_expr(p);
            }
        } else {
            while (!parser_at_eof(p)) {
                (void)parser_expr(p);
                int k = parser_peek(p, 0)->kind;
                if (k == TK_EOF)
                    break;
                if (k != TK_COMMA && k != TK_SEMI)
                    fatal(parser_peek(p, 0)->span,
                          "%s!: expected `,` between arguments", name);
                parser_set_pos(p, parser_pos(p) + 1);
            }
        }
        if (!parser_at_eof(p))
            fatal(parser_peek(p, 0)->span, "%s!: trailing tokens", name);
        checked++;
        i = close;
    }
    printf("%d macro calls re-parsed\n", checked);
    return 0;
}

/*
 * --expand ROOT.rs [--expand-compat] [--fmt-root PATH] [--crate-name N]
 *          [--cfg SPEC]... [--env K=V]...
 * Prints the fully expanded crate as Rust source (expander test mode).
 */
static int expand_main(int argc, char **argv) {
    static const char *cfgs[256], *envs[256];
    ExpandOptions o;
    memset(&o, 0, sizeof o);
    o.cfgs = cfgs;
    o.envs = envs;
    const char *root = NULL, *name = NULL;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (strcmp(a, "--expand") == 0 && i + 1 < argc) root = argv[++i];
        else if (strcmp(a, "--expand-compat") == 0) o.compat = true;
        else if (strcmp(a, "--fmt-root") == 0 && i + 1 < argc) o.fmt_root = argv[++i];
        else if (strcmp(a, "--crate-name") == 0 && i + 1 < argc) name = argv[++i];
        else if (strcmp(a, "--cfg") == 0 && i + 1 < argc && o.ncfgs < 256) cfgs[o.ncfgs++] = argv[++i];
        else if (strcmp(a, "--env") == 0 && i + 1 < argc && o.nenvs < 256) envs[o.nenvs++] = argv[++i];
        else {
            fprintf(stderr, "rcc --expand: unknown option `%s`\n", a);
            return 2;
        }
    }
    if (!root) {
        usage();
        return 2;
    }
    if (!name) {
        const char *b = strrchr(root, '/');
        b = b ? b + 1 : root;
        size_t n = strlen(b);
        if (n > 3 && strcmp(b + n - 3, ".rs") == 0)
            n -= 3;
        char *s = arena_strndup(b, n);
        for (char *c = s; *c; c++)
            if (*c == '-')
                *c = '_';
        name = s;
    }
    lex_init();
    Crate *c = expand_crate(root, name, &o);
    ast_print_crate(stdout, c);
    return 0;
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++)
        if (strcmp(argv[i], "--expand") == 0)
            return expand_main(argc, argv);
    if (rcc_is_compile_command(argc, argv))
        return rcc_compile_main(argc, argv);
    if (argc != 3) {
        usage();
        return 2;
    }
    const char *mode = argv[1];
    const char *path = argv[2];
    lex_init();
    if (strcmp(mode, "--tokens") == 0) {
        uint32_t f = source_load(path);
        dump_tokens(lex_file(f));
        return 0;
    }
    if (strcmp(mode, "--parse") == 0) {
        (void)parse_file(path);
        return 0;
    }
    if (strcmp(mode, "--dump-ast") == 0) {
        Crate *c = parse_file(path);
        ast_dump_crate(stdout, c);
        return 0;
    }
    if (strcmp(mode, "--check-macros") == 0)
        return check_macros(path);
    usage();
    return 2;
}
