/*
 * thir.h - the typed IR produced by typeck and consumed by MIR building.
 *
 * A Body is the typed form of a fn body, const/static initializer, or
 * anonymous constant. In a finished Body
 *   - every TExpr/TPat/Local has a fully resolved type (no T_INFER; it may
 *     mention the owner's generic parameters as T_PARAM and unnormalizable
 *     projections of them),
 *   - every implicit step is an explicit node: auto-ref (TE_REF), auto-deref
 *     (TE_DEREF, or a TE_CALL of Deref::deref wrapped in TE_DEREF),
 *     coercions (TE_UNSIZE, TE_REIFY, TE_CLOSURE_FNPTR, TE_NEVER_TO_ANY,
 *     TE_CAST), overloaded operators and indexing (TE_CALL), method calls
 *     (TE_CALL with a TE_FN callee carrying DefId + substs), `for` loops and
 *     `?` (desugared to TE_MATCH/TE_LOOP),
 *   - calls to trait methods name the TRAIT's method with substs
 *     [Self, trait params.., method params..]; mono selects the impl,
 *   - patterns have explicit binding modes and explicit TP_DEREF nodes
 *     where default binding modes looked through a reference.
 *
 * Place expressions (things with an address) are TE_LOCAL, TE_STATIC,
 * TE_DEREF, TE_FIELD and TE_INDEX; everything else is a value.
 */
#ifndef RCC_THIR_H
#define RCC_THIR_H

#include <stdio.h>

#include "types.h"

typedef struct TExpr TExpr;
typedef struct TPat TPat;
typedef struct TStmt TStmt;
typedef struct TArm TArm;
typedef struct Local Local;
typedef struct Body Body;

/* A local variable (binding or parameter). */
struct Local {
    uint32_t id;  /* dense per Body */
    Sym name;
    Type *ty;
    bool is_mut;
    DefId closure; /* innermost closure declaring it, or 0 (fn body) */
    Span span;
};

typedef enum TExprKind {
    TE_LIT_INT,     /* int: integers, bools (0/1) and chars */
    TE_LIT_FLOAT,   /* fval */
    TE_LIT_STR,     /* str: &'static str */
    TE_LIT_BYTESTR, /* str: &'static [u8; N] */
    TE_LOCAL,       /* local */
    TE_STATIC,      /* def */
    TE_CONST,       /* def, substs: const item or associated const (maybe the
                       trait's; mono selects the impl) */
    TE_CONST_PARAM, /* index: value of a const generic parameter */
    TE_FN,          /* def, substs: fn item or tuple-struct/variant ctor as a
                       value (type T_FNDEF) */
    TE_UNARY,       /* op (UN_NEG / UN_NOT), a: builtin */
    TE_BINARY,      /* op, a, b: builtin arithmetic / comparison */
    TE_LOGICAL,     /* op (BIN_AND / BIN_OR), a, b: short-circuit */
    TE_ASSIGN,      /* a = b */
    TE_ASSIGN_OP,   /* a op= b: builtin */
    TE_CAST,        /* a as ty */
    TE_CALL,        /* a(args): a has type T_FNDEF, T_FNPTR or T_CLOSURE */
    TE_FIELD,       /* a.index */
    TE_INDEX,       /* a[b]: builtin array/slice indexing by usize */
    TE_DEREF,       /* *a: builtin (a is &T, *T or Box<T>) */
    TE_REF,         /* &a / &mut a (is_mut) */
    TE_ADDR_OF,     /* &raw const a / &raw mut a */
    TE_ADT,         /* struct/variant literal: variant, args, field_idx, base */
    TE_TUPLE,       /* args */
    TE_ARRAY,       /* args */
    TE_REPEAT,      /* [a; N], N in ty->len */
    TE_CLOSURE,     /* def (DEF_CLOSURE) */
    TE_BLOCK,       /* stmts, a = tail (or NULL); label for `'l: { }` */
    TE_IF,          /* a cond, b then, c else (or NULL) */
    TE_LET,         /* `let pat = a` in a condition; type bool */
    TE_MATCH,       /* a scrutinee, arms */
    TE_LOOP,        /* a body; label */
    TE_WHILE,       /* a cond, b body; label */
    TE_BREAK,       /* label, a value (or NULL) */
    TE_CONTINUE,    /* label */
    TE_RETURN,      /* a (or NULL) */
    TE_NEVER_TO_ANY,  /* a: ! coerced to ty */
    TE_UNSIZE,        /* a: &[T;N] -> &[T], &T -> &dyn Tr, Box<T> -> Box<dyn> */
    TE_REIFY,         /* a: fn item -> fn pointer */
    TE_CLOSURE_FNPTR, /* a: non-capturing closure -> fn pointer */
} TExprKind;

struct TArm {
    TPat *pat;
    TExpr *guard; /* or NULL */
    TExpr *body;
    Span span;
};

