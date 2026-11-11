/*
 * rt.c - the C runtime linked into every program compiled by rcc.
 *
 * std (lib/std) reaches the operating system only through the `rcc_*`
 * functions defined here (declared `extern "C"` on the Rust side), so the
 * emitted C never needs a system header.
 */
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef unsigned long usize;

static int rcc_argc;
static char **rcc_argv;

void rcc_rt_init(int argc, char **argv) {
    rcc_argc = argc;
    rcc_argv = argv;
}

_Noreturn void rcc_abort(void) {
    fflush(stdout);
    abort();
}

_Noreturn void rcc_exit(int code) {
    fflush(stdout);
    exit(code);
}

/* Print a panic message (already formatted by std) and exit with 101. */
_Noreturn void rcc_panic_exit(const uint8_t *msg, usize len) {
    fflush(stdout);
    fwrite(msg, 1, len, stderr);
    fputc('\n', stderr);
    exit(101);
}

/* On a spawned thread, hands a runtime panic to std (rcc_thr_ block below):
   does not return there; returns on the main thread. */
void rcc_thr_c_panic(const char *msg, usize len, int is_static);

_Noreturn void rcc_panic_cstr(const char *msg) {
    rcc_thr_c_panic(msg, strlen(msg), 1);
    fflush(stdout);
    fprintf(stderr, "thread 'main' panicked:\n%s\n", msg);
    exit(101);
}

_Noreturn void rcc_panic_bounds(usize index, usize len) {
    char buf[128];
    int n = snprintf(buf, sizeof buf, "index out of bounds: the len is %lu but the index is %lu", len, index);
    rcc_thr_c_panic(buf, n < 0 ? 0 : (usize)n, 0);
    fflush(stdout);
    fprintf(stderr, "thread 'main' panicked:\nindex out of bounds: the len is %lu but the index is %lu\n",
            len, index);
    exit(101);
}

_Noreturn void rcc_unreachable(void) {
    fflush(stdout);
    fprintf(stderr, "rcc: internal error: entered unreachable code\n");
    abort();
}

/* ---- allocation ---- */

uint8_t *rcc_alloc(usize size, usize align) {
    void *p;
    if (size == 0)
        size = 1;
    if (align <= 16) {
        p = malloc(size);
    } else {
        if (posix_memalign(&p, align, size) != 0)
            p = NULL;
    }
    if (!p) {
        fprintf(stderr, "memory allocation of %lu bytes failed\n", size);
        abort();
    }
    return p;
}

uint8_t *rcc_realloc(uint8_t *p, usize new_size) {
    if (new_size == 0)
        new_size = 1;
    void *n = realloc(p, new_size);
    if (!n) {
        fprintf(stderr, "memory allocation of %lu bytes failed\n", new_size);
        abort();
    }
    return n;
}

void rcc_dealloc(uint8_t *p) { free(p); }
void rcc_box_free(void *p) { free(p); }

/* ---- basic I/O ---- */

/* Write all bytes to fd 1 or 2; returns 0 or errno. */
int rcc_write(int fd, const uint8_t *buf, usize len) {
    if (fd == 1) {
        if (fwrite(buf, 1, len, stdout) != len)
            return errno ? errno : EIO;
        return 0;
    }
    fflush(stdout);
    while (len) {
        ssize_t n = write(fd, buf, len);
        if (n < 0) {
            if (errno == EINTR)
                continue;
            return errno;
        }
        buf += n;
        len -= (usize)n;
    }
    return 0;
}

void rcc_flush_stdout(void) { fflush(stdout); }

/* ---- floats ---- */

/* Shortest decimal digits that round-trip `x` (x finite, > 0). Writes the
   digits (no dot) to buf and returns their count; *exp10 is the decimal
   exponent such that value = 0.d1d2d3... * 10^exp10. */
int rcc_f64_shortest(double x, uint8_t *buf, int *exp10, int is_f32) {
    char tmp[40];
    int prec;
    int maxp = is_f32 ? 9 : 17;
    for (prec = 1; prec <= maxp; prec++) {
        snprintf(tmp, sizeof tmp, "%.*e", prec - 1, x);
        if (is_f32 ? (strtof(tmp, NULL) == (float)x) : (strtod(tmp, NULL) == x))
            break;
    }
    if (prec <= maxp) {
        /* printf rounds exact ties to even; the shortest-digits algorithm
           Rust uses rounds them up. Detect an exact tie and bump the last
           digit if the result still round-trips. */
        char wide[80];
        snprintf(wide, sizeof wide, "%.*e", prec - 1 + 40, x);
        char *we = strchr(wide, 'e');
        /* digits after position prec (skipping the '.') */
        const char *rest = wide + (prec == 1 ? 2 : prec + 1);
        int tie = rest < we && rest[0] == '5';
        for (const char *q = rest + 1; tie && q < we; q++)
            if (*q != '0')
                tie = 0;
        if (tie) {
            char up[40];
            memcpy(up, tmp, sizeof up);
            char *ue = strchr(up, 'e');
            char *d = ue - 1;
            char kept = rest[-1] == '.' ? rest[-2] : rest[-1];
            if (*d != '.' && *d == kept && *d != '9') {
                (*d)++;
                if (is_f32 ? (strtof(up, NULL) == (float)x) : (strtod(up, NULL) == x))
                    memcpy(tmp, up, sizeof up);
            }
        }
    }
    /* tmp = d.ddddde[+-]XX */
    int n = 0;
    char *p = tmp;
    while (*p && *p != 'e') {
        if (*p != '.')
            buf[n++] = (uint8_t)*p;
        p++;
    }
    int e = atoi(p + 1);
    while (n > 1 && buf[n - 1] == '0')
        n--;
    *exp10 = e + 1;
    return n;
}

/* Fixed-precision formatting ("%.*f"); returns the length written. */
int rcc_f64_fixed(double x, int precision, uint8_t *buf, usize cap) {
    int n = snprintf((char *)buf, cap, "%.*f", precision, x);
    return n < 0 ? 0 : (n >= (int)cap ? (int)cap - 1 : n);
}

/* Exponent formatting ("%.*e" with given digits, or shortest if precision < 0)
   is composed on the Rust side from rcc_f64_shortest. */

double rcc_strtod(const uint8_t *s, usize len, int *ok) {
    char stack[128];
    char *buf = len < sizeof stack ? stack : malloc(len + 1);
    memcpy(buf, s, len);
    buf[len] = 0;
    char *end;
    double v = strtod(buf, &end);
    *ok = (end == buf + len) && len > 0;
    if (buf != stack)
        free(buf);
    return v;
}

