#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <curl/curl.h>

typedef struct CyHttpResponse {
    long status_code;
    char* body;
    long body_len;
    char** header_names;
    char** header_values;
    long header_count;
    long header_cap;
} CyHttpResponse;

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

    /* Find colon separator */
    char* colon = memchr(buffer, ':', len);
    if (!colon) return (size_t)len; /* status line or empty line */

    long name_len = colon - buffer;
    /* Skip colon and leading whitespace in value */
    char* val_start = colon + 1;
    long remaining = len - (val_start - buffer);
    while (remaining > 0 && (*val_start == ' ' || *val_start == '\t')) {
        val_start++;
        remaining--;
    }
    /* Trim trailing \r\n */
    while (remaining > 0 && (val_start[remaining - 1] == '\r' || val_start[remaining - 1] == '\n')) {
        remaining--;
    }

    add_header(resp, buffer, name_len, val_start, remaining);
    return (size_t)len;
}

static CyHttpResponse* make_error_response(void) {
    CyHttpResponse* resp = malloc(sizeof(CyHttpResponse));
    resp->status_code = 0;
    resp->body = malloc(1);
    resp->body[0] = '\0';
    resp->body_len = 0;
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
    resp->body_len = 0;
    resp->header_names = NULL;
    resp->header_values = NULL;
    resp->header_count = 0;
    resp->header_cap = 0;
    return resp;
}

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
        resp->body_len = buf.size;
    } else {
        resp->body = malloc(1);
        resp->body[0] = '\0';
        resp->body_len = 0;
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

    /* Set Content-Type header */
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
        resp->body_len = buf.size;
    } else {
        resp->body = malloc(1);
        resp->body[0] = '\0';
        resp->body_len = 0;
    }

    curl_easy_cleanup(curl);
    return resp;
}

long cy_http_status(CyHttpResponse* resp) {
    return resp->status_code;
}

char* cy_http_body(CyHttpResponse* resp) {
    return resp->body;
}

char* cy_http_header(CyHttpResponse* resp, const char* name) {
    /* Lowercase the search name for case-insensitive comparison */
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

    /* Not found — return empty string */
    char* empty = malloc(1);
    empty[0] = '\0';
    return empty;
}

long cy_http_ok(CyHttpResponse* resp) {
    return (resp->status_code >= 200 && resp->status_code < 300) ? 1 : 0;
}
