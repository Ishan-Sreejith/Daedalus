/*
 * main.c — Daedalus OS Kernel Entry Point
 *
 * Sets up the shell state, installs the SIGINT handler,
 * prints the boot banner, and runs the REPL loop.
 */
#include "daedalus.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <termios.h>
#include <unistd.h>

struct termios orig_termios;

void disable_raw_mode(void) {
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
}

void enable_raw_mode(void) {
    tcgetattr(STDIN_FILENO, &orig_termios);
    atexit(disable_raw_mode);
    struct termios raw = orig_termios;
    raw.c_lflag &= ~(ECHO | ICANON | ISIG);
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
}

/* Global shell state pointer so the signal handler can reach it */
static ShellState *g_state = NULL;

static void sigint_handler(int sig) {
    (void)sig;
    if (g_state) shell_interrupt(g_state);
    /* Re-arm — POSIX signals are reset-once on some systems */
    signal(SIGINT, sigint_handler);
    /* Re-print prompt after interrupt */
    if (g_state) shell_prompt(g_state);
}

int main(void) {
    /* Init filesystem + shell */
    FSNode     *fs = fs_init();
    ShellState  *s = shell_init(fs);
    g_state = s;

    /* Install SIGINT handler */
    signal(SIGINT, sigint_handler);

    /* Boot banner */
    fputs(
        "\033[2J\033[H"      /* clear screen */
        "\n"
        "  ____                _       _\n"
        " |  _ \\  __ _  ___  __| | __ _| |_   _ ___\n"
        " | | | |/ _` |/ _ \\/ _` |/ _` | | | | / __|\n"
        " | |_| | (_| |  __/ (_| | (_| | | |_| \\__ \\\n"
        " |____/ \\__,_|\\___|\\__,_|\\__,_|_|\\__,_|___/\n"
        "\033[0m",
        stdout
    );
    /* Color it after (avoids ANSI in the raw string above for readability) */
    fputs(
        "\033[96m\033[1m"
        "  ____                _       _\n"
        "\033[0m",
        stdout
    );
    /* Print clean version */
    fputs("\033[2J\033[H", stdout);
    printf(
        "\033[96m\033[1m"
        "  ____                _       _\n"
        " |  _ \\  __ _  ___  __| | __ _| |_   _ ___\n"
        " | | | |/ _` |/ _ \\/ _` |/ _` | | | | / __|\n"
        " | |_| | (_| |  __/ (_| | (_| | | |_| \\__ \\\n"
        " |____/ \\__,_|\\___|\\__,_|\\__,_|_|\\__,_|___/\n"
        "\033[0m"
        "\033[90m"
        "  Daedalus OS v2.0  ·  C kernel  ·  type 'help' to begin\n"
        "  ────────────────────────────────────────────────────────\n"
        "\033[0m\n",
        stdout
    );
    fflush(stdout);

    /* REPL */
    enable_raw_mode();
    shell_prompt(s);
    char line[MAX_LINE];
    int len = 0;
    line[0] = '\0';
    int hist_idx = s->hist_count;

    while (1) {
        char c;
        if (read(STDIN_FILENO, &c, 1) != 1) break;

        if (c == '\r' || c == '\n') {
            write(STDOUT_FILENO, "\r\n", 2);
            line[len] = '\0';
            /* Only execute if we have typed something */
            shell_execute(s, line, 0);
            len = 0;
            line[0] = '\0';
            hist_idx = s->hist_count;
        } else if (c == 3) { /* Ctrl-C */
            shell_interrupt(s);
            len = 0;
            line[0] = '\0';
            hist_idx = s->hist_count;
            shell_prompt(s);
        } else if (c == 4) { /* Ctrl-D */
            break;
        } else if (c == 127 || c == 8) { /* Backspace */
            if (len > 0) {
                len--;
                line[len] = '\0';
                write(STDOUT_FILENO, "\b \b", 3);
            }
        } else if (c == 27) { /* Escape seq */
            char seq[3];
            if (read(STDIN_FILENO, &seq[0], 1) != 1) continue;
            if (read(STDIN_FILENO, &seq[1], 1) != 1) continue;
            if (seq[0] == '[') {
                if (seq[1] == 'A') { /* Up */
                    if (hist_idx > 0) {
                        hist_idx--;
                        while(len > 0) { write(STDOUT_FILENO, "\b \b", 3); len--; }
                        strncpy(line, s->history[hist_idx], MAX_LINE-1);
                        len = strlen(line);
                        write(STDOUT_FILENO, line, len);
                    }
                } else if (seq[1] == 'B') { /* Down */
                    if (hist_idx < s->hist_count) {
                        hist_idx++;
                        while(len > 0) { write(STDOUT_FILENO, "\b \b", 3); len--; }
                        if (hist_idx < s->hist_count) {
                            strncpy(line, s->history[hist_idx], MAX_LINE-1);
                        } else {
                            line[0] = '\0';
                        }
                        len = strlen(line);
                        write(STDOUT_FILENO, line, len);
                    }
                }
            }
        } else if (c >= 32 && c <= 126) {
            if (len < MAX_LINE - 1) {
                line[len++] = c;
                line[len] = '\0';
                write(STDOUT_FILENO, &c, 1);
            }
        }
    }
    disable_raw_mode();

    /* EOF — clean shutdown */
    printf("\n\033[92mDaedalus OS halted.\033[0m\n");
    shell_free(s);
    return 0;
}