float rcc_strtof(const uint8_t *s, usize len, int *ok) {
    char stack[128];
    char *buf = len < sizeof stack ? stack : malloc(len + 1);
    memcpy(buf, s, len);
    buf[len] = 0;
    char *end;
    float v = strtof(buf, &end);
    *ok = (end == buf + len) && len > 0;
    if (buf != stack)
        free(buf);
    return v;
}

/* ---- process ---- */

usize rcc_args_count(void) { return (usize)rcc_argc; }
const uint8_t *rcc_args_get(usize i, usize *len) {
    *len = strlen(rcc_argv[i]);
    return (const uint8_t *)rcc_argv[i];
}

/* "%.*e"-style formatting with Rust's exponent syntax (e-7, e21: no
   padding, no plus sign). Returns the length written. */
int rcc_f64_exp(double x, int precision, int upper, uint8_t *buf, usize cap) {
    char tmp[512];
    snprintf(tmp, sizeof tmp, "%.*e", precision, x);
    char *e = strchr(tmp, 'e');
    usize n = 0;
    if (!e)
        return 0;
    for (char *p = tmp; p < e && n + 1 < cap; p++)
        buf[n++] = (uint8_t)*p;
    if (n + 1 < cap)
        buf[n++] = upper ? 'E' : 'e';
    int ex = atoi(e + 1);
    char eb[16];
    snprintf(eb, sizeof eb, "%d", ex);
    for (char *p = eb; *p && n + 1 < cap; p++)
        buf[n++] = (uint8_t)*p;
    return (int)n;
}

/* ===================================================================== */
/* rcc_thr_*: threads, thread-locals, atomics and panic capture for       */
/* std::thread and std::sync (lib/std/src/thread, lib/std/src/sync).      */
/* Callbacks from here into Rust are plain Rust fn pointers whose C type  */
/* is int32_t (*)(uint8_t *) (or the panic-hook signature below).         */
/* ===================================================================== */

#include <limits.h>
#include <pthread.h>
#include <sched.h>
#include <setjmp.h>
#include <time.h>
#ifndef PTHREAD_STACK_MIN
#define PTHREAD_STACK_MIN 16384
#endif

typedef int32_t (*rcc_thr_fn)(uint8_t *);
typedef int32_t (*rcc_thr_panic_fn)(const uint8_t *, usize, int32_t);

/* One per thread spawned through std::thread. The main thread has none. */
typedef struct RccThread {
    pthread_t tid;
    rcc_thr_fn body, on_panic;
    uint8_t *arg;
    int refs; /* the thread and its JoinHandle; the last one frees */
    int done; /* the panic landing pad is gone (TLS destructors run) */
    jmp_buf jb;
} RccThread;

static rcc_thr_panic_fn rcc_thr_c_panic_hook;

/* ---- thread-locals: per-thread table keyed by the LocalKey's address ---- */

typedef struct {
    const void *key;
    uint8_t *val;
    rcc_thr_fn dtor;
} RccTlsEnt;

typedef struct {
    RccTlsEnt *ent; /* insertion order (destructors run in reverse) */
    usize n, cap;
    usize *idx; /* open addressing: entry index + 1, 0 = empty */
    usize icap;
} RccTls;

/* Everything thread-local in one _Thread_local object. ccc workaround
   (tests/core/ccc_bugs/tlv_call_lost_arg.c): ccc can drop call arguments
   that are live across a thread-local access, so the object is reached
   only through a call of a volatile function pointer (never inlined),
   always before anything else in the function. */
typedef struct {
    RccThread *self; /* NULL on the main thread */
    RccTls tls;
} RccThrLocal;

static _Thread_local RccThrLocal rcc_thr_local_obj;
static RccThrLocal *rcc_thr_local_get(void) { return &rcc_thr_local_obj; }
static RccThrLocal *(*volatile rcc_thr_local)(void) = rcc_thr_local_get;

static usize rcc_thr_hash(const void *k) {
    uint64_t x = (uint64_t)(uintptr_t)k;
    x ^= x >> 29;
    x *= 0x9E3779B97F4A7C15ull;
    return (usize)(x >> 32);
}

static RccTlsEnt *rcc_thr_tls_find(RccTls *t, const void *key) {
    if (!t->icap)
        return NULL;
    usize m = t->icap - 1, h = rcc_thr_hash(key) & m;
    for (;;) {
        usize e = t->idx[h];
        if (!e)
            return NULL;
        if (t->ent[e - 1].key == key)
            return &t->ent[e - 1];
        h = (h + 1) & m;
    }
}

/* The calling thread's value for `key`, or NULL if not initialised yet. */
uint8_t *rcc_thr_tls_get(const void *key) {
    RccTls *t = &rcc_thr_local()->tls;
    RccTlsEnt *e = rcc_thr_tls_find(t, key);
    return e ? e->val : NULL;
}

void rcc_thr_tls_set(const void *key, uint8_t *val, rcc_thr_fn dtor) {
    RccTls *t = &rcc_thr_local()->tls;
    RccTlsEnt *e = rcc_thr_tls_find(t, key);
    if (e) {
        e->val = val;
        e->dtor = dtor;
        return;
    }
    if (t->n == t->cap) {
        t->cap = t->cap ? t->cap * 2 : 8;
        t->ent = realloc(t->ent, t->cap * sizeof *t->ent);
        if (!t->ent)
            abort();
    }
    t->ent[t->n].key = key;
    t->ent[t->n].val = val;
    t->ent[t->n].dtor = dtor;
    t->n++;
    if (t->n * 2 > t->icap) {
        usize nc = t->icap ? t->icap * 2 : 16;
        free(t->idx);
        t->idx = calloc(nc, sizeof *t->idx);
        if (!t->idx)
            abort();
        t->icap = nc;
        for (usize i = 0; i < t->n; i++) {
            usize h = rcc_thr_hash(t->ent[i].key) & (nc - 1);
            while (t->idx[h])
                h = (h + 1) & (nc - 1);
            t->idx[h] = i + 1;
        }
    } else {
        usize h = rcc_thr_hash(key) & (t->icap - 1);
        while (t->idx[h])
            h = (h + 1) & (t->icap - 1);
        t->idx[h] = t->n;
    }
}

/* Run the destructors of this thread's thread-locals (LIFO). A destructor
   that touches a destroyed key re-initialises it; that value is destroyed
   by a later pass. */
static void rcc_thr_tls_destroy(void) {
    RccTls *t = &rcc_thr_local()->tls;
    int again = 1;
    for (int pass = 0; again && pass < 8; pass++) {
        again = 0;
        for (usize i = t->n; i-- > 0;) {
            uint8_t *v = t->ent[i].val;
            if (v && t->ent[i].dtor) {
                t->ent[i].val = NULL;
                t->ent[i].dtor(v);
                again = 1;
            }
        }
    }
    free(t->ent);
    free(t->idx);
    memset(t, 0, sizeof *t);
}

