#include <stddef.h>
#include <stdint.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int alloc_smoke(void) {
    size_t n = 64;
    unsigned char *p = (unsigned char *)malloc(n);
    if (!p) {
        return 0;
    }

    memset(p, 0xA5, n);
    for (size_t i = 0; i < n; i++) {
        if (p[i] != 0xA5) {
            free(p);
            return 0;
        }
    }

    p = (unsigned char *)realloc(p, 128);
    if (!p) {
        return 0;
    }
    if (p[0] != 0xA5 || p[63] != 0xA5) {
        free(p);
        return 0;
    }

    free(p);
    return 1;
}

static volatile uint64_t g_thread_sum = 0;

static void *thread_entry(void *arg) {
    (void)arg;

    // Deterministic computation (small, bounded).
    uint64_t sum = 0;
    for (uint64_t i = 1; i <= 1000; i++) {
        sum += i;
    }
    g_thread_sum = sum;

    return (void *)(uintptr_t)sum;
}

static int thread_smoke(void) {
    pthread_t t;
    printf("smoke:thread: create\n");
    fflush(stdout);
    if (pthread_create(&t, NULL, thread_entry, NULL) != 0) {
        return 0;
    }
    void *ret = NULL;
    if (pthread_join(t, &ret) != 0) {
        return 0;
    }
    printf("smoke:thread: joined\n");
    fflush(stdout);
    uint64_t expected = (1000ULL * 1001ULL) / 2ULL;
    return g_thread_sum == expected && (uintptr_t)ret == (uintptr_t)expected;
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;

    printf("Testing printf\n");
    fflush(stdout);

    if (!alloc_smoke()) {
        printf("smoke:alloc: failed\n");
        fflush(stdout);
        return 1;
    }
    printf("smoke:alloc: ok\n");
    fflush(stdout);

    if (!thread_smoke()) {
        printf("smoke:thread: failed\n");
        fflush(stdout);
        return 1;
    }
    printf("smoke:thread: ok\n");
    fflush(stdout);

    return 0;
}
