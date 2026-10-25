/*
 * Discord Quest Completer - Linux dummy game runner.
 *
 * Discord on Linux (1.0.x, discord_utils.node) detects games like this:
 *
 *   1. the client fetches https://cdn.discordapp.com/detectables/games-v1.json
 *      (24883 apps, every platform's executables) and hands *all* of them to
 *      the native process observer through setObservedGamesCallback2();
 *   2. process_observer_linux.cpp polls /proc/<pid>/exe, ignores system paths
 *      (/bin, /sbin, wine-preloader, proton / steamlinuxruntime / steamworks
 *      shared) and matches the lowercase executable file name against those
 *      candidates;
 *   3. a match becomes a "running game", which is what quest heartbeats use.
 *      No window is required, unlike the Windows / macOS runners.
 *
 * Therefore: this binary is copied to <app-id>/<executable name from the
 * detectable list> and executed. The file name IS the registration, argv[0]
 * basename is the fallback, and comm (PR_SET_NAME, 15 byte cap) is set so the
 * process shows the game name in ps/top as well.
 *
 * Arguments:
 *   --title <name>      cosmetic, ignored on Linux (kept for parity)
 *   --duration <secs>   exit after N seconds, 0 or absent = run forever
 *   anything else       kept in our command line on purpose: some detectable
 *                       executables are registered together with a required
 *                       argument and Discord matches against the cmdline.
 *
 * Build: ./build.sh (gcc) or `cargo build --release` (build.rs -> cc).
 */

#define _GNU_SOURCE
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <time.h>
#include <unistd.h>

static volatile sig_atomic_t g_run = 1;

static void on_signal(int sig) {
    (void)sig;
    g_run = 0;
}

static const char *base_name(const char *path) {
    const char *slash = strrchr(path, '/');
    return slash ? slash + 1 : path;
}

int main(int argc, char **argv) {
    const char *title = NULL;
    long duration = 0;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--title") == 0 && i + 1 < argc) {
            title = argv[++i];
        } else if (strcmp(argv[i], "--duration") == 0 && i + 1 < argc) {
            duration = strtol(argv[++i], NULL, 10);
        } else if (strcmp(argv[i], "--help") == 0) {
            fprintf(stderr,
                    "usage: %s [--title <name>] [--duration <seconds>] [game args...]\n",
                    base_name(argv[0]));
            return 0;
        }
        /* unknown args stay in argv: Discord may match the command line */
    }

    /* comm is capped at 15 bytes, a longer name is not an error */
    char comm[16];
    const char *prog = base_name(argv[0]);
    size_t len = strlen(prog);
    if (len > 15) len = 15;
    memcpy(comm, prog, len);
    comm[len] = '\0';
    prctl(PR_SET_NAME, comm, 0, 0, 0);
    (void)title;

    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = on_signal;
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);
    /* survive the launcher and the terminal going away */
    signal(SIGHUP, SIG_IGN);
    signal(SIGPIPE, SIG_IGN);

    if (duration > 0) {
        struct timespec started;
        clock_gettime(CLOCK_MONOTONIC, &started);
        struct timespec tick = {0, 250 * 1000 * 1000};
        for (;;) {
            if (!g_run) break;
            nanosleep(&tick, NULL);
            struct timespec now;
            clock_gettime(CLOCK_MONOTONIC, &now);
            long long elapsed = (long long)(now.tv_sec - started.tv_sec);
            if (elapsed >= duration) break;
        }
    } else {
        while (g_run) pause();
    }

    return 0;
}
