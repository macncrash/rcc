/*
 * ast.h - the Rust abstract syntax tree produced by the parser.
 *
 * ===================== READ THIS FIRST ==================================
 *
 * Conventions
 *   - Every node struct that a later pass may want to annotate begins with
 *     NODE_HDR: `Span span; NodeId id;`. The parser assigns every node a
 *     fresh, unique, nonzero NodeId (ast_fresh_id()); passes that create
 *     nodes must do the same (the ast_new_* helpers below do). Use NodeIds as
 *     keys of side tables (resolution results, types, ...).
 *   - Nodes are tagged unions: a `kind` enum plus an anonymous union of
 *     per-kind structs, accessed as e.g. `e->binary.lhs`, `t->ref.elem`.
 *     Union member names that collide with C keywords get a trailing
 *     underscore (`if_`, `while_`, `for_`, `break_`, `continue_`, `try_`).
 *   - Lists are VEC(T*) typedefs (`ExprList`, `PatList`, ...): fields
 *     `.data`, `.len`. Lists always hold pointers.
 *   - Names are interned Syms (util.h). Optional names use Sym 0.
 *     Optional sub-nodes use NULL. Keyword names (self, Self, super,
 *     crate, _) are the KW_* Syms from lex.h, e.g. seg->name == KW_SELF_TYPE.
 *   - All memory is arena-allocated and never freed. Nodes may be mutated or
 *     replaced in place by later passes (e.g. expansion replaces EXPR_MAC).
 *   - Parentheses are NOT represented: `(a + b) * c` is just
 *     BINARY(*, BINARY(+, a, b), c). A 1-tuple is EXPR_TUPLE with one elem.
 *
 * Things the parser cannot decide (resolved later):
 *   - A lone identifier pattern `x` / `None` is PAT_IDENT; name resolution
 *     decides whether it is a binding or a unit struct/variant/const.
 *   - A generic argument written as a bare path (`Foo<N>`) is GA_TYPE even if
 *     N turns out to be a const parameter. `{expr}`, literals and `-lit` are
 *     GA_CONST.
 *   - Macro invocations are unexpanded (EXPR_MAC, STMT_MAC, ITEM_MAC, TY_MAC,
 *     PAT_MAC) holding a MacCall: path + delimiter + raw tokens.
 *   - `mod foo;` is ITEM_MOD with mod.inline_ == false and no items; the
 *     expander loads the file.
 *   - #[cfg], #[derive], #[test] etc. are kept as Attr nodes, unprocessed.
 *
 * Blocks: Block.stmts is the statement list. The block's value ("tail
 * expression") is the last statement iff it is STMT_EXPR with semi == false
 * (use block_tail()). A block-like expression statement in the middle of a
 * block (`if c { a() } b()`) is also STMT_EXPR with semi == false; it must
 * have type () in that position. A trailing macro call `{ foo!(x) }` is
 * STMT_MAC with semi == false: the expander decides whether it is an
 * expression or statements.
 * ======================================================================
 */
#ifndef RCC_AST_H
#define RCC_AST_H

#include <stdio.h>

#include "lex.h"
#include "util.h"

typedef uint32_t NodeId;

#define NODE_HDR                                                               \
    Span span;                                                                 \
    NodeId id

typedef struct Expr Expr;
typedef struct Ty Ty;
typedef struct Pat Pat;
typedef struct Stmt Stmt;
typedef struct Item Item;
typedef struct Block Block;
typedef struct Path Path;
typedef struct PathSegment PathSegment;
typedef struct GenericArgs GenericArgs;
typedef struct GenericArg GenericArg;
typedef struct GenericParam GenericParam;
typedef struct GenericBound GenericBound;
typedef struct WherePredicate WherePredicate;
typedef struct Attr Attr;
typedef struct MacCall MacCall;
typedef struct Arm Arm;
typedef struct ExprField ExprField;
typedef struct PatField PatField;
typedef struct Param Param;
typedef struct FieldDef FieldDef;
typedef struct Variant Variant;
typedef struct UseTree UseTree;
typedef struct FnPtrParam FnPtrParam;

