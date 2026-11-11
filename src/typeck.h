/*
 * typeck.h - type checking: item signatures ("collect"), the trait system,
 * type inference, and body checking producing THIR (thir.h).
 *
 * Files
 *   typeck_collect.c  generics_of / predicates_of / type_of / fn_sig /
 *                     adt_def / trait_def / impl_def, AST type lowering,
 *                     impl tables
 *   typeck_infer.c    inference variables, unification, snapshots
 *   typeck_traits.c   trait selection, projection normalization, pending
 *                     obligations, closed-type queries used by mono
 *   typeck_expr.c     expressions, statements, coercions, calls, operators
 *   typeck_method.c   method probing, associated-item paths
 *   typeck_pat.c      patterns and default binding modes
 *   typeck_closure.c  closures, capture analysis, writeback, typeck_body()
 *   typeck_const.c    constant evaluation over THIR
 *
 * All item-level queries are lazy and memoized in the Def. Bodies are
 * checked on demand (typeck_body), normally only when mono reaches them.
 */
#ifndef RCC_TYPECK_H
#define RCC_TYPECK_H

#include "resolve.h"
#include "thir.h"

/* ------------------------------------------------------------------ */
/* Item signatures                                                     */
/* ------------------------------------------------------------------ */

typedef struct GenericParamDef {
    Sym name;
    DefId def;         /* DEF_TY_PARAM / DEF_CONST_PARAM; 0 for Self and for
                          synthetic `impl Trait` parameters */
    uint32_t index;    /* position in the item's full substs */
    bool is_const;
    Ty *default_ast;   /* `= Default`, or NULL */
    Ty *impl_trait;    /* synthetic: the `impl Trait` type node */
} GenericParamDef;

/*
 * The generic parameters of an item. Substs for the item are laid out
 * parent-first: [parent params.., own params..]. For traits, own[0] is
 * `Self`. Associated items have their impl/trait as parent.
 */
typedef struct GenericsDef {
    DefId owner;
    DefId parent; /* 0 if none */
    uint32_t parent_count;
    uint32_t count; /* parent_count + nown */
    GenericParamDef *own;
    uint32_t nown;
    bool has_self; /* trait (own[0]) or inherited from a trait parent */
} GenericsDef;

GenericsDef *generics_of(DefId def);
/* [T_PARAM(0), T_PARAM(1), ...] for the item. */
Substs *identity_substs(DefId def);
GenericParamDef *generics_param(GenericsDef *g, uint32_t index);

typedef struct TraitRef {
    DefId trait;
    Substs *substs; /* substs[0] is the Self type */
} TraitRef;

typedef enum PredKind {
    PRED_TRAIT, /* tr holds */
    PRED_PROJ,  /* <tr>::assoc == ty */
} PredKind;

typedef struct Predicate {
    PredKind kind;
    TraitRef tr;
    DefId assoc; /* PRED_PROJ: DEF_ASSOC_TY in tr.trait */
    Type *ty;    /* PRED_PROJ */
} Predicate;

typedef struct PredList {
    Predicate *data;
    uint32_t len, cap;
} PredList;

/* Declared predicates of the item and of its parents (not elaborated). */
PredList *predicates_of(DefId def);
/* predicates_of + all supertrait predicates: the caller bounds in scope
   inside the item. */
PredList *param_env(DefId def);
/* Append to `out` the predicates implied by `p` through supertraits. */
void elaborate_pred(PredList *out, const Predicate *p);

typedef struct FnSigT {
    Substs *inputs;
    Type *output;
    bool has_self;
    bool variadic;
} FnSigT;

/* Signature of a fn, or of the constructor of a tuple struct / variant. */
FnSigT *fn_sig(DefId def);

/*
 * struct/enum/union: the ADT type with identity substs. alias: the aliased
 * type. const/static/assoc const: its type. assoc type in an impl: the
 * type. impl: the Self type. type/const param: the T_PARAM. variant: the
 * enum type.
 */
