/* mir_dump.c - textual MIR (--dump-mir). */
#include "mir.h"

static void place_str(StrBuf *sb, const Place *p) {
    for (uint32_t i = 0; i < p->nprojs; i++)
        if (p->projs[i].kind == PJ_DEREF)
            sb_puts(sb, "(*");
    sb_printf(sb, "_%u", p->local);
    for (uint32_t i = 0; i < p->nprojs; i++) {
        const Proj *pj = &p->projs[i];
        switch ((ProjKind)pj->kind) {
        case PJ_DEREF: sb_puts(sb, ")"); break;
        case PJ_FIELD: sb_printf(sb, ".%u", pj->index); break;
        case PJ_DOWNCAST: sb_printf(sb, " as v%u", pj->index); break;
        case PJ_INDEX: sb_printf(sb, "[_%u]", pj->local); break;
        case PJ_CONST_INDEX: sb_printf(sb, pj->from_end ? "[-%u]" : "[%u]", pj->index); break;
        case PJ_SUBSLICE: sb_printf(sb, "[%u..-%u]", pj->index, pj->to); break;
        }
    }
}

const char *mir_place_str(const MirBody *mir, const Place *p) {
    StrBuf sb = {0};
    (void)mir;
    place_str(&sb, p);
    return sb_str(&sb);
}

static void operand_str(StrBuf *sb, const Operand *o) {
    switch ((OperandKind)o->kind) {
    case OP_COPY:
        sb_puts(sb, "copy ");
        place_str(sb, &o->place);
        break;
    case OP_MOVE:
        sb_puts(sb, "move ");
        place_str(sb, &o->place);
        break;
    case OP_CONST:
        switch ((ConstKind)o->c.kind) {
        case MC_INT: sb_printf(sb, "const %lld_%s", (long long)o->c.lo, ty_to_str(o->c.ty)); break;
        case MC_FLOAT: sb_printf(sb, "const %g", o->c.fval); break;
        case MC_STR: sb_printf(sb, "const \"%.*s\"", (int)(o->c.len > 40 ? 40 : o->c.len), o->c.str); break;
        case MC_BYTESTR: sb_printf(sb, "const b\"..\"(%u)", o->c.len); break;
        case MC_ZST: sb_printf(sb, "const <%s>", ty_to_str(o->c.ty)); break;
        case MC_FN: sb_printf(sb, "const fn %s", o->c.inst->name); break;
        }
        break;
    }
}

static void rvalue_str(StrBuf *sb, const Rvalue *rv) {
    static const char *casts[] = {"int_int", "int_float", "float_int", "float_float", "ptr_ptr",
                                  "ptr_int", "int_ptr", "unsize_array", "unsize_dyn", "fnptr_int",
                                  "transmute"};
    switch ((RvalueKind)rv->kind) {
    case RV_USE: operand_str(sb, &rv->a); break;
    case RV_REF:
        sb_puts(sb, "&");
        place_str(sb, &rv->place);
        break;
    case RV_BINARY:
        sb_printf(sb, "%s(", binop_str((BinOp)rv->op));
        operand_str(sb, &rv->a);
        sb_puts(sb, ", ");
        operand_str(sb, &rv->b);
        sb_puts(sb, ")");
        break;
    case RV_UNARY:
        sb_printf(sb, "%s(", unop_str((UnOp)rv->op));
        operand_str(sb, &rv->a);
        sb_puts(sb, ")");
        break;
    case RV_CAST:
        sb_printf(sb, "cast<%s>(", casts[rv->cast]);
        operand_str(sb, &rv->a);
        sb_printf(sb, ") as %s", ty_to_str(rv->ty));
        break;
    case RV_AGGREGATE:
        sb_printf(sb, "%s", ty_to_str(rv->ty));
        if (rv->agg == AGG_ADT)
            sb_printf(sb, "::v%u", rv->variant);
        sb_puts(sb, " {");
        for (uint32_t i = 0; i < rv->nops; i++) {
            sb_puts(sb, i ? ", " : " ");
            if (rv->field_idx)
                sb_printf(sb, "%u: ", rv->field_idx[i]);
            operand_str(sb, &rv->ops[i]);
        }
        sb_puts(sb, " }");
        break;
    case RV_REPEAT:
        sb_puts(sb, "[");
        operand_str(sb, &rv->a);
        sb_printf(sb, "; %llu]", (unsigned long long)rv->count);
        break;
    case RV_LEN:
        sb_puts(sb, "len(");
        place_str(sb, &rv->place);
        sb_puts(sb, ")");
        break;
    case RV_DISCR:
        sb_puts(sb, "discr(");
        place_str(sb, &rv->place);
        sb_puts(sb, ")");
        break;
    case RV_GLOBAL_ADDR: sb_printf(sb, "&global %s", rv->global->name); break;
    }
}