typedef VEC(Expr *) ExprList;
typedef VEC(Ty *) TyList;
typedef VEC(Pat *) PatList;
typedef VEC(Stmt *) StmtList;
typedef VEC(Item *) ItemList;
typedef VEC(PathSegment *) PathSegList;
typedef VEC(GenericArg *) GenericArgList;
typedef VEC(GenericParam *) GenericParamList;
typedef VEC(GenericBound *) BoundList;
typedef VEC(WherePredicate *) WherePredList;
typedef VEC(Attr *) AttrList;
typedef VEC(Arm *) ArmList;
typedef VEC(ExprField *) ExprFieldList;
typedef VEC(PatField *) PatFieldList;
typedef VEC(Param *) ParamList;
typedef VEC(FieldDef *) FieldDefList;
typedef VEC(Variant *) VariantList;
typedef VEC(UseTree *) UseTreeList;
typedef VEC(FnPtrParam *) FnPtrParamList;

/* ------------------------------------------------------------------ */
/* Token trees, macros, attributes                                     */
/* ------------------------------------------------------------------ */

typedef enum Delim {
    DELIM_NONE,
    DELIM_PAREN,   /* ( ) */
    DELIM_BRACKET, /* [ ] */
    DELIM_BRACE,   /* { } */
} Delim;

/*
 * A macro invocation `path!(tokens)`, `path![tokens]`, `path!{tokens}`.
 * `tokens` are the tokens strictly inside the outer delimiters (a flat array
 * with balanced inner delimiters; see tt_end() in lex.h). It is NOT
 * terminated by TK_EOF; to re-parse, hand it to the parse_*_from_tokens
 * functions in parse.h, which append an EOF themselves.
 * `macro_rules!` definitions are NOT MacCalls; see ITEM_MACRO_RULES.
 */
struct MacCall {
    NODE_HDR;
    Path *path;
    Delim delim;
    TokenList tokens;
};

typedef enum AttrArgsKind {
    ATTR_ARGS_NONE,      /* #[test]                                 */
    ATTR_ARGS_DELIMITED, /* #[derive(Clone, Debug)], #[cfg(test)]   */
    ATTR_ARGS_EQ,        /* #[doc = "x"], #[path = "foo.rs"]         */
} AttrArgsKind;

/*
 * An attribute `#[path args]` (outer) or `#![path args]` (inner).
 * Doc comments are dropped by the lexer and never appear as attributes.
 */
struct Attr {
    NODE_HDR;
    bool inner;
    Path *path;
    AttrArgsKind args_kind;
    Delim delim;      /* ATTR_ARGS_DELIMITED */
    TokenList tokens; /* DELIMITED: inside the delimiters.
                         EQ: the tokens after `=`. */
    Expr *value;      /* ATTR_ARGS_EQ: the parsed expression after `=` */
};

/* ------------------------------------------------------------------ */
/* Paths and generics                                                  */
/* ------------------------------------------------------------------ */

/*
 * A path `a::b::<T>::c`. `global` is set for a leading `::` (`::std::x`).
 * Segments may be keywords: self, Self, super, crate (as KW_* syms).
 * In expression/pattern paths, generic args come from turbofish `::<>`; in
 * type paths from plain `<>` (or `::<>`), or from `Fn(A) -> B` sugar.
 */
struct Path {
    NODE_HDR;
    bool global;
    PathSegList segs;
};

struct PathSegment {
    NODE_HDR;
    Sym name;
    GenericArgs *args; /* NULL if none were written */
};

typedef enum GenericArgsKind {
    GENERIC_ARGS_ANGLE, /* <T, 'a, N, Item = U, Item: Bound> */
    GENERIC_ARGS_PAREN, /* Fn(A, B) -> C sugar                */
} GenericArgsKind;

struct GenericArgs {
    Span span;
    GenericArgsKind kind;
    GenericArgList args; /* ANGLE */
    TyList inputs;       /* PAREN */
    Ty *output;          /* PAREN: NULL means `-> ()` (nothing written) */
};

