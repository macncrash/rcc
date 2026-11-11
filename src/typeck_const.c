/*
 * typeck_const.c - compile-time evaluation of scalar THIR expressions
 * (integers, bool, char): array lengths, const generic arguments, enum
 * discriminants, constant patterns, and `const` items used as scalars.
 * Anything else (aggregates, references, calls) is not handled here: such
 * constants are materialized as runtime-initialized globals by mono/cgen.
 */
#include "typeck.h"

typedef unsigned __int128 u128_t;
typedef __int128 i128_t;

static u128_t cv_get(ConstVal v) { return ((u128_t)v.hi << 64) | v.lo; }

static ConstVal cv_make(u128_t x) {
    ConstVal v = {(uint64_t)x, (uint64_t)(x >> 64)};
    return v;
}

ConstVal const_wrap(ConstVal v, Type *t) {
    uint32_t bits;
    bool is_signed = false;
    switch ((TypeKind)t->kind) {
    case T_INT:
        bits = ik_bits((IntKind)t->ik);
        is_signed = ik_signed((IntKind)t->ik);
        break;
    case T_BOOL: bits = 1; break;
    case T_CHAR: bits = 32; break;
    default: return v;
    }
    if (bits >= 128)
        return v;
    u128_t x = cv_get(v);
    u128_t mask = ((u128_t)1 << bits) - 1;
    x &= mask;
    if (is_signed && (x >> (bits - 1)) & 1)
        x |= ~mask;
    return cv_make(x);
}

static bool type_signed(Type *t) { return t->kind == T_INT && ik_signed((IntKind)t->ik); }

static uint32_t type_bits(Type *t) {
    return t->kind == T_INT ? ik_bits((IntKind)t->ik) : t->kind == T_CHAR ? 32 : 8;
}

static bool eval(const TExpr *e, Substs *substs, ConstVal *out, int depth);

static bool eval_const_item(const TExpr *e, Substs *substs, ConstVal *out, int depth) {
    DefId def = e->item.def;
    Substs *is = e->item.substs;
    if (substs)
        is = substs_subst(is, substs);
    Def *d = def_get(def);
    if (d->kind == DEF_ASSOC_CONST && def_get(d->parent)->kind == DEF_TRAIT) {
        if (is->flags & (TYF_HAS_PARAM | TYF_HAS_INFER))
            return false;
        is = norm_closed_substs(is, e->span);
        TraitRef tr = {d->parent, is};
        Selection sel = select_closed(tr, e->span);
        if (sel.kind == SEL_IMPL) {
            DefId it = impl_item(sel.impl, d->name);
            if (it) {
                def = it;
                is = sel.substs;
            }
        } else {
            return false;
        }
    }
    Body *b = typeck_body(def);
    if (!b)
        return false;
    return eval(b->value, is, out, depth + 1);
}

