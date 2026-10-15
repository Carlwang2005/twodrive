#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <pthread.h>
#include <unistd.h>

// Model a slow shared sync queue only inside the disposable core test process.
static pthread_mutex_t sync_gate = PTHREAD_MUTEX_INITIALIZER;

static int delayed_sync(int fd, const char *symbol) {
    int (*original)(int) = dlsym(RTLD_NEXT, symbol);
    pthread_mutex_lock(&sync_gate);
    usleep(40000);
    int result = original(fd);
    int saved_errno = errno;
    pthread_mutex_unlock(&sync_gate);
    errno = saved_errno;
    return result;
}

int fsync(int fd) {
    return delayed_sync(fd, "fsync");
}

int fdatasync(int fd) {
    return delayed_sync(fd, "fdatasync");
}
