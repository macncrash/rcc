#include <stdatomic.h>
#include <stdio.h>
static _Atomic unsigned long x;
int main(void){ atomic_fetch_add(&x, 3); printf("%lu\n", (unsigned long)atomic_load(&x)); return 0; }