/* ---- threads ---- */

static void rcc_thr_release(RccThread *t) {
    if (__atomic_sub_fetch(&t->refs, 1, __ATOMIC_ACQ_REL) == 0)
        free(t);
}

static void *rcc_thr_start(void *p) {
    RccThrLocal *l = rcc_thr_local();
    l->self = (RccThread *)p;
    /* after setjmp returns again, only re-read memory (no locals) */
    if (setjmp(l->self->jb) == 0)
        l->self->body(l->self->arg);
    else
        rcc_thr_local()->self->on_panic(rcc_thr_local()->self->arg); /* after a panic, see rcc_thr_unwind */
    RccThread *t = rcc_thr_local()->self;
    t->done = 1;
    rcc_thr_tls_destroy();
    rcc_thr_release(t);
    return NULL;
}

/* Start a thread running body(arg); on a panic in it, on_panic(arg) runs
   instead on the same thread. stack = 0 means the default. Returns a
   handle for rcc_thr_join/rcc_thr_detach, or NULL with *err = errno. */
void *rcc_thr_spawn(usize stack, rcc_thr_fn body, rcc_thr_fn on_panic, uint8_t *arg, int32_t *err) {
    RccThread *t = calloc(1, sizeof *t);
    if (!t) {
        *err = ENOMEM;
        return NULL;
    }
    t->body = body;
    t->on_panic = on_panic;
    t->arg = arg;
    t->refs = 2;
    pthread_attr_t a;
    pthread_attr_init(&a);
    if (stack) {
        long pg = sysconf(_SC_PAGESIZE);
        usize page = pg > 0 ? (usize)pg : 4096;
        stack = (stack + page - 1) / page * page;
        if (stack < (usize)PTHREAD_STACK_MIN)
            stack = (usize)PTHREAD_STACK_MIN;
        int r = pthread_attr_setstacksize(&a, stack);
        if (r) {
            pthread_attr_destroy(&a);
            free(t);
            *err = r;
            return NULL;
        }
    }
    int r = pthread_create(&t->tid, &a, rcc_thr_start, t);
    pthread_attr_destroy(&a);
    if (r) {
        free(t);
        *err = r;
        return NULL;
    }
    *err = 0;
    return t;
}

int32_t rcc_thr_join(void *h) {
    RccThread *t = h;
    int r = pthread_join(t->tid, NULL);
    rcc_thr_release(t);
    return r;
}

void rcc_thr_detach(void *h) {
    RccThread *t = h;
    pthread_detach(t->tid);
    rcc_thr_release(t);
}

/* 1 on the main thread (any thread not started by rcc_thr_spawn). */
int32_t rcc_thr_is_main(void) { return rcc_thr_local()->self == NULL; }

/* Leave the current spawned thread after a panic: jump back to its entry
   (no destructors run) where on_panic takes over. */
_Noreturn void rcc_thr_unwind(void) {
    RccThread *t = rcc_thr_local()->self;
    if (!t || t->done) {
        fflush(stdout);
        fprintf(stderr, "fatal runtime error: panic outside a panic-capturing thread context, aborting\n");
        abort();
    }
    longjmp(t->jb, 1);
}

/* std registers the function that turns a runtime (C-side) panic message
   into a Rust panic; set before the first thread is created. */
void rcc_thr_set_c_panic_hook(rcc_thr_panic_fn f) { rcc_thr_c_panic_hook = f; }

void rcc_thr_c_panic(const char *msg, usize len, int is_static) {
    RccThread *t = rcc_thr_local()->self;
    rcc_thr_panic_fn h = rcc_thr_c_panic_hook;
    if (t && !t->done && h)
        h((const uint8_t *)msg, len, is_static);
}

void rcc_thr_yield(void) { sched_yield(); }

void rcc_thr_sleep(uint64_t secs, uint32_t nanos) {
    struct timespec ts;
    ts.tv_sec = (time_t)secs;
    ts.tv_nsec = (long)nanos;
    while (nanosleep(&ts, &ts) == -1 && errno == EINTR) {
    }
}

usize rcc_thr_ncpu(void) {
    long n = sysconf(_SC_NPROCESSORS_ONLN);
    return n < 1 ? 1 : (usize)n;
}

/* ---- atomics: every operation is sequentially consistent ----
   op: 0 load, 1 store, 2 swap, 3 add, 4 sub, 5 and, 6 or, 7 xor,
   8 compare-exchange (v = expected, w = new; returns the old value). */
#define RCC_THR_ATOMIC_CASE(T)                                                  \
    {                                                                           \
        T *q = (T *)p, a = (T)v;                                                \
        switch (op) {                                                           \
        case 0: return __atomic_load_n(q, __ATOMIC_SEQ_CST);                    \
        case 1: __atomic_store_n(q, a, __ATOMIC_SEQ_CST); return 0;             \
        case 2: return __atomic_exchange_n(q, a, __ATOMIC_SEQ_CST);             \
        case 3: return __atomic_fetch_add(q, a, __ATOMIC_SEQ_CST);              \
        case 4: return __atomic_fetch_sub(q, a, __ATOMIC_SEQ_CST);              \
        case 5: return __atomic_fetch_and(q, a, __ATOMIC_SEQ_CST);              \
        case 6: return __atomic_fetch_or(q, a, __ATOMIC_SEQ_CST);               \
        case 7: return __atomic_fetch_xor(q, a, __ATOMIC_SEQ_CST);              \
        case 8: {                                                               \
            T e = a;                                                            \
            __atomic_compare_exchange_n(q, &e, (T)w, 0, __ATOMIC_SEQ_CST,       \
                                        __ATOMIC_SEQ_CST);                      \
            return e;                                                           \
        }                                                                       \
        }                                                                       \
        break;                                                                  \
    }

uint64_t rcc_thr_atomic(void *p, int32_t size, int32_t op, uint64_t v, uint64_t w) {
    switch (size) {
    case 1: RCC_THR_ATOMIC_CASE(uint8_t)
    case 2: RCC_THR_ATOMIC_CASE(uint16_t)
    case 4: RCC_THR_ATOMIC_CASE(uint32_t)
    case 8: RCC_THR_ATOMIC_CASE(uint64_t)
    }
    fprintf(stderr, "rcc: bad atomic operation %d/%d\n", (int)size, (int)op);
    abort();
}

/* ===================== end of rcc_thr_* ===================== */

