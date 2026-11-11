/*
 * mir_drop.c - drop elaboration.
 *
 * Input: MIR in which ST_DROP(local) means "drop the local if (and as far
 * as) it is still initialized here", OP_MOVE operands and ST_FORGET mark
 * moves, and ST_DROP of any other place (an assignment target, a
 * drop_in_place pointee) is unconditional.
 *
 * Output: every remaining ST_DROP is unconditional.
 *
 * Method (dynamic drop flags, no dataflow):
 *   - A *move path* is a local followed by field / enum-variant-field /
 *     Box-deref projections. Paths are created for every place that is
 *     moved out of and whose root local has a scheduled drop.
 *   - A local needs flags if something is moved out of it (wholly or
 *     partly) or it may be uninitialized at its drop (MirLocal
 *     .maybe_uninit). Each path of such a local gets a bool flag local:
 *     "this place is initialized, except for the sub-places that have
 *     their own path". Whole assignment sets the flags of the path and all
 *     its descendants; a move clears them.
 *   - ST_DROP of a flagged path becomes `if flag { drop; flag = false }`,
 *     where the drop is "open" (per field / per variant / Box contents
 *     then shallow free) when the path has sub-paths.
 *   - Peephole: a temporary assigned once and moved once later in the same
 *     block needs neither flag nor drops (this covers call arguments).
 */
#include "mir.h"

#include <stdio.h>
#include <stdlib.h>

typedef struct MPath {
    Place place;
    int parent;
    int first_child, next_sibling;
    LocalId flag; /* 0 = none (downcast nodes, unflagged) */
} MPath;

typedef struct LocalInfo {
    bool droppable;  /* has a scheduled ST_DROP */
    bool needs_flag; /* some drop cannot be decided statically, or the
                        local is moved out of in parts */
    int root;        /* path index or -1 */
    uint32_t nassign, nwhole, nsub;
    BlockId assign_blk, move_blk;
    uint32_t assign_idx, move_idx;
} LocalInfo;

typedef struct Elab {
    MirBody *mir;
    LocalInfo *li;
    uint32_t nlocals0;
    VEC(MPath) paths;
    BlockId cur;
    Span sp;
} Elab;

static bool is_box_ty(Type *t) { return t->kind == T_ADT && t->def == lang.box && lang.box; }

/* Length of the leading run of projections that form a move path; -1 if
   the place goes through a reference / pointer / index (moves out of such
   places are not tracked). */
static int movable_len(MirBody *mir, const Place *p) {
    Type *t = mir->locals[p->local].ty;
    for (uint32_t i = 0; i < p->nprojs; i++) {
        const Proj *pj = &p->projs[i];
        if (pj->kind == PJ_FIELD || pj->kind == PJ_DOWNCAST) {
            /* ok */
        } else if (pj->kind == PJ_DEREF && is_box_ty(t)) {
            /* ok */
        } else {
            return -1;
        }
        t = pj->ty;
    }
    return (int)p->nprojs;
}

static bool proj_eq(const Proj *a, const Proj *b) { return a->kind == b->kind && a->index == b->index; }

static int find_child(Elab *el, int node, const Proj *pj) {
    for (int c = el->paths.data[node].first_child; c >= 0; c = el->paths.data[c].next_sibling) {
        MPath *cp = &el->paths.data[c];
        if (proj_eq(&cp->place.projs[cp->place.nprojs - 1], pj))
            return c;
    }
    return -1;
}

static int new_path(Elab *el, Place place, int parent) {
    MPath p;
    p.place = place;
    p.parent = parent;
    p.first_child = -1;
    p.next_sibling = -1;
    p.flag = 0;
    vec_push(el->paths, p);
    int idx = (int)el->paths.len - 1;
    if (parent >= 0) {
        p.next_sibling = el->paths.data[parent].first_child;
        el->paths.data[idx].next_sibling = p.next_sibling;
        el->paths.data[parent].first_child = idx;
    }
    return idx;
}

