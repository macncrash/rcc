/*
 * mir.h - monomorphic instances and the mid-level IR.
 *
 * mono.c    owns the instance table and the worklist: starting from `main`
 *           it builds MIR for every reachable Instance; building MIR for one
 *           instance discovers the instances it calls. It also resolves
 *           trait method calls to impls, assigns vtables and globals, and
 *           answers layout-ish questions (needs_drop).
 * mir_build.c  lowers one THIR Body (with concrete substs) to a MirBody:
 *           a CFG of basic blocks over locals, with explicit temporaries,
 *           pattern tests, bounds checks, and Drop statements at scope
 *           exits.
 * mir_drop.c   drop elaboration: turns the builder's "drop if still
 *           initialized" statements into unconditional drops, or drops
 *           guarded by dynamic drop flags that track moves.
 * mir_dump.c   --dump-mir.
 *
 * All types in MIR are closed (no parameters, projections or inference
 * variables): the builder substitutes and normalizes every THIR type.
 *
 * Places. A Place is a local plus projections. Projections record the
 * type they produce. A place may be unsized ([T], str, dyn Trait) only as
 * the result of dereferencing a fat pointer; such a place can be indexed,
 * sub-sliced, measured (RV_LEN) or re-borrowed (RV_REF), nothing else.
 * Statics are reached by RV_GLOBAL_ADDR + PJ_DEREF, never copied.
 */
#ifndef RCC_MIR_H
#define RCC_MIR_H

#include <stdio.h>

#include "typeck.h"

typedef uint32_t LocalId;
typedef uint32_t BlockId;

typedef struct Instance Instance;
typedef struct Global Global;
typedef struct Vtable Vtable;
typedef struct MirBody MirBody;

/* ------------------------------------------------------------------ */
/* Instances, globals, vtables                                         */
/* ------------------------------------------------------------------ */

typedef enum InstanceKind {
    INST_FN,            /* def + substs: a fn, method or default method */
    INST_CLOSURE,       /* closure body: ty = the closure type.
                           C signature: (Self-as-kind env, args tuple) */
    INST_CLOSURE_ONCE,  /* call_once shim for an Fn/FnMut closure:
                           (env by value, args tuple) */
    INST_FNPTR_SHIM,    /* closure -> fn pointer: (args...) for a
                           non-capturing closure of type ty */
    INST_FN_TRAIT_SHIM, /* Fn*::call* for a fn item / fn pointer type ty:
                           (self, args tuple); used in vtables */
    INST_BOX_ONCE_SHIM, /* FnOnce::call_once through Box<dyn FnOnce>:
                           (Box<ty>, args tuple); moves the callee out of
                           the box, frees the box, calls it */
    INST_GLOBAL_INIT,   /* initializer of `global`: () -> T */
    INST_EXTERN,        /* foreign fn (no body): def */
} InstanceKind;

struct Instance {
    InstanceKind kind;
    uint32_t id; /* dense, creation order */
    DefId def;
    Substs *substs;
    Type *ty;        /* closure / fn type for the shim kinds */
    uint8_t shim_self; /* INST_FN_TRAIT_SHIM: 0 = &self, 1 = &mut self, 2 = self */
    Global *global;  /* INST_GLOBAL_INIT */
    const char *name; /* C symbol */
    /* Signature in closed types. */
    Type **params;
    uint32_t nparams;
    Type *ret;
    bool variadic;
    MirBody *mir; /* NULL for INST_EXTERN */
    Span span;
};

typedef enum GlobalKind {
    GLOBAL_STATIC,   /* `static` item: def */
    GLOBAL_CONST,    /* non-scalar `const` item instance: def + substs */
    GLOBAL_PROMOTED, /* promoted constant expression / extended temporary
                        of a global initializer */
} GlobalKind;

struct Global {
    GlobalKind kind;
    uint32_t id;
    DefId def;
    Substs *substs;
    Type *ty;
    const char *name;
    Instance *init; /* NULL for foreign statics */
    bool is_foreign;
    const TExpr *expr; /* GLOBAL_PROMOTED: initializer expression... */
    Body *body;        /* ...inside this body... */
    Substs *body_substs; /* ...under these substs */
    DefId closure;     /* ...and closure context (0 if none) */
    uint8_t visit;     /* init-order DFS state */
};