typedef enum GenericArgKind {
    GA_LIFETIME,   /* 'a                               : lifetime */
    GA_TYPE,       /* T                                : ty       */
    GA_CONST,      /* {N + 1}, 3, -1                   : expr     */
    GA_BINDING,    /* Item = T   (associated type eq)  : name, ty */
    GA_CONSTRAINT, /* Item: Bound (assoc bound)        : name, bounds */
} GenericArgKind;

struct GenericArg {
    Span span;
    GenericArgKind kind;
    Sym lifetime;
    Ty *ty;
    Expr *expr;
    Sym name;
    GenericArgs *name_args; /* GA_BINDING/CONSTRAINT: `Item<'a> = T` (GATs) */
    BoundList bounds;
};

/*
 * `<ty as Trait>::rest` or `<ty>::rest`. The accompanying Path holds the
 * trait's segments followed by the rest; the first `position` segments are
 * the trait (position == 0 for `<ty>::rest`). Example:
 *   <Vec<T> as IntoIterator>::Item  => qself.ty = Vec<T>, position = 1,
 *                                      path = IntoIterator::Item
 */
typedef struct QSelf {
    Span span;
    Ty *ty;
    uint32_t position;
} QSelf;

typedef enum BoundKind {
    BOUND_TRAIT,    /* [for<'a>] [?]Path */
    BOUND_LIFETIME, /* 'a                */
} BoundKind;

struct GenericBound {
    Span span;
    BoundKind kind;
    Sym lifetime;                   /* BOUND_LIFETIME */
    Path *path;                     /* BOUND_TRAIT */
    GenericParamList for_lifetimes; /* BOUND_TRAIT: for<'a, 'b> */
    bool maybe;                     /* BOUND_TRAIT: ?Sized */
};

typedef enum GenericParamKind {
    GP_LIFETIME, /* 'a: 'b + 'c                */
    GP_TYPE,     /* T: Bound + 'a = Default     */
    GP_CONST,    /* const N: usize = 3          */
} GenericParamKind;

struct GenericParam {
    NODE_HDR;
    AttrList attrs;
    GenericParamKind kind;
    Sym name;           /* lifetime names are stored without the quote */
    BoundList bounds;   /* LIFETIME: BOUND_LIFETIME outlives; TYPE: bounds */
    Ty *default_ty;     /* TYPE */
    Ty *const_ty;       /* CONST */
    Expr *const_default; /* CONST */
};

typedef enum WherePredKind {
    WP_BOUND,    /* [for<'a>] Ty: Bounds */
    WP_LIFETIME, /* 'a: 'b + 'c          */
} WherePredKind;

struct WherePredicate {
    Span span;
    WherePredKind kind;
    GenericParamList for_lifetimes; /* WP_BOUND */
    Ty *ty;                         /* WP_BOUND */
    Sym lifetime;                   /* WP_LIFETIME */
    BoundList bounds;
};

/* Generic parameters plus where clause; embedded by value in items. */
typedef struct Generics {
    Span span;
    GenericParamList params;
    WherePredList where;
} Generics;

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

typedef enum TyKind {
    TY_PATH,  /* path:  [qself] path  (incl. Self, i32, Vec<T>, T::Item) */
    TY_REF,   /* ref:   &'a mut T                                        */
    TY_PTR,   /* ptr:   *const T / *mut T                                */
    TY_SLICE, /* slice: [T]                                              */
    TY_ARRAY, /* array: [T; len]                                         */
    TY_TUPLE, /* tuple: (A, B), () is the empty tuple                    */
    TY_NEVER, /* !                                                       */
    TY_FN,    /* fn_:   [for<'a>] [unsafe] [extern "abi"] fn(A, B) -> R  */
    TY_DYN,   /* bounds: dyn Trait + Send + 'a                           */
    TY_IMPL,  /* bounds: impl Trait + 'a                                 */
    TY_INFER, /* _                                                       */
    TY_MAC,   /* mac:   foo!(...) in type position                       */
} TyKind;

/* One parameter of a fn pointer type; `name` is 0 if unnamed. */
struct FnPtrParam {
    Span span;
    Sym name;
    Ty *ty;
};

