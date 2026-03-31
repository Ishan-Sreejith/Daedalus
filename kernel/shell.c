#include "daedalus.h"

#include <ctype.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

extern void fs_path_normalize(const char *src, char *out, size_t outsz);
extern void fs_path_resolve(const char *cwd, const char *input, char *out,
                            size_t outsz);

static void print_colored(const char *color, const char *msg) {
  printf("%s%s%s\n", color, msg, ANSI_RESET);
  fflush(stdout);
}
static void print_error(const char *msg) { print_colored(ANSI_BRED, msg); }
static void print_info(const char *msg) { print_colored(ANSI_BGREEN, msg); }
static void print_warn(const char *msg) { print_colored(ANSI_BYELLOW, msg); }

static const char *TRAIN_ART = ANSI_YELLOW
    "      ====        ________                ___________\n"
    "  _D _|  |_______/        \\__I_I_____===__|_________|\n"
    "   |(_)---  |   H\\________/ |   |        =|___ ___|\n"
    "   /     |  |   H  |  |     |   |         ||_| |_||\n"
    "  |      |  |   H  |__--------------------| [___] |\n"
    "  | ________|___H__/__|_____/[][]~\\_______|       |\n"
    "  |/ |   |-----------I_____I [][] []  D   |=======|__\n"
    " __/ =| o |=-~~\\  /~~\\  /~~\\  /~~\\ ____Y___________|__\n"
    " |/-=|___|=O=====O=====O=====O=====|_____/~\\___/\n" ANSI_RESET;

typedef struct {
  ShellState *state;
  char cmd[MAX_LINE];
} LoopArg;

static void *repeat_thread(void *arg) {
  LoopArg *la = (LoopArg *)arg;
  ShellState *s = la->state;
  while (s->loop_active) {
    shell_execute(s, la->cmd, 1);
    struct timespec ts = {.tv_sec = 0, .tv_nsec = 20 * 1000 * 1000};
    nanosleep(&ts, NULL);
  }
  free(la);
  return NULL;
}

/* ─── ShellState lifecycle ───────────────────────────────────────────────── */
ShellState *shell_init(FSNode *fs) {
  ShellState *s = calloc(1, sizeof(ShellState));
  if (!s) {
    perror("calloc");
    exit(1);
  }
  s->fs_root = fs;
  s->boot_time = (long)time(NULL);
  strncpy(s->cwd, "/home/daedalus", MAX_PATH - 1);
  return s;
}

void shell_free(ShellState *s) {
  if (!s)
    return;
  fs_free(s->fs_root);
  free(s);
}

/* ─── Prompt ────────────────────────────────────────────────────────────────
 */
void shell_prompt(ShellState *s) {
  const char *home = "/home/daedalus";
  const char *suffix = "";
  if (strncmp(s->cwd, home, strlen(home)) == 0)
    suffix = s->cwd + strlen(home);
  else
    suffix = s->cwd;

  printf(ANSI_BCYAN ANSI_BOLD "daedalus" ANSI_RESET ANSI_CYAN
                              "%s" ANSI_RESET ANSI_BWHITE "::" ANSI_RESET " ",
         suffix);
  fflush(stdout);
}

/* ─── History ───────────────────────────────────────────────────────────────
 */
static void history_push(ShellState *s, const char *line) {
  if (s->hist_count < HISTORY_SIZE) {
    strncpy(s->history[s->hist_count++], line, MAX_LINE - 1);
  } else {
    memmove(s->history[0], s->history[1], (HISTORY_SIZE - 1) * MAX_LINE);
    strncpy(s->history[HISTORY_SIZE - 1], line, MAX_LINE - 1);
  }
}

/* ─── Alias helpers ─────────────────────────────────────────────────────────
 */
static const char *alias_lookup(ShellState *s, const char *key) {
  for (int i = 0; i < s->alias_count; i++)
    if (strcmp(s->aliases[i].key, key) == 0)
      return s->aliases[i].val;
  return NULL;
}

