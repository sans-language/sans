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

CyHttpServer* cy_http_listen(long port) {
    int server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd < 0) {
        fprintf(stderr, "http_listen: socket() failed\n");
        CyHttpServer* s = malloc(sizeof(CyHttpServer));
        s->server_fd = -1;
        s->port = (int)port;
        return s;
    }

    int opt = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((uint16_t)port);

    if (bind(server_fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        fprintf(stderr, "http_listen: bind() failed on port %ld\n", port);
        close(server_fd);
        CyHttpServer* s = malloc(sizeof(CyHttpServer));
        s->server_fd = -1;
        s->port = (int)port;
        return s;
    }

    if (listen(server_fd, 128) < 0) {
        fprintf(stderr, "http_listen: listen() failed\n");
        close(server_fd);
        CyHttpServer* s = malloc(sizeof(CyHttpServer));
        s->server_fd = -1;
        s->port = (int)port;
        return s;
    }

    CyHttpServer* s = malloc(sizeof(CyHttpServer));
    s->server_fd = server_fd;
    s->port = (int)port;
    return s;
}

CyHttpRequest* cy_http_accept(CyHttpServer* server) {
    if (server->server_fd < 0) {
        CyHttpRequest* req = malloc(sizeof(CyHttpRequest));
        req->client_fd = -1;
        req->method = strdup("");
        req->path = strdup("");
        req->body = strdup("");
        return req;
    }

    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept(server->server_fd, (struct sockaddr*)&client_addr, &client_len);
    if (client_fd < 0) {
        CyHttpRequest* req = malloc(sizeof(CyHttpRequest));
        req->client_fd = -1;
        req->method = strdup("");
        req->path = strdup("");
        req->body = strdup("");
        return req;
    }

    /* Read the request */
    char buf[8192];
    long n = read(client_fd, buf, sizeof(buf) - 1);
    if (n <= 0) {
        close(client_fd);
        CyHttpRequest* req = malloc(sizeof(CyHttpRequest));
        req->client_fd = -1;
        req->method = strdup("");
        req->path = strdup("");
        req->body = strdup("");
        return req;
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
    char* body;
    if (body_start) {
        body = strdup(body_start + 4);
    } else {
        body = strdup("");
    }

    CyHttpRequest* req = malloc(sizeof(CyHttpRequest));
    req->client_fd = client_fd;
    req->method = method;
    req->path = path;
    req->body = body;
    return req;
}

char* cy_http_request_path(CyHttpRequest* req) {
    return req->path;
}

char* cy_http_request_method(CyHttpRequest* req) {
    return req->method;
}

char* cy_http_request_body(CyHttpRequest* req) {
    return req->body;
}

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
        "Content-Type: text/plain\r\n"
        "Connection: close\r\n"
        "\r\n",
        status, status_text, body_len);

    write(req->client_fd, header, header_len);
    write(req->client_fd, body, body_len);
    close(req->client_fd);
    req->client_fd = -1;
    return 1;
}
