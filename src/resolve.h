/*
 * resolve.h - name resolution: def collection, the module tree, imports,
 * and scope-chain path lookup.
 *
 * Phases
 *   1. resolve_add_crate() for every crate (std first): walks the expanded
 *      AST and creates a Def for every item, including items nested in
 *      function bodies (those live in an anonymous "block module"), generic
 *      parameters, enum variants and associated items. `use` declarations
 *      are recorded as Imports.
 *   2. Imports and globs are resolved lazily, on first lookup, and memoized.
 *   3. Paths inside signatures and bodies are resolved on demand by typeck
 *      through a Scope chain: typeck pushes SCOPE_LOCALS ribs as it walks a
 *      body; resolve.c provides the chain for an item (scope_for_def) and
 *      the lookup rules (locals, generic params, Self, block modules,
 *      module items/imports/globs, std prelude, extern crates, primitive
 *      types).
 *
 * Visibility is not checked, except that glob imports only see non-private
 * names when the importing module is not inside the glob's target module
 * (this keeps private helper names from leaking through `use foo::*`).
 */
#ifndef RCC_RESOLVE_H
#define RCC_RESOLVE_H

#include <stdio.h>

#include "types.h"

typedef enum Namespace { NS_TYPE, NS_VALUE } Namespace;

typedef enum ResKind {
    RES_NONE,    /* not found */
    RES_DEF,     /* def */
    RES_LOCAL,   /* local: the pointer given to scope_bind() */
    RES_PRIM,    /* prim: a primitive type */
    RES_SELF_TY, /* `Self`; def = the impl or trait */
} ResKind;

typedef struct Res {
    ResKind kind;
    DefId def;
    Type *prim;
    void *local;
} Res;

typedef struct Import Import;
typedef VEC(Import *) ImportList;

/* Per-module (and per-enum) name tables. */
typedef struct ModData {
    HMap items[2];       /* Sym -> DefId (stored as pointer-sized int) */
    HMap imports;        /* Sym -> Import* (chained via Import.next) */
    ImportList all_imports; /* named + anonymous (`as _`), in order */
    ImportList globs;
    DefIdList members;   /* all defs declared here, in order (for dumps) */
    bool is_block;       /* anonymous module of a block with items */
    HMap *trait_set;     /* lazily computed: traits nameable here */
    bool trait_set_busy;
} ModData;

typedef struct CrateInfo {
    Sym name;
    DefId root;
    Crate *ast;
} CrateInfo;

/* Collect all defs of a crate. The first crate added named "std" becomes the
   standard library (also reachable as `core` and `alloc`). Returns the crate
   index. */
uint32_t resolve_add_crate(Sym name, Crate *ast);
uint32_t crate_count(void);
CrateInfo *crate_get(uint32_t idx);
/* Call once after all crates were added (locates the prelude). */
void resolve_finish(void);

/* All impl blocks of all crates, in collection order. */
const DefIdList *resolve_all_impls(void);
/* All defs with a body-like initializer that typeck may want to visit
   eagerly (--check-all): fns, consts, statics. */
const DefIdList *resolve_all_bodies(void);

/* `#[lang = "name"]` items. lang_item() returns 0 if missing; lang_req()
   fatals. */
DefId lang_item(const char *name);
DefId lang_req(const char *name);

/* ---- scopes ------------------------------------------------------- */

typedef enum ScopeKind {
    SCOPE_MODULE,   /* def = module (possibly a block module) */
    SCOPE_GENERICS, /* def = item whose generic params are in scope */
    SCOPE_SELF,     /* def = impl or trait providing `Self` */
    SCOPE_LOCALS,   /* local variable bindings */
} ScopeKind;

typedef struct ScopeBinding {
    Sym name;
    void *local;
} ScopeBinding;

typedef struct Scope Scope;
struct Scope {
    Scope *parent;
    ScopeKind kind;
    DefId def;
    ScopeBinding *binds;
    uint32_t nbinds, cap;
};

/* The scope in which the signature and body of `def` are resolved. */
Scope *scope_for_def(DefId def);
Scope *scope_push(Scope *parent, ScopeKind kind, DefId def);
void scope_bind(Scope *locals, Sym name, void *local);
/* The anonymous module holding the items of block `b`, or 0. */
DefId resolve_block_module(const Block *b);
/* Innermost non-block module enclosing the scope. */
DefId scope_module(const Scope *sc);

/* ---- lookup ------------------------------------------------------- */

/* Look up a single name lexically. */
Res resolve_ident(Scope *sc, Sym name, Namespace ns);

typedef struct PathRes {
    Res res;
    uint32_t nsegs; /* number of leading path segments consumed; the rest
                       are type-relative (associated items) */
} PathRes;

/*
 * Resolve a path as far as modules and enums go. `ns` is the namespace of
 * the final segment. On failure res.kind == RES_NONE (nothing is printed).
 * Stops early (nsegs < len) at a non-module def such as a struct, trait,
 * type parameter, `Self` or primitive type: typeck resolves the remaining
 * segments as associated items.
 */
PathRes resolve_path(Scope *sc, const Path *p, Namespace ns);

/* Member lookup in a module or enum (items, imports, globs). */
Res module_lookup(DefId module, Sym name, Namespace ns);

/* Is `trait` nameable (hence its methods usable) from this scope? */
bool trait_in_scope(Scope *sc, DefId trait);

void resolve_dump(FILE *out);

#endif
