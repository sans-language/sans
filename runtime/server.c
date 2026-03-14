#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

typedef struct CyHttpServer {
    int server_fd;
    int port;
} CyHttpServer;

typedef struct CyHttpRequest {
    int client_fd;
    char* method;
    char* path;
    char* body;
} CyHttpRequest;

static CyHttpServer* make_error_server(int port) {
    CyHttpServer* s = malloc(sizeof(CyHttpServer));
    s->server_fd = -1;
    s->port = port;
    return s;
}

CyHttpServer* cy_http_listen(long port) {
    /* Try IPv6 dual-stack first (browsers prefer IPv6) */
    int server_fd = socket(AF_INET6, SOCK_STREAM, 0);
    if (server_fd >= 0) {
        int off = 0;
        setsockopt(server_fd, IPPROTO_IPV6, IPV6_V6ONLY, &off, sizeof(off));
        int opt = 1;
        setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

        struct sockaddr_in6 addr6;
        memset(&addr6, 0, sizeof(addr6));
        addr6.sin6_family = AF_INET6;
        addr6.sin6_addr = in6addr_any;
        addr6.sin6_port = htons((uint16_t)port);

        if (bind(server_fd, (struct sockaddr*)&addr6, sizeof(addr6)) < 0) {
            close(server_fd);
            server_fd = -1; /* fall through to IPv4 */
        }
    }

    /* Fallback to IPv4 */
    if (server_fd < 0) {
        server_fd = socket(AF_INET, SOCK_STREAM, 0);
        if (server_fd < 0) {
            fprintf(stderr, "http_listen: socket() failed\n");
            return make_error_server((int)port);
        }
        int opt = 1;
        setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

        struct sockaddr_in addr4;
        memset(&addr4, 0, sizeof(addr4));
        addr4.sin_family = AF_INET;
        addr4.sin_addr.s_addr = INADDR_ANY;
        addr4.sin_port = htons((uint16_t)port);

        if (bind(server_fd, (struct sockaddr*)&addr4, sizeof(addr4)) < 0) {
            fprintf(stderr, "http_listen: bind() failed on port %ld\n", port);
            close(server_fd);
            return make_error_server((int)port);
        }
    }

    if (listen(server_fd, 128) < 0) {
        fprintf(stderr, "http_listen: listen() failed\n");
        close(server_fd);
        return make_error_server((int)port);
    }

    CyHttpServer* s = malloc(sizeof(CyHttpServer));
    s->server_fd = server_fd;
    s->port = (int)port;
    return s;
}

static CyHttpRequest* make_empty_request(void) {
    CyHttpRequest* req = malloc(sizeof(CyHttpRequest));
    req->client_fd = -1;
    req->method = strdup("");
    req->path = strdup("");
    req->body = strdup("");
    return req;
}

CyHttpRequest* cy_http_accept(CyHttpServer* server) {
    if (server->server_fd < 0) return make_empty_request();

    struct sockaddr_storage client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept(server->server_fd, (struct sockaddr*)&client_addr, &client_len);
    if (client_fd < 0) return make_empty_request();

    /* Read the request */
    char buf[8192];
    long n = read(client_fd, buf, sizeof(buf) - 1);
    if (n <= 0) {
        close(client_fd);
        return make_empty_request();
    }
    buf[n] = '\0';

    /* Parse method and path from first line: "GET /path HTTP/1.1\r\n" */
    char* method_end = strchr(buf, ' ');
    char* path_start = method_end ? method_end + 1 : buf;
    char* path_end = strchr(path_start, ' ');

    long method_len = method_end ? (method_end - buf) : 0;
    long path_len = path_end ? (path_end - path_start) : 0;

    char* method = malloc(method_len + 1);
    memcpy(method, buf, method_len);
    method[method_len] = '\0';

    char* path = malloc(path_len + 1);
    memcpy(path, path_start, path_len);
    path[path_len] = '\0';

    /* Find body (after \r\n\r\n) */
    char* body_start = strstr(buf, "\r\n\r\n");
    char* body = body_start ? strdup(body_start + 4) : strdup("");

    CyHttpRequest* req = malloc(sizeof(CyHttpRequest));
    req->client_fd = client_fd;
    req->method = method;
    req->path = path;
    req->body = body;
    return req;
}

char* cy_http_request_path(CyHttpRequest* req) { return req->path; }
char* cy_http_request_method(CyHttpRequest* req) { return req->method; }
char* cy_http_request_body(CyHttpRequest* req) { return req->body; }

long cy_http_respond(CyHttpRequest* req, long status, const char* body) {
    if (req->client_fd < 0) return 0;

    const char* status_text = "OK";
    if (status == 404) status_text = "Not Found";
    else if (status == 500) status_text = "Internal Server Error";
    else if (status == 400) status_text = "Bad Request";

    long body_len = (long)strlen(body);
    char header[512];
    int header_len = snprintf(header, sizeof(header),
        "HTTP/1.1 %ld %s\r\n"
        "Content-Length: %ld\r\n"
        "Content-Type: text/html; charset=utf-8\r\n"
        "Connection: close\r\n"
        "\r\n",
        status, status_text, body_len);

    /* Send header and body together to avoid partial writes */
    char* full_response = malloc(header_len + body_len + 1);
    memcpy(full_response, header, header_len);
    memcpy(full_response + header_len, body, body_len);
    long total = header_len + body_len;
    long sent = 0;
    while (sent < total) {
        long n = write(req->client_fd, full_response + sent, total - sent);
        if (n <= 0) break;
        sent += n;
    }
    free(full_response);
    shutdown(req->client_fd, SHUT_RDWR);
    close(req->client_fd);
    req->client_fd = -1;
    return 1;
}