/* ===================================================================== */
/* rcc_os_*: files, directories, environment, processes, time and stdio  */
/* for std::{io,fs,env,path,process,time} (lib/std/src/os/sys.rs holds   */
/* the Rust declarations). Every libc struct (stat, dirent, ...) stays   */
/* on this side: only scalars and byte buffers cross the boundary.       */
/* Conventions: paths arrive as (ptr, len) and are copied into a NUL-    */
/* terminated buffer here; failures return -errno (or errno where noted) */
/* ===================================================================== */

#include <dirent.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>

extern char **environ;

/* NUL-terminated copy of (p, len); NULL if p contains a NUL byte. */
static char *rcc_os_cstr(const uint8_t *p, usize len) {
    if (memchr(p, 0, len))
        return NULL;
    char *s = malloc(len + 1);
    if (!s) {
        fprintf(stderr, "memory allocation of %lu bytes failed\n", len + 1);
        abort();
    }
    memcpy(s, p, len);
    s[len] = 0;
    return s;
}
#define RCC_OS_PATH(var, p, len)                                               \
    char *var = rcc_os_cstr(p, len);                                           \
    if (!var)                                                                  \
        return -EINVAL;

/* ---- errors ---- */

/* std::io::ErrorKind index (must match the enum order in io/error.rs). */
enum {
    RK_NotFound, RK_PermissionDenied, RK_ConnectionRefused, RK_ConnectionReset,
    RK_HostUnreachable, RK_NetworkUnreachable, RK_ConnectionAborted, RK_NotConnected,
    RK_AddrInUse, RK_AddrNotAvailable, RK_NetworkDown, RK_BrokenPipe,
    RK_AlreadyExists, RK_WouldBlock, RK_NotADirectory, RK_IsADirectory,
    RK_DirectoryNotEmpty, RK_ReadOnlyFilesystem, RK_FilesystemLoop,
    RK_StaleNetworkFileHandle, RK_InvalidInput, RK_InvalidData, RK_TimedOut,
    RK_WriteZero, RK_StorageFull, RK_NotSeekable, RK_QuotaExceeded,
    RK_FileTooLarge, RK_ResourceBusy, RK_ExecutableFileBusy, RK_Deadlock,
    RK_CrossesDevices, RK_TooManyLinks, RK_InvalidFilename,
    RK_ArgumentListTooLong, RK_Interrupted, RK_Unsupported, RK_UnexpectedEof,
    RK_OutOfMemory, RK_InProgress, RK_Other, RK_Uncategorized
};

int32_t rcc_os_error_kind(int32_t e) {
    switch (e) {
    case E2BIG: return RK_ArgumentListTooLong;
    case EADDRINUSE: return RK_AddrInUse;
    case EADDRNOTAVAIL: return RK_AddrNotAvailable;
    case EBUSY: return RK_ResourceBusy;
    case ECONNABORTED: return RK_ConnectionAborted;
    case ECONNREFUSED: return RK_ConnectionRefused;
    case ECONNRESET: return RK_ConnectionReset;
    case EDEADLK: return RK_Deadlock;
    case EDQUOT: return RK_QuotaExceeded;
    case EEXIST: return RK_AlreadyExists;
    case EFBIG: return RK_FileTooLarge;
    case EHOSTUNREACH: return RK_HostUnreachable;
    case EINTR: return RK_Interrupted;
    case EINVAL: return RK_InvalidInput;
    case EISDIR: return RK_IsADirectory;
    case ELOOP: return RK_FilesystemLoop;
    case ENOENT: return RK_NotFound;
    case ENOMEM: return RK_OutOfMemory;
    case ENOSPC: return RK_StorageFull;
    case ENOSYS: return RK_Unsupported;
    case EMLINK: return RK_TooManyLinks;
    case ENAMETOOLONG: return RK_InvalidFilename;
    case ENETDOWN: return RK_NetworkDown;
    case ENETUNREACH: return RK_NetworkUnreachable;
    case ENOTCONN: return RK_NotConnected;
    case ENOTDIR: return RK_NotADirectory;
    case ENOTEMPTY: return RK_DirectoryNotEmpty;
    case EPIPE: return RK_BrokenPipe;
    case EROFS: return RK_ReadOnlyFilesystem;
    case ESPIPE: return RK_NotSeekable;
    case ESTALE: return RK_StaleNetworkFileHandle;
    case ETIMEDOUT: return RK_TimedOut;
    case ETXTBSY: return RK_ExecutableFileBusy;
    case EXDEV: return RK_CrossesDevices;
    case EINPROGRESS: return RK_InProgress;
    case EACCES:
    case EPERM: return RK_PermissionDenied;
    default:
        if (e == EAGAIN || e == EWOULDBLOCK)
            return RK_WouldBlock;
        return RK_Uncategorized;
    }
}

/* strerror text of `e`; returns its length (truncated to cap). */
usize rcc_os_strerror(int32_t e, uint8_t *buf, usize cap) {
    char tmp[256];
    tmp[0] = 0;
#if defined(__GLIBC__) && defined(_GNU_SOURCE)
    const char *s = strerror_r(e, tmp, sizeof tmp);
#else
    const char *s = tmp;
    if (strerror_r(e, tmp, sizeof tmp) != 0 && tmp[0] == 0)
        snprintf(tmp, sizeof tmp, "Unknown error %d", e);
#endif
    usize n = strlen(s);
    if (n > cap)
        n = cap;
    memcpy(buf, s, n);
    return n;
}

int32_t rcc_os_errno_nul(void) { return EINVAL; }

/* ---- raw fds ---- */

int64_t rcc_os_read(int32_t fd, uint8_t *buf, usize len) {
    if (len > 0x7ffff000)
        len = 0x7ffff000;
    for (;;) {
        ssize_t n = read(fd, buf, len);
        if (n >= 0)
            return n;
        if (errno != EINTR)
            return -errno;
    }
}

int64_t rcc_os_write(int32_t fd, const uint8_t *buf, usize len) {
    if (len > 0x7ffff000)
        len = 0x7ffff000;
    for (;;) {
        ssize_t n = write(fd, buf, len);
        if (n >= 0)
            return n;
        if (errno != EINTR)
            return -errno;
    }
}

int32_t rcc_os_close(int32_t fd) { return close(fd) == 0 ? 0 : -errno; }

int32_t rcc_os_dup(int32_t fd) {
    int r = fcntl(fd, F_DUPFD_CLOEXEC, 3);
    return r >= 0 ? r : -errno;
}

int32_t rcc_os_isatty(int32_t fd) { return isatty(fd); }

/* ---- stdout (C stdio, LineWriter semantics) and stdin (8 KiB buffer) ---- */

/* Everything up to the last newline is written and flushed; a trailing
   partial line stays buffered until the next newline, flush or exit. */