/* One vtable per (concrete type, dyn type) pair used in an unsizing
   coercion. Layout: drop glue, size, align, then one slot per object
   method (the principal trait's methods, then its supertraits', in
   supertraits_of order). */
struct Vtable {
    uint32_t id;
    Type *concrete;
    Type *dyn;
    const char *name;
    Instance **slots; /* NULL entries for methods that cannot be called
                         through an object */
    uint32_t nslots;
};

/* How a call is dispatched. */
typedef enum CalleeKind {
    CALLEE_DIRECT,    /* inst */
    CALLEE_INDIRECT,  /* through fn pointer operand `fnptr` */
    CALLEE_VIRTUAL,   /* args[0] is a fat pointer; slot in its vtable */
    CALLEE_INTRINSIC, /* compiler intrinsic `intrinsic` with type args */
} CalleeKind;

/* ------------------------------------------------------------------ */
/* MIR                                                                 */
/* ------------------------------------------------------------------ */

typedef enum ProjKind {
    PJ_DEREF,       /* *p  (p: &T, *T, or Box<T>) */
    PJ_FIELD,       /* .index of a struct, tuple, closure, or of the
                       enum variant selected by a preceding PJ_DOWNCAST */
    PJ_DOWNCAST,    /* view an enum as variant `index` (type unchanged) */
    PJ_INDEX,       /* [local]  (bounds already checked) */
    PJ_CONST_INDEX, /* [index], or [len - index] if from_end */
    PJ_SUBSLICE,    /* [index .. len - to]: sub-slice / sub-array */
} ProjKind;

typedef struct Proj {
    uint8_t kind; /* ProjKind */
    bool from_end;
    uint32_t index;
    uint32_t to;
    LocalId local;
    Type *ty; /* type of the place after this projection */
} Proj;

typedef struct Place {
    LocalId local;
    Proj *projs;
    uint32_t nprojs;
    Type *ty; /* type of the whole place */
} Place;

typedef enum ConstKind {
    MC_INT,     /* lo/hi: ints, bool, char */
    MC_FLOAT,   /* fval */
    MC_STR,     /* &'static str: str/len */
    MC_BYTESTR, /* &'static [u8; N]: str/len */
    MC_ZST,     /* (), fn items, empty structs */
    MC_FN,      /* fn pointer to inst */
} ConstKind;

typedef struct MirConst {
    uint8_t kind; /* ConstKind */
    Type *ty;
    uint64_t lo, hi;
    double fval;
    const char *str;
    uint32_t len;
    Instance *inst;
} MirConst;

typedef enum OperandKind { OP_COPY, OP_MOVE, OP_CONST } OperandKind;

typedef struct Operand {
    uint8_t kind; /* OperandKind */
    Place place;
    MirConst c;
} Operand;

typedef enum CastKind {
    CAST_INT_INT,     /* ints, bool, char (source type decides extension) */
    CAST_INT_FLOAT,
    CAST_FLOAT_INT,   /* saturating */
    CAST_FLOAT_FLOAT,
    CAST_PTR_PTR,     /* thin->thin, fat->thin, fat->fat (same metadata) */
    CAST_PTR_INT,
    CAST_INT_PTR,
    CAST_UNSIZE_ARRAY, /* *[T; N] -> *[T]  (also Box) */
    CAST_UNSIZE_DYN,   /* *T -> *dyn Trait (also Box); vtable */
    CAST_FNPTR_INT,
    CAST_TRANSMUTE,    /* bit reinterpretation between same-size types */
} CastKind;

typedef enum AggKind { AGG_TUPLE, AGG_ARRAY, AGG_ADT, AGG_CLOSURE } AggKind;

typedef enum RvalueKind {
    RV_USE,         /* a */
    RV_REF,         /* &place / &mut place / &raw place */
    RV_BINARY,      /* a op b (wrapping; comparisons give bool) */
    RV_UNARY,       /* op a */
    RV_CAST,        /* a as ty, cast */
    RV_AGGREGATE,   /* agg: ops[], for AGG_ADT also variant + field_idx */
    RV_REPEAT,      /* [a; count] */
    RV_LEN,         /* length of an array or slice place */
    RV_DISCR,       /* discriminant value of an enum place, as its tag type */
    RV_GLOBAL_ADDR, /* &raw GLOBAL */
} RvalueKind;