static void alias_set(ShellState *s, const char *key, const char *val) {
  for (int i = 0; i < s->alias_count; i++) {
    if (strcmp(s->aliases[i].key, key) == 0) {
      strncpy(s->aliases[i].val, val, MAX_NAME - 1);
      return;
    }
  }
  if (s->alias_count < MAX_ALIASES) {
    strncpy(s->aliases[s->alias_count].key, key, MAX_NAME - 1);
    strncpy(s->aliases[s->alias_count].val, val, MAX_NAME - 1);
    s->alias_count++;
  }
}

static void alias_unset(ShellState *s, const char *key) {
  for (int i = 0; i < s->alias_count; i++) {
    if (strcmp(s->aliases[i].key, key) == 0) {
      for (int j = i; j < s->alias_count - 1; j++)
        s->aliases[j] = s->aliases[j + 1];
      s->alias_count--;
      return;
    }
  }
}

/* ─── Env helpers ───────────────────────────────────────────────────────────
 */
static void env_set(ShellState *s, const char *key, const char *val) {
  for (int i = 0; i < s->env_count; i++) {
    if (strcmp(s->envvars[i].key, key) == 0) {
      strncpy(s->envvars[i].val, val, MAX_NAME - 1);
      return;
    }
  }
  if (s->env_count < MAX_ENV) {
    strncpy(s->envvars[s->env_count].key, key, MAX_NAME - 1);
    strncpy(s->envvars[s->env_count].val, val, MAX_NAME - 1);
    s->env_count++;
  }
}

static void env_unset(ShellState *s, const char *key) {
  for (int i = 0; i < s->env_count; i++) {
    if (strcmp(s->envvars[i].key, key) == 0) {
      for (int j = i; j < s->env_count - 1; j++)
        s->envvars[j] = s->envvars[j + 1];
      s->env_count--;
      return;
    }
  }
}

/* ─── String utilities ──────────────────────────────────────────────────────
 */

/* Trim leading/trailing whitespace in-place. Returns pointer to first non-WS */
static char *strtrim(char *s) {
  while (isspace((unsigned char)*s))
    s++;
  char *end = s + strlen(s) - 1;
  while (end > s && isspace((unsigned char)*end))
    *end-- = '\0';
  return s;
}

/* Split 'line' into argc/argv (max MAX_ARGS tokens). Modifies line in-place. */
#define MAX_ARGS 32
static int tokenize(char *line, char *argv[]) {
  int argc = 0;
  char *tok = strtok(line, " \t");
  while (tok && argc < MAX_ARGS) {
    argv[argc++] = tok;
    tok = strtok(NULL, " \t");
  }
  return argc;
}

/* ─── Interrupt ─────────────────────────────────────────────────────────────
 */
void shell_interrupt(ShellState *s) {
  if (s->loop_active) {
    s->loop_active = 0;
    printf("\n");
    print_warn("Loop interrupted.");
  } else {
    printf("\n");
  }
  fflush(stdout);
}

/* ══════════════════════════════════════════════════════════════════════════════
 * shell_execute — main command dispatcher
 * ══════════════════════════════════════════════════════════════════════════════
 */
