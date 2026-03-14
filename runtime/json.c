#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define JSON_NULL   0
#define JSON_BOOL   1
#define JSON_INT    2
#define JSON_STRING 3
#define JSON_ARRAY  4
#define JSON_OBJECT 5

typedef struct CyJsonValue {
    int tag;
    union {
        long bool_val;
        long int_val;
        char* string_val;
        struct {
            struct CyJsonValue** items;
            long len;
            long cap;
        } array_val;
        struct {
            char** keys;
            struct CyJsonValue** values;
            long len;
            long cap;
        } object_val;
    };
} CyJsonValue;

CyJsonValue* cy_json_null(void) {
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_NULL;
    return v;
}

CyJsonValue* cy_json_bool(long b) {
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_BOOL;
    v->bool_val = b ? 1 : 0;
    return v;
}

CyJsonValue* cy_json_int(long n) {
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_INT;
    v->int_val = n;
    return v;
}

CyJsonValue* cy_json_string(const char* s) {
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_STRING;
    v->string_val = (char*)malloc(strlen(s) + 1);
    strcpy(v->string_val, s);
    return v;
}

CyJsonValue* cy_json_object(void) {
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_OBJECT;
    v->object_val.keys = (char**)malloc(8 * sizeof(char*));
    v->object_val.values = (CyJsonValue**)malloc(8 * sizeof(CyJsonValue*));
    v->object_val.len = 0;
    v->object_val.cap = 8;
    return v;
}

CyJsonValue* cy_json_array(void) {
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_ARRAY;
    v->array_val.items = (CyJsonValue**)malloc(8 * sizeof(CyJsonValue*));
    v->array_val.len = 0;
    v->array_val.cap = 8;
    return v;
}

CyJsonValue* cy_json_get(CyJsonValue* obj, const char* key) {
    if (!obj || obj->tag != JSON_OBJECT) return cy_json_null();
    for (long i = 0; i < obj->object_val.len; i++) {
        if (strcmp(obj->object_val.keys[i], key) == 0) {
            return obj->object_val.values[i];
        }
    }
    return cy_json_null();
}

CyJsonValue* cy_json_get_index(CyJsonValue* arr, long index) {
    if (!arr || arr->tag != JSON_ARRAY) return cy_json_null();
    if (index < 0 || index >= arr->array_val.len) return cy_json_null();
    return arr->array_val.items[index];
}

char* cy_json_get_string(CyJsonValue* v) {
    if (!v || v->tag != JSON_STRING) {
        char* empty = (char*)malloc(1);
        empty[0] = '\0';
        return empty;
    }
    char* copy = (char*)malloc(strlen(v->string_val) + 1);
    strcpy(copy, v->string_val);
    return copy;
}

long cy_json_get_int(CyJsonValue* v) {
    if (!v || v->tag != JSON_INT) return 0;
    return v->int_val;
}

long cy_json_get_bool(CyJsonValue* v) {
    if (!v || v->tag != JSON_BOOL) return 0;
    return v->bool_val;
}

long cy_json_len(CyJsonValue* v) {
    if (!v) return 0;
    if (v->tag == JSON_ARRAY) return v->array_val.len;
    if (v->tag == JSON_OBJECT) return v->object_val.len;
    return 0;
}

char* cy_json_type_of(CyJsonValue* v) {
    const char* name;
    if (!v) name = "null";
    else switch (v->tag) {
        case JSON_NULL:   name = "null"; break;
        case JSON_BOOL:   name = "bool"; break;
        case JSON_INT:    name = "int"; break;
        case JSON_STRING: name = "string"; break;
        case JSON_ARRAY:  name = "array"; break;
        case JSON_OBJECT: name = "object"; break;
        default:          name = "null"; break;
    }
    char* result = (char*)malloc(strlen(name) + 1);
    strcpy(result, name);
    return result;
}

void cy_json_set(CyJsonValue* obj, const char* key, CyJsonValue* val) {
    if (!obj || obj->tag != JSON_OBJECT) return;
    for (long i = 0; i < obj->object_val.len; i++) {
        if (strcmp(obj->object_val.keys[i], key) == 0) {
            obj->object_val.values[i] = val;
            return;
        }
    }
    if (obj->object_val.len == obj->object_val.cap) {
        long new_cap = obj->object_val.cap * 2;
        char** new_keys = (char**)malloc(new_cap * sizeof(char*));
        CyJsonValue** new_values = (CyJsonValue**)malloc(new_cap * sizeof(CyJsonValue*));
        memcpy(new_keys, obj->object_val.keys, obj->object_val.len * sizeof(char*));
        memcpy(new_values, obj->object_val.values, obj->object_val.len * sizeof(CyJsonValue*));
        free(obj->object_val.keys);
        free(obj->object_val.values);
        obj->object_val.keys = new_keys;
        obj->object_val.values = new_values;
        obj->object_val.cap = new_cap;
    }
    obj->object_val.keys[obj->object_val.len] = (char*)malloc(strlen(key) + 1);
    strcpy(obj->object_val.keys[obj->object_val.len], key);
    obj->object_val.values[obj->object_val.len] = val;
    obj->object_val.len++;
}