static int root_path(Elab *el, LocalId l, bool create) {
    if (el->li[l].root < 0 && create)
        el->li[l].root = new_path(el, mir_place_local(el->mir, l), -1);
    return el->li[l].root;
}

/* The path node for exactly this place, or -1. */
static int lookup_path(Elab *el, const Place *p, bool create) {
    if (p->local >= el->nlocals0)
        return -1;
    LocalInfo *li = &el->li[p->local];
    if (!li->droppable || !li->needs_flag)
        return -1;
    if (movable_len(el->mir, p) < 0)
        return -1;
    int node = root_path(el, p->local, create);
    if (node < 0)
        return -1;
    for (uint32_t i = 0; i < p->nprojs; i++) {
        int c = find_child(el, node, &p->projs[i]);
        if (c < 0) {
            if (!create)
                return -1;
            Place sub = *p;
            sub.nprojs = i + 1;
            sub.ty = p->projs[i].ty;
            c = new_path(el, sub, node);
        }
        node = c;
    }
    return node;
}

/* ---- scanning for moves -------------------------------------------- */

typedef void (*MoveFn)(Elab *el, const Place *p, BlockId blk, uint32_t idx);

static void each_operand_move(Elab *el, const Operand *o, MoveFn fn, BlockId blk, uint32_t idx) {
    if (o->kind == OP_MOVE && ty_needs_drop(o->place.ty))
        fn(el, &o->place, blk, idx);
}

static void each_move(Elab *el, const MirStmt *s, MoveFn fn, BlockId blk, uint32_t idx) {
    switch ((StmtKindM)s->kind) {
    case ST_ASSIGN:
        each_operand_move(el, &s->rv.a, fn, blk, idx);
        each_operand_move(el, &s->rv.b, fn, blk, idx);
        for (uint32_t i = 0; i < s->rv.nops; i++)
            each_operand_move(el, &s->rv.ops[i], fn, blk, idx);
        break;
    case ST_CALL:
        each_operand_move(el, &s->rv.a, fn, blk, idx);
        for (uint32_t i = 0; i < s->nargs; i++)
            each_operand_move(el, &s->args[i], fn, blk, idx);
        break;
    case ST_FORGET:
        if (ty_needs_drop(s->place.ty))
            fn(el, &s->place, blk, idx);
        break;
    default:
        break;
    }
}

static void count_move(Elab *el, const Place *p, BlockId blk, uint32_t idx) {
    if (p->local >= el->nlocals0)
        return;
    LocalInfo *li = &el->li[p->local];
    if (movable_len(el->mir, p) < 0)
        return;
    if (p->nprojs == 0) {
        li->nwhole++;
        li->move_blk = blk;
        li->move_idx = idx;
    } else {
        li->nsub++;
    }
}

static void create_move_path(Elab *el, const Place *p, BlockId blk, uint32_t idx) {
    (void)blk;
    (void)idx;
    (void)lookup_path(el, p, true);
}

/* ---- initializedness dataflow --------------------------------------- */

/*
 * For every droppable local, two forward bit-vector analyses over the CFG:
 * "maybe initialized" (union over predecessors) and "definitely
 * initialized" (intersection). A whole-local assignment (or call result)
 * initializes; a whole move, ST_FORGET or drop de-initializes. Each
 * ST_DROP(local) is then classified:
 *     not maybe-initialized     -> the drop is dead and removed
 *     definitely initialized    -> unconditional drop
 *     otherwise                 -> the local needs dynamic drop flags
 * Locals with partial moves always use flags.
 */
enum { DROP_NA, DROP_DEAD, DROP_SURE, DROP_MAYBE };

typedef struct Flow {
    uint32_t nbits, nwords;
    int *bit;        /* local -> bit index or -1 */
    uint64_t *gen, *kill, *maybe_in, *def_in; /* nblocks * nwords */
    uint64_t *cur_maybe, *cur_def;
} Flow;