int32_t rcc_os_stdout_write(const uint8_t *buf, usize len) {
    const uint8_t *nl = NULL;
    for (usize i = len; i > 0; i--)
        if (buf[i - 1] == '\n') {
            nl = buf + i;
            break;
        }
    if (nl) {
        usize head = (usize)(nl - buf);
        if (fwrite(buf, 1, head, stdout) != head || fflush(stdout) != 0)
            return errno ? errno : EIO;
        buf += head;
        len -= head;
    }
    if (len && fwrite(buf, 1, len, stdout) != len)
        return errno ? errno : EIO;
    return 0;
}

int32_t rcc_os_stdout_flush(void) { return fflush(stdout) == 0 ? 0 : (errno ? errno : EIO); }

static uint8_t rcc_os_stdin_buf[8192];
static usize rcc_os_stdin_pos, rcc_os_stdin_cap;

/* Bytes buffered from stdin (reads more if the buffer is empty);
   0 at EOF, -errno on error. */
int64_t rcc_os_stdin_fill(const uint8_t **ptr) {
    if (rcc_os_stdin_pos >= rcc_os_stdin_cap) {
        int64_t n = rcc_os_read(0, rcc_os_stdin_buf, sizeof rcc_os_stdin_buf);
        if (n < 0) {
            if (n == -EBADF) /* closed stdin reads as empty, like Rust */
                n = 0;
            else
                return n;
        }
        rcc_os_stdin_pos = 0;
        rcc_os_stdin_cap = (usize)n;
    }
    *ptr = rcc_os_stdin_buf + rcc_os_stdin_pos;
    return (int64_t)(rcc_os_stdin_cap - rcc_os_stdin_pos);
}

void rcc_os_stdin_consume(usize n) {
    rcc_os_stdin_pos += n;
    if (rcc_os_stdin_pos > rcc_os_stdin_cap)
        rcc_os_stdin_pos = rcc_os_stdin_cap;
}

int64_t rcc_os_stdin_read(uint8_t *buf, usize len) {
    if (rcc_os_stdin_pos >= rcc_os_stdin_cap && len >= sizeof rcc_os_stdin_buf) {
        int64_t n = rcc_os_read(0, buf, len);
        return n == -EBADF ? 0 : n;
    }
    const uint8_t *p;
    int64_t avail = rcc_os_stdin_fill(&p);
    if (avail <= 0)
        return avail;
    usize n = (usize)avail < len ? (usize)avail : len;
    memcpy(buf, p, n);
    rcc_os_stdin_consume(n);
    return (int64_t)n;
}

/* ---- files ---- */

/* flags: 1 read, 2 write, 4 append, 8 truncate, 16 create, 32 create_new;
   custom: extra O_* flags (OpenOptionsExt::custom_flags). */
int32_t rcc_os_open(const uint8_t *path, usize len, int32_t flags, int32_t custom, uint32_t mode) {
    int acc;
    int rd = flags & 1, wr = (flags & 2) != 0, ap = (flags & 4) != 0;
    if (rd && !wr && !ap)
        acc = O_RDONLY;
    else if (!rd && (wr || ap))
        acc = ap ? (O_WRONLY | O_APPEND) : O_WRONLY;
    else if (rd && (wr || ap))
        acc = ap ? (O_RDWR | O_APPEND) : O_RDWR;
    else
        return -EINVAL;
    int tr = (flags & 8) != 0, cr = (flags & 16) != 0, cn = (flags & 32) != 0;
    int cflags;
    if (!wr && !ap) {
        if (tr || cr || cn)
            return -EINVAL;
    } else if (ap && tr && !cn) {
        return -EINVAL;
    }
    if (cn)
        cflags = O_CREAT | O_EXCL;
    else if (cr && tr)
        cflags = O_CREAT | O_TRUNC;
    else if (cr)
        cflags = O_CREAT;
    else if (tr)
        cflags = O_TRUNC;
    else
        cflags = 0;
    RCC_OS_PATH(p, path, len);
    int fd;
    do
        fd = open(p, acc | cflags | O_CLOEXEC | (custom & ~O_ACCMODE), (mode_t)mode);
    while (fd < 0 && errno == EINTR);
    int e = errno;
    free(p);
    return fd >= 0 ? fd : -e;
}

/* lseek; whence 0 set, 1 cur, 2 end. */
int64_t rcc_os_lseek(int32_t fd, int64_t off, int32_t whence) {
    int w = whence == 0 ? SEEK_SET : whence == 1 ? SEEK_CUR : SEEK_END;
    off_t r = lseek(fd, (off_t)off, w);
    return r >= 0 ? (int64_t)r : -errno;
}

int32_t rcc_os_ftruncate(int32_t fd, uint64_t size) {
    int r;
    do
        r = ftruncate(fd, (off_t)size);
    while (r < 0 && errno == EINTR);
    return r == 0 ? 0 : -errno;
}

int32_t rcc_os_fsync(int32_t fd, int32_t data_only) {
    (void)data_only;
#ifdef __APPLE__
    if (fcntl(fd, F_FULLFSYNC) == 0)
        return 0;
#endif
    return fsync(fd) == 0 ? 0 : -errno;
}

int32_t rcc_os_fchmod(int32_t fd, uint32_t mode) { return fchmod(fd, (mode_t)mode) == 0 ? 0 : -errno; }

/* stat results: out[0] file type (1 file, 2 dir, 3 symlink, 4 block dev,
   5 char dev, 6 fifo, 7 socket, 0 unknown), [1] st_mode, [2] size,
   [3,4] mtime s/ns, [5,6] atime, [7,8] ctime, [9,10] birth time, [11] has
   birth time, [12] dev, [13] ino, [14] nlink, [15] uid, [16] gid,
   [17] rdev, [18] blksize, [19] blocks. */
static int rcc_os_ftype(mode_t m) {
    switch (m & S_IFMT) {
    case S_IFREG: return 1;
    case S_IFDIR: return 2;
    case S_IFLNK: return 3;
    case S_IFBLK: return 4;
    case S_IFCHR: return 5;
    case S_IFIFO: return 6;
    case S_IFSOCK: return 7;
    default: return 0;
    }
}

