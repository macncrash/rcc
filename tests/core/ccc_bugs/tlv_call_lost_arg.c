/* ccc bug (arm64-darwin, -O1): an indirect call made after reading a
   _Thread_local variable loses arguments that are held in temporaries
   across the TLV getter call. In `outer` below ccc inlines `call_hook`,
   keeps strlen's result in x12 across the `blr` to the TLV getter, and
   then calls the hook with x0 and x2 set but never moves x12 into x1.
   (Standalone, `call_hook` has the same defect: it saves x0..x2 in
   x12..x14 and tail-calls with only `mov x0, x12`.)
   Expected: "hook(abc, 17, 1)". With ccc: the length is garbage.
   Found by the std::thread work (rt/rt.c rcc_thr_c_panic); rt.c works
   around it by reading the thread-local through a separate function. */
#include <stdio.h>
#include <string.h>
typedef unsigned long usize;
typedef int (*hook_fn)(const unsigned char *, usize, int);
typedef struct T { void *a, *b, *c; int refs; int done; } T;
static _Thread_local T *tl_self;
static hook_fn hook;
static T dummy;

static int print_hook(const unsigned char *s, usize n, int k) {
    printf("hook(%s, %lu, %d)\n", (const char *)s, n, k);
    return 0;
}

void call_hook(const char *msg, usize len, int k) {
    if (tl_self && !tl_self->done && hook)
        hook((const unsigned char *)msg, len, k);
}

void outer(const char *msg) {
    call_hook(msg, strlen(msg) * 5 + 2, 1);
    printf("after\n");
}

int main(void) {
    tl_self = &dummy;
    hook = print_hook;
    void (*volatile f)(const char *) = outer; /* keep `outer` out of line */
    f("abc");
    return 0;
}