struct Ty {
    NODE_HDR;
    TyKind kind;
    union {
        struct {
            QSelf *qself; /* NULL unless <T as Tr>::X or <T>::X */
            Path *path;
        } path;
        struct {
            Sym lifetime; /* 0 if elided */
            bool is_mut;
            Ty *elem;
        } ref;
        struct {
            bool is_mut; /* false: *const */
            Ty *elem;
        } ptr;
        struct {
            Ty *elem;
        } slice;
        struct {
            Ty *elem;
            Expr *len;
        } array;
        struct {
            TyList elems;
        } tuple;
        struct {
            GenericParamList for_lifetimes;
            bool is_unsafe;
            Sym abi; /* 0 = Rust ABI; `extern fn` without string => "C" */
            FnPtrParamList params;
            Ty *ret; /* NULL => () */
            bool variadic;
        } fn_;
        BoundList bounds; /* TY_DYN, TY_IMPL */
        MacCall *mac;     /* TY_MAC */
    };
};

/* ------------------------------------------------------------------ */
/* Patterns                                                            */
/* ------------------------------------------------------------------ */

typedef enum PatKind {
    PAT_WILD,         /* _                                               */
    PAT_IDENT,        /* ident: [ref] [mut] name [@ sub]                 */
    PAT_LIT,          /* lit: expr is EXPR_LIT or UNARY(NEG, EXPR_LIT)   */
    PAT_RANGE,        /* range: lo..=hi, lo..hi, lo.., ..=hi             */
    PAT_PATH,         /* path: a::B, <T as Tr>::C  (never a single ident) */
    PAT_TUPLE,        /* elems: (a, b, ..)                                */
    PAT_TUPLE_STRUCT, /* tuple_struct: Some(x), Foo::Bar(a, ..)          */
    PAT_STRUCT,       /* struct_: Foo { a, b: c, ref d, .. }             */
    PAT_SLICE,        /* elems: [a, rest @ .., b]                         */
    PAT_REST,         /* ..  (only inside tuple / tuple-struct / slice)   */
    PAT_REF,          /* ref: &pat, &mut pat                              */
    PAT_OR,           /* elems: a | b | c  (flattened, >= 2 alternatives) */
    PAT_MAC,          /* mac                                              */
} PatKind;

typedef enum RangeEnd {
    RANGE_HALF_OPEN, /* ..  */
    RANGE_INCLUSIVE, /* ..= (and legacy ...) */
} RangeEnd;

struct PatField {
    NODE_HDR;
    AttrList attrs;
    Sym name;       /* field name (or "0", "1" for tuple-struct fields) */
    Pat *pat;       /* for shorthand `ref mut a`, the PAT_IDENT itself  */
    bool shorthand; /* `a` / `ref mut a` rather than `a: pat`           */
};

struct Pat {
    NODE_HDR;
    PatKind kind;
    union {
        struct {
            bool by_ref; /* `ref` */
            bool is_mut; /* `mut` */
            Sym name;
            Pat *sub; /* `name @ sub`, else NULL */
        } ident;
        Expr *lit; /* PAT_LIT */
        struct {
            Expr *lo; /* NULL for ..=hi  (EXPR_LIT, negated lit, EXPR_PATH) */
            Expr *hi; /* NULL for lo..   */
            RangeEnd end;
        } range;
        struct {
            QSelf *qself;
            Path *path;
        } path;
        PatList elems; /* PAT_TUPLE, PAT_SLICE, PAT_OR */
        struct {
            QSelf *qself;
            Path *path;
            PatList elems;
        } tuple_struct;
        struct {
            QSelf *qself;
            Path *path;
            PatFieldList fields;
            bool has_rest; /* trailing `..` */
        } struct_;
        struct {
            bool is_mut;
            Pat *pat;
        } ref;
        MacCall *mac;
    };
};

/* ------------------------------------------------------------------ */
/* Expressions                                                         */
/* ------------------------------------------------------------------ */