Type *type_of(DefId def);

typedef struct FieldT {
    Sym name; /* "0", "1", .. for tuple fields */
    bool is_private; /* no `pub`: visible only inside the ADT's module */
    Ty *ast;
    Type *ty; /* in the ADT's generics */
} FieldT;

typedef struct VariantDef {
    DefId def; /* the variant, or the struct/union itself */
    Sym name;
    uint8_t ctor; /* VariantDataKind */
    FieldT *fields;
    uint32_t nfields;
    Expr *disc_expr;
    int64_t disc;
} VariantDef;

typedef struct AdtDef {
    DefId def;
    bool is_enum, is_union;
    VariantDef *variants; /* structs/unions have exactly one */
    uint32_t nvariants;
    bool fields_done, discs_done;
    Type *repr; /* #[repr(int)] type, or NULL */
} AdtDef;

/* The ADT with all field types lowered. For a DEF_VARIANT gives the enum. */
AdtDef *adt_def(DefId def);
/* Discriminant values (fills VariantDef.disc on first use). */
void adt_compute_discs(AdtDef *adt);
/* Field index by name, or -1. */
int variant_field_index(const VariantDef *v, Sym name);

typedef struct TraitDef {
    DefId def;
    HMap items;          /* Sym -> DefId of own associated items */
    PredList supers;     /* direct supertrait predicates (Self = param 0) */
    PredList item_bounds; /* bounds declared on associated types */
    bool is_auto;        /* auto/marker-like: every type implements it */
    bool supers_done;
} TraitDef;

TraitDef *trait_def(DefId def);
/* Own associated item by name, or 0. */
DefId trait_item(DefId trait, Sym name);
/* Direct supertrait predicates and associated-type bounds (lazy). */
PredList *trait_supers(DefId trait);
PredList *trait_item_bounds(DefId trait);
/* Find associated item `name` in tr.trait or its supertraits. On success
   *owner is the (super)trait ref that declares it. */
DefId trait_item_deep(TraitRef tr, Sym name, TraitRef *owner);

typedef struct ImplDef {
    DefId def;
    bool of_trait;
    bool negative;
    TraitRef trait_ref; /* in the impl's generics */
    Type *self_ty;
    HMap items;         /* Sym -> DefId */
} ImplDef;

ImplDef *impl_def(DefId def);
DefId impl_item(DefId impl, Sym name);

typedef struct OpaqueInfo {
    uint32_t n;
    PredList *bounds; /* per ordinal; Self type = the opaque type */
    Type **concrete;  /* per ordinal; set by typeck of the fn body */
} OpaqueInfo;

/* Build the impl tables. Call after resolve_finish(). */
void typeck_init(void);

/* Impls of `trait` that could apply to `self` (already shallow-resolved):
   *exact = impls whose self type has the same head constructor (all impls
   if self is an inference variable), *blanket = impls for a bare type
   parameter. Either may be NULL. */
void trait_impls_for(DefId trait, Type *self, const DefIdList **exact, const DefIdList **blanket);
/* Inherent impls whose self type has the same head constructor. */
const DefIdList *inherent_impls_for(Type *self);
/* Trait items (DefIds) named `name`, across all traits. */
const DefIdList *trait_items_named(Sym name);

/* Cached lang items and well-known defs, valid after typeck_init(). */
typedef struct LangItems {
    DefId sized, copy, clone, drop, deref, deref_mut, index, index_mut;
    DefId fn, fn_mut, fn_once, fn_once_output;
    DefId box, option, result, string;
    DefId rc, arc; /* smart pointers that support unsizing like Box: field 0
                      is the raw pointer to the first type argument */
    DefId range, range_from, range_to, range_inclusive, range_to_inclusive, range_full;
    DefId into_iterator, iterator, from, partial_eq, partial_ord;
    DefId binop[18];        /* by BinOp: Add.. traits (0 where none) */
    DefId binop_assign[18]; /* AddAssign.. */
    DefId neg, not_;
    DefId deref_target;
} LangItems;
extern LangItems lang;

