/* typeck_dump.c - S-expression dump of THIR bodies (--dump-types). */
#include "typeck.h"

static const char *kind_names[] = {
    "lit_int", "lit_float", "lit_str", "lit_bytestr", "local", "static", "const", "const_param",
    "fn", "unary", "binary", "logical", "assign", "assign_op", "cast", "call", "field", "index",
    "deref", "ref", "addr_of", "adt", "tuple", "array", "repeat", "closure", "block", "if", "let",
    "match", "loop", "while", "break", "continue", "return", "never_to_any", "unsize", "reify",
    "closure_fnptr"};

static void ind(FILE *out, int n) { fprintf(out, "%*s", n * 2, ""); }

static void dump_pat(FILE *out, const TPat *p) {
    static const char *modes[] = {"", "ref ", "ref mut "};
    if (!p) {
        fprintf(out, "<nopat>");
        return;
    }
    switch ((TPatKind)p->kind) {
    case TP_WILD: fprintf(out, "_"); break;
    case TP_BIND:
        fprintf(out, "%s%s#%u", modes[p->bind_mode], sym_str(p->local->name), p->local->id);
        if (p->sub) {
            fprintf(out, " @ ");
            dump_pat(out, p->sub);
        }
        break;
    case TP_CONST: fprintf(out, "<const>"); break;
    case TP_RANGE: fprintf(out, "<range>"); break;
    case TP_TUPLE:
    case TP_VARIANT:
        if (p->kind == TP_VARIANT)
            fprintf(out, "%s", sym_str(adt_def(p->adt)->variants[p->variant].name));
        fprintf(out, "(");
        for (uint32_t i = 0; i < p->nsubs; i++) {
            fprintf(out, "%s%u: ", i ? ", " : "", p->field_idx[i]);
            dump_pat(out, p->subs[i]);
        }
        fprintf(out, ")");
        break;
    case TP_DEREF:
        fprintf(out, "&");
        dump_pat(out, p->sub);
        break;
    case TP_SLICE:
        fprintf(out, "[");
        for (uint32_t i = 0; i < p->nsubs; i++) {
            if (i == p->nprefix && p->has_rest) {
                dump_pat(out, p->rest);
                fprintf(out, " @ .., ");
            }
            dump_pat(out, p->subs[i]);
            fprintf(out, ", ");
        }
        if (p->has_rest && p->nprefix == p->nsubs) {
            if (p->rest)
                dump_pat(out, p->rest);
            fprintf(out, " ..");
        }
        fprintf(out, "]");
        break;
    case TP_OR:
        for (uint32_t i = 0; i < p->nsubs; i++) {
            fprintf(out, "%s", i ? " | " : "");
            dump_pat(out, p->subs[i]);
        }
        break;
    }
}

void thir_dump_expr(FILE *out, const TExpr *e, int n) {
    if (!e)
        return;
    ind(out, n);
    fprintf(out, "(%s : %s", kind_names[e->kind], ty_to_str(e->ty));
    switch ((TExprKind)e->kind) {
    case TE_LIT_INT: fprintf(out, " %llu", (unsigned long long)e->int_.lo); break;
    case TE_LIT_FLOAT: fprintf(out, " %g", e->fval); break;
    case TE_LIT_STR: fprintf(out, " \"%.*s\"", (int)e->str.len, e->str.s); break;
    case TE_LOCAL: fprintf(out, " %s#%u", sym_str(e->local->name), e->local->id); break;
    case TE_STATIC: case TE_CONST: case TE_FN: case TE_CLOSURE:
        fprintf(out, " %s%s", def_path_str(e->item.def), substs_to_str(e->item.substs));
        break;
    case TE_UNARY: fprintf(out, " %s", unop_str((UnOp)e->op)); break;
    case TE_BINARY: case TE_LOGICAL: case TE_ASSIGN_OP:
        fprintf(out, " %s", binop_str((BinOp)e->op));
        break;
    case TE_FIELD: case TE_CONST_PARAM: fprintf(out, " .%u", e->index); break;
    case TE_REF: case TE_ADDR_OF: fprintf(out, e->is_mut ? " mut" : ""); break;
    case TE_ADT: fprintf(out, " variant=%u", e->variant); break;
    case TE_LOOP: case TE_WHILE: case TE_BREAK: case TE_CONTINUE:
        fprintf(out, " 'L%u", e->label);
        break;
    case TE_LET:
        fprintf(out, " ");
        dump_pat(out, e->pat);
        break;
    default: break;
    }
    bool nl = e->a || e->b || e->c || e->nargs || e->nstmts || e->narms || e->kind == TE_CLOSURE;
    if (nl)
        fprintf(out, "\n");
    if (e->kind == TE_CLOSURE) {
        ClosureInfo *ci = def_get(e->item.def)->closure;
        static const char *kinds[] = {"Fn", "FnMut", "FnOnce"};
        static const char *modes[] = {"ref", "mut", "value"};
        ind(out, n + 1);
        fprintf(out, "[%s%s; captures:", kinds[ci->kind], ci->is_move ? " move" : "");
        for (uint32_t i = 0; i < ci->ncaptures; i++)
        {
            fprintf(out, " %s", sym_str(ci->captures[i].var->name));
            for (uint32_t k = 0; k < ci->captures[i].npath; k++)
                fprintf(out, ".%u", ci->captures[i].path[k]);
            fprintf(out, ":%s", modes[ci->captures[i].mode]);
        }
        fprintf(out, "; params:");
        for (uint32_t i = 0; i < ci->nparams; i++) {
            fprintf(out, " ");
            dump_pat(out, ci->params[i]);
        }
        fprintf(out, "]\n");
        thir_dump_expr(out, ci->body, n + 1);
    }
    for (uint32_t i = 0; i < e->nstmts; i++) {
        const TStmt *s = e->stmts[i];
        if (s->kind == TS_LET) {
            ind(out, n + 1);
            fprintf(out, "(let ");
            dump_pat(out, s->pat);
            fprintf(out, "\n");
            thir_dump_expr(out, s->init, n + 2);
            thir_dump_expr(out, s->els, n + 2);
            ind(out, n + 1);
            fprintf(out, ")\n");
        } else {
            thir_dump_expr(out, s->expr, n + 1);
        }
    }
    thir_dump_expr(out, e->a, n + 1);
    thir_dump_expr(out, e->b, n + 1);
    thir_dump_expr(out, e->c, n + 1);
    for (uint32_t i = 0; i < e->nargs; i++)
        thir_dump_expr(out, e->args[i], n + 1);
    for (uint32_t i = 0; i < e->narms; i++) {
        ind(out, n + 1);
        fprintf(out, "(arm ");
        dump_pat(out, e->arms[i].pat);
        fprintf(out, "\n");
        thir_dump_expr(out, e->arms[i].guard, n + 2);
        thir_dump_expr(out, e->arms[i].body, n + 2);
        ind(out, n + 1);
        fprintf(out, ")\n");
    }
    if (nl)
        ind(out, n);
    fprintf(out, ")\n");
}

void thir_dump_body(FILE *out, const Body *b) {
    fprintf(out, "body %s -> %s\n", b->owner ? def_path_str(b->owner) : "<anon>", ty_to_str(b->ret_ty));
    for (uint32_t i = 0; i < b->nlocals; i++)
        fprintf(out, "  local %s#%u: %s\n", sym_str(b->locals[i]->name), b->locals[i]->id,
                ty_to_str(b->locals[i]->ty));
    thir_dump_expr(out, b->value, 1);
}
