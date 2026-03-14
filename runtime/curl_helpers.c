#include <curl/curl.h>

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
