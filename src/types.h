/*
 * types.h - the semantic core shared by resolve, typeck, mono, MIR and cgen:
 *
 *   - HMap: a hash map keyed by one or two 64-bit integers.
 *   - Def / DefId: one entry per named program entity (module, struct, fn,
 *     impl, generic parameter, closure, ...). The Def table is global and
 *     spans all crates compiled together.
 *   - Type / Substs: hash-consed (interned) semantic types. Two types are
 *     equal iff their pointers are equal. Inference variables (T_INFER) are
 *     the one exception: each is a unique node.
 *
 * Lifetimes are erased everywhere: `&'a T` is just `&T`, generic lifetime
 * parameters do not occupy substitution slots.
 */
#ifndef RCC_TYPES_H
#define RCC_TYPES_H

#include "ast.h"
#include "util.h"

/* ------------------------------------------------------------------ */
/* Hash map                                                            */
/* ------------------------------------------------------------------ */

/*
 * Open-addressing map from a key pair (a, b) to a pointer. `a` must be
 * nonzero (Syms, DefIds, NodeIds, pointers and type ids all are). A zeroed
 * HMap is empty. Iteration order is unspecified: never iterate a map to
 * produce output; keep a VEC alongside when order matters.
 */
typedef struct HMapEntry {
    uint64_t a, b;
    void *val;
} HMapEntry;

typedef struct HMap {
    HMapEntry *ents;
    uint32_t cap, len;
} HMap;

void *hmap_get2(const HMap *m, uint64_t a, uint64_t b);
void hmap_put2(HMap *m, uint64_t a, uint64_t b, void *val);
static inline void *hmap_get(const HMap *m, uint64_t a) { return hmap_get2(m, a, 0); }
static inline void hmap_put(HMap *m, uint64_t a, void *val) { hmap_put2(m, a, 0, val); }
#define PTRKEY(p) ((uint64_t)(uintptr_t)(p))

/* A growable string buffer (arena backed). */
typedef struct StrBuf {
    char *data;
    uint32_t len, cap;
} StrBuf;
void sb_putc(StrBuf *sb, char c);
void sb_puts(StrBuf *sb, const char *s);
void sb_printf(StrBuf *sb, const char *fmt, ...);
/* NUL-terminated contents (valid until the next append). */
const char *sb_str(StrBuf *sb);

/* ------------------------------------------------------------------ */
/* Defs                                                                */
/* ------------------------------------------------------------------ */

typedef uint32_t DefId; /* 0 = none */

typedef enum DefKind {
    DEF_NONE,
    DEF_MOD,        /* module, crate root, or anonymous block module */
    DEF_STRUCT,
    DEF_UNION,
    DEF_ENUM,
    DEF_VARIANT,    /* parent = enum */
    DEF_TRAIT,
    DEF_TYPE_ALIAS, /* module-level `type X = ..` */
    DEF_ASSOC_TY,   /* `type X` inside a trait or impl */
    DEF_FN,         /* free fn, method, assoc fn, foreign fn */
    DEF_CONST,      /* module-level or fn-local const */
    DEF_ASSOC_CONST,
    DEF_STATIC,
    DEF_IMPL,
    DEF_TY_PARAM,   /* generic type parameter (also synthetic `impl Trait`) */
    DEF_CONST_PARAM,
    DEF_CLOSURE,    /* created by typeck, parent = enclosing fn-like def */
} DefKind;

typedef struct Type Type;
typedef struct Substs Substs;

typedef VEC(DefId) DefIdList;

