#include <stdio.h>

static int cy_log_level = 0;  /* 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR */

long cy_log_debug(const char* msg) {
    if (cy_log_level <= 0) fprintf(stderr, "[DEBUG] %s\n", msg);
    return 0;
}

long cy_log_info(const char* msg) {
    if (cy_log_level <= 1) fprintf(stderr, "[INFO] %s\n", msg);
    return 0;
}

long cy_log_warn(const char* msg) {
    if (cy_log_level <= 2) fprintf(stderr, "[WARN] %s\n", msg);
    return 0;
}

long cy_log_error(const char* msg) {
    if (cy_log_level <= 3) fprintf(stderr, "[ERROR] %s\n", msg);
    return 0;
}

long cy_log_set_level(long level) {
    cy_log_level = (int)level;
    return 0;
}
