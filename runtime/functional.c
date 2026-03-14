#include <stdlib.h>
#include <string.h>

/* Array struct layout: { long* data, long len, long cap } */
typedef struct {
    long* data;
    long len;
    long cap;
} CyArray;

typedef long (*MapFn)(long);
typedef long (*FilterFn)(long);

/* Map: apply fn to each element, return new array */
CyArray* cy_array_map(CyArray* arr, MapFn f) {
    CyArray* result = malloc(sizeof(CyArray));
    result->len = arr->len;
    result->cap = arr->len > 0 ? arr->len : 1;
    result->data = malloc(result->cap * sizeof(long));
    for (long i = 0; i < arr->len; i++) {
        result->data[i] = f(arr->data[i]);
    }
    return result;
}

/* Filter: return new array with elements where fn returns non-zero */
CyArray* cy_array_filter(CyArray* arr, FilterFn f) {
    CyArray* result = malloc(sizeof(CyArray));
    result->len = 0;
    result->cap = arr->len > 0 ? arr->len : 1;
    result->data = malloc(result->cap * sizeof(long));
    for (long i = 0; i < arr->len; i++) {
        if (f(arr->data[i])) {
            result->data[result->len] = arr->data[i];
            result->len++;
        }
    }
    return result;
}
