/*
 * kernel.c — Minimal C primitives that Sans cannot implement itself.
 * Everything else should be written in Sans.
 *
 * This file provides:
 * - stderr output (Sans print only goes to stdout)
 * - Global mutable state (Sans has no global variables yet)
 */

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* ── stderr output ────────────────────────────────────────────── */

long cy_print_err(const char* msg) {
    fprintf(stderr, "%s\n", msg);
    return 0;
}

/* ── Global log level ─────────────────────────────────────────── */

static long _cy_log_level = 0;  /* 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR */

long cy_get_log_level(void) {
    return _cy_log_level;
}

long cy_set_log_level(long level) {
    _cy_log_level = level;
    return 0;
}
