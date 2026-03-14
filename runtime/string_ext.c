#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

/* Trim whitespace from both ends. Returns malloc'd string. */
char* cy_string_trim(const char* s) {
    long len = (long)strlen(s);
    long start = 0;
    while (start < len && isspace((unsigned char)s[start])) start++;
    long end = len;
    while (end > start && isspace((unsigned char)s[end - 1])) end--;
    long new_len = end - start;
    char* result = malloc(new_len + 1);
    memcpy(result, s + start, new_len);
    result[new_len] = '\0';
    return result;
}

/* Check if s starts with prefix. Returns 1 or 0. */
long cy_string_starts_with(const char* s, const char* prefix) {
    long prefix_len = (long)strlen(prefix);
    long s_len = (long)strlen(s);
    if (prefix_len > s_len) return 0;
    return memcmp(s, prefix, prefix_len) == 0 ? 1 : 0;
}

/* Check if s contains substring. Returns 1 or 0. */
long cy_string_contains(const char* s, const char* needle) {
    return strstr(s, needle) != NULL ? 1 : 0;
}

/* Split string by delimiter. Returns a Cyflym array pointer.
   The array is a 24-byte struct: { i64* data, i64 len, i64 cap }
   Each element is a char* stored as i64. */
void* cy_string_split(const char* s, const char* delim) {
    long delim_len = (long)strlen(delim);
    /* Allocate the array struct (3 * i64) */
    long* arr = malloc(3 * sizeof(long));
    arr[1] = 0; /* len */
    arr[2] = 8; /* cap */
    long* data = malloc(8 * sizeof(long));
    arr[0] = (long)data;

    if (delim_len == 0) {
        /* Empty delimiter: return single element with full string */
        char* copy = strdup(s);
        data[0] = (long)copy;
        arr[1] = 1;
        return arr;
    }

    const char* pos = s;
    while (1) {
        const char* found = strstr(pos, delim);
        long part_len;
        if (found) {
            part_len = found - pos;
        } else {
            part_len = (long)strlen(pos);
        }

        /* Grow if needed */
        if (arr[1] >= arr[2]) {
            arr[2] *= 2;
            data = realloc(data, arr[2] * sizeof(long));
            arr[0] = (long)data;
        }

        char* part = malloc(part_len + 1);
        memcpy(part, pos, part_len);
        part[part_len] = '\0';
        data[arr[1]] = (long)part;
        arr[1]++;

        if (!found) break;
        pos = found + delim_len;
    }

    return arr;
}

/* Replace all occurrences of old with new_str. Returns malloc'd string. */
char* cy_string_replace(const char* s, const char* old, const char* new_str) {
    long old_len = (long)strlen(old);
    long new_len = (long)strlen(new_str);
    long s_len = (long)strlen(s);

    if (old_len == 0) {
        return strdup(s);
    }

    /* Count occurrences */
    long count = 0;
    const char* pos = s;
    while ((pos = strstr(pos, old)) != NULL) {
        count++;
        pos += old_len;
    }

    long result_len = s_len + count * (new_len - old_len);
    char* result = malloc(result_len + 1);
    char* out = result;
    pos = s;
    while (1) {
        const char* found = strstr(pos, old);
        if (!found) {
            strcpy(out, pos);
            break;
        }
        long prefix_len = found - pos;
        memcpy(out, pos, prefix_len);
        out += prefix_len;
        memcpy(out, new_str, new_len);
        out += new_len;
        pos = found + old_len;
    }

    return result;
}