/* ------------------------------------------------------------------ */
/* AST type lowering                                                   */
/* ------------------------------------------------------------------ */

struct FnCtxt;

typedef enum ImplTraitMode { IT_ERROR, IT_ARG, IT_RET } ImplTraitMode;

typedef struct LowerCtx {
    Scope *scope;
    DefId item;         /* item whose generics are in scope (may be 0) */
    struct FnCtxt *fcx; /* non-NULL inside bodies: `_` and elided generic
                           args become inference variables */
    uint8_t impl_trait; /* ImplTraitMode */
} LowerCtx;

Type *lower_ty(LowerCtx *lc, Ty *ast);
/* Lower `Trait<Args, Assoc = T>` (the last segment of `path` carries the
   args) with the given Self type. Associated-type bindings are appended to
   `projs` as PRED_PROJ if it is non-NULL. */
TraitRef lower_trait_ref(LowerCtx *lc, DefId trait, PathSegment *seg, Type *self, PredList *projs,
                         Span sp);
/* Substs for `def`'s OWN parameters from written args (may be NULL),
   appended to `parent`. Missing args: inference variables if
   `infer_missing` (expression paths in bodies), else the declared
   defaults, else fatal. */
Substs *lower_generic_args(LowerCtx *lc, DefId def, GenericArgs *args, Substs *parent,
                           bool infer_missing, Span sp);
/* Find `Trait::name` reachable from the bounds on a type (a T_PARAM, or
   Self); returns the projection type, or NULL. */
Type *lower_assoc_ty_path(LowerCtx *lc, Type *self, Sym name, Span sp);
/* Evaluate a const expression used as array length / const generic arg. */
Type *lower_const_expr(LowerCtx *lc, Expr *e);

void pred_push(PredList *l, Predicate p);

/* ------------------------------------------------------------------ */
/* Inference                                                           */
/* ------------------------------------------------------------------ */

typedef struct TyVar {
    Type *node;
    Type *value; /* NULL while unbound */
    Span origin;
    bool diverging;
} TyVar;

typedef struct Obligation {
    Predicate pred;
    Span span;
} Obligation;

typedef struct InferCtxt {
    TyVar *vars;
    uint32_t nvars, vars_cap;
    uint32_t *undo; /* ids of variables bound, in order */
    uint32_t nundo, undo_cap;
    Obligation *pending;
    uint32_t npending, pending_cap;
    PredList *env; /* caller bounds (may be NULL) */
    int probe_depth; /* > 0 while inside a probe (errors are not fatal) */
} InferCtxt;

typedef struct Snapshot {
    uint32_t nundo, npending;
} Snapshot;

InferCtxt *infer_new(PredList *env);
Type *infer_var(InferCtxt *icx, InferKind k, Span origin);
/* Resolve a top-level variable (follows chains). */
Type *infer_shallow(InferCtxt *icx, Type *t);
/* Replace all bound variables inside t. */
Type *infer_resolve(InferCtxt *icx, Type *t);
Substs *infer_resolve_substs(InferCtxt *icx, Substs *s);
/* Unify; on failure nothing is changed. */
bool infer_unify(InferCtxt *icx, Type *a, Type *b);
bool infer_unify_substs(InferCtxt *icx, Substs *a, Substs *b);
Snapshot infer_snapshot(InferCtxt *icx);
void infer_rollback(InferCtxt *icx, Snapshot s);
/* Fresh variables for all generic parameters of def. */
Substs *infer_fresh_substs(InferCtxt *icx, DefId def, Span sp);
/* Fresh variables for def's own parameters appended to `parent`. */
Substs *infer_fresh_own_substs(InferCtxt *icx, DefId def, Substs *parent, Span sp);

/* ------------------------------------------------------------------ */
/* Traits                                                              */
/* ------------------------------------------------------------------ */