typedef enum BinOp {
    BIN_ADD, BIN_SUB, BIN_MUL, BIN_DIV, BIN_REM,
    BIN_AND,    /* && */
    BIN_OR,     /* || */
    BIN_BITXOR, BIN_BITAND, BIN_BITOR, BIN_SHL, BIN_SHR,
    BIN_EQ, BIN_LT, BIN_LE, BIN_NE, BIN_GE, BIN_GT,
} BinOp;

typedef enum UnOp {
    UN_NEG,   /* -x */
    UN_NOT,   /* !x */
    UN_DEREF, /* *x */
} UnOp;

typedef enum ExprKind {
    EXPR_LIT,         /* lit                                             */
    EXPR_PATH,        /* path: [qself] path  (x, a::b, Vec::<T>::new)    */
    EXPR_UNARY,       /* unary: -x, !x, *x                               */
    EXPR_ADDR_OF,     /* addr_of: &x, &mut x, &raw const x, &raw mut x   */
    EXPR_BINARY,      /* binary: lhs op rhs (incl. && and ||)            */
    EXPR_ASSIGN,      /* assign: lhs = rhs      (assign.op unused)       */
    EXPR_ASSIGN_OP,   /* assign: lhs op= rhs                             */
    EXPR_CAST,        /* cast: expr as ty                                */
    EXPR_CALL,        /* call: callee(args)                              */
    EXPR_METHOD_CALL, /* method_call: receiver.name::<T>(args)           */
    EXPR_FIELD,       /* field: base.name  or base.0                     */
    EXPR_INDEX,       /* index: base[index]                              */
    EXPR_STRUCT,      /* struct_: Path { a: x, b, ..base }               */
    EXPR_TUPLE,       /* elems: (a, b), (a,), () (unit)                  */
    EXPR_ARRAY,       /* elems: [a, b, c]                                */
    EXPR_REPEAT,      /* repeat: [elem; count]                           */
    EXPR_RANGE,       /* range: a..b a.. ..b .. a..=b ..=b               */
    EXPR_CLOSURE,     /* closure: [move] |params| [-> ret] body          */
    EXPR_BLOCK,       /* block: ['label:] [unsafe] { ... }               */
    EXPR_IF,          /* if_: if cond { } [else ...]                     */
    EXPR_LET,         /* let: `let pat = scrutinee` (in if/while conds)  */
    EXPR_MATCH,       /* match: match scrutinee { arms }                 */
    EXPR_WHILE,       /* while_: ['label:] while cond { }  (incl. let)   */
    EXPR_FOR,         /* for_: ['label:] for pat in iter { }             */
    EXPR_LOOP,        /* loop: ['label:] loop { }                        */
    EXPR_BREAK,       /* break_: break ['label] [value]                  */
    EXPR_CONTINUE,    /* continue_: continue ['label]                    */
    EXPR_RETURN,      /* ret: return [value]                             */
    EXPR_TRY,         /* try_: operand?                                  */
    EXPR_MAC,         /* mac: foo!(...)                                  */
    EXPR_UNDERSCORE,  /* _  (lhs of destructuring assignment)            */
} ExprKind;

struct Arm {
    NODE_HDR;
    AttrList attrs;
    Pat *pat;
    Expr *guard; /* `if guard`, or NULL */
    Expr *body;
};

struct ExprField {
    NODE_HDR;
    AttrList attrs;
    Sym name; /* field name; numeric fields (`0: x`) use the Sym "0" */
    Expr *expr;
    bool shorthand; /* `Foo { a }`: expr is the EXPR_PATH `a` */
};

/* Function or closure parameter. For closures `ty` may be NULL. */
struct Param {
    NODE_HDR;
    AttrList attrs;
    Pat *pat;
    Ty *ty;
};