typedef struct Rvalue {
    uint8_t kind; /* RvalueKind */
    uint8_t op;   /* BinOp / UnOp */
    uint8_t cast; /* CastKind */
    uint8_t agg;  /* AggKind */
    Type *ty;     /* result type */
    Operand a, b;
    Place place;
    Operand *ops;
    uint32_t nops;
    uint32_t *field_idx; /* AGG_ADT: field of each op (NULL = positional) */
    uint32_t variant;
    uint64_t count;
    Global *global;
    Vtable *vtable;
} Rvalue;

typedef enum StmtKindM {
    ST_ASSIGN,       /* place = rv */
    ST_CALL,         /* place = callee(args) */
    ST_DROP,         /* drop the value in `place` */
    ST_BOUNDS_CHECK, /* panic unless a (index) < b (len) */
    ST_FORGET,       /* `place` has been moved out of (no code; consumed by
                        drop elaboration) */
    ST_NOP,
} StmtKindM;

typedef struct MirStmt {
    uint8_t kind; /* StmtKindM */
    uint8_t callee; /* CalleeKind */
    bool diverges; /* ST_CALL of a `-> !` function */
    /* ST_DROP before elaboration: the place may have been moved out of
       (fully or partly); after elaboration: unconditional. */
    bool shallow;  /* ST_DROP of a Box whose contents were moved out:
                      free the allocation only */
    Span span;
    Place place;
    Rvalue rv;
    Instance *inst;
    /* ST_CALL through a fn pointer (CALLEE_INDIRECT): the pointer operand is
       rv.a. ST_BOUNDS_CHECK: panic unless rv.a (index) < rv.b (length). */
    uint32_t slot;
    Sym intrinsic;
    Substs *substs; /* CALLEE_INTRINSIC type arguments */
    Operand *args;
    uint32_t nargs;
} MirStmt;

typedef enum TermKind { TM_NONE, TM_GOTO, TM_SWITCH, TM_RETURN, TM_UNREACHABLE } TermKind;

typedef struct Terminator {
    uint8_t kind; /* TermKind */
    BlockId target;   /* TM_GOTO; TM_SWITCH: otherwise */
    Operand discr;    /* TM_SWITCH: integer / bool / char operand */
    ConstVal *values; /* TM_SWITCH */
    BlockId *targets;
    uint32_t ntargets;
} Terminator;

typedef struct MirBlock {
    MirStmt *stmts;
    uint32_t nstmts, cap;
    Terminator term;
} MirBlock;

typedef struct MirLocal {
    Type *ty;
    Sym name;          /* user variable name, or 0 for temporaries */
    bool maybe_uninit; /* may reach its scheduled drop uninitialized */
    bool is_arg;
    LocalId flag;      /* drop flag local (after elaboration), or 0 */
} MirLocal;

struct MirBody {
    Instance *inst;
    MirLocal *locals; /* [0] = return place, [1..=nargs] = arguments */
    uint32_t nlocals, locals_cap;
    uint32_t nargs;
    MirBlock *blocks; /* [0] = entry */
    uint32_t nblocks, blocks_cap;
};

/* ------------------------------------------------------------------ */
/* mono.c                                                              */
/* ------------------------------------------------------------------ */

typedef VEC(Instance *) InstanceList;
typedef VEC(Global *) GlobalList;
typedef VEC(Vtable *) VtableList;
typedef VEC(Type *) TypeList;

typedef struct Program {
    InstanceList instances; /* creation order; includes externs */
    GlobalList globals;     /* creation order */
    GlobalList init_order;  /* globals with initializers, dependencies first */
    VtableList vtables;
    TypeList drop_glue;     /* types needing a drop glue function */
    Instance *user_main; /* the Rust `fn main` */
    Instance *start;     /* std's `#[lang = "start"] fn<T>(fn() -> T) -> i32`
                            instantiated for main's return type, or NULL */
} Program;

/* Monomorphize the whole program starting from `main` in the root crate. */
Program *mono_program(DefId main_fn);