static bool eval(const TExpr *e, Substs *substs, ConstVal *out, int depth) {
    if (depth > 64)
        return false;
    Type *ty = e->ty;
    switch ((TExprKind)e->kind) {
    case TE_LIT_INT:
        *out = const_wrap((ConstVal){e->int_.lo, e->int_.hi}, ty);
        return true;
    case TE_CONST:
        if (!eval_const_item(e, substs, out, depth))
            return false;
        return true;
    case TE_CONST_PARAM: {
        if (!substs || e->index >= substs->len || substs->t[e->index]->kind != T_CONST)
            return false;
        *out = cv_make(substs->t[e->index]->cval);
        return true;
    }
    case TE_UNARY: {
        ConstVal a;
        if (!eval(e->a, substs, &a, depth))
            return false;
        u128_t x = cv_get(a);
        if (e->op == UN_NEG)
            x = (u128_t)0 - x;
        else if (ty->kind == T_BOOL)
            x = !x;
        else
            x = ~x;
        *out = const_wrap(cv_make(x), ty);
        return true;
    }
    case TE_LOGICAL: {
        ConstVal a, b;
        if (!eval(e->a, substs, &a, depth))
            return false;
        bool av = a.lo != 0;
        if (e->op == BIN_AND ? !av : av) {
            *out = cv_make(av);
            return true;
        }
        if (!eval(e->b, substs, &b, depth))
            return false;
        *out = cv_make(b.lo != 0);
        return true;
    }
    case TE_BINARY: {
        ConstVal a, b;
        if (!eval(e->a, substs, &a, depth) || !eval(e->b, substs, &b, depth))
            return false;
        Type *ot = e->a->ty;
        if (ot->kind != T_INT && ot->kind != T_BOOL && ot->kind != T_CHAR)
            return false;
        u128_t x = cv_get(a), y = cv_get(b), r = 0;
        bool sg = type_signed(ot);
        uint32_t bits = type_bits(ot);
        switch ((BinOp)e->op) {
        case BIN_ADD: r = x + y; break;
        case BIN_SUB: r = x - y; break;
        case BIN_MUL: r = x * y; break;
        case BIN_DIV:
        case BIN_REM:
            if (y == 0)
                return false;
            if (sg) {
                if ((i128_t)y == -1) {
                    r = e->op == BIN_DIV ? (u128_t)0 - x : 0;
                } else {
                    r = e->op == BIN_DIV ? (u128_t)((i128_t)x / (i128_t)y)
                                         : (u128_t)((i128_t)x % (i128_t)y);
                }
            } else {
                r = e->op == BIN_DIV ? x / y : x % y;
            }
            break;
        case BIN_BITXOR: r = x ^ y; break;
        case BIN_BITAND: r = x & y; break;
        case BIN_BITOR: r = x | y; break;
        case BIN_SHL: r = x << (y & (bits - 1)); break;
        case BIN_SHR:
            r = sg ? (u128_t)((i128_t)x >> (y & (bits - 1))) : x >> (y & (bits - 1));
            break;
        case BIN_EQ: r = x == y; break;
        case BIN_NE: r = x != y; break;
        case BIN_LT: r = sg ? (i128_t)x < (i128_t)y : x < y; break;
        case BIN_LE: r = sg ? (i128_t)x <= (i128_t)y : x <= y; break;
        case BIN_GT: r = sg ? (i128_t)x > (i128_t)y : x > y; break;
        case BIN_GE: r = sg ? (i128_t)x >= (i128_t)y : x >= y; break;
        default: return false;
        }
        *out = const_wrap(cv_make(r), ty);
        return true;
    }
    case TE_CAST: {
        Type *st = e->a->ty;
        if (ty->kind != T_INT && ty->kind != T_BOOL && ty->kind != T_CHAR)
            return false;
        if (e->a->kind == TE_ADT && st->kind == T_ADT && e->a->nargs == 0) {
            AdtDef *adt = adt_def(st->def);
            if (!adt->is_enum)
                return false;
            adt_compute_discs(adt);
            *out = const_wrap(cv_make((u128_t)(i128_t)adt->variants[e->a->variant].disc), ty);
            return true;
        }
        if (st->kind != T_INT && st->kind != T_BOOL && st->kind != T_CHAR)
            return false;
        ConstVal a;
        if (!eval(e->a, substs, &a, depth))
            return false;
        *out = const_wrap(a, ty);
        return true;
    }
    case TE_BLOCK:
        if (e->nstmts == 0 && e->a)
            return eval(e->a, substs, out, depth);
        return false;
    case TE_IF: {
        ConstVal c;
        if (!e->c || !eval(e->a, substs, &c, depth))
            return false;
        return eval(c.lo ? e->b : e->c, substs, out, depth);
    }
    default:
        return false;
    }
}

bool const_eval(const TExpr *e, Substs *substs, ConstVal *out) {
    Type *t = e->ty;
    if (t->kind != T_INT && t->kind != T_BOOL && t->kind != T_CHAR)
        return false;
    return eval(e, substs, out, 0);
}