void cy_json_push(CyJsonValue* arr, CyJsonValue* val) {
    if (!arr || arr->tag != JSON_ARRAY) return;
    if (arr->array_val.len == arr->array_val.cap) {
        long new_cap = arr->array_val.cap * 2;
        CyJsonValue** new_items = (CyJsonValue**)malloc(new_cap * sizeof(CyJsonValue*));
        memcpy(new_items, arr->array_val.items, arr->array_val.len * sizeof(CyJsonValue*));
        free(arr->array_val.items);
        arr->array_val.items = new_items;
        arr->array_val.cap = new_cap;
    }
    arr->array_val.items[arr->array_val.len] = val;
    arr->array_val.len++;
}

static void stringify_append(char** buf, long* len, long* cap, const char* str) {
    long slen = (long)strlen(str);
    while (*len + slen + 1 > *cap) {
        *cap *= 2;
        *buf = (char*)realloc(*buf, *cap);
    }
    memcpy(*buf + *len, str, slen);
    *len += slen;
    (*buf)[*len] = '\0';
}

static void stringify_append_char(char** buf, long* len, long* cap, char c) {
    if (*len + 2 > *cap) {
        *cap *= 2;
        *buf = (char*)realloc(*buf, *cap);
    }
    (*buf)[*len] = c;
    *len += 1;
    (*buf)[*len] = '\0';
}

static void stringify_value(CyJsonValue* v, char** buf, long* len, long* cap);

static void stringify_string(const char* s, char** buf, long* len, long* cap) {
    stringify_append_char(buf, len, cap, '"');
    for (const char* p = s; *p; p++) {
        switch (*p) {
            case '"':  stringify_append(buf, len, cap, "\\\""); break;
            case '\\': stringify_append(buf, len, cap, "\\\\"); break;
            case '\n': stringify_append(buf, len, cap, "\\n"); break;
            case '\t': stringify_append(buf, len, cap, "\\t"); break;
            case '\r': stringify_append(buf, len, cap, "\\r"); break;
            default:   stringify_append_char(buf, len, cap, *p); break;
        }
    }
    stringify_append_char(buf, len, cap, '"');
}

static void stringify_value(CyJsonValue* v, char** buf, long* len, long* cap) {
    if (!v || v->tag == JSON_NULL) {
        stringify_append(buf, len, cap, "null");
        return;
    }
    switch (v->tag) {
        case JSON_BOOL:
            stringify_append(buf, len, cap, v->bool_val ? "true" : "false");
            break;
        case JSON_INT: {
            char num[32];
            snprintf(num, sizeof(num), "%ld", v->int_val);
            stringify_append(buf, len, cap, num);
            break;
        }
        case JSON_STRING:
            stringify_string(v->string_val, buf, len, cap);
            break;
        case JSON_ARRAY:
            stringify_append_char(buf, len, cap, '[');
            for (long i = 0; i < v->array_val.len; i++) {
                if (i > 0) stringify_append_char(buf, len, cap, ',');
                stringify_value(v->array_val.items[i], buf, len, cap);
            }
            stringify_append_char(buf, len, cap, ']');
            break;
        case JSON_OBJECT:
            stringify_append_char(buf, len, cap, '{');
            for (long i = 0; i < v->object_val.len; i++) {
                if (i > 0) stringify_append_char(buf, len, cap, ',');
                stringify_string(v->object_val.keys[i], buf, len, cap);
                stringify_append_char(buf, len, cap, ':');
                stringify_value(v->object_val.values[i], buf, len, cap);
            }
            stringify_append_char(buf, len, cap, '}');
            break;
    }
}

char* cy_json_stringify(CyJsonValue* v) {
    long cap = 64;
    long len = 0;
    char* buf = (char*)malloc(cap);
    buf[0] = '\0';
    stringify_value(v, &buf, &len, &cap);
    return buf;
}

typedef struct {
    const char* src;
    long pos;
} JsonParser;

static void skip_whitespace(JsonParser* p) {
    while (p->src[p->pos] == ' ' || p->src[p->pos] == '\t' ||
           p->src[p->pos] == '\n' || p->src[p->pos] == '\r') {
        p->pos++;
    }
}

static CyJsonValue* parse_value(JsonParser* p);

