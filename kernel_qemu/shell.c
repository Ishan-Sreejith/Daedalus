#include "daedalus.h"
#include "libc.h"

extern void fs_path_normalize(const char *src, char *out, size_t outsz);
extern void fs_path_resolve(const char *cwd, const char *input, char *out,
                            size_t outsz);

static void print_colored(const char *color, const char *msg) {
  printf("%s%s%s\n", color, msg, ANSI_RESET);
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

/* ─── ShellState lifecycle ───────────────────────────────────────────────── */
ShellState *shell_init(FSNode *fs) {
  ShellState *s = calloc(1, sizeof(ShellState));
  if (!s) {
    printf("Error: shell_init calloc failed\n");
    while(1);
  }
  s->fs_root = fs;
  s->boot_time = 0; // Simplified
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
                              "%s" ANSI_RESET ANSI_BWHITE " $ " ANSI_RESET,
         suffix);
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

  /* Push to history (not for loop-al executions) */
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
   * COMMANDS
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
        "    train            " ANSI_GRAY "sl\n" ANSI_RESET
        "    exit / quit      " ANSI_GRAY "exit the kernel\n" ANSI_RESET "\n");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── exit / quit ───────────────────────────────────────────── */
  if (strcmp(cmd, "exit") == 0 || strcmp(cmd, "quit") == 0) {
    print_info("Daedalus OS halted. Goodbye.");
    while(1) { __asm__("hlt"); }
  }

  /* ── here (pwd) ────────────────────────────────────────────── */
  if (strcmp(cmd, "here") == 0) {
    printf(ANSI_BBLUE "%s\n" ANSI_RESET, s->cwd);
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
      printf("%s", node->content);
      if (node->content[0] && node->content[strlen(node->content) - 1] != '\n')
        printf("\n");
    }
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

  /* ── del (rm) ──────────────────────────────────────────────── */
  if (strcmp(cmd, "del") == 0) {
    int recursive = 0;
    const char *path_arg = NULL;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-r") == 0) recursive = 1;
        else path_arg = argv[i];
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
      print_error("del: Directory not empty (use -r)");
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── say (echo) — supports > and >> redirection ────────────── */
  if (strcmp(cmd, "say") == 0) {
    if (argc < 2) {
      printf("\n");
      if (!from_loop)
        shell_prompt(s);
      return;
    }

    /* Reconstruct text from argv[1..] */
    char msg[MAX_CONTENT] = "";
    int redir_idx = -1;
    int append = 0;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], ">") == 0) { redir_idx = i + 1; break; }
        if (strcmp(argv[i], ">>") == 0) { redir_idx = i + 1; append = 1; break; }
        if (i > 1) strncat(msg, " ", MAX_CONTENT - strlen(msg) - 1);
        strncat(msg, argv[i], MAX_CONTENT - strlen(msg) - 1);
    }

    if (redir_idx != -1 && redir_idx < argc) {
      char target[MAX_PATH];
      fs_path_resolve(s->cwd, argv[redir_idx], target, MAX_PATH);
      FSNode *node = fs_get_node(s->fs_root, target);
      if (!node) {
        fs_make_file(s->fs_root, target);
        node = fs_get_node(s->fs_root, target);
      }
      if (node && node->type == NODE_FILE) {
        if (!append) node->content[0] = '\0';
        strncat(node->content, msg, MAX_CONTENT - strlen(node->content) - 1);
        strncat(node->content, "\n", MAX_CONTENT - strlen(node->content) - 1);
      } else {
        print_error("say: invalid redirect target");
      }
    } else {
      printf("%s\n", msg);
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
    char text[MAX_CONTENT] = "";
    for (int i = 2; i < argc; i++) {
      if (i > 2) strncat(text, " ", MAX_CONTENT - strlen(text) - 1);
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
        if (i > 2) strncat(text, " ", MAX_CONTENT - strlen(text) - 1);
        strncat(text, argv[i], MAX_CONTENT - strlen(text) - 1);
    }
    strncat(text, "\n", MAX_CONTENT - strlen(text) - 1);
    strncpy(node->content, text, MAX_CONTENT - 1);
    printf("emod: wrote %s\n", argv[1]);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── mod ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "mod") == 0) {
    if (argc < 2) {
      print_error("mod: usage: mod <file>");
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
      print_error("mod: not a file");
      if (!from_loop)
        shell_prompt(s);
      return;
    }

    printf(ANSI_BYELLOW "── Daedalus Editor ── %s ──\n" ANSI_RESET, argv[1]);
    printf("  type '.save' to save, '.cancel' to discard\n");

    char new_content[MAX_CONTENT] = "";
    char edit_line[MAX_LINE];
    while (fgets(edit_line, MAX_LINE, stdin)) {
      edit_line[strcspn(edit_line, "\n")] = '\0';
      if (strcmp(edit_line, ".save") == 0) {
        strncpy(node->content, new_content, MAX_CONTENT - 1);
        print_info("Saved.");
        break;
      }
      if (strcmp(edit_line, ".cancel") == 0) {
        print_warn("Cancelled.");
        break;
      }
      strncat(new_content, edit_line, MAX_CONTENT - strlen(new_content) - 2);
      strncat(new_content, "\n", MAX_CONTENT - strlen(new_content) - 1);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── find ─────────────────────────────────────────────────── */
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
      print_error("find: file not found");
    } else {
      char content_copy[MAX_CONTENT];
      strncpy(content_copy, node->content, MAX_CONTENT - 1);
      char *saveptr;
      char *tok2 = strtok_r(content_copy, "\n", &saveptr);
      int found = 0;
      while (tok2) {
        if (strstr(tok2, argv[1])) {
          printf("%s\n", tok2);
          found = 1;
        }
        tok2 = strtok_r(NULL, "\n", &saveptr);
      }
      if (!found) print_warn("No matches.");
    }
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
      print_error("wc: file not found");
    } else {
      int lines = 0, words = 0, chars = (int)strlen(node->content);
      int in_word = 0;
      for (int i = 0; node->content[i]; i++) {
        char c = node->content[i];
        if (c == '\n') lines++;
        if (isspace((unsigned char)c)) in_word = 0;
        else if (!in_word) { words++; in_word = 1; }
      }
      printf("%d %d %d %s\n", lines, words, chars, argv[1]);
    }
    if (!from_loop)
        shell_prompt(s);
    return;
  }

  /* ── sys ───────────────────────────────────────────────────── */
  if (strcmp(cmd, "sys") == 0) {
    const char *sub = argc > 1 ? argv[1] : "info";
    if (strcmp(sub, "cpu") == 0) printf("x86 32-bit Bare-Metal\n");
    else if (strcmp(sub, "memory") == 0) printf("1MB static heap\n");
    else if (strcmp(sub, "version") == 0) printf("Daedalus Runtime v2.1 (Handcrafted)\n");
    else printf("Daedalus OS x86 Kernel\n");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── whoami ────────────────────────────────────────────────── */
  if (strcmp(cmd, "whoami") == 0) {
    printf("daedalus-pilot\n");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── uptime ────────────────────────────────────────────────── */
  if (strcmp(cmd, "uptime") == 0) {
    printf("0s (bare-metal mock)\n");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── ports ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "ports") == 0) {
    char listing[MAX_CONTENT];
    FSNode *ports = fs_get_node(s->fs_root, "/ports");
    if (ports) {
      fs_list_dir(ports, listing, sizeof(listing));
      printf("%s\n", listing);
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── r - <cmd> (loop) ───────────────────────────────────────── */
  if (strcmp(cmd, "r") == 0 && argc >= 3 && strcmp(argv[1], "-") == 0) {
    char cmd_to_run[MAX_LINE] = "";
    for (int i = 2; i < argc; i++) {
      if (i > 2) strncat(cmd_to_run, " ", MAX_LINE - strlen(cmd_to_run) - 1);
      strncat(cmd_to_run, argv[i], MAX_LINE - strlen(cmd_to_run) - 1);
    }
    s->loop_active = 1;
    print_info("Starting loop. Press keys to exit (bare-metal mock).");
    while (s->loop_active) {
        shell_execute(s, cmd_to_run, 1);
        // Busy wait
        for (volatile int i = 0; i < 10000000; i++);
        s->loop_active = 0; // Only run once for now in bare-metal
    }
    if (!from_loop)
        shell_prompt(s);
    return;
  }

  /* ── history ───────────────────────────────────────────────── */
  if (strcmp(cmd, "history") == 0) {
    for (int i = 0; i < s->hist_count; i++)
      printf("%3d  %s\n", i + 1, s->history[i]);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── alias ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "alias") == 0) {
    if (argc == 1) {
      for (int i = 0; i < s->alias_count; i++)
        printf("alias %s='%s'\n", s->aliases[i].key, s->aliases[i].val);
    } else if (argc >= 3) {
      alias_set(s, argv[1], argv[2]);
    } else {
      print_error("alias: usage: alias [name value]");
    }
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── unalias ───────────────────────────────────────────────── */
  if (strcmp(cmd, "unalias") == 0) {
    if (argc < 2) print_error("unalias: usage: unalias <name>");
    else alias_unset(s, argv[1]);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── set (env) ─────────────────────────────────────────────── */
  if (strcmp(cmd, "set") == 0) {
    if (argc >= 3) env_set(s, argv[1], argv[2]);
    else print_error("set: usage: set <key> <val>");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── env ──────────────────────────────────────────────────── */
  if (strcmp(cmd, "env") == 0) {
    for (int i = 0; i < s->env_count; i++)
      printf("%s=%s\n", s->envvars[i].key, s->envvars[i].val);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── unset ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "unset") == 0) {
    if (argc < 2) print_error("unset: usage: unset <key>");
    else env_unset(s, argv[1]);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── clear ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "cls") == 0 || strcmp(cmd, "clear") == 0) {
    // VGA Clear is handled by high-level if possible, but for now just scroll
    for (int i=0; i<25; i++) printf("\n");
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── train ─────────────────────────────────────────────────── */
  if (strcmp(cmd, "train") == 0) {
    printf("%s\n", TRAIN_ART);
    if (!from_loop)
      shell_prompt(s);
    return;
  }

  /* ── Command not found ─────────────────────────────────────── */
  char err[MAX_NAME + 32];
  snprintf(err, sizeof(err), "daedalus: %s: command not found", cmd);
  print_error(err);
  if (!from_loop)
    shell_prompt(s);
}