static void flow_set(Flow *f, uint64_t *maybe, uint64_t *def, LocalId l, bool init) {
    int b = f->bit[l];
    if (b < 0)
        return;
    uint64_t m = (uint64_t)1 << (b & 63);
    if (init) {
        maybe[b >> 6] |= m;
        def[b >> 6] |= m;
    } else {
        maybe[b >> 6] &= ~m;
        def[b >> 6] &= ~m;
    }
}

typedef struct FlowCb {
    Flow *f;
    uint64_t *maybe, *def;
} FlowCb;

static FlowCb flow_cb;

static void flow_move(Elab *el, const Place *p, BlockId blk, uint32_t idx) {
    (void)blk;
    (void)idx;
    if (p->nprojs == 0 && p->local < el->nlocals0)
        flow_set(flow_cb.f, flow_cb.maybe, flow_cb.def, p->local, false);
}

/* Apply the effect of statement s to (maybe, def). */
static void flow_stmt(Elab *el, Flow *f, const MirStmt *s, uint64_t *maybe, uint64_t *def) {
    flow_cb.f = f;
    flow_cb.maybe = maybe;
    flow_cb.def = def;
    each_move(el, s, flow_move, 0, 0);
    if (s->kind == ST_DROP && s->place.nprojs == 0)
        flow_set(f, maybe, def, s->place.local, false);
    if ((s->kind == ST_ASSIGN || s->kind == ST_CALL) && s->place.nprojs == 0 && !s->diverges)
        flow_set(f, maybe, def, s->place.local, true);
}

static uint32_t flow_nsucc(const Terminator *t) {
    if (t->kind == TM_GOTO)
        return 1;
    if (t->kind == TM_SWITCH)
        return t->ntargets + 1;
    return 0;
}

static BlockId flow_succ(const Terminator *t, uint32_t i) {
    if (t->kind == TM_GOTO || i == 0)
        return t->target;
    return t->targets[i - 1];
}

