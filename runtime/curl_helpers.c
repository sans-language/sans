#include <curl/curl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

/* ── Low-level curl wrappers (unchanged) ─────────────────────────── */

long cy_curl_init(void) {
    CURL* h = curl_easy_init();
    return (long)h;
}

long cy_curl_setopt_str(long handle, long opt, const char* val) {
    return (long)curl_easy_setopt((CURL*)handle, (CURLoption)opt, val);
}

long cy_curl_setopt_long(long handle, long opt, long val) {
    return (long)curl_easy_setopt((CURL*)handle, (CURLoption)opt, val);
}

long cy_curl_perform(long handle) {
    return (long)curl_easy_perform((CURL*)handle);
}

long cy_curl_cleanup(long handle) {
    curl_easy_cleanup((CURL*)handle);
    return 0;
}

long cy_curl_getinfo(long handle, long info, long buf) {
    return (long)curl_easy_getinfo((CURL*)handle, (CURLINFO)info, (void*)buf);
}

/* ── HTTP response struct ────────────────────────────────────────── */
/*
 * Layout (48 bytes, 6 x i64 fields):
 *   offset  0: status_code  (long)
 *   offset  8: body         (char*)
 *   offset 16: header_names (char**)
 *   offset 24: header_values(char**)
 *   offset 32: header_count (long)
 *   offset 40: header_cap   (long)
 *
 * Sans accessor functions (cy_http_status, cy_http_body, cy_http_ok)
 * read these offsets directly via load64().
 */

typedef struct CyHttpResponse {
    long status_code;
    char* body;
    char** header_names;
    char** header_values;
    long header_count;
    long header_cap;
} CyHttpResponse;

/* ── Curl callbacks (must be C function pointers) ────────────────── */

typedef struct {
    char* data;
    long size;
    long cap;
} WriteBuffer;

static size_t write_callback(char* ptr, size_t size, size_t nmemb, void* userdata) {
    WriteBuffer* buf = (WriteBuffer*)userdata;
    long bytes = (long)(size * nmemb);
    while (buf->size + bytes + 1 > buf->cap) {
        buf->cap = buf->cap ? buf->cap * 2 : 1024;
        buf->data = realloc(buf->data, buf->cap);
    }
    memcpy(buf->data + buf->size, ptr, bytes);
    buf->size += bytes;
    buf->data[buf->size] = '\0';
    return (size_t)bytes;
}

static void add_header(CyHttpResponse* resp, const char* name, long name_len, const char* value, long value_len) {
    if (resp->header_count >= resp->header_cap) {
        resp->header_cap = resp->header_cap ? resp->header_cap * 2 : 16;
        resp->header_names = realloc(resp->header_names, sizeof(char*) * resp->header_cap);
        resp->header_values = realloc(resp->header_values, sizeof(char*) * resp->header_cap);
    }
    char* n = malloc(name_len + 1);
    for (long i = 0; i < name_len; i++) {
        n[i] = (char)tolower((unsigned char)name[i]);
    }
    n[name_len] = '\0';

    char* v = malloc(value_len + 1);
    memcpy(v, value, value_len);
    v[value_len] = '\0';

    resp->header_names[resp->header_count] = n;
    resp->header_values[resp->header_count] = v;
    resp->header_count++;
}

static size_t header_callback(char* buffer, size_t size, size_t nitems, void* userdata) {
    CyHttpResponse* resp = (CyHttpResponse*)userdata;
    long len = (long)(size * nitems);

    char* colon = memchr(buffer, ':', len);
    if (!colon) return (size_t)len;

    long name_len = colon - buffer;
    char* val_start = colon + 1;
    long remaining = len - (val_start - buffer);
    while (remaining > 0 && (*val_start == ' ' || *val_start == '\t')) {
        val_start++;
        remaining--;
    }
    while (remaining > 0 && (val_start[remaining - 1] == '\r' || val_start[remaining - 1] == '\n')) {
        remaining--;
    }

    add_header(resp, buffer, name_len, val_start, remaining);
    return (size_t)len;
}

/* ── Error / init helpers ────────────────────────────────────────── */

static CyHttpResponse* make_error_response(void) {
    CyHttpResponse* resp = malloc(sizeof(CyHttpResponse));
    resp->status_code = 0;
    resp->body = malloc(1);
    resp->body[0] = '\0';
    resp->header_names = NULL;
    resp->header_values = NULL;
    resp->header_count = 0;
    resp->header_cap = 0;
    return resp;
}

static CyHttpResponse* init_response(void) {
    CyHttpResponse* resp = malloc(sizeof(CyHttpResponse));
    resp->status_code = 0;
    resp->body = NULL;
    resp->header_names = NULL;
    resp->header_values = NULL;
    resp->header_count = 0;
    resp->header_cap = 0;
    return resp;
}

/* ── High-level HTTP functions (moved from http.c) ───────────────── */

CyHttpResponse* cy_http_get(const char* url) {
    CURL* curl = curl_easy_init();
    if (!curl) return make_error_response();

    CyHttpResponse* resp = init_response();
    WriteBuffer buf = {NULL, 0, 0};

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_MAXREDIRS, 10L);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buf);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, header_callback);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, resp);

    CURLcode res = curl_easy_perform(curl);
    if (res != CURLE_OK) {
        curl_easy_cleanup(curl);
        free(buf.data);
        free(resp->header_names);
        free(resp->header_values);
        free(resp);
        return make_error_response();
    }

    long status;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    resp->status_code = status;
    if (buf.data) {
        resp->body = buf.data;
    } else {
        resp->body = malloc(1);
        resp->body[0] = '\0';
    }

    curl_easy_cleanup(curl);
    return resp;
}

CyHttpResponse* cy_http_post(const char* url, const char* body, const char* content_type) {
    CURL* curl = curl_easy_init();
    if (!curl) return make_error_response();

    CyHttpResponse* resp = init_response();
    WriteBuffer buf = {NULL, 0, 0};

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_MAXREDIRS, 10L);
    curl_easy_setopt(curl, CURLOPT_POST, 1L);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)strlen(body));
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buf);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, header_callback);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, resp);

    struct curl_slist* headers = NULL;
    char header_buf[512];
    snprintf(header_buf, sizeof(header_buf), "Content-Type: %s", content_type);
    headers = curl_slist_append(headers, header_buf);
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);

    CURLcode res = curl_easy_perform(curl);
    curl_slist_free_all(headers);

    if (res != CURLE_OK) {
        curl_easy_cleanup(curl);
        free(buf.data);
        free(resp->header_names);
        free(resp->header_values);
        free(resp);
        return make_error_response();
    }

    long status;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    resp->status_code = status;
    if (buf.data) {
        resp->body = buf.data;
    } else {
        resp->body = malloc(1);
        resp->body[0] = '\0';
    }

    curl_easy_cleanup(curl);
    return resp;
}

/* Header lookup (needs C for case-insensitive compare with stored lowercase keys) */
char* cy_http_header(CyHttpResponse* resp, const char* name) {
    long name_len = (long)strlen(name);
    char* lower_name = malloc(name_len + 1);
    for (long i = 0; i < name_len; i++) {
        lower_name[i] = (char)tolower((unsigned char)name[i]);
    }
    lower_name[name_len] = '\0';

    for (long i = 0; i < resp->header_count; i++) {
        if (strcmp(resp->header_names[i], lower_name) == 0) {
            free(lower_name);
            return resp->header_values[i];
        }
    }
    free(lower_name);

    char* empty = malloc(1);
    empty[0] = '\0';
    return empty;
}