struct Expr {
    NODE_HDR;
    ExprKind kind;
    AttrList attrs; /* outer attributes written directly on the expression */
    union {
        Lit *lit;
        struct {
            QSelf *qself;
            Path *path;
        } path;
        struct {
            UnOp op;
            Expr *operand;
        } unary;
        struct {
            bool is_mut;
            bool raw; /* &raw const / &raw mut */
            Expr *operand;
        } addr_of;
        struct {
            BinOp op;
            Expr *lhs, *rhs;
        } binary;
        struct {
            BinOp op; /* EXPR_ASSIGN_OP only */
            Expr *lhs, *rhs;
        } assign;
        struct {
            Expr *expr;
            Ty *ty;
        } cast;
        struct {
            Expr *callee;
            ExprList args;
        } call;
        struct {
            Expr *receiver;
            PathSegment *method; /* name + optional turbofish args */
            ExprList args;
        } method_call;
        struct {
            Expr *base;
            Sym name;      /* named field; for tuple index, the Sym "0" etc. */
            bool is_index; /* base.0 */
            uint32_t index;
        } field;
        struct {
            Expr *base;
            Expr *index;
        } index;
        struct {
            QSelf *qself;
            Path *path;
            ExprFieldList fields;
            Expr *base;    /* `..base`, or NULL */
            bool has_rest; /* `..` without base (destructuring assignment) */
        } struct_;
        ExprList elems; /* EXPR_TUPLE, EXPR_ARRAY */
        struct {
            Expr *elem;
            Expr *count;
        } repeat;
        struct {
            Expr *lo, *hi; /* either may be NULL */
            bool inclusive; /* ..= */
        } range;
        struct {
            bool is_move;
            ParamList params;
            Ty *ret; /* NULL if not written (then body is any expr) */
            Expr *body;
        } closure;
        struct {
            Block *block; /* Block.is_unsafe for `unsafe { }` */
            Sym label;    /* 'label: { ... }, else 0 */
        } block;
        struct {
            Expr *cond; /* may contain EXPR_LET (possibly in && chains) */
            Block *then;
            Expr *els; /* NULL, EXPR_BLOCK or EXPR_IF */
        } if_;
        struct {
            Pat *pat;
            Expr *scrutinee;
        } let;
        struct {
            Expr *scrutinee;
            ArmList arms;
        } match;
        struct {
            Expr *cond; /* may be EXPR_LET for `while let` */
            Block *body;
            Sym label;
        } while_;
        struct {
            Pat *pat;
            Expr *iter;
            Block *body;
            Sym label;
        } for_;
        struct {
            Block *body;
            Sym label;
        } loop;
        struct {
            Sym label;   /* label without quote, or 0 */
            Expr *value; /* or NULL */
        } break_;
        struct {
            Sym label;
        } continue_;
        Expr *ret;  /* EXPR_RETURN value, or NULL */
        Expr *try_; /* EXPR_TRY operand */
        MacCall *mac;
    };
};

/* ------------------------------------------------------------------ */
/* Statements and blocks                                               */
/* ------------------------------------------------------------------ */

typedef enum StmtKind {
    STMT_LET,   /* let: let pat [: ty] [= init [else { els }]];         */
    STMT_ITEM,  /* item (the item carries its own attrs)                */
    STMT_EXPR,  /* expr + semi                                          */
    STMT_MAC,   /* mac + semi: macro call in statement position         */
    STMT_EMPTY, /* a lone `;`                                           */
} StmtKind;

struct Stmt {
    NODE_HDR;
    StmtKind kind;
    AttrList attrs; /* outer attributes (not used for STMT_ITEM) */
    union {
        struct {
            Pat *pat;
            Ty *ty;     /* or NULL */
            Expr *init; /* or NULL */
            Block *els; /* let-else block, or NULL */
        } let;
        Item *item;
        struct {
            Expr *expr;
            bool semi; /* followed by `;` */
        } expr;
        struct {
            MacCall *mac;
            bool semi;
        } mac;
    };
};

struct Block {
    NODE_HDR;
    AttrList attrs; /* inner attributes `#![...]` at the start of the block */
    StmtList stmts;
    bool is_unsafe;
};

/* Returns the tail expression of a block, or NULL (see header comment). */
Expr *block_tail(const Block *b);

/* ------------------------------------------------------------------ */
/* Items                                                               */
/* ------------------------------------------------------------------ */