/* Returns per-block arrays of drop classes (parallel to the statements). */
static uint8_t **flow_analyze(Elab *el, Flow *f) {
    MirBody *mir = el->mir;
    uint32_t nb = mir->nblocks, nl = mir->nlocals;
    f->bit = malloc(sizeof(int) * (nl ? nl : 1));
    f->nbits = 0;
    for (uint32_t l = 0; l < nl; l++)
        f->bit[l] = el->li[l].droppable ? (int)f->nbits++ : -1;
    f->nwords = (f->nbits + 63) / 64;
    uint32_t nw = f->nwords ? f->nwords : 1;
    size_t total = (size_t)nb * nw;
    f->gen = calloc(total, sizeof(uint64_t));
    f->kill = calloc(total, sizeof(uint64_t));
    f->maybe_in = calloc(total, sizeof(uint64_t));
    f->def_in = malloc(total * sizeof(uint64_t));
    f->cur_maybe = calloc(nw, sizeof(uint64_t));
    f->cur_def = calloc(nw, sizeof(uint64_t));
    bool *reach = calloc(nb, sizeof(bool));
    bool *queued = calloc(nb, sizeof(bool));
    BlockId *work = malloc(sizeof(BlockId) * (nb + 1));
    if (!f->gen || !f->kill || !f->maybe_in || !f->def_in || !f->cur_maybe || !f->cur_def || !reach ||
        !queued || !work)
        fatal(el->sp, "out of memory in drop elaboration");
    memset(f->def_in, 0xff, total * sizeof(uint64_t));
    /* block summaries: the last effect on each local wins */
    uint64_t *ones = malloc(nw * sizeof(uint64_t)), *zeros = calloc(nw, sizeof(uint64_t));
    for (BlockId b = 0; b < nb; b++) {
        /* run the block on all-ones and on all-zeros: bits set from zeros
           are generated, bits cleared from ones are killed */
        memset(ones, 0xff, nw * sizeof(uint64_t));
        memset(zeros, 0, nw * sizeof(uint64_t));
        MirBlock *blk = &mir->blocks[b];
        for (uint32_t i = 0; i < blk->nstmts; i++) {
            flow_stmt(el, f, &blk->stmts[i], ones, ones);
            flow_stmt(el, f, &blk->stmts[i], zeros, zeros);
        }
        for (uint32_t w = 0; w < nw; w++) {
            f->gen[(size_t)b * nw + w] = zeros[w];
            f->kill[(size_t)b * nw + w] = ~ones[w];
        }
    }
    free(ones);
    free(zeros);
    /* entry state: arguments are initialized */
    memset(f->def_in, 0, nw * sizeof(uint64_t));
    for (uint32_t l = 0; l < nl; l++)
        if (mir->locals[l].is_arg)
            flow_set(f, f->maybe_in, f->def_in, l, true);
    /* worklist iteration */
    uint32_t head = 0, tail = 0, cap = nb + 1;
    work[tail++] = 0;
    queued[0] = true;
    reach[0] = true;
    while (head != tail) {
        BlockId b = work[head];
        head = (head + 1) % cap;
        queued[b] = false;
        const Terminator *t = &mir->blocks[b].term;
        uint32_t ns = flow_nsucc(t);
        for (uint32_t k = 0; k < ns; k++) {
            BlockId s = flow_succ(t, k);
            bool changed = !reach[s];
            reach[s] = true;
            for (uint32_t w = 0; w < nw; w++) {
                size_t bi = (size_t)b * nw + w, si = (size_t)s * nw + w;
                uint64_t mo = (f->maybe_in[bi] & ~f->kill[bi]) | f->gen[bi];
                uint64_t dout = (f->def_in[bi] & ~f->kill[bi]) | f->gen[bi];
                uint64_t nm = f->maybe_in[si] | mo, nd = f->def_in[si] & dout;
                if (nm != f->maybe_in[si] || nd != f->def_in[si]) {
                    f->maybe_in[si] = nm;
                    f->def_in[si] = nd;
                    changed = true;
                }
            }
            if (changed && !queued[s]) {
                queued[s] = true;
                work[tail] = s;
                tail = (tail + 1) % cap;
            }
        }
    }
    /* classify the drops */
    uint8_t **cls = NEW_ARRAY(uint8_t *, nb ? nb : 1);
    for (BlockId b = 0; b < nb; b++) {
        MirBlock *blk = &mir->blocks[b];
        cls[b] = NEW_ARRAY(uint8_t, blk->nstmts ? blk->nstmts : 1);
        if (!reach[b]) {
            for (uint32_t i = 0; i < blk->nstmts; i++)
                if (blk->stmts[i].kind == ST_DROP && blk->stmts[i].place.nprojs == 0)
                    cls[b][i] = DROP_DEAD;
            continue;
        }
        memcpy(f->cur_maybe, &f->maybe_in[(size_t)b * nw], nw * sizeof(uint64_t));
        memcpy(f->cur_def, &f->def_in[(size_t)b * nw], nw * sizeof(uint64_t));
        for (uint32_t i = 0; i < blk->nstmts; i++) {
            MirStmt *s = &blk->stmts[i];
            if (s->kind == ST_DROP && s->place.nprojs == 0) {
                int bit = f->bit[s->place.local];
                uint64_t m = (uint64_t)1 << (bit & 63);
                if (!(f->cur_maybe[bit >> 6] & m))
                    cls[b][i] = DROP_DEAD;
                else if (f->cur_def[bit >> 6] & m)
                    cls[b][i] = DROP_SURE;
                else {
                    cls[b][i] = DROP_MAYBE;
                    el->li[s->place.local].needs_flag = true;
                }
            }
            flow_stmt(el, f, s, f->cur_maybe, f->cur_def);
        }
    }
    free(f->bit);
    free(f->gen);
    free(f->kill);
    free(f->maybe_in);
    free(f->def_in);
    free(f->cur_maybe);
    free(f->cur_def);
    free(reach);
    free(queued);
    free(work);
    return cls;
}