Instance *mono_fn_instance(DefId def, Substs *substs, Span sp);
Instance *mono_closure_instance(Type *closure_ty, Span sp);
Instance *mono_closure_once_shim(Type *closure_ty, Span sp);
Instance *mono_fnptr_shim(Type *closure_ty, Span sp);
Instance *mono_fn_trait_shim(Type *fn_ty, int self_mode, Span sp);
Instance *mono_box_once_shim(Type *callee_ty, Span sp);
Global *mono_static(DefId def, Span sp);
Global *mono_const_global(DefId def, Substs *substs, Span sp);
Global *mono_promoted(const TExpr *e, Body *body, Substs *substs, DefId closure, Type *ty, Span sp);
/* Storage-only global (no initializer function): a temporary borrowed by a
   static/const initializer. */
Global *mono_hidden_global(const void *key, Substs *substs, Type *ty);
Vtable *mono_vtable(Type *concrete, Type *dyn, Span sp);
/* Slot of `method` (a trait fn) in the vtable of dyn type `dyn`. */
uint32_t mono_vtable_slot(Type *dyn, DefId method, Span sp);

/* A resolved call target for `def` called with closed `substs`. */
typedef struct CalleeRes {
    enum {
        CR_INSTANCE,     /* inst (direct) */
        CR_VIRTUAL,      /* slot; args[0] is the fat self pointer */
        CR_CLOSURE,      /* Fn* method on closure type self_ty: call the
                            closure (args: self, tuple) */
        CR_FN_UNTUPLED,  /* Fn* method on a fn item / fn pointer self_ty:
                            call it with the tuple spread */
        CR_CLONE_COPY,   /* builtin Clone of a Copy-like type: *args[0] */
        CR_CTOR,         /* tuple struct / variant constructor */
        CR_INTRINSIC,    /* name */
    } kind;
    Instance *inst;
    uint32_t slot;
    Type *self_ty;
    Sym intrinsic;
    Substs *substs;
} CalleeRes;

CalleeRes mono_resolve_callee(DefId def, Substs *substs, Span sp);

/* Does dropping a value of this (closed) type do anything? */
bool ty_needs_drop(Type *t);
/* Record that cgen must emit drop glue for t (and transitively). */
void mono_require_drop_glue(Type *t, Span sp);
/* The user's `Drop::drop` instance for an ADT type, or NULL. */
Instance *mono_drop_impl(Type *t, Span sp);
/* Substitute + normalize a THIR type. */
Type *mono_ty(Type *t, Substs *substs, Span sp);
/* Signature (closed) of calling a closure body: env type by kind. */
Type *closure_env_param_ty(Type *closure_ty);

/* ------------------------------------------------------------------ */
/* mir_build.c / mir_drop.c / mir_dump.c                               */
/* ------------------------------------------------------------------ */

/* Build MIR for an instance (sets inst->mir, params, ret). */
void mir_build_instance(Instance *inst);
/* Drop elaboration (run once per body, after building). */
void mir_elaborate_drops(MirBody *mir);
void mir_dump_body(FILE *out, const MirBody *mir);
const char *mir_place_str(const MirBody *mir, const Place *p);

/* Builder helpers shared with mir_drop.c. */
LocalId mir_new_local(MirBody *mir, Type *ty, Sym name);
BlockId mir_new_block(MirBody *mir);
MirStmt *mir_push_stmt(MirBody *mir, BlockId b, StmtKindM kind, Span sp);
Place mir_place_local(MirBody *mir, LocalId l);
Place mir_place_proj(Place base, ProjKind kind, uint32_t index, Type *ty);
Operand mir_const_int(Type *ty, uint64_t lo, uint64_t hi);

/* Layout-ish helpers on closed types (mono.c). */
/* Field types of a struct / tuple / closure / enum variant. */
uint32_t ty_field_count(Type *t, uint32_t variant);
Type *ty_field_type(Type *t, uint32_t variant, uint32_t field, Span sp);
/* The integer type used for an enum's tag. */
Type *enum_tag_type(Type *t);
/* Is a pointer to this type fat ({ptr,len} or {ptr,vtable})? */
static inline bool ty_is_unsized(const Type *t) {
    return t->kind == T_SLICE || t->kind == T_STR || t->kind == T_DYN;
}
/* The pointee if t is &T, *T or Box<T>, else NULL. */
Type *ty_pointee(Type *t);

#endif