typedef enum VisKind {
    VIS_INHERITED, /* (nothing written) */
    VIS_PUB,       /* pub             */
    VIS_CRATE,     /* pub(crate)      */
    VIS_SUPER,     /* pub(super)      */
    VIS_SELF,      /* pub(self)       */
    VIS_IN,        /* pub(in path)    */
} VisKind;

typedef struct Visibility {
    Span span;
    VisKind kind;
    Path *path; /* VIS_IN */
} Visibility;

typedef enum UseTreeKind {
    USE_SIMPLE, /* prefix [as rename]   e.g. a::b, a::b as c, self      */
    USE_GLOB,   /* prefix::*                                            */
    USE_NESTED, /* prefix::{ items }  (prefix may be empty: `use {a, b}`) */
} UseTreeKind;

/*
 * `use` tree. prefix may contain self/super/crate segments and may have
 * zero segments (`use ::*`, `use {a, b}`, nested `{*}`); prefix->global marks
 * a leading `::`. Within a nested group, each child's prefix is relative to
 * the parent's prefix. `use a::{self}` gives a child USE_SIMPLE whose
 * prefix is the single segment `self`.
 */
struct UseTree {
    NODE_HDR;
    UseTreeKind kind;
    Path *prefix;
    Sym rename;        /* USE_SIMPLE: 0 if none; KW_UNDERSCORE for `as _` */
    UseTreeList items; /* USE_NESTED */
};

typedef enum SelfKind {
    SELF_VALUE,    /* self, mut self                        */
    SELF_REF,      /* &self, &'a self, &mut self, &'a mut self */
    SELF_EXPLICIT, /* self: Ty, mut self: Ty                */
} SelfKind;

typedef struct SelfParam {
    NODE_HDR;
    AttrList attrs;
    SelfKind kind;
    bool is_mut;  /* VALUE/EXPLICIT: `mut self` binding; REF: `&mut self` */
    Sym lifetime; /* REF: optional lifetime */
    Ty *ty;       /* EXPLICIT: the written type */
} SelfParam;

typedef struct FnSig {
    Span span;
    bool is_const;
    bool is_unsafe;
    bool is_extern; /* `extern ["abi"] fn` */
    Sym abi;        /* 0 if not extern; "C" if `extern fn` without string */
    SelfParam *self_param; /* NULL if no self receiver */
    ParamList params;      /* excluding self */
    Ty *ret;               /* NULL => () */
    bool variadic;         /* trailing `...` (extern fns only) */
} FnSig;

typedef enum VariantDataKind {
    VD_STRUCT, /* { a: T, b: U } */
    VD_TUPLE,  /* (T, U)         */
    VD_UNIT,   /* nothing        */
} VariantDataKind;

typedef struct VariantData {
    VariantDataKind kind;
    FieldDefList fields;
} VariantData;

struct FieldDef {
    NODE_HDR;
    AttrList attrs;
    Visibility vis;
    Sym name; /* 0 for tuple fields */
    Ty *ty;
};

struct Variant {
    NODE_HDR;
    AttrList attrs;
    Visibility vis; /* always inherited in valid Rust */
    Sym name;
    VariantData data;
    Expr *disc; /* explicit discriminant `= expr`, or NULL */
};

typedef enum ItemKind {
    ITEM_USE,          /* use_tree                                      */
    ITEM_EXTERN_CRATE, /* extern_crate: extern crate orig [as name];   */
    ITEM_MOD,          /* mod: mod name; / mod name { items }           */
    ITEM_FN,           /* fn_                                           */
    ITEM_STRUCT,       /* adt: struct (generics + data)                 */
    ITEM_UNION,        /* adt: union                                    */
    ITEM_ENUM,         /* enum_                                         */
    ITEM_TRAIT,        /* trait                                         */
    ITEM_IMPL,         /* impl                                          */
    ITEM_TYPE_ALIAS,   /* type_alias: type X<..>: Bounds = Ty;          */
    ITEM_CONST,        /* const_: const NAME: Ty [= expr];  name may be _ */
    ITEM_STATIC,       /* static_: static [mut] NAME: Ty [= expr];      */
    ITEM_FOREIGN_MOD,  /* foreign_mod: extern "C" { items }             */
    ITEM_MAC,          /* mac: foo! { ... } or foo!(...); in item position */
    ITEM_MACRO_RULES,  /* macro_rules: macro_rules! name { rules }      */
} ItemKind;