/* ---- emission ------------------------------------------------------- */

static void set_flag(Elab *el, LocalId flag, bool v) {
    MirStmt *s = mir_push_stmt(el->mir, el->cur, ST_ASSIGN, el->sp);
    s->place = mir_place_local(el->mir, flag);
    memset(&s->rv, 0, sizeof s->rv);
    s->rv.kind = RV_USE;
    s->rv.ty = ty_bool();
    s->rv.a = mir_const_int(ty_bool(), v, 0);
}

static void set_flags_rec(Elab *el, int node, bool v) {
    MPath *p = &el->paths.data[node];
    if (p->flag)
        set_flag(el, p->flag, v);
    for (int c = p->first_child; c >= 0; c = el->paths.data[c].next_sibling)
        set_flags_rec(el, c, v);
}

static void push_drop(Elab *el, Place place, bool shallow) {
    MirStmt *s = mir_push_stmt(el->mir, el->cur, ST_DROP, el->sp);
    s->place = place;
    s->shallow = shallow;
    if (!shallow)
        mono_require_drop_glue(place.ty, el->sp);
    else
        mono_require_drop_glue(place.ty->substs->t[0], el->sp);
}

static void set_goto(Elab *el, BlockId target) {
    Terminator *t = &el->mir->blocks[el->cur].term;
    t->kind = TM_GOTO;
    t->target = target;
}

static void cond_drop(Elab *el, int node);

static void open_drop(Elab *el, int node) {
    MPath *p = &el->paths.data[node];
    Place place = p->place;
    Type *t = place.ty;
    if (p->first_child < 0) {
        push_drop(el, place, false);
        return;
    }
    if (is_box_ty(t)) {
        cond_drop(el, el->paths.data[node].first_child);
        push_drop(el, place, true);
        return;
    }
    int first = p->first_child;
    if (el->paths.data[first].place.projs[el->paths.data[first].place.nprojs - 1].kind == PJ_DOWNCAST) {
        /* enum: dispatch on the discriminant */
        Type *tag = enum_tag_type(t);
        LocalId d = mir_new_local(el->mir, tag, 0);
        MirStmt *s = mir_push_stmt(el->mir, el->cur, ST_ASSIGN, el->sp);
        s->place = mir_place_local(el->mir, d);
        memset(&s->rv, 0, sizeof s->rv);
        s->rv.kind = RV_DISCR;
        s->rv.ty = tag;
        s->rv.place = place;
        uint32_t n = 0;
        for (int c = first; c >= 0; c = el->paths.data[c].next_sibling)
            n++;
        BlockId join = mir_new_block(el->mir), other = mir_new_block(el->mir);
        Terminator sw;
        memset(&sw, 0, sizeof sw);
        sw.kind = TM_SWITCH;
        sw.discr.kind = OP_COPY;
        sw.discr.place = mir_place_local(el->mir, d);
        sw.values = NEW_ARRAY(ConstVal, n);
        sw.targets = NEW_ARRAY(BlockId, n);
        sw.ntargets = n;
        sw.target = other;
        AdtDef *adt = adt_def(t->def);
        adt_compute_discs(adt);
        BlockId swblk = el->cur;
        uint32_t k = 0;
        for (int c = first; c >= 0; c = el->paths.data[c].next_sibling, k++) {
            Place vp = el->paths.data[c].place;
            uint32_t variant = vp.projs[vp.nprojs - 1].index;
            int64_t disc = adt->variants[variant].disc;
            ConstVal cv = {(uint64_t)disc, disc < 0 ? ~0ull : 0};
            sw.values[k] = const_wrap(cv, tag);
            BlockId vb = mir_new_block(el->mir);
            sw.targets[k] = vb;
            el->cur = vb;
            uint32_t nf = ty_field_count(t, variant);
            for (uint32_t f = 0; f < nf; f++) {
                Type *fty = ty_field_type(t, variant, f, el->sp);
                Proj pj;
                memset(&pj, 0, sizeof pj);
                pj.kind = PJ_FIELD;
                pj.index = f;
                int fc = find_child(el, c, &pj);
                if (fc >= 0)
                    cond_drop(el, fc);
                else if (ty_needs_drop(fty))
                    push_drop(el, mir_place_proj(vp, PJ_FIELD, f, fty), false);
            }
            set_goto(el, join);
        }
        el->mir->blocks[swblk].term = sw;
        el->cur = other;
        push_drop(el, place, false);
        set_goto(el, join);
        el->cur = join;
        return;
    }
    /* struct / tuple / closure: field by field */
    uint32_t nf = ty_field_count(t, 0);
    for (uint32_t f = 0; f < nf; f++) {
        Type *fty = ty_field_type(t, 0, f, el->sp);
        if (t->kind == T_CLOSURE && def_get(t->def)->closure->captures[f].mode != CAP_VALUE)
            continue;
        Proj pj;
        memset(&pj, 0, sizeof pj);
        pj.kind = PJ_FIELD;
        pj.index = f;
        int fc = find_child(el, node, &pj);
        if (fc >= 0)
            cond_drop(el, fc);
        else if (ty_needs_drop(fty))
            push_drop(el, mir_place_proj(place, PJ_FIELD, f, fty), false);
    }
}

