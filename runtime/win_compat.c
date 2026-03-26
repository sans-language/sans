// Windows compatibility shim
// Maps POSIX poll() to WSAPoll() for MinGW builds
#ifdef _WIN32
#include <winsock2.h>

int poll(struct pollfd *fds, int nfds, int timeout) {
    return WSAPoll(fds, nfds, timeout);
}
#endif