/*
 * Items appear at module level, in blocks (STMT_ITEM), and as associated
 * items inside traits/impls (ITEM_FN, ITEM_CONST, ITEM_TYPE_ALIAS,
 * ITEM_MAC) and extern blocks (ITEM_FN without body, ITEM_STATIC without
 * expr, ITEM_TYPE_ALIAS without ty). `name` is 0 for ITEM_USE,
 * ITEM_IMPL, ITEM_FOREIGN_MOD and ITEM_MAC.
 * For ITEM_MOD with a body and for the crate root, inner attributes
 * `#![...]` are appended to attrs with Attr.inner == true.
 */
struct Item {
    NODE_HDR;
    AttrList attrs;
    Visibility vis;
    Sym name;
    ItemKind kind;
    union {
        UseTree *use_tree;
        struct {
            Sym orig; /* crate name; `name` is the local name */
        } extern_crate;
        struct {
            bool is_unsafe;
            bool inline_; /* has `{ ... }` body */
            ItemList items;
        } mod;
        struct {
            FnSig sig;
            Generics generics;
            Block *body; /* NULL for `fn f();` (trait/extern decls) */
        } fn_;
        struct {
            Generics generics;
            VariantData data;
        } adt;
        struct {
            Generics generics;
            VariantList variants;
        } enum_;
        struct {
            bool is_unsafe;
            bool is_auto;
            Generics generics;
            BoundList supertraits;
            ItemList items;
        } trait;
        struct {
            bool is_unsafe;
            bool negative; /* impl !Trait for T */
            Generics generics;
            Path *trait_path; /* NULL for inherent impls */
            Ty *self_ty;
            ItemList items;
        } impl;
        struct {
            Generics generics;
            BoundList bounds; /* associated types: `type X: Bound;` */
            Ty *ty;           /* NULL if no `= Ty` */
        } type_alias;
        struct {
            Generics generics; /* always empty in stable Rust */
            Ty *ty;
            Expr *expr; /* NULL in trait declarations */
        } const_;
        struct {
            bool is_mut;
            Ty *ty;
            Expr *expr; /* NULL in extern blocks */
        } static_;
        struct {
            bool is_unsafe;
            Sym abi; /* "C" if not written */
            ItemList items;
        } foreign_mod;
        MacCall *mac;
        struct {
            Delim delim;
            TokenList body; /* tokens inside the outer delimiters */
        } macro_rules;
    };
};

typedef struct Crate {
    Span span;
    NodeId id;
    uint32_t file;
    AttrList attrs; /* inner attributes of the file */
    ItemList items;
} Crate;

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

/* Next unique NodeId (never 0). */
NodeId ast_fresh_id(void);

/* Allocate a zeroed node with the given kind/span and a fresh id. */
Expr *ast_new_expr(ExprKind kind, Span sp);
Ty *ast_new_ty(TyKind kind, Span sp);
Pat *ast_new_pat(PatKind kind, Span sp);
Stmt *ast_new_stmt(StmtKind kind, Span sp);
Item *ast_new_item(ItemKind kind, Span sp);
Block *ast_new_block(Span sp);
Path *ast_new_path(Span sp);
/* Single-segment path with the given name. */
Path *ast_path_from_sym(Sym name, Span sp);

const char *binop_str(BinOp op);
const char *unop_str(UnOp op);

/* S-expression dumps (astdump.c). Output is indented and deterministic. */
void ast_dump_crate(FILE *out, const Crate *c);
void ast_dump_item(FILE *out, const Item *it);
void ast_dump_expr(FILE *out, const Expr *e);
void ast_dump_ty(FILE *out, const Ty *t);
void ast_dump_pat(FILE *out, const Pat *p);
void ast_dump_stmt(FILE *out, const Stmt *s);

#endif