static void rcc_os_fill_stat(const struct stat *st, uint64_t *out) {
    memset(out, 0, 20 * sizeof *out);
    out[0] = (uint64_t)rcc_os_ftype(st->st_mode);
    out[1] = (uint64_t)st->st_mode;
    out[2] = (uint64_t)st->st_size;
#ifdef __APPLE__
    out[3] = (uint64_t)st->st_mtimespec.tv_sec;
    out[4] = (uint64_t)st->st_mtimespec.tv_nsec;
    out[5] = (uint64_t)st->st_atimespec.tv_sec;
    out[6] = (uint64_t)st->st_atimespec.tv_nsec;
    out[7] = (uint64_t)st->st_ctimespec.tv_sec;
    out[8] = (uint64_t)st->st_ctimespec.tv_nsec;
    out[9] = (uint64_t)st->st_birthtimespec.tv_sec;
    out[10] = (uint64_t)st->st_birthtimespec.tv_nsec;
    out[11] = 1;
#else
    out[3] = (uint64_t)st->st_mtim.tv_sec;
    out[4] = (uint64_t)st->st_mtim.tv_nsec;
    out[5] = (uint64_t)st->st_atim.tv_sec;
    out[6] = (uint64_t)st->st_atim.tv_nsec;
    out[7] = (uint64_t)st->st_ctim.tv_sec;
    out[8] = (uint64_t)st->st_ctim.tv_nsec;
#endif
    out[12] = (uint64_t)st->st_dev;
    out[13] = (uint64_t)st->st_ino;
    out[14] = (uint64_t)st->st_nlink;
    out[15] = (uint64_t)st->st_uid;
    out[16] = (uint64_t)st->st_gid;
    out[17] = (uint64_t)st->st_rdev;
    out[18] = (uint64_t)st->st_blksize;
    out[19] = (uint64_t)st->st_blocks;
}

int32_t rcc_os_stat(const uint8_t *path, usize len, int32_t follow, uint64_t *out) {
    RCC_OS_PATH(p, path, len);
    struct stat st;
    int r = follow ? stat(p, &st) : lstat(p, &st);
    int e = errno;
    free(p);
    if (r != 0)
        return -e;
    rcc_os_fill_stat(&st, out);
    return 0;
}

int32_t rcc_os_fstat(int32_t fd, uint64_t *out) {
    struct stat st;
    if (fstat(fd, &st) != 0)
        return -errno;
    rcc_os_fill_stat(&st, out);
    return 0;
}

/* ---- paths ---- */

#define RCC_OS_PATH_OP1(name, call)                                            \
    int32_t name(const uint8_t *path, usize len) {                             \
        RCC_OS_PATH(p, path, len);                                             \
        int r = call;                                                          \
        int e = errno;                                                         \
        free(p);                                                               \
        return r == 0 ? 0 : -e;                                                \
    }
RCC_OS_PATH_OP1(rcc_os_unlink, unlink(p))
RCC_OS_PATH_OP1(rcc_os_rmdir, rmdir(p))
RCC_OS_PATH_OP1(rcc_os_chdir, chdir(p))

int32_t rcc_os_mkdir(const uint8_t *path, usize len, uint32_t mode) {
    RCC_OS_PATH(p, path, len);
    int r = mkdir(p, (mode_t)mode);
    int e = errno;
    free(p);
    return r == 0 ? 0 : -e;
}

int32_t rcc_os_chmod(const uint8_t *path, usize len, uint32_t mode) {
    RCC_OS_PATH(p, path, len);
    int r;
    do
        r = chmod(p, (mode_t)mode);
    while (r != 0 && errno == EINTR);
    int e = errno;
    free(p);
    return r == 0 ? 0 : -e;
}

/* op: 0 rename, 1 hard link, 2 symlink (a = target, b = link path) */
int32_t rcc_os_path2(int32_t op, const uint8_t *a, usize alen, const uint8_t *b, usize blen) {
    RCC_OS_PATH(pa, a, alen);
    char *pb = rcc_os_cstr(b, blen);
    if (!pb) {
        free(pa);
        return -EINVAL;
    }
    int r = op == 0 ? rename(pa, pb) : op == 1 ? linkat(AT_FDCWD, pa, AT_FDCWD, pb, 0) : symlink(pa, pb);
    int e = errno;
    free(pa);
    free(pb);
    return r == 0 ? 0 : -e;
}

/* realpath (op 0) or readlink (op 1) into a malloc'd buffer *out (free
   with rcc_dealloc); returns the length or -errno. */
int64_t rcc_os_path_query(int32_t op, const uint8_t *path, usize len, uint8_t **out) {
    RCC_OS_PATH(p, path, len);
    char *res = NULL;
    int64_t n;
    if (op == 0) {
        res = realpath(p, NULL);
        n = res ? (int64_t)strlen(res) : -errno;
    } else {
        usize cap = 256;
        for (;;) {
            res = malloc(cap);
            ssize_t k = readlink(p, res, cap);
            if (k < 0) {
                n = -errno;
                free(res);
                res = NULL;
                break;
            }
            if ((usize)k < cap) {
                n = k;
                break;
            }
            free(res);
            cap *= 2;
        }
    }
    free(p);
    *out = (uint8_t *)res;
    return n;
}

/* getcwd into a malloc'd buffer. */
int64_t rcc_os_getcwd(uint8_t **out) {
    usize cap = 512;
    for (;;) {
        char *b = malloc(cap);
        if (getcwd(b, cap)) {
            *out = (uint8_t *)b;
            return (int64_t)strlen(b);
        }
        int e = errno;
        free(b);
        if (e != ERANGE)
            return -e;
        cap *= 2;
    }
}

/* Path of the running executable into a malloc'd buffer. */
#ifdef __APPLE__
extern int _NSGetExecutablePath(char *buf, uint32_t *bufsize);
#endif
int64_t rcc_os_current_exe(uint8_t **out) {
#ifdef __APPLE__
    uint32_t sz = 0;
    _NSGetExecutablePath(NULL, &sz);
    char *b = malloc(sz + 1);
    if (_NSGetExecutablePath(b, &sz) != 0) {
        free(b);
        return -ENOENT;
    }
    char *r = realpath(b, NULL);
    if (r) {
        free(b);
        b = r;
    }
    *out = (uint8_t *)b;
    return (int64_t)strlen(b);
#else
    static const uint8_t self[] = "/proc/self/exe";
    return rcc_os_path_query(1, self, sizeof self - 1, out);
#endif
}

/* ---- directories ---- */

/* opendir; returns the handle or 0 with *err = errno. */
usize rcc_os_opendir(const uint8_t *path, usize len, int32_t *err) {
    char *p = rcc_os_cstr(path, len);
    if (!p) {
        *err = EINVAL;
        return 0;
    }
    DIR *d = opendir(p);
    *err = d ? 0 : errno;
    free(p);
    return (usize)d;
}

/* Next entry other than "." and "..": 1 with name/type/inode, 0 at the
   end, -errno on error. The name stays valid until the next call. */
