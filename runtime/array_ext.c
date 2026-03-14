#include <stdlib.h>
#include <string.h>

/* Array struct layout: { long* data, long len, long cap } */
typedef struct {
    long* data;
    long len;
    long cap;
} CyArray;

/* Check if array contains value. Returns 1 or 0. */
long cy_array_contains(CyArray* arr, long value) {
    for (long i = 0; i < arr->len; i++) {
        if (arr->data[i] == value) return 1;
    }
    return 0;
}

/* Pop last element. Returns the element value. Does NOT check for empty. */
long cy_array_pop(CyArray* arr) {
    if (arr->len <= 0) return 0;
    arr->len--;
    return arr->data[arr->len];
}

/* Remove element at index. Shifts remaining elements left. Returns removed value. */
long cy_array_remove(CyArray* arr, long index) {
    if (index < 0 || index >= arr->len) return 0;
    long value = arr->data[index];
    memmove(&arr->data[index], &arr->data[index + 1], (arr->len - index - 1) * sizeof(long));
    arr->len--;
    return value;
}
