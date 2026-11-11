#include <stdio.h>
struct L { unsigned char tag; int v; struct L *next; };
void pr(struct L *cur) {
    struct L **q;
    for (;;) {
        if (cur->tag) break;
        q = &cur->next;
        printf("%d ", cur->v);
        cur = *q;
    }
    printf("nil\n");
}
int main(void) {
    struct L c = {1, 0, 0}, b = {0, 2, &c}, a = {0, 1, &b};
    pr(&a);
    return 0;
}