int32_t rcc_os_readdir(usize dir, const uint8_t **name, usize *len, int32_t *type, uint64_t *ino) {
    for (;;) {
        errno = 0;
        struct dirent *e = readdir((DIR *)dir);
        if (!e)
            return errno ? -errno : 0;
        const char *n = e->d_name;
        if (n[0] == '.' && (n[1] == 0 || (n[1] == '.' && n[2] == 0)))
            continue;
        *name = (const uint8_t *)n;
        *len = strlen(n);
        *ino = (uint64_t)e->d_ino;
        switch (e->d_type) {
        case DT_REG: *type = 1; break;
        case DT_DIR: *type = 2; break;
        case DT_LNK: *type = 3; break;
        case DT_BLK: *type = 4; break;
        case DT_CHR: *type = 5; break;
        case DT_FIFO: *type = 6; break;
        case DT_SOCK: *type = 7; break;
        default: *type = 0; break;
        }
        return 1;
    }
}

void rcc_os_closedir(usize dir) { closedir((DIR *)dir); }

/* ---- environment ---- */

/* Value of an environment variable (copied into a malloc'd buffer *out);
   returns its length or -1 if unset. */
int64_t rcc_os_getenv(const uint8_t *key, usize len, uint8_t **out) {
    char *k = rcc_os_cstr(key, len);
    if (!k || len == 0 || memchr(key, '=', len)) {
        free(k);
        return -1;
    }
    const char *v = getenv(k);
    free(k);
    if (!v)
        return -1;
    usize n = strlen(v);
    uint8_t *b = malloc(n + 1);
    memcpy(b, v, n);
    *out = b;
    return (int64_t)n;
}

int32_t rcc_os_setenv(const uint8_t *key, usize klen, const uint8_t *val, usize vlen) {
    char *k = rcc_os_cstr(key, klen);
    char *v = rcc_os_cstr(val, vlen);
    int r = -EINVAL;
    if (k && v && klen > 0 && !memchr(key, '=', klen))
        r = setenv(k, v, 1) == 0 ? 0 : -errno;
    free(k);
    free(v);
    return r;
}

int32_t rcc_os_unsetenv(const uint8_t *key, usize klen) {
    char *k = rcc_os_cstr(key, klen);
    int r = -EINVAL;
    if (k && klen > 0 && !memchr(key, '=', klen))
        r = unsetenv(k) == 0 ? 0 : -errno;
    free(k);
    return r;
}

/* Snapshot of the environment as "K=V\0K=V\0..." (malloc'd); returns
   the total length. */
usize rcc_os_environ(uint8_t **out) {
    usize total = 0;
    for (char **e = environ; e && *e; e++)
        total += strlen(*e) + 1;
    uint8_t *b = malloc(total + 1);
    usize o = 0;
    for (char **e = environ; e && *e; e++) {
        usize n = strlen(*e) + 1;
        memcpy(b + o, *e, n);
        o += n;
    }
    *out = b;
    return total;
}

uint32_t rcc_os_getpid(void) { return (uint32_t)getpid(); }

/* ---- time ---- */

/* which: 0 monotonic, 1 realtime. */
void rcc_os_clock(int32_t which, int64_t *secs, int64_t *nanos) {
    struct timespec ts;
#if defined(__APPLE__) && defined(CLOCK_UPTIME_RAW)
    clock_gettime(which == 0 ? CLOCK_UPTIME_RAW : CLOCK_REALTIME, &ts);
#else
    clock_gettime(which == 0 ? CLOCK_MONOTONIC : CLOCK_REALTIME, &ts);
#endif
    *secs = (int64_t)ts.tv_sec;
    *nanos = (int64_t)ts.tv_nsec;
}

void rcc_os_sleep(uint64_t secs, uint32_t nanos) {
    while (secs > 0 || nanos > 0) {
        struct timespec ts, rem;
        ts.tv_sec = secs > 0x7fffffff ? 0x7fffffff : (time_t)secs;
        ts.tv_nsec = (long)nanos;
        secs -= (uint64_t)ts.tv_sec;
        nanos = 0;
        while (nanosleep(&ts, &rem) != 0 && errno == EINTR)
            ts = rem;
    }
}

/* ---- processes ---- */

/* Spawn a child.
   prog: NUL-terminated program (searched in PATH by execvp);
   args: "argv0\0arg1\0..." with argc entries;
   envs: "K=V\0..." with envc entries, or envc < 0 to inherit;
   cwd: NUL-terminated directory or NULL;
   stdio[i] for fd i: -1 inherit, -2 /dev/null, -3 new pipe, >= 0 dup2
   that fd. On success returns 0, sets *pid and, for piped slots, our
   end in pipes[i] (else -1). On failure returns errno (positive), which
   includes the child's exec failure. */
int32_t rcc_os_spawn(const uint8_t *prog, const uint8_t *args, usize argc, const uint8_t *envs, int64_t envc,
                     const uint8_t *cwd, const int32_t *stdio, int32_t *pid_out, int32_t *pipes) {
    char **argv = malloc((argc + 1) * sizeof *argv);
    const char *s = (const char *)args;
    for (usize i = 0; i < argc; i++) {
        argv[i] = (char *)s;
        s += strlen(s) + 1;
    }
    argv[argc] = NULL;
    char **envp = NULL;
    if (envc >= 0) {
        envp = malloc(((usize)envc + 1) * sizeof *envp);
        s = (const char *)envs;
        for (int64_t i = 0; i < envc; i++) {
            envp[i] = (char *)s;
            s += strlen(s) + 1;
        }
        envp[envc] = NULL;
    }
    int child_fd[3] = {-1, -1, -1};
    int ours[3] = {-1, -1, -1};
    int close_child[3] = {0, 0, 0};
    int err = 0;
    for (int i = 0; i < 3 && !err; i++) {
        if (stdio[i] == -2) {
            int fd = open("/dev/null", (i == 0 ? O_RDONLY : O_WRONLY) | O_CLOEXEC);
            if (fd < 0)
                err = errno;
            child_fd[i] = fd;
            close_child[i] = 1;
        } else if (stdio[i] == -3) {
            int p[2];
            if (pipe(p) != 0) {
                err = errno;
                break;
            }
            fcntl(p[0], F_SETFD, FD_CLOEXEC);
            fcntl(p[1], F_SETFD, FD_CLOEXEC);
            child_fd[i] = i == 0 ? p[0] : p[1];
            ours[i] = i == 0 ? p[1] : p[0];
            close_child[i] = 1;
        } else if (stdio[i] >= 0) {
            child_fd[i] = stdio[i];
        }
    }
    int ep[2] = {-1, -1};
    if (!err && pipe(ep) != 0)
        err = errno;
    pid_t pid = -1;
    if (!err) {
        fcntl(ep[0], F_SETFD, FD_CLOEXEC);
        fcntl(ep[1], F_SETFD, FD_CLOEXEC);
        fflush(stdout);
        pid = fork();
        if (pid < 0)
            err = errno;
    }
    if (pid == 0) {
        /* child: only async-signal-safe calls from here */
        for (int i = 0; i < 3; i++) {
            if (child_fd[i] >= 0) {
                if (child_fd[i] == i) {
                    int fl = fcntl(i, F_GETFD);
                    if (fl >= 0)
                        fcntl(i, F_SETFD, fl & ~FD_CLOEXEC);
                } else if (dup2(child_fd[i], i) < 0) {
                    goto fail;
                }
            }
        }
        if (cwd && chdir((const char *)cwd) != 0)
            goto fail;
        signal(SIGPIPE, SIG_DFL);
        if (envp)
            environ = envp;
        execvp((const char *)prog, argv);
    fail:;
        int e = errno;
        unsigned char b[4] = {(unsigned char)e, (unsigned char)(e >> 8), (unsigned char)(e >> 16),
                              (unsigned char)(e >> 24)};
        ssize_t w = write(ep[1], b, 4);
        (void)w;
        _exit(127);
    }
    for (int i = 0; i < 3; i++)
        if (close_child[i] && child_fd[i] >= 0)
            close(child_fd[i]);
    if (ep[1] >= 0)
        close(ep[1]);
    if (!err) {
        unsigned char b[4];
        ssize_t n;
        do
            n = read(ep[0], b, 4);
        while (n < 0 && errno == EINTR);
        if (n == 4) {
            err = b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24);
            int st;
            while (waitpid(pid, &st, 0) < 0 && errno == EINTR)
                ;
        }
    }
    if (ep[0] >= 0)
        close(ep[0]);
    free(argv);
    free(envp);
    if (err) {
        for (int i = 0; i < 3; i++)
            if (ours[i] >= 0)
                close(ours[i]);
        return err;
    }
    *pid_out = (int32_t)pid;
    for (int i = 0; i < 3; i++)
        pipes[i] = ours[i];
    return 0;
}