static char* parse_string_raw(JsonParser* p) {
    if (p->src[p->pos] != '"') return NULL;
    p->pos++;
    long cap = 32;
    long len = 0;
    char* buf = (char*)malloc(cap);
    while (p->src[p->pos] && p->src[p->pos] != '"') {
        if (p->src[p->pos] == '\\') {
            p->pos++;
            char c;
            switch (p->src[p->pos]) {
                case '"':  c = '"'; break;
                case '\\': c = '\\'; break;
                case '/':  c = '/'; break;
                case 'n':  c = '\n'; break;
                case 't':  c = '\t'; break;
                case 'r':  c = '\r'; break;
                case 'b':  c = '\b'; break;
                case 'f':  c = '\f'; break;
                default:   c = p->src[p->pos]; break;
            }
            if (len + 1 >= cap) { cap *= 2; buf = (char*)realloc(buf, cap); }
            buf[len++] = c;
        } else {
            if (len + 1 >= cap) { cap *= 2; buf = (char*)realloc(buf, cap); }
            buf[len++] = p->src[p->pos];
        }
        p->pos++;
    }
    if (p->src[p->pos] == '"') p->pos++;
    buf[len] = '\0';
    return buf;
}

static CyJsonValue* parse_string(JsonParser* p) {
    char* s = parse_string_raw(p);
    if (!s) return NULL;
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_STRING;
    v->string_val = s;
    return v;
}

static CyJsonValue* parse_number(JsonParser* p) {
    char* end;
    long val = strtol(p->src + p->pos, &end, 10);
    if (end == p->src + p->pos) return NULL;
    p->pos = end - p->src;
    if (p->src[p->pos] == '.') {
        p->pos++;
        while (p->src[p->pos] >= '0' && p->src[p->pos] <= '9') p->pos++;
    }
    if (p->src[p->pos] == 'e' || p->src[p->pos] == 'E') {
        p->pos++;
        if (p->src[p->pos] == '+' || p->src[p->pos] == '-') p->pos++;
        while (p->src[p->pos] >= '0' && p->src[p->pos] <= '9') p->pos++;
    }
    CyJsonValue* v = (CyJsonValue*)malloc(sizeof(CyJsonValue));
    v->tag = JSON_INT;
    v->int_val = val;
    return v;
}

static CyJsonValue* parse_object(JsonParser* p) {
    p->pos++;
    CyJsonValue* obj = cy_json_object();
    skip_whitespace(p);
    if (p->src[p->pos] == '}') { p->pos++; return obj; }
    while (1) {
        skip_whitespace(p);
        char* key = parse_string_raw(p);
        if (!key) { return cy_json_null(); }
        skip_whitespace(p);
        if (p->src[p->pos] != ':') { free(key); return cy_json_null(); }
        p->pos++;
        skip_whitespace(p);
        CyJsonValue* val = parse_value(p);
        if (!val) { free(key); return cy_json_null(); }
        cy_json_set(obj, key, val);
        free(key);
        skip_whitespace(p);
        if (p->src[p->pos] == ',') { p->pos++; continue; }
        if (p->src[p->pos] == '}') { p->pos++; return obj; }
        return cy_json_null();
    }
}

static CyJsonValue* parse_array(JsonParser* p) {
    p->pos++;
    CyJsonValue* arr = cy_json_array();
    skip_whitespace(p);
    if (p->src[p->pos] == ']') { p->pos++; return arr; }
    while (1) {
        skip_whitespace(p);
        CyJsonValue* val = parse_value(p);
        if (!val) { return cy_json_null(); }
        cy_json_push(arr, val);
        skip_whitespace(p);
        if (p->src[p->pos] == ',') { p->pos++; continue; }
        if (p->src[p->pos] == ']') { p->pos++; return arr; }
        return cy_json_null();
    }
}

static int match_literal(JsonParser* p, const char* lit) {
    long l = (long)strlen(lit);
    if (strncmp(p->src + p->pos, lit, l) == 0) {
        p->pos += l;
        return 1;
    }
    return 0;
}

static CyJsonValue* parse_value(JsonParser* p) {
    skip_whitespace(p);
    char c = p->src[p->pos];
    if (c == '"') return parse_string(p);
    if (c == '{') return parse_object(p);
    if (c == '[') return parse_array(p);
    if (c == 't') { if (match_literal(p, "true"))  return cy_json_bool(1); return NULL; }
    if (c == 'f') { if (match_literal(p, "false")) return cy_json_bool(0); return NULL; }
    if (c == 'n') { if (match_literal(p, "null"))  return cy_json_null();  return NULL; }
    if (c == '-' || (c >= '0' && c <= '9')) return parse_number(p);
    return NULL;
}

CyJsonValue* cy_json_parse(const char* src) {
    if (!src) return cy_json_null();
    JsonParser p = { src, 0 };
    CyJsonValue* result = parse_value(&p);
    if (!result) return cy_json_null();
    return result;
}
