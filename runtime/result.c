#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct CyResult {
    int tag;         /* 0 = ok, 1 = err */
    long value;      /* ok value as i64 (int, bool, or pointer) */
    char* error;     /* error message (NULL if ok) */
} CyResult;

CyResult* cy_result_ok(long value) {
    CyResult* r = malloc(sizeof(CyResult));
    r->tag = 0;
    r->value = value;
    r->error = NULL;
    return r;
}

CyResult* cy_result_err(const char* message) {
    CyResult* r = malloc(sizeof(CyResult));
    r->tag = 1;
    r->value = 0;
    r->error = strdup(message);
    return r;
}

long cy_result_is_ok(CyResult* r) {
    return r->tag == 0 ? 1 : 0;
}

long cy_result_is_err(CyResult* r) {
    return r->tag == 1 ? 1 : 0;
}

long cy_result_unwrap(CyResult* r) {
    if (r->tag == 0) return r->value;
    fprintf(stderr, "unwrap() called on err: %s\n", r->error ? r->error : "(unknown error)");
    exit(1);
}

long cy_result_unwrap_or(CyResult* r, long default_val) {
    if (r->tag == 0) return r->value;
    return default_val;
}

char* cy_result_error(CyResult* r) {
    if (r->tag == 1 && r->error) return r->error;
    char* empty = malloc(1);
    empty[0] = '\0';
    return empty;
}