void mir_dump_body(FILE *out, const MirBody *mir) {
    const Instance *inst = mir->inst;
    fprintf(out, "fn %s(", inst->name);
    for (uint32_t i = 0; i < mir->nargs; i++)
        fprintf(out, "%s_%u: %s", i ? ", " : "", i + 1, ty_to_str(mir->locals[i + 1].ty));
    fprintf(out, ") -> %s {\n", ty_to_str(mir->locals[0].ty));
    for (uint32_t i = mir->nargs + 1; i < mir->nlocals; i++) {
        const MirLocal *l = &mir->locals[i];
        fprintf(out, "    let _%u: %s;", i, ty_to_str(l->ty));
        if (l->name)
            fprintf(out, " // %s", sym_str(l->name));
        if (l->flag)
            fprintf(out, " [flag _%u]", l->flag);
        fprintf(out, "\n");
    }
    for (uint32_t b = 0; b < mir->nblocks; b++) {
        const MirBlock *blk = &mir->blocks[b];
        fprintf(out, "  bb%u:\n", b);
        for (uint32_t i = 0; i < blk->nstmts; i++) {
            const MirStmt *s = &blk->stmts[i];
            StrBuf sb = {0};
            switch ((StmtKindM)s->kind) {
            case ST_ASSIGN:
                place_str(&sb, &s->place);
                sb_puts(&sb, " = ");
                rvalue_str(&sb, &s->rv);
                break;
            case ST_CALL:
                place_str(&sb, &s->place);
                sb_puts(&sb, " = ");
                if (s->callee == CALLEE_DIRECT)
                    sb_puts(&sb, s->inst->name);
                else if (s->callee == CALLEE_INDIRECT) {
                    sb_puts(&sb, "(*");
                    operand_str(&sb, &s->rv.a);
                    sb_puts(&sb, ")");
                } else if (s->callee == CALLEE_VIRTUAL)
                    sb_printf(&sb, "virtual#%u", s->slot);
                else
                    sb_printf(&sb, "intrinsic %s%s", sym_str(s->intrinsic), substs_to_str(s->substs));
                sb_puts(&sb, "(");
                for (uint32_t k = 0; k < s->nargs; k++) {
                    if (k)
                        sb_puts(&sb, ", ");
                    operand_str(&sb, &s->args[k]);
                }
                sb_puts(&sb, ")");
                if (s->diverges)
                    sb_puts(&sb, " -> !");
                break;
            case ST_DROP:
                sb_puts(&sb, s->shallow ? "free(" : "drop(");
                place_str(&sb, &s->place);
                sb_puts(&sb, ")");
                break;
            case ST_BOUNDS_CHECK:
                sb_puts(&sb, "bounds_check(");
                operand_str(&sb, &s->rv.a);
                sb_puts(&sb, " < ");
                operand_str(&sb, &s->rv.b);
                sb_puts(&sb, ")");
                break;
            case ST_FORGET:
                sb_puts(&sb, "forget(");
                place_str(&sb, &s->place);
                sb_puts(&sb, ")");
                break;
            case ST_NOP:
                sb_puts(&sb, "nop");
                break;
            }
            fprintf(out, "    %s\n", sb_str(&sb));
        }
        const Terminator *t = &blk->term;
        StrBuf sb = {0};
        switch ((TermKind)t->kind) {
        case TM_NONE: sb_puts(&sb, "<none>"); break;
        case TM_GOTO: sb_printf(&sb, "goto bb%u", t->target); break;
        case TM_SWITCH:
            sb_puts(&sb, "switch ");
            operand_str(&sb, &t->discr);
            sb_puts(&sb, " {");
            for (uint32_t k = 0; k < t->ntargets; k++)
                sb_printf(&sb, " %lld => bb%u,", (long long)t->values[k].lo, t->targets[k]);
            sb_printf(&sb, " _ => bb%u }", t->target);
            break;
        case TM_RETURN: sb_puts(&sb, "return"); break;
        case TM_UNREACHABLE: sb_puts(&sb, "unreachable"); break;
        }
        fprintf(out, "    %s\n", sb_str(&sb));
    }
    fprintf(out, "}\n\n");
}