typedef struct Def {
    DefKind kind;
    Sym name;
    DefId parent;  /* lexical parent: module, trait, impl, enum, or fn */
    DefId module;  /* innermost enclosing DEF_MOD (itself for modules) */
    uint32_t crate_; /* index into the crate table */
    Span span;
    /* AST backing (whichever applies) */
    Item *item;           /* items, incl. assoc items and foreign items */
    Variant *variant;     /* DEF_VARIANT */
    GenericParam *gparam; /* DEF_TY_PARAM / DEF_CONST_PARAM */
    Expr *closure_expr;   /* DEF_CLOSURE */
    uint32_t index;       /* DEF_VARIANT: index in enum; params: own index */
    DefIdList gparams;    /* type/const generic parameter defs, in order */
    DefIdList children;   /* DEF_ENUM: variants; DEF_TRAIT/DEF_IMPL: assoc
                             items; in declaration order */
    bool is_foreign;      /* declared in an `extern` block */
    Sym abi;              /* foreign items: ABI string ("C", "rcc-intrinsic") */
    Sym lang;             /* #[lang = "..."] name, 0 if none */
    struct ModData *mod;  /* DEF_MOD and DEF_ENUM (variants as members) */
    /* Lazily computed by typeck (see typeck.h); NULL until queried. */
    struct GenericsDef *generics;
    struct PredList *preds;
    Type *type_of;
    struct FnSigT *sig;
    struct AdtDef *adt;
    struct TraitDef *trait;
    struct ImplDef *impl;
    struct Body *body;       /* typed body (thir.h) */
    struct ClosureInfo *closure;
    struct OpaqueInfo *opaques; /* fns returning `impl Trait` */
    uint8_t query_state;     /* cycle detection bits for lazy queries */
} Def;

Def *def_get(DefId id);
DefId def_new(DefKind kind, Sym name, DefId parent, Span sp);
uint32_t def_count(void);
/* `crate::a::b::name` for diagnostics, dumps and symbol names. */
const char *def_path_str(DefId id);
const char *def_kind_str(DefKind k);

/* Attribute helpers. */
bool attrs_have(const AttrList *attrs, const char *name);
/* For `#[name = "value"]`: the string value, or NULL. */
const char *attrs_str_value(const AttrList *attrs, const char *name);

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

typedef enum IntKind {
    IK_I8, IK_I16, IK_I32, IK_I64, IK_I128, IK_ISIZE,
    IK_U8, IK_U16, IK_U32, IK_U64, IK_U128, IK_USIZE,
    IK_COUNT
} IntKind;

typedef enum FloatKind { FK_F32, FK_F64 } FloatKind;

static inline bool ik_signed(IntKind k) { return k <= IK_ISIZE; }
/* Width in bits (isize/usize are 64). */
uint32_t ik_bits(IntKind k);
const char *ik_name(IntKind k);

typedef enum TypeKind {
    T_BOOL,
    T_CHAR,
    T_INT,     /* ik */
    T_FLOAT,   /* ik (FloatKind) */
    T_STR,     /* unsized str */
    T_NEVER,   /* ! */
    T_TUPLE,   /* substs = element types; () is the empty tuple */
    T_ADT,     /* def (struct/enum/union), substs */
    T_REF,     /* elem, is_mut */
    T_PTR,     /* elem, is_mut */
    T_SLICE,   /* elem (unsized [T]) */
    T_ARRAY,   /* elem, len (T_CONST, const T_PARAM or T_INFER) */
    T_FNPTR,   /* substs = param types followed by the return type */
    T_FNDEF,   /* zero-sized type of a fn item / tuple ctor: def, substs */
    T_CLOSURE, /* def (DEF_CLOSURE), substs = enclosing item's substs */
    T_DYN,     /* def = principal trait, substs = its args WITHOUT Self,
                  binds = associated type bindings */
    T_PARAM,   /* index into the substs of the item in scope, name */
    T_INFER,   /* index = inference variable id, ik = InferKind */
    T_PROJ,    /* <substs[0] as Trait<substs[1..]>>::def (def = DEF_ASSOC_TY
                  in the trait) */
    T_OPAQUE,  /* return-position `impl Trait`: def = fn, index = ordinal,
                  substs = fn's substs. Revealed at mono time. */
    T_CONST,   /* const generic value: cval */
    T_ERROR,
} TypeKind;

typedef enum InferKind { INFER_TY, INFER_INT, INFER_FLOAT } InferKind;

/* Type flags (computed bottom-up at interning). */
enum {
    TYF_HAS_INFER = 1, /* contains T_INFER */
    TYF_HAS_PARAM = 2, /* contains T_PARAM */
    TYF_HAS_PROJ = 4,  /* contains T_PROJ or T_OPAQUE (needs normalizing) */
};

