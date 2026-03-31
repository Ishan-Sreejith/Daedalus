/*
 * daedalus.h — Daedalus OS Kernel
 * Shared types, constants, and prototypes.
 */
#pragma once

#include <stddef.h>
#include <stdint.h>

/* ─── Limits ──────────────────────────────────────────────────────────────── */
#define MAX_NAME        64
#define MAX_PATH        256
#define MAX_CONTENT     4096
#define MAX_CHILDREN    32
#define HISTORY_SIZE    64
#define MAX_ALIASES     32
#define MAX_ENV         32
#define MAX_LINE        512

/* ─── ANSI Colors ─────────────────────────────────────────────────────────── */
#define ANSI_RESET      "\033[0m"
#define ANSI_BOLD       "\033[1m"
#define ANSI_DIM        "\033[2m"
#define ANSI_CYAN       "\033[36m"
#define ANSI_BCYAN      "\033[96m"
#define ANSI_GREEN      "\033[32m"
#define ANSI_BGREEN     "\033[92m"
#define ANSI_YELLOW     "\033[33m"
#define ANSI_BYELLOW    "\033[93m"
#define ANSI_RED        "\033[31m"
#define ANSI_BRED       "\033[91m"
#define ANSI_BLUE       "\033[34m"
#define ANSI_BBLUE      "\033[94m"
#define ANSI_MAGENTA    "\033[35m"
#define ANSI_BMAGENTA   "\033[95m"
#define ANSI_WHITE      "\033[37m"
#define ANSI_BWHITE     "\033[97m"
#define ANSI_GRAY       "\033[90m"

/* ─── Filesystem ──────────────────────────────────────────────────────────── */
typedef enum {
    NODE_FILE,
    NODE_DIR,
} FSNodeType;

typedef struct FSNode FSNode;

struct FSNode {
    FSNodeType  type;
    char        name[MAX_NAME];
    /* file */
    char        content[MAX_CONTENT];
    /* dir */
    FSNode     *children[MAX_CHILDREN];
    int         child_count;
};

/* ─── Shell State ─────────────────────────────────────────────────────────── */
typedef struct {
    char    key[MAX_NAME];
    char    val[MAX_NAME];
} KVPair;

typedef struct {
    FSNode *fs_root;
    char    cwd[MAX_PATH];

    /* history */
    char    history[HISTORY_SIZE][MAX_LINE];
    int     hist_count;

    /* aliases */
    KVPair  aliases[MAX_ALIASES];
    int     alias_count;

    /* env vars */
    KVPair  envvars[MAX_ENV];
    int     env_count;

    /* repeat loop */
    volatile int loop_active;
    char         loop_cmd[MAX_LINE];

    /* boot time */
    long boot_time;
} ShellState;

/* ─── fs.c prototypes ─────────────────────────────────────────────────────── */
FSNode *fs_init(void);
FSNode *fs_get_node(FSNode *root, const char *path);
int     fs_ensure_parent(FSNode *root, const char *path,
                         FSNode **parent_out, char name_out[MAX_NAME]);
void    fs_list_dir(FSNode *node, char *buf, size_t bufsz);
int     fs_make_file(FSNode *root, const char *path);
int     fs_make_dir(FSNode *root, const char *path);
int     fs_delete(FSNode *root, const char *path, int recursive);
int     fs_copy(FSNode *root, const char *src, const char *dst);
int     fs_move(FSNode *root, const char *src, const char *dst);
void    fs_free(FSNode *node);

/* ─── shell.c prototypes ──────────────────────────────────────────────────── */
ShellState *shell_init(FSNode *fs);
void        shell_free(ShellState *s);
void        shell_execute(ShellState *s, const char *line, int from_loop);
void        shell_prompt(ShellState *s);
void        shell_interrupt(ShellState *s);
