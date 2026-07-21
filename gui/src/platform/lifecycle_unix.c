#if defined(__APPLE__) || defined(__linux__)

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <pthread.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

extern void sd300_platform_open(void);
extern void sd300_platform_quit(void);

static int sd300_lifecycle_fd = -1;
static char sd300_lifecycle_dir[PATH_MAX];
static char sd300_lifecycle_path[PATH_MAX];

static int sd300_owned_private_directory(const char *path) {
    struct stat info;
    if (lstat(path, &info) != 0) return 0;
    return S_ISDIR(info.st_mode) && info.st_uid == geteuid() &&
           (info.st_mode & 0077) == 0;
}

static int sd300_prepare_directory(void) {
#if defined(__linux__)
    const char *xdg_runtime = getenv("XDG_RUNTIME_DIR");
    if (xdg_runtime != NULL && xdg_runtime[0] == '/') {
        if (snprintf(sd300_lifecycle_dir, sizeof(sd300_lifecycle_dir),
                     "%s/sd300", xdg_runtime) >=
            (int)sizeof(sd300_lifecycle_dir)) {
            return 0;
        }
    } else
#endif
    {
        if (snprintf(sd300_lifecycle_dir, sizeof(sd300_lifecycle_dir),
                     "/tmp/sd300-%lu", (unsigned long)geteuid()) >=
            (int)sizeof(sd300_lifecycle_dir)) {
            return 0;
        }
    }

    if (mkdir(sd300_lifecycle_dir, 0700) != 0 && errno != EEXIST) return 0;
    if (!sd300_owned_private_directory(sd300_lifecycle_dir)) return 0;
    if (snprintf(sd300_lifecycle_path, sizeof(sd300_lifecycle_path),
                 "%s/gui.sock", sd300_lifecycle_dir) >=
        (int)sizeof(sd300_lifecycle_path)) {
        return 0;
    }
    return strlen(sd300_lifecycle_path) < sizeof(((struct sockaddr_un *)0)->sun_path);
}

static int sd300_connect_and_send(const char *command) {
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) return 0;
    struct sockaddr_un address;
    memset(&address, 0, sizeof(address));
    address.sun_family = AF_UNIX;
    memcpy(address.sun_path, sd300_lifecycle_path,
           strlen(sd300_lifecycle_path) + 1);
    int connected = connect(fd, (const struct sockaddr *)&address,
                            sizeof(address)) == 0;
    if (connected) {
        size_t length = strlen(command);
        connected = write(fd, command, length) == (ssize_t)length;
    }
    close(fd);
    return connected;
}

static int sd300_owned_socket(void) {
    struct stat info;
    if (lstat(sd300_lifecycle_path, &info) != 0) return errno == ENOENT;
    return S_ISSOCK(info.st_mode) && info.st_uid == geteuid();
}

static void sd300_lifecycle_cleanup(void) {
    if (sd300_lifecycle_fd >= 0) {
        close(sd300_lifecycle_fd);
        sd300_lifecycle_fd = -1;
    }
    if (sd300_lifecycle_path[0] != '\0') unlink(sd300_lifecycle_path);
    if (sd300_lifecycle_dir[0] != '\0') rmdir(sd300_lifecycle_dir);
}

static void *sd300_lifecycle_thread(void *unused) {
    (void)unused;
    for (;;) {
        int client = accept(sd300_lifecycle_fd, NULL, NULL);
        if (client < 0) {
            if (errno == EINTR) continue;
            return NULL;
        }
        char command[32] = {0};
        ssize_t count = read(client, command, sizeof(command) - 1);
        close(client);
        if (count <= 0) continue;
        if (strcmp(command, "open\n") == 0) {
            sd300_platform_open();
        } else if (strcmp(command, "quit\n") == 0) {
            sd300_platform_quit();
            return NULL;
        }
    }
}

// Returns 1 for the primary instance and 0 after routing this launch to an
// already running per-user instance. The socket lives in a mode-0700,
// same-UID directory and itself is mode 0600.
int sd300_claim_unix_instance(void) {
    if (!sd300_prepare_directory()) return -1;
    if (sd300_connect_and_send("open\n")) return 0;
    struct stat existing;
    if (lstat(sd300_lifecycle_path, &existing) == 0) {
        // A first instance may have bound immediately before its accept loop
        // became schedulable. Give that atomic bind a bounded chance to win
        // before treating an owned endpoint as stale.
        for (int attempt = 0; attempt < 40; ++attempt) {
            usleep(25000);
            if (sd300_connect_and_send("open\n")) return 0;
        }
    }
    if (!sd300_owned_socket()) return -1;
    if (unlink(sd300_lifecycle_path) != 0 && errno != ENOENT) return -1;

    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    (void)fcntl(fd, F_SETFD, FD_CLOEXEC);
    struct sockaddr_un address;
    memset(&address, 0, sizeof(address));
    address.sun_family = AF_UNIX;
    memcpy(address.sun_path, sd300_lifecycle_path,
           strlen(sd300_lifecycle_path) + 1);
    if (bind(fd, (const struct sockaddr *)&address, sizeof(address)) != 0 ||
        chmod(sd300_lifecycle_path, 0600) != 0 || listen(fd, 4) != 0) {
        close(fd);
        unlink(sd300_lifecycle_path);
        return -1;
    }
    sd300_lifecycle_fd = fd;
    atexit(sd300_lifecycle_cleanup);
    pthread_t thread;
    if (pthread_create(&thread, NULL, sd300_lifecycle_thread, NULL) != 0) {
        sd300_lifecycle_cleanup();
        return -1;
    }
    pthread_detach(thread);
    return 1;
}

#endif