typedef struct DynBinding {
    DefId assoc; /* DEF_ASSOC_TY in the trait (or a supertrait) */
    Type *ty;
} DynBinding;

struct Type {
    uint8_t kind;  /* TypeKind */
    uint8_t flags; /* TYF_HAS_* */
    uint8_t ik;    /* IntKind / FloatKind / InferKind */
    bool is_mut;
    uint32_t id;   /* unique, dense, deterministic (creation order) */
    DefId def;
    uint32_t index;
    Sym name;
    Type *elem;
    Type *len;
    Substs *substs;
    uint64_t cval;
    DynBinding *binds;
    uint32_t nbinds;
};

struct Substs {
    uint32_t len;
    uint32_t id; /* unique, nonzero */
    uint8_t flags;
    Type *t[];
};

void types_init(void);

Type *ty_bool(void);
Type *ty_char(void);
Type *ty_str(void);
Type *ty_never(void);
Type *ty_unit(void);
Type *ty_error(void);
Type *ty_int(IntKind k);
Type *ty_float(FloatKind k);
static inline Type *ty_usize(void) { return ty_int(IK_USIZE); }
Type *ty_tuple(Substs *elems);
Type *ty_tuple2(Type *a, Type *b);
Type *ty_adt(DefId def, Substs *substs);
Type *ty_ref(Type *elem, bool is_mut);
Type *ty_ptr(Type *elem, bool is_mut);
Type *ty_slice(Type *elem);
Type *ty_array(Type *elem, Type *len);
Type *ty_fnptr(Substs *params_and_ret);
Type *ty_fndef(DefId def, Substs *substs);
Type *ty_closure(DefId def, Substs *substs);
Type *ty_dyn(DefId trait, Substs *args, DynBinding *binds, uint32_t nbinds);
Type *ty_param(uint32_t index, Sym name);
Type *ty_proj(DefId assoc, Substs *trait_substs);
Type *ty_opaque(DefId fn, uint32_t ordinal, Substs *substs);
Type *ty_const(uint64_t v);
/* Fresh, un-interned inference variable node. */
Type *ty_infer_node(uint32_t var, InferKind k);

Substs *substs_new(Type **t, uint32_t n);
Substs *substs_empty(void);
Substs *substs_1(Type *a);
Substs *substs_2(Type *a, Type *b);
/* a ++ b */
Substs *substs_concat(Substs *a, Substs *b);
/* s[from .. to] */
Substs *substs_slice(Substs *s, uint32_t from, uint32_t to);

static inline bool ty_is_unit(const Type *t) {
    return t->kind == T_TUPLE && t->substs->len == 0;
}
static inline bool ty_is_integral(const Type *t) { return t->kind == T_INT; }
static inline bool ty_is_numeric(const Type *t) {
    return t->kind == T_INT || t->kind == T_FLOAT;
}
/* Number of parameters of a T_FNPTR. */
static inline uint32_t fnptr_nparams(const Type *t) { return t->substs->len - 1; }
static inline Type *fnptr_ret(const Type *t) { return t->substs->t[t->substs->len - 1]; }

/*
 * Type folding. `fold` is called on every type node whose flags intersect
 * `mask` (others are returned unchanged); it normally handles the leaves
 * it cares about and delegates the rest to ty_super_fold(), which rebuilds
 * the node from folded children.
 */
typedef struct TypeFolder TypeFolder;
struct TypeFolder {
    Type *(*fold)(TypeFolder *f, Type *t);
    uint8_t mask;
    void *ctx;
    Substs *substs; /* for ty_subst */
};
Type *ty_fold(TypeFolder *f, Type *t);
Type *ty_super_fold(TypeFolder *f, Type *t);
Substs *substs_fold(TypeFolder *f, Substs *s);

/* Replace every T_PARAM by s->t[index]. Fatal if index is out of range. */
Type *ty_subst(Type *t, Substs *s);
Substs *substs_subst(Substs *x, Substs *s);

/* Rust-like rendering for diagnostics, dumps and C comments. */
const char *ty_to_str(Type *t);
const char *substs_to_str(Substs *s); /* "<A, B>" or "" */
void ty_print(StrBuf *sb, Type *t);

#endif