void shell_execute(ShellState *s, const char *raw_line, int from_loop) {
  /* Make a mutable copy */
  char buf[MAX_LINE];
  strncpy(buf, raw_line, MAX_LINE - 1);
  char *line = strtrim(buf);

  if (!line || line[0] == '\0') {
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* Comments */
  if (line[0] == '#') {
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* Push to history (not for loop-internal executions) */
  if (!from_loop)
    history_push(s, line);

  /* ── Alias expansion ─────────────────────────────────────────────────── */
  char expanded[MAX_LINE];
  {
    char tmp2[MAX_LINE];
    strncpy(tmp2, line, MAX_LINE - 1);
    char *sp = strchr(tmp2, ' ');
    char *rest_alias = sp ? sp + 1 : NULL;
    if (sp)
      *sp = '\0';
    const char *aval = alias_lookup(s, tmp2);
    if (aval) {
      if (rest_alias)
        snprintf(expanded, MAX_LINE, "%s %s", aval, rest_alias);
      else
        strncpy(expanded, aval, MAX_LINE - 1);
      line = expanded;
    }
  }

  /* ── Tokenise ─────────────────────────────────────────────────────────── */
  char arg_buf[MAX_LINE];
  strncpy(arg_buf, line, MAX_LINE - 1);
  char *argv[MAX_ARGS];
  int argc = tokenize(arg_buf, argv);
  if (argc == 0) {
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  const char *cmd = argv[0];

  /* ══════════════════════════════════════════════════
   * BUILT-IN COMMANDS
   * ══════════════════════════════════════════════════ */

  /* ── help ──────────────────────────────────────────────────── */
  if (strcmp(cmd, "help") == 0) {
    printf(
        ANSI_BCYAN ANSI_BOLD
        "\n  Daedalus OS — Command Reference\n"
        "  ─────────────────────────────────────────\n" ANSI_RESET ANSI_BWHITE
        "  Navigation\n" ANSI_RESET "    here             " ANSI_GRAY
        "print working directory\n" ANSI_RESET "    list [path]      " ANSI_GRAY
        "list directory contents\n" ANSI_RESET "    go [path]        " ANSI_GRAY
        "change directory\n" ANSI_RESET ANSI_BWHITE "  Files\n" ANSI_RESET
        "    read <file>      " ANSI_GRAY "print file contents\n" ANSI_RESET
        "    new <file>       " ANSI_GRAY "create empty file\n" ANSI_RESET
        "    make <dir>       " ANSI_GRAY "create directory\n" ANSI_RESET
        "    del [-r] <path>  " ANSI_GRAY
        "delete file or directory\n" ANSI_RESET
        "    write <f> <text> " ANSI_GRAY
        "overwrite file with text\n" ANSI_RESET
        "    mod <file>       " ANSI_GRAY
        "open line editor (.save/.cancel)\n" ANSI_RESET
        "    emod <f> <text>  " ANSI_GRAY "inline overwrite\n" ANSI_RESET
        "    find <pat> <f>   " ANSI_GRAY "grep pattern in file\n" ANSI_RESET
        "    wc <file>        " ANSI_GRAY
        "word/line/char count\n" ANSI_RESET ANSI_BWHITE "  Output\n" ANSI_RESET
        "    say <text>       " ANSI_GRAY
        "print text (> or >> to redirect)\n" ANSI_RESET ANSI_BWHITE
        "  System\n" ANSI_RESET "    sys cpu|memory|info|version\n"
        "    whoami           " ANSI_GRAY "print current user\n" ANSI_RESET
        "    uptime           " ANSI_GRAY
        "print seconds since boot\n" ANSI_RESET
        "    ports            " ANSI_GRAY "list /ports devices\n" ANSI_RESET
        "    ping <h> <port>  " ANSI_GRAY
        "mock network probe\n" ANSI_RESET ANSI_BWHITE "  Shell\n" ANSI_RESET
        "    r - <cmd>        " ANSI_GRAY
        "repeat command (Ctrl+C to stop)\n" ANSI_RESET
        "    history          " ANSI_GRAY "show command history\n" ANSI_RESET
        "    alias k v        " ANSI_GRAY "set alias k → v\n" ANSI_RESET
        "    alias            " ANSI_GRAY "list all aliases\n" ANSI_RESET
        "    unalias k        " ANSI_GRAY "remove alias\n" ANSI_RESET
        "    set k v          " ANSI_GRAY "set env variable\n" ANSI_RESET
        "    env              " ANSI_GRAY "list env variables\n" ANSI_RESET
        "    unset k          " ANSI_GRAY "remove env variable\n" ANSI_RESET
        "    cls / clear      " ANSI_GRAY "clear terminal\n" ANSI_RESET
        "    desk             " ANSI_GRAY "desktop (web UI only)\n" ANSI_RESET
        "    train            " ANSI_GRAY "sl\n" ANSI_RESET
        "    exit / quit      " ANSI_GRAY "exit the kernel\n" ANSI_RESET "\n");
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── exit / quit ───────────────────────────────────────────── */
  if (strcmp(cmd, "exit") == 0 || strcmp(cmd, "quit") == 0) {
    print_info("Daedalus OS halted. Goodbye.");
    shell_free(s);
    exit(0);
  }

  /* ── here (pwd) ────────────────────────────────────────────── */
  if (strcmp(cmd, "here") == 0) {
    printf(ANSI_BBLUE "%s\n" ANSI_RESET, s->cwd);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── list (ls) ─────────────────────────────────────────────── */
  if (strcmp(cmd, "list") == 0) {
    const char *path_arg = argc > 1 ? argv[1] : ".";
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, path_arg, target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node) {
      char err[MAX_PATH + 64];
      snprintf(err, sizeof(err),
               "list: cannot access '%s': No such file or directory", path_arg);
      print_error(err);
    } else if (node->type == NODE_FILE) {
      printf("%s\n", node->name);
    } else {
      char listing[MAX_CONTENT];
      fs_list_dir(node, listing, sizeof(listing));
      printf("%s\n", listing);
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── go (cd) ───────────────────────────────────────────────── */
  if (strcmp(cmd, "go") == 0) {
    const char *path_arg = argc > 1 ? argv[1] : "/home/daedalus";
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, path_arg, target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node || node->type != NODE_DIR) {
      char err[MAX_PATH + 32];
      snprintf(err, sizeof(err), "go: %s: No such directory", path_arg);
      print_error(err);
    } else {
      strncpy(s->cwd, target, MAX_PATH - 1);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── read (cat) ────────────────────────────────────────────── */
  if (strcmp(cmd, "read") == 0) {
    if (argc < 2) {
      print_error("read: usage: read <file>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node || node->type != NODE_FILE) {
      char err[MAX_PATH + 32];
      snprintf(err, sizeof(err), "read: %s: No such file", argv[1]);
      print_error(err);
    } else {
      fputs(node->content, stdout);
      if (node->content[0] && node->content[strlen(node->content) - 1] != '\n')
        putchar('\n');
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── new (touch) ───────────────────────────────────────────── */
  if (strcmp(cmd, "new") == 0) {
    if (argc < 2) {
      print_error("new: usage: new <file>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    if (fs_make_file(s->fs_root, target) < 0) {
      char err[MAX_PATH + 48];
      snprintf(err, sizeof(err), "new: cannot create '%s': No such directory",
               argv[1]);
      print_error(err);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── make (mkdir) ──────────────────────────────────────────── */
  if (strcmp(cmd, "make") == 0) {
    if (argc < 2) {
      print_error("make: usage: make <dir>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    if (fs_make_dir(s->fs_root, target) < 0) {
      char err[MAX_PATH + 48];
      snprintf(err, sizeof(err), "make: cannot create '%s': No such directory",
               argv[1]);
      print_error(err);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── cp ────────────────────────────────────────────────────── */
  if (strcmp(cmd, "cp") == 0) {
    if (argc < 3) {
      print_error("cp: usage: cp <src> <dst>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char src[MAX_PATH];
    char dst[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], src, MAX_PATH);
    fs_path_resolve(s->cwd, argv[2], dst, MAX_PATH);
    int r = fs_copy(s->fs_root, src, dst);
    if (r == -1)
      print_error("cp: cannot stat file or directory");
    else if (r == -2)
      print_error("cp: destination exists");
    else if (r == -3)
      print_error("cp: memory allocation failed");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── mv ────────────────────────────────────────────────────── */
  if (strcmp(cmd, "mv") == 0) {
    if (argc < 3) {
      print_error("mv: usage: mv <src> <dst>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char src[MAX_PATH];
    char dst[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], src, MAX_PATH);
    fs_path_resolve(s->cwd, argv[2], dst, MAX_PATH);
    int r = fs_move(s->fs_root, src, dst);
    if (r == -1)
      print_error("mv: cannot stat file or directory");
    else if (r == -2)
      print_error("mv: destination exists");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── del (rm) ──────────────────────────────────────────────── */
  if (strcmp(cmd, "del") == 0) {
    int recursive = 0;
    const char *path_arg = NULL;
    for (int i = 1; i < argc; i++) {
      if (strcmp(argv[i], "-r") == 0 || strcmp(argv[i], "-rf") == 0)
        recursive = 1;
      else
        path_arg = argv[i];
    }
    if (!path_arg) {
      print_error("del: usage: del [-r] <path>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, path_arg, target, MAX_PATH);
    int r = fs_delete(s->fs_root, target, recursive);
    if (r == -1) {
      char err[MAX_PATH + 48];
      snprintf(err, sizeof(err),
               "del: cannot remove '%s': No such file or directory", path_arg);
      print_error(err);
    } else if (r == -2) {
      char err[MAX_PATH + 48];
      snprintf(err, sizeof(err),
               "del: cannot remove '%s': Directory not empty (use -r)",
               path_arg);
      print_error(err);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── say (echo) — supports > and >> redirection ────────────── */
  if (strcmp(cmd, "say") == 0) {
    if (argc < 2) {
      printf("\n");
      fflush(stdout);
      if (!from_loop)
        shell_prompt(s);
      return;
    }

    /* Reconstruct the rest of the line after "say " */
    const char *rest = strstr(raw_line, "say ");
    if (!rest)
      rest = "";
    else
      rest += 4;

    /* Check for redirection */
    const char *redir_gg = strstr(rest, " >> ");
    const char *redir_g = strstr(rest, " > ");

    if (redir_gg) {
      char msg[MAX_CONTENT];
      char file_arg[MAX_NAME];
      size_t mlen = (size_t)(redir_gg - rest);
      strncpy(msg, rest, mlen);
      msg[mlen] = '\0';
      strncpy(file_arg, redir_gg + 4, MAX_NAME - 1);
      char target[MAX_PATH];
      fs_path_resolve(s->cwd, file_arg, target, MAX_PATH);
      FSNode *parent;
      char leaf[MAX_NAME];
      if (fs_ensure_parent(s->fs_root, target, &parent, leaf) < 0) {
        print_error("say: No such file or directory");
      } else {
        FSNode *node = fs_get_node(s->fs_root, target);
        if (!node) {
          fs_make_file(s->fs_root, target);
          node = fs_get_node(s->fs_root, target);
        }
        if (node)
          strncat(node->content, msg, MAX_CONTENT - strlen(node->content) - 1);
      }
    } else if (redir_g) {
      char msg[MAX_CONTENT];
      char file_arg[MAX_NAME];
      size_t mlen = (size_t)(redir_g - rest);
      strncpy(msg, rest, mlen);
      msg[mlen] = '\0';
      strncpy(file_arg, redir_g + 3, MAX_NAME - 1);
      char target[MAX_PATH];
      fs_path_resolve(s->cwd, file_arg, target, MAX_PATH);
      if (fs_ensure_parent(s->fs_root, target, NULL, NULL) == 0 || 1) {
        FSNode *node = fs_get_node(s->fs_root, target);
        if (!node) {
          fs_make_file(s->fs_root, target);
          node = fs_get_node(s->fs_root, target);
        }
        if (node && node->type == NODE_FILE) {
          strncpy(node->content, msg, MAX_CONTENT - 1);
          strncat(node->content, "\n", MAX_CONTENT - strlen(node->content) - 1);
        }
      }
    } else {
      printf("%s\n", rest);
      fflush(stdout);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── write ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "write") == 0) {
    if (argc < 3) {
      print_error("write: usage: write <file> <text>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node) {
      fs_make_file(s->fs_root, target);
      node = fs_get_node(s->fs_root, target);
    }
    if (!node || node->type != NODE_FILE) {
      print_error("write: not a file");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    /* reconstruct text from argv[2..] */
    char text[MAX_CONTENT] = "";
    for (int i = 2; i < argc; i++) {
      if (i > 2)
        strncat(text, " ", MAX_CONTENT - strlen(text) - 1);
      strncat(text, argv[i], MAX_CONTENT - strlen(text) - 1);
    }
    strncat(text, "\n", MAX_CONTENT - strlen(text) - 1);
    strncpy(node->content, text, MAX_CONTENT - 1);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── emod ──────────────────────────────────────────────────── */
  if (strcmp(cmd, "emod") == 0) {
    if (argc < 3) {
      print_error("emod: usage: emod <file> <text>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node) {
      fs_make_file(s->fs_root, target);
      node = fs_get_node(s->fs_root, target);
    }
    char text[MAX_CONTENT] = "";
    for (int i = 2; i < argc; i++) {
      if (i > 2)
        strncat(text, " ", MAX_CONTENT - strlen(text) - 1);
      strncat(text, argv[i], MAX_CONTENT - strlen(text) - 1);
    }
    strncat(text, "\n", MAX_CONTENT - strlen(text) - 1);
    strncpy(node->content, text, MAX_CONTENT - 1);
    char msg[MAX_NAME + 32];
    snprintf(msg, sizeof(msg), "emod: wrote %s", argv[1]);
    print_info(msg);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── mod (nano-like editor) ────────────────────────────────── */
  if (strcmp(cmd, "mod") == 0) {
    if (argc < 2) {
      print_error("mod: usage: mod <file>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    if (fs_ensure_parent(s->fs_root, target, NULL, NULL) < 0 &&
        fs_get_node(s->fs_root, target) == NULL) {
      print_error("mod: No such directory");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node) {
      fs_make_file(s->fs_root, target);
      node = fs_get_node(s->fs_root, target);
    }
    if (!node || node->type != NODE_FILE) {
      print_error("mod: not a file");
      if (!from_loop)
        shell_prompt(s);
      return;
    }

    printf(ANSI_BYELLOW
           "─── Daedalus Editor ── %s ─────────────────\n" ANSI_RESET,
           argv[1]);
    printf(ANSI_GRAY "  ^S  type .save to save\n"
                     "  ^C  type .cancel to discard\n" ANSI_RESET);
    /* Show existing content */
    if (node->content[0]) {
      printf(ANSI_DIM "--- existing content ---\n" ANSI_RESET);
      fputs(node->content, stdout);
      printf(ANSI_DIM "--- end ---\n" ANSI_RESET);
    }
    printf(ANSI_BWHITE "Enter new content:\n" ANSI_RESET);

    char new_content[MAX_CONTENT] = "";
    char edit_line[MAX_LINE];
    while (fgets(edit_line, MAX_LINE, stdin)) {
      /* strip newline */
      edit_line[strcspn(edit_line, "\n")] = '\0';
      if (strcmp(edit_line, ".save") == 0) {
        strncpy(node->content, new_content, MAX_CONTENT - 1);
        print_info("[mod] saved.");
        break;
      }
      if (strcmp(edit_line, ".cancel") == 0) {
        print_warn("[mod] cancelled.");
        break;
      }
      strncat(new_content, edit_line, MAX_CONTENT - strlen(new_content) - 2);
      strncat(new_content, "\n", MAX_CONTENT - strlen(new_content) - 1);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── find (grep) ───────────────────────────────────────────── */
  if (strcmp(cmd, "find") == 0) {
    if (argc < 3) {
      print_error("find: usage: find <pattern> <file>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[2], target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node || node->type != NODE_FILE) {
      char err[MAX_PATH + 32];
      snprintf(err, sizeof(err), "find: %s: No such file", argv[2]);
      print_error(err);
    } else {
      char content_copy[MAX_CONTENT];
      strncpy(content_copy, node->content, MAX_CONTENT - 1);
      char *saveptr;
      char *tok2 = strtok_r(content_copy, "\n", &saveptr);
      int found = 0;
      int lineno = 1;
      while (tok2) {
        if (strstr(tok2, argv[1])) {
          printf(ANSI_GRAY "%3d:" ANSI_RESET " %s\n", lineno, tok2);
          found = 1;
        }
        tok2 = strtok_r(NULL, "\n", &saveptr);
        lineno++;
      }
      if (!found)
        print_warn("find: no matches.");
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── wc ────────────────────────────────────────────────────── */
  if (strcmp(cmd, "wc") == 0) {
    if (argc < 2) {
      print_error("wc: usage: wc <file>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    char target[MAX_PATH];
    fs_path_resolve(s->cwd, argv[1], target, MAX_PATH);
    FSNode *node = fs_get_node(s->fs_root, target);
    if (!node || node->type != NODE_FILE) {
      char err[MAX_PATH + 32];
      snprintf(err, sizeof(err), "wc: %s: No such file", argv[1]);
      print_error(err);
    } else {
      int lines = 0, words = 0, chars = (int)strlen(node->content);
      int in_word = 0;
      for (int i = 0; node->content[i]; i++) {
        char c = node->content[i];
        if (c == '\n')
          lines++;
        if (isspace((unsigned char)c))
          in_word = 0;
        else {
          if (!in_word) {
            words++;
            in_word = 1;
          }
        }
      }
      printf(ANSI_BWHITE "%4d %4d %5d " ANSI_RESET "%s\n", lines, words, chars,
             argv[1]);
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── sys ───────────────────────────────────────────────────── */
  if (strcmp(cmd, "sys") == 0) {
    if (argc < 2) {
      print_warn("sys: usage: sys cpu|memory|info|version");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    if (strcmp(argv[1], "cpu") == 0)
      print_info("CPU: RP2040 — Dual-core ARM Cortex-M0+ @ 133 MHz");
    else if (strcmp(argv[1], "memory") == 0)
      print_info("MEM: 264 KB SRAM  |  2 MB Flash");
    else if (strcmp(argv[1], "info") == 0)
      print_info("RP2040  133 MHz  264 KB RAM  2 MB flash  [Daedalus OS v2.0]");
    else if (strcmp(argv[1], "version") == 0)
      print_info("Daedalus OS v2.0  (C kernel build)");
    else {
      char err[64];
      snprintf(err, sizeof(err), "sys: unknown sub-command '%s'", argv[1]);
      print_error(err);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── whoami ────────────────────────────────────────────────── */
  if (strcmp(cmd, "whoami") == 0) {
    print_info("daedalus");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── uptime ────────────────────────────────────────────────── */
  if (strcmp(cmd, "uptime") == 0) {
    long up = (long)time(NULL) - s->boot_time;
    printf(ANSI_BGREEN "up %ld seconds\n" ANSI_RESET, up);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── ping (nc) ─────────────────────────────────────────────── */
  if (strcmp(cmd, "ping") == 0) {
    const char *host = argc > 1 ? argv[1] : "localhost";
    const char *port = argc > 2 ? argv[2] : "0";
    printf(ANSI_BGREEN "ping: probing %s:%s ... " ANSI_BWHITE "open" ANSI_GRAY
                       " (simulated)\n" ANSI_RESET,
           host, port);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── ports ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "ports") == 0) {
    char target[MAX_PATH];
    strncpy(target, "/ports", MAX_PATH - 1);
    FSNode *node = fs_get_node(s->fs_root, target);
    char listing[MAX_CONTENT];
    fs_list_dir(node, listing, sizeof(listing));
    printf("Devices in /ports:\n  %s\n", listing);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── train (sl) ────────────────────────────────────────────── */
  if (strcmp(cmd, "train") == 0) {
    fputs(TRAIN_ART, stdout);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── desk ──────────────────────────────────────────────────── */
  if (strcmp(cmd, "desk") == 0) {
    print_warn("Desktop mode is only available in the web UI (index.html).");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── cls / clear ───────────────────────────────────────────── */
  if (strcmp(cmd, "cls") == 0 || strcmp(cmd, "clear") == 0) {
    fputs("\033[2J\033[H", stdout);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── history ───────────────────────────────────────────────── */
  if (strcmp(cmd, "history") == 0) {
    if (s->hist_count == 0) {
      print_warn("history: no commands yet.");
    } else {
      int start = s->hist_count > 20 ? s->hist_count - 20 : 0;
      for (int i = start; i < s->hist_count; i++)
        printf(ANSI_GRAY "%3d " ANSI_RESET "%s\n", i + 1, s->history[i]);
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── alias ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "alias") == 0) {
    if (argc == 1) {
      if (s->alias_count == 0)
        print_warn("alias: no aliases set.");
      else
        for (int i = 0; i < s->alias_count; i++)
          printf(ANSI_BYELLOW "alias " ANSI_BWHITE "%s" ANSI_RESET "='%s'\n",
                 s->aliases[i].key, s->aliases[i].val);
    } else if (argc >= 3) {
      alias_set(s, argv[1], argv[2]);
      printf(ANSI_BGREEN "alias set: " ANSI_RESET "%s → %s\n", argv[1],
             argv[2]);
    } else {
      print_error("alias: usage: alias <key> <value>");
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── unalias ───────────────────────────────────────────────── */
  if (strcmp(cmd, "unalias") == 0) {
    if (argc < 2) {
      print_error("unalias: usage: unalias <key>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    alias_unset(s, argv[1]);
    print_info("alias removed.");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── set ───────────────────────────────────────────────────── */
  if (strcmp(cmd, "set") == 0) {
    if (argc >= 3) {
      env_set(s, argv[1], argv[2]);
      printf(ANSI_BGREEN "set: " ANSI_RESET "%s=%s\n", argv[1], argv[2]);
    } else {
      print_error("set: usage: set <key> <value>");
    }
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── env ───────────────────────────────────────────────────── */
  if (strcmp(cmd, "env") == 0) {
    if (s->env_count == 0)
      print_warn("env: no variables set.");
    else
      for (int i = 0; i < s->env_count; i++)
        printf(ANSI_BYELLOW "%s" ANSI_RESET "=%s\n", s->envvars[i].key,
               s->envvars[i].val);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── unset ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "unset") == 0) {
    if (argc < 2) {
      print_error("unset: usage: unset <key>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    env_unset(s, argv[1]);
    print_info("unset done.");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── r - (repeat loop) ─────────────────────────────────────── */
  if (strncmp(line, "r - ", 4) == 0) {
    const char *loop_cmd = line + 4;
    if (!loop_cmd || loop_cmd[0] == '\0') {
      print_error("r: usage: r - <command>");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    if (s->loop_active) {
      print_warn(
          "r: a loop is already running. Press Ctrl+C to stop it first.");
      if (!from_loop)
        shell_prompt(s);
      return;
    }
    s->loop_active = 1;
    LoopArg *la = malloc(sizeof(LoopArg));
    la->state = s;
    strncpy(la->cmd, loop_cmd, MAX_LINE - 1);

    pthread_t tid;
    pthread_attr_t attr;
    pthread_attr_init(&attr);
    pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
    pthread_create(&tid, &attr, repeat_thread, la);
    pthread_attr_destroy(&attr);

    printf(ANSI_BGREEN "Repeating: " ANSI_RESET "%s  " ANSI_GRAY
                       "(Ctrl+C to stop)\n" ANSI_RESET,
           loop_cmd);
    fflush(stdout);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── mock commands ─────────────────────────────────────────── */
  static const char *mock_cmds[] = {"scan",  "remove", "run",  "start", "stop",
                                    "reset", "peek",   "sync", NULL};
  for (int i = 0; mock_cmds[i]; i++) {
    if (strcmp(cmd, mock_cmds[i]) == 0) {
      printf(ANSI_GRAY "[mock] %s executed.\n" ANSI_RESET, cmd);
      fflush(stdout);
      if (!from_loop)
        shell_prompt(s);
      return;
    }
  }

  /* ── unknown ───────────────────────────────────────────────── */
  char err[MAX_LINE + 32];
  snprintf(err, sizeof(err), "daedalus: command not found: %s  (try 'help')",
           cmd);
  print_error(err);
  if (!from_loop)
    shell_prompt(s);
}
