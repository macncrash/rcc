/*
 * typeck_fn.h - internal to typeck_*.c: the per-body checking context.
 *
 * One FnCtxt checks one Body (fn body, const/static initializer or anonymous
 * constant) including all closures nested in it. FnCtxts are independent
 * heap objects, so checking one body may trigger checking another (e.g. an
 * array length constant, or a callee's `impl Trait` return type).
 *
 * Conventions used by every check_* function:
 *   - `expected` is a HINT (may be NULL or contain inference variables). It
 *     is never enforced by the callee; the caller coerces the result with
 *     coerce() when a type is actually required.
 *   - Returned TExprs carry types that may still contain inference
 *     variables; writeback (typeck_closure.c) resolves them at the end.
 *   - fcx->diverges is set when control flow cannot continue past the
 *     expression just checked.
 */
#ifndef RCC_TYPECK_FN_H
#define RCC_TYPECK_FN_H

#include "typeck.h"

typedef enum BreakKind { BS_LOOP, BS_WHILE, BS_BLOCK } BreakKind;

typedef struct BreakScope {
    struct BreakScope *parent;
    Sym label;
    uint32_t id;
    uint8_t kind; /* BreakKind */
    Type *ty;     /* type of `break` values (BS_LOOP, BS_BLOCK) */
    bool has_break;
} BreakScope;

typedef VEC(Local *) LocalVec;
typedef VEC(TExpr *) TExprVec;
typedef VEC(TStmt *) TStmtVec;

typedef struct FnCtxt {
    InferCtxt *icx; /* MUST be the first member (see LowerCtx.fcx users) */
    DefId owner;
    Scope *scope;
    LowerCtx lc;
    Type *ret_ty;
    BreakScope *breaks;
    DefId closure; /* innermost closure being checked, or 0 */
    uint32_t next_label;
    Substs *identity;
    bool diverges;
    LocalVec locals;
    TExprVec deferred_ops; /* operator nodes to finalize at writeback */
    DefIdList closures;
    HMap *pat_names; /* bindings of the pattern being checked: Sym -> Local* */
    Type **opaque_vars; /* inference variables standing for the owner's
                           return-position impl Trait types */
} FnCtxt;

/* ---- typeck_expr.c ---- */
TExpr *te_new(TExprKind kind, Span sp, Type *ty);
TExpr *check_expr(FnCtxt *fcx, Expr *e, Type *expected);
/* check_expr + coerce to `target`. */
TExpr *check_expr_coerce(FnCtxt *fcx, Expr *e, Type *target);
TExpr *check_block(FnCtxt *fcx, Block *b, Type *expected, Sym label);
/* Coerce e to `target` (fatal on mismatch). */
TExpr *coerce(FnCtxt *fcx, TExpr *e, Type *target);
/* Like coerce but returns NULL instead of failing (no side effects then). */
TExpr *try_coerce(FnCtxt *fcx, TExpr *e, Type *target);
/* Shallow-resolve, processing pending obligations if still a variable. */
Type *structurally_resolve(FnCtxt *fcx, Type *t);
/* As above but fatal ("type annotations needed") if unresolved. */
Type *resolve_or_fatal(FnCtxt *fcx, Type *t, Span sp);
void unify_or_fatal(FnCtxt *fcx, Type *expected, Type *actual, Span sp);
Local *new_local(FnCtxt *fcx, Sym name, Type *ty, bool is_mut, Span sp);
/* One auto-deref step applied to an expression (builtin for &T and Box<T>,
   overloaded through Deref); NULL if the type cannot be dereferenced. */
TExpr *deref_once(FnCtxt *fcx, TExpr *e);
/* Type after one auto-deref step, or NULL. */
Type *autoderef_ty(FnCtxt *fcx, Type *t, Span sp);
/* Make a place expression usable mutably: switches overloaded derefs and
   index operations in it to DerefMut / IndexMut. */
void fix_place_mut(FnCtxt *fcx, TExpr *e);
/* Check call arguments against formal input types. */
TExpr **check_args(FnCtxt *fcx, Substs *formals, uint32_t first_formal, ExprList *args,
                   Type **hints, bool variadic, Span sp, uint32_t *nout);
/* Hints for arguments derived from the expected result type (or NULL). */
Type **expected_inputs(FnCtxt *fcx, Type *expected, Type *output, Substs *formals);
/* A call of `def` (trait method or fn) with explicit substs and prepared
   arguments; result type is `ty`. */
TExpr *make_call(Span sp, DefId def, Substs *substs, Type *ty, TExpr **args, uint32_t nargs);
TExpr *make_ref(Span sp, TExpr *a, bool is_mut);
bool fcx_is_copy(FnCtxt *fcx, Type *t);
/* Writeback hook: turn a deferred operator node into a builtin operation
   or a trait method call (operand types must be final). */
void finalize_deferred_op(TExpr *x);
Type *lower_body_ty(FnCtxt *fcx, Ty *ast);
Type *fcx_normalize(FnCtxt *fcx, Type *t, Span sp);
const char *fcx_ty_str(FnCtxt *fcx, Type *t);

/* ---- typeck_method.c ---- */
TExpr *check_path_expr(FnCtxt *fcx, Expr *e, Type *expected);
TExpr *check_method_call(FnCtxt *fcx, Expr *e, Type *expected);
/* Resolve the path of a struct literal / struct pattern / tuple-struct
   pattern to (adt type, variant index). */
Type *resolve_variant_path(FnCtxt *fcx, QSelf *qself, Path *path, Namespace ns, uint32_t *variant,
                           DefId *ctor_def, Span sp);
/* Instantiate a value-namespace path that names an item (not a local). */
TExpr *instantiate_value_path(FnCtxt *fcx, QSelf *qself, Path *path, Span sp);
/* Find a method `name` on receiver `recv` and build the adjusted receiver.
   On success returns the callee's def/substs through the out params and
   the adjusted receiver expression; NULL if no method was found. */
TExpr *lookup_method(FnCtxt *fcx, TExpr *recv, Sym name, GenericArgs *turbofish, Span sp,
                     DefId *out_def, Substs **out_substs);

/* ---- typeck_pat.c ---- */
/* Check a pattern against `expected`; bindings are added to fcx->scope
   (which must be a SCOPE_LOCALS rib). */
TPat *check_pat_top(FnCtxt *fcx, Pat *p, Type *expected);
/* Strongest use the pattern makes of its scrutinee: 0 inspect/borrow,
   1 mutable borrow, 2 move. */
int pat_use_kind(FnCtxt *fcx, const TPat *p);

/* ---- typeck_closure.c ---- */
TExpr *check_closure(FnCtxt *fcx, Expr *e, Type *expected);

#endif