/* if flag(node) { drop(node); clear flags } */
static void cond_drop(Elab *el, int node) {
    MPath *p = &el->paths.data[node];
    if (!p->flag) {
        open_drop(el, node);
        return;
    }
    BlockId dropb = mir_new_block(el->mir), cont = mir_new_block(el->mir);
    Terminator *t = &el->mir->blocks[el->cur].term;
    memset(t, 0, sizeof *t);
    t->kind = TM_SWITCH;
    t->discr.kind = OP_COPY;
    t->discr.place = mir_place_local(el->mir, el->paths.data[node].flag);
    t->values = NEW_ARRAY(ConstVal, 1);
    t->targets = NEW_ARRAY(BlockId, 1);
    t->values[0].lo = 0;
    t->values[0].hi = 0;
    t->targets[0] = cont;
    t->ntargets = 1;
    t->target = dropb;
    el->cur = dropb;
    open_drop(el, node);
    set_flags_rec(el, node, false);
    set_goto(el, cont);
    el->cur = cont;
}

static void clear_move_flags(Elab *el, const Place *p, BlockId blk, uint32_t idx) {
    (void)blk;
    (void)idx;
    int node = lookup_path(el, p, false);
    if (node >= 0)
        set_flags_rec(el, node, false);
}

void mir_elaborate_drops(MirBody *mir) {
    Elab el;
    memset(&el, 0, sizeof el);
    el.mir = mir;
    el.nlocals0 = mir->nlocals;
    el.li = NEW_ARRAY(LocalInfo, mir->nlocals);
    for (uint32_t i = 0; i < mir->nlocals; i++)
        el.li[i].root = -1;
    uint32_t nblocks0 = mir->nblocks;

    /* Pass A: which locals have scheduled drops; assignment/move counts. */
    bool any_drop = false;
    for (BlockId b = 0; b < nblocks0; b++) {
        MirBlock *blk = &mir->blocks[b];
        for (uint32_t i = 0; i < blk->nstmts; i++) {
            MirStmt *s = &blk->stmts[i];
            if (s->kind == ST_DROP) {
                any_drop = true;
                if (s->place.nprojs == 0)
                    el.li[s->place.local].droppable = true;
            }
            if ((s->kind == ST_ASSIGN || s->kind == ST_CALL) && s->place.nprojs == 0) {
                LocalInfo *li = &el.li[s->place.local];
                li->nassign++;
                li->assign_blk = b;
                li->assign_idx = i;
            }
            each_move(&el, s, count_move, b, i);
        }
    }
    if (!any_drop) {
        for (BlockId b = 0; b < nblocks0; b++)
            for (uint32_t i = 0; i < mir->blocks[b].nstmts; i++)
                if (mir->blocks[b].stmts[i].kind == ST_FORGET)
                    mir->blocks[b].stmts[i].kind = ST_NOP;
        return;
    }
    for (uint32_t l = 0; l < el.nlocals0; l++)
        if (el.li[l].droppable && el.li[l].nsub)
            el.li[l].needs_flag = true;
    Flow flow;
    uint8_t **cls = flow_analyze(&el, &flow);
    /* Pass B: move paths. */
    for (BlockId b = 0; b < nblocks0; b++)
        for (uint32_t i = 0; i < mir->blocks[b].nstmts; i++)
            each_move(&el, &mir->blocks[b].stmts[i], create_move_path, b, i);
    /* Pass C: flags. */
    for (uint32_t l = 0; l < el.nlocals0; l++) {
        LocalInfo *li = &el.li[l];
        if (li->droppable && li->needs_flag)
            (void)root_path(&el, l, true);
    }
    for (uint32_t i = 0; i < el.paths.len; i++) {
        MPath *p = &el.paths.data[i];
        if (p->place.nprojs && p->place.projs[p->place.nprojs - 1].kind == PJ_DOWNCAST)
            continue;
        LocalId f = mir_new_local(mir, ty_bool(), 0);
        el.paths.data[i].flag = f;
        if (p->place.nprojs == 0)
            mir->locals[p->place.local].flag = f;
    }

    /* Pass D: rewrite every original block. */
    for (BlockId b = 0; b < nblocks0; b++) {
        MirStmt *old = mir->blocks[b].stmts;
        uint32_t n = mir->blocks[b].nstmts;
        Terminator old_term = mir->blocks[b].term;
        mir->blocks[b].stmts = NULL;
        mir->blocks[b].nstmts = 0;
        mir->blocks[b].cap = 0;
        memset(&mir->blocks[b].term, 0, sizeof(Terminator));
        el.cur = b;
        if (b == 0) {
            /* flags start out false; arguments are initialized */
            for (uint32_t i = 0; i < el.paths.len; i++)
                if (el.paths.data[i].flag) {
                    LocalId root = el.paths.data[i].place.local;
                    set_flag(&el, el.paths.data[i].flag, mir->locals[root].is_arg);
                }
        }
        for (uint32_t i = 0; i < n; i++) {
            MirStmt s = old[i];
            el.sp = s.span;
            each_move(&el, &s, clear_move_flags, b, i);
            if (s.kind == ST_FORGET)
                continue;
            if (s.kind == ST_DROP) {
                if (s.place.nprojs == 0 && s.place.local < el.nlocals0 &&
                    !el.li[s.place.local].needs_flag) {
                    if (cls[b][i] == DROP_SURE)
                        push_drop(&el, s.place, false);
                    continue;
                }
                int node = lookup_path(&el, &s.place, false);
                if (node >= 0) {
                    cond_drop(&el, node);
                } else {
                    push_drop(&el, s.place, false);
                }
                continue;
            }
            MirStmt *ns = mir_push_stmt(mir, el.cur, (StmtKindM)s.kind, s.span);
            *ns = s;
            if (s.kind == ST_ASSIGN || s.kind == ST_CALL) {
                int node = lookup_path(&el, &s.place, false);
                if (node >= 0 && !s.diverges)
                    set_flags_rec(&el, node, true);
            }
        }
        mir->blocks[el.cur].term = old_term;
    }
}