/* waitpid: 1 and *status when reaped, 0 if still running (nohang), -errno. */
int32_t rcc_os_wait(int32_t pid, int32_t *status, int32_t nohang) {
    int st;
    for (;;) {
        pid_t r = waitpid((pid_t)pid, &st, nohang ? WNOHANG : 0);
        if (r < 0 && errno == EINTR)
            continue;
        if (r < 0)
            return -errno;
        if (r == 0)
            return 0;
        *status = st;
        return 1;
    }
}

int32_t rcc_os_kill(int32_t pid, int32_t sig) { return kill((pid_t)pid, sig) == 0 ? 0 : -errno; }

int32_t rcc_os_sigkill(void) { return SIGKILL; }

/* Decode a wait status: returns 0 exited (*val = code), 1 signaled (*val =
   signal, *core), 2 stopped (*val = signal), 3 continued, -1 other. */
int32_t rcc_os_wstatus(int32_t st, int32_t *val, int32_t *core) {
    *core = 0;
    if (WIFEXITED(st)) {
        *val = WEXITSTATUS(st);
        return 0;
    }
    if (WIFSIGNALED(st)) {
        *val = WTERMSIG(st);
#ifdef WCOREDUMP
        *core = WCOREDUMP(st) ? 1 : 0;
#endif
        return 1;
    }
    if (WIFSTOPPED(st)) {
        *val = WSTOPSIG(st);
        return 2;
    }
#ifdef WIFCONTINUED
    if (WIFCONTINUED(st))
        return 3;
#endif
    return -1;
}

/* Rust's abbreviation for a signal ("SIGKILL"), or "" if unknown. */
const char *rcc_os_signame(int32_t sig) {
    switch (sig) {
    case SIGHUP: return "SIGHUP";
    case SIGINT: return "SIGINT";
    case SIGQUIT: return "SIGQUIT";
    case SIGILL: return "SIGILL";
    case SIGTRAP: return "SIGTRAP";
    case SIGABRT: return "SIGABRT";
    case SIGBUS: return "SIGBUS";
    case SIGFPE: return "SIGFPE";
    case SIGKILL: return "SIGKILL";
    case SIGUSR1: return "SIGUSR1";
    case SIGSEGV: return "SIGSEGV";
    case SIGUSR2: return "SIGUSR2";
    case SIGPIPE: return "SIGPIPE";
    case SIGALRM: return "SIGALRM";
    case SIGTERM: return "SIGTERM";
    case SIGCHLD: return "SIGCHLD";
    case SIGCONT: return "SIGCONT";
    case SIGSTOP: return "SIGSTOP";
    case SIGTSTP: return "SIGTSTP";
    case SIGTTIN: return "SIGTTIN";
    case SIGTTOU: return "SIGTTOU";
    case SIGURG: return "SIGURG";
    case SIGXCPU: return "SIGXCPU";
    case SIGXFSZ: return "SIGXFSZ";
    case SIGVTALRM: return "SIGVTALRM";
    case SIGPROF: return "SIGPROF";
    case SIGWINCH: return "SIGWINCH";
    case SIGIO: return "SIGIO";
    case SIGSYS: return "SIGSYS";
#ifdef SIGEMT
    case SIGEMT: return "SIGEMT";
#endif
#ifdef SIGINFO
    case SIGINFO: return "SIGINFO";
#endif
#if defined(SIGSTKFLT)
    case SIGSTKFLT: return "SIGSTKFLT";
#endif
#if defined(SIGPWR) && (!defined(SIGINFO) || SIGPWR != SIGINFO)
    case SIGPWR: return "SIGPWR";
#endif
    default: return "";
    }
}

/* Wait until fd a or b (either may be -1) is readable or hung up.
   Returns a bit mask (1 = a, 2 = b) or -errno. */
int32_t rcc_os_poll2(int32_t a, int32_t b) {
    struct pollfd p[2];
    int n = 0, ia = -1, ib = -1;
    if (a >= 0) {
        p[n].fd = a;
        p[n].events = POLLIN;
        ia = n++;
    }
    if (b >= 0) {
        p[n].fd = b;
        p[n].events = POLLIN;
        ib = n++;
    }
    for (;;) {
        int r = poll(p, (nfds_t)n, -1);
        if (r < 0 && errno == EINTR)
            continue;
        if (r < 0)
            return -errno;
        int m = 0;
        if (ia >= 0 && p[ia].revents)
            m |= 1;
        if (ib >= 0 && p[ib].revents)
            m |= 2;
        if (m)
            return m;
    }
}

int32_t rcc_os_pipe(int32_t *fds) {
    int p[2];
    if (pipe(p) != 0)
        return -errno;
    fcntl(p[0], F_SETFD, FD_CLOEXEC);
    fcntl(p[1], F_SETFD, FD_CLOEXEC);
    fds[0] = p[0];
    fds[1] = p[1];
    return 0;
}

/* ===================== end of rcc_os_* ===================== */