typedef enum EvalResult { EVAL_NO, EVAL_AMBIG, EVAL_YES } EvalResult;

typedef enum SelKind {
    SEL_NONE,    /* no candidate applies */
    SEL_AMBIG,   /* cannot decide yet (inference variables) */
    SEL_IMPL,    /* impl + substs for the impl's generics */
    SEL_PARAM,   /* a caller bound / item bound */
    SEL_OBJECT,  /* dyn Trait implements Trait */
    SEL_CLOSURE, /* closure implements Fn* */
    SEL_FNPTR,   /* fn item / fn pointer implements Fn* */
    SEL_BUILTIN, /* Sized, auto traits, Copy/Clone of fn items and closures */
} SelKind;

typedef struct Selection {
    SelKind kind;
    DefId impl;
    Substs *substs;
} Selection;

/* Select an implementation. On success unifications are committed and the
   nested obligations (impl where-clauses) are registered as pending. */
Selection trait_select(InferCtxt *icx, TraitRef tr, Span sp);
/* Would the predicate hold? No side effects. */
EvalResult trait_eval(InferCtxt *icx, const Predicate *p);
void infer_register(InferCtxt *icx, Predicate p, Span sp);
/* Register preds[..] substituted by `substs` (after normalizing). */
void infer_register_preds(InferCtxt *icx, PredList *preds, Substs *substs, Span sp);
/* Process pending obligations until no progress. Fatal on an obligation
   that definitely fails. Returns true if anything was resolved. */
bool infer_select_pending(InferCtxt *icx);
/* Replace projections by their definitions; ambiguous ones become fresh
   variables with a pending PRED_PROJ obligation. */
Type *infer_normalize(InferCtxt *icx, Type *t, Span sp);
/* One step of <ty as Deref>::Target for overloaded deref, or NULL. */
Type *infer_overloaded_deref(InferCtxt *icx, Type *ty, Span sp);
/* All supertraits (transitively, including `trait` itself) each with
   substs derived from `tr`; appends to out. */
void supertraits_of(TraitRef tr, PredList *out);

/* ---- closed-type queries (no inference variables, no params) ---- */

/* Fully normalize a closed type (after substitution at mono time),
   revealing opaque types. */
Type *norm_closed(Type *t, Span sp);
Substs *norm_closed_substs(Substs *s, Span sp);
/* Select the impl for a closed trait ref (cached). Fatal if none. */
Selection select_closed(TraitRef tr, Span sp);
bool implements_closed(DefId trait, Type *self);
bool ty_is_copy(Type *t);
/* The projection <tr>::assoc for closed tr. */
Type *project_closed(DefId assoc, Substs *trait_substs, Span sp);

/* ------------------------------------------------------------------ */
/* Bodies                                                              */
/* ------------------------------------------------------------------ */

/* Type-check the body of a fn / const / static (memoized). Returns NULL
   for bodiless fns (foreign, required trait methods). */
Body *typeck_body(DefId def);
/* Type-check a stand-alone constant expression (array length, enum
   discriminant) against `expected`. */
Body *typeck_anon_const(Scope *scope, DefId item, Expr *e, Type *expected);
/* Concrete type behind `impl Trait` #ordinal returned by fn (in fn's
   generics). */
Type *opaque_concrete(DefId fn, uint32_t ordinal);

/* Constant evaluation of scalar THIR (ints, bool, char). `substs` gives
   values for generic parameters (may be NULL). Returns false if the
   expression is not a compile-time scalar we can evaluate. */
typedef struct ConstVal {
    uint64_t lo, hi; /* 128-bit two's complement, truncated to the type */
} ConstVal;
bool const_eval(const TExpr *e, Substs *substs, ConstVal *out);
/* Wrap v to the width/signedness of integer-like type t (sign-extending
   to 128 bits for signed types). */
ConstVal const_wrap(ConstVal v, Type *t);

#endif