struct TExpr {
    uint8_t kind; /* TExprKind */
    uint8_t op;   /* BinOp / UnOp */
    bool is_mut;
    Span span;
    Type *ty;
    TExpr *a, *b, *c;
    union {
        struct {
            uint64_t lo, hi;
        } int_;
        double fval;
        struct {
            const char *s;
            uint32_t len;
        } str;
        Local *local;
        struct {
            DefId def;
            Substs *substs;
        } item; /* TE_STATIC, TE_CONST, TE_FN, TE_CLOSURE */
        uint32_t index; /* TE_FIELD, TE_CONST_PARAM */
        uint32_t label; /* TE_LOOP, TE_WHILE, TE_BREAK, TE_CONTINUE, TE_BLOCK
                           (0 = unlabeled block) */
        uint32_t variant; /* TE_ADT */
    };
    TExpr **args;        /* TE_CALL, TE_ADT, TE_TUPLE, TE_ARRAY */
    uint32_t nargs;
    uint32_t *field_idx; /* TE_ADT: field index of each arg */
    TStmt **stmts;       /* TE_BLOCK */
    uint32_t nstmts;
    TArm *arms;          /* TE_MATCH */
    uint32_t narms;
    TPat *pat;           /* TE_LET */
};

typedef enum TStmtKind { TS_LET, TS_EXPR } TStmtKind;

struct TStmt {
    TStmtKind kind;
    Span span;
    TPat *pat;   /* TS_LET */
    TExpr *init; /* TS_LET (or NULL) */
    TExpr *els;  /* TS_LET: diverging else block (or NULL) */
    TExpr *expr; /* TS_EXPR */
};

typedef enum TPatKind {
    TP_WILD,
    TP_BIND,    /* local, bind_mode, sub (or NULL) */
    TP_CONST,   /* value: compare with == */
    TP_RANGE,   /* lo, hi (either may be NULL), inclusive */
    TP_TUPLE,   /* subs + field_idx: tuple or struct destructuring */
    TP_VARIANT, /* adt, variant, subs + field_idx */
    TP_DEREF,   /* sub: matches through a reference (or Box) */
    TP_SLICE,   /* subs[0..nprefix], rest, subs[nprefix..nsubs] (suffix);
                   for arrays and slices */
    TP_OR,      /* subs */
} TPatKind;

typedef enum BindMode { BIND_VALUE, BIND_REF, BIND_REF_MUT } BindMode;

struct TPat {
    uint8_t kind; /* TPatKind */
    uint8_t bind_mode;
    bool inclusive;
    bool has_rest; /* TP_SLICE: a `..` is present (rest may still be NULL) */
    Span span;
    Type *ty; /* type of the value being matched */
    Local *local;
    TPat *sub;
    TExpr *value, *lo, *hi;
    DefId adt;
    uint32_t variant;
    TPat **subs;
    uint32_t *field_idx;
    uint32_t nsubs, nprefix;
    TPat *rest; /* TP_SLICE: binding for the middle part, or NULL */
};

typedef enum ClosureKind { CK_FN, CK_FN_MUT, CK_FN_ONCE } ClosureKind;
typedef enum CaptureMode { CAP_REF, CAP_MUT_REF, CAP_VALUE } CaptureMode;

#define CAPTURE_MAX_PATH 6

/*
 * One captured place: a variable of an enclosing scope, or (edition 2021
 * disjoint capture, by-value captures only) a field path into it such as
 * `s.a.0`. By-reference captures always capture the whole variable.
 */
typedef struct Capture {
    Local *var;
    uint8_t mode; /* CaptureMode */
    uint8_t npath;
    uint32_t path[CAPTURE_MAX_PATH]; /* field indices from the variable */
    Type *place_ty; /* type of the captured place */
    Type *ty;       /* type of the environment field: T, &T or &mut T */
} Capture;

/* Everything known about one closure expression (Def.closure). Types are
   expressed in the enclosing item's generic parameters. */
typedef struct ClosureInfo {
    DefId def;
    DefId owner;        /* the fn/const/static whose Body contains it */
    DefId parent_closure; /* enclosing closure, or 0 */
    bool is_move;
    uint8_t kind;       /* ClosureKind, valid after typeck of the owner */
    bool analyzed;      /* captures and kind have been computed */
    TPat **params;
    uint32_t nparams;
    Substs *inputs;     /* parameter types */
    Type *output;
    TExpr *body;
    Capture *captures;
    uint32_t ncaptures;
    Span span;
} ClosureInfo;

struct Body {
    DefId owner;
    Local **locals;
    uint32_t nlocals;
    TPat **params; /* fn parameters (incl. self), as patterns */
    uint32_t nparams;
    Type *ret_ty;
    TExpr *value;
    DefIdList closures; /* in source order */
};

void thir_dump_body(FILE *out, const Body *b);
void thir_dump_expr(FILE *out, const TExpr *e, int indent);

#endif
