#include <sys/socket.h>
#include <netinet/in.h>
#include <string.h>

long cy_bind_port(long fd, long port) {
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((uint16_t)port);
    int opt = 1;
    setsockopt((int)fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    return bind((int)fd, (struct sockaddr*)&addr, sizeof(addr));
}
