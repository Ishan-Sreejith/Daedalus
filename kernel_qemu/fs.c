/*
 * fs.c — Daedalus OS Virtual Filesystem
 *
 * A fully in-memory, tree-based filesystem. Nodes are heap-allocated;
 * the maximum branching factor per directory is MAX_CHILDREN.
 */
#include "daedalus.h"

#include "libc.h"

/* ─── Internal helpers ────────────────────────────────────────────────────── */

static FSNode *node_new(FSNodeType type, const char *name) {
    FSNode *n = calloc(1, sizeof(FSNode));
    if (!n) { while(1); }
    n->type = type;
    strncpy(n->name, name, MAX_NAME - 1);
    return n;
}

static FSNode *dir_new(const char *name) { return node_new(NODE_DIR,  name); }
static FSNode *file_new(const char *name, const char *content) {
    FSNode *n = node_new(NODE_FILE, name);
    if (content) strncpy(n->content, content, MAX_CONTENT - 1);
    return n;
}

static int dir_add(FSNode *dir, FSNode *child) {
    if (dir->child_count >= MAX_CHILDREN) return -1;
    dir->children[dir->child_count++] = child;
    return 0;
}

/* Sort children array (insertion sort — small N, no malloc) */
static void dir_sort(FSNode *dir) {
    for (int i = 1; i < dir->child_count; i++) {
        FSNode *key = dir->children[i];
        int j = i - 1;
        while (j >= 0 && strcmp(dir->children[j]->name, key->name) > 0) {
            dir->children[j + 1] = dir->children[j];
            j--;
        }
        dir->children[j + 1] = key;
    }
}

/* ─── Path utilities ──────────────────────────────────────────────────────── */

/* Normalise an absolute path, resolving . and .. */
static void path_normalize(const char *src, char *out, size_t outsz) {
    char parts[32][MAX_NAME];
    int  depth = 0;

    char tmp[MAX_PATH];
    strncpy(tmp, src, MAX_PATH - 1);
    char *tok = strtok(tmp, "/");
    while (tok) {
        if (strcmp(tok, ".") == 0) {
            /* skip */
        } else if (strcmp(tok, "..") == 0) {
            if (depth > 0) depth--;
        } else if (depth < 32) {
            strncpy(parts[depth++], tok, MAX_NAME - 1);
        }
        tok = strtok(NULL, "/");
    }

    out[0] = '\0';
    char acc[MAX_PATH] = "/";
    strncpy(out, "/", outsz);
    for (int i = 0; i < depth; i++) {
        if (i > 0) strncat(out, "/", outsz - strlen(out) - 1);
        strncat(out, parts[i], outsz - strlen(out) - 1);
    }
    if (depth == 0) strncpy(out, "/", outsz);
    (void)acc;
}

/* Resolve path relative to cwd, store normalised result in out */
static void path_resolve(const char *cwd, const char *input, char *out, size_t outsz) {
    if (!input || input[0] == '\0' || strcmp(input, ".") == 0) {
        strncpy(out, cwd, outsz - 1);
        return;
    }
    if (input[0] == '/') {
        path_normalize(input, out, outsz);
        return;
    }
    char combined[MAX_PATH];
    strcpy(combined, cwd);
    strcpy(combined + strlen(combined), "/");
    strcpy(combined + strlen(combined), input);
    path_normalize(combined, out, outsz);
}

/* ─── Public API ──────────────────────────────────────────────────────────── */

FSNode *fs_init(void) {
    FSNode *root = dir_new("/");

    /* /home/daedalus */
    FSNode *home  = dir_new("home");
    FSNode *daed  = dir_new("daedalus");
    dir_add(daed, file_new("readme.txt",
        "Daedalus OS v2.0 — C kernel edition.\n"
        "Type 'help' to explore all commands.\n"
        "Use 'history' to recall past commands.\n"));
    dir_add(daed, file_new("notes.log",
        "loop: r - sys info\n"
        "interrupt: ctrl+c\n"
        "tip: try 'alias' for shortcut commands\n"));
    dir_add(daed, file_new("motd.txt",
        "Welcome, pilot. The maze has no exit.\n"));
    dir_add(home, daed);
    dir_add(root, home);

    /* /sys */
    FSNode *sys = dir_new("sys");
    dir_add(sys, file_new("cpu.info",    "RP2040 @133MHz — Dual-core ARM Cortex-M0+\n"));
    dir_add(sys, file_new("memory.info", "SRAM: 264 KB  Flash: 2 MB\n"));
    dir_add(sys, file_new("version",     "Daedalus OS 2.0 (C kernel)\n"));
    dir_add(root, sys);

    /* /apps */
    dir_add(root, dir_new("apps"));

    /* /ports */
    FSNode *ports = dir_new("ports");
    dir_add(ports, file_new("uart0.dev", "mock UART port — 115200 baud\n"));
    dir_add(ports, file_new("gpio.dev",  "mock GPIO controller\n"));
    dir_add(root, ports);

    /* /tmp */
    dir_add(root, dir_new("tmp"));

    dir_sort(root);
    return root;
}

/* Walk the tree to find the node at 'path' (absolute, normalised). */
FSNode *fs_get_node(FSNode *root, const char *path) {
    if (strcmp(path, "/") == 0) return root;

    char tmp[MAX_PATH];
    strncpy(tmp, path, MAX_PATH - 1);

    FSNode *cur = root;
    char *tok = strtok(tmp, "/");
    while (tok && cur) {
        if (cur->type != NODE_DIR) return NULL;
        FSNode *found = NULL;
        for (int i = 0; i < cur->child_count; i++) {
            if (strcmp(cur->children[i]->name, tok) == 0) {
                found = cur->children[i];
                break;
            }
        }
        cur = found;
        tok = strtok(NULL, "/");
    }
    return cur;
}

/*
 * Find the parent directory of 'path' and write the leaf name into name_out.
 * Returns 0 on success, -1 if parent doesn't exist or isn't a dir.
 */
int fs_ensure_parent(FSNode *root, const char *path,
                     FSNode **parent_out, char name_out[MAX_NAME]) {
    /* split into parent path + leaf */
    char tmp[MAX_PATH];
    strncpy(tmp, path, MAX_PATH - 1);

    char *slash = strrchr(tmp, '/');
    if (!slash) return -1;

    char leaf[MAX_NAME];
    strncpy(leaf, slash + 1, MAX_NAME - 1);
    *slash = '\0';

    const char *parent_path = (tmp[0] == '\0') ? "/" : tmp;
    FSNode *parent = fs_get_node(root, parent_path);
    if (!parent || parent->type != NODE_DIR) return -1;

    *parent_out = parent;
    strncpy(name_out, leaf, MAX_NAME - 1);
    return 0;
}

/* Write sorted listing of dir children into buf. */
void fs_list_dir(FSNode *node, char *buf, size_t bufsz) {
    buf[0] = '\0';
    if (!node || node->type != NODE_DIR) return;
    dir_sort(node);
    for (int i = 0; i < node->child_count; i++) {
        FSNode *c = node->children[i];
        /* dirs in bold cyan, files in white */
        char entry[MAX_NAME + 32];
        if (c->type == NODE_DIR) {
            strcpy(entry, ANSI_BCYAN ANSI_BOLD);
            strcpy(entry + strlen(entry), c->name);
            strcpy(entry + strlen(entry), "/" ANSI_RESET);
        } else {
            strcpy(entry, c->name);
        }
        if (i > 0) strncat(buf, "  ", bufsz - strlen(buf) - 1);
        strncat(buf, entry, bufsz - strlen(buf) - 1);
    }
}

static FSNode *dir_get_child(FSNode *dir, const char *name) {
    for (int i = 0; i < dir->child_count; i++)
        if (strcmp(dir->children[i]->name, name) == 0)
            return dir->children[i];
    return NULL;
}

int fs_make_file(FSNode *root, const char *path) {
    FSNode *parent; char leaf[MAX_NAME];
    if (fs_ensure_parent(root, path, &parent, leaf) < 0) return -1;
    if (dir_get_child(parent, leaf)) return 0; /* already exists */
    return dir_add(parent, file_new(leaf, ""));
}

int fs_make_dir(FSNode *root, const char *path) {
    FSNode *parent; char leaf[MAX_NAME];
    if (fs_ensure_parent(root, path, &parent, leaf) < 0) return -1;
    if (dir_get_child(parent, leaf)) return 0;
    return dir_add(parent, dir_new(leaf));
}

int fs_delete(FSNode *root, const char *path, int recursive) {
    FSNode *parent; char leaf[MAX_NAME];
    if (fs_ensure_parent(root, path, &parent, leaf) < 0) return -1;

    int idx = -1;
    for (int i = 0; i < parent->child_count; i++) {
        if (strcmp(parent->children[i]->name, leaf) == 0) { idx = i; break; }
    }
    if (idx < 0) return -1;

    FSNode *target = parent->children[idx];
    if (target->type == NODE_DIR && target->child_count > 0 && !recursive)
        return -2; /* dir not empty */

    fs_free(target);
    /* Compact the children array */
    for (int i = idx; i < parent->child_count - 1; i++)
        parent->children[i] = parent->children[i + 1];
    parent->children[--parent->child_count] = NULL;
    return 0;
}

static FSNode *node_clone(FSNode *orig) {
    if (!orig) return NULL;
    FSNode *n = calloc(1, sizeof(FSNode));
    if (!n) return NULL;
    n->type = orig->type;
    strncpy(n->name, orig->name, MAX_NAME - 1);
    if (orig->type == NODE_FILE) {
        strncpy(n->content, orig->content, MAX_CONTENT - 1);
    } else {
        for (int i = 0; i < orig->child_count; i++) {
            n->children[i] = node_clone(orig->children[i]);
            if (n->children[i]) n->child_count++;
        }
    }
    return n;
}

int fs_copy(FSNode *root, const char *src, const char *dst) {
    FSNode *src_node = fs_get_node(root, src);
    if (!src_node) return -1;
    FSNode *dst_parent; char dst_leaf[MAX_NAME];
    if (fs_ensure_parent(root, dst, &dst_parent, dst_leaf) < 0) return -1;
    for (int i=0; i < dst_parent->child_count; i++) {
        if (strcmp(dst_parent->children[i]->name, dst_leaf) == 0) return -2;
    }
    FSNode *clone = node_clone(src_node);
    if (!clone) return -3;
    strncpy(clone->name, dst_leaf, MAX_NAME - 1);
    if (dir_add(dst_parent, clone) < 0) { fs_free(clone); return -3; }
    return 0;
}

int fs_move(FSNode *root, const char *src, const char *dst) {
    FSNode *src_parent; char src_leaf[MAX_NAME];
    if (fs_ensure_parent(root, src, &src_parent, src_leaf) < 0) return -1;
    int idx = -1;
    for (int i = 0; i < src_parent->child_count; i++) {
        if (strcmp(src_parent->children[i]->name, src_leaf) == 0) { idx = i; break; }
    }
    if (idx < 0) return -1;
    FSNode *target = src_parent->children[idx];

    FSNode *dst_parent; char dst_leaf[MAX_NAME];
    if (fs_ensure_parent(root, dst, &dst_parent, dst_leaf) < 0) return -1;
    for (int i=0; i < dst_parent->child_count; i++) {
        if (strcmp(dst_parent->children[i]->name, dst_leaf) == 0) return -2;
    }
    strncpy(target->name, dst_leaf, MAX_NAME - 1);
    if (dir_add(dst_parent, target) < 0) return -3;

    for (int i = idx; i < src_parent->child_count - 1; i++) {
        src_parent->children[i] = src_parent->children[i+1];
    }
    src_parent->children[--src_parent->child_count] = NULL;
    return 0;
}

void fs_free(FSNode *node) {
    if (!node) return;
    if (node->type == NODE_DIR)
        for (int i = 0; i < node->child_count; i++)
            fs_free(node->children[i]);
    free(node);
}

/* Exported path helpers used by shell.c */
void fs_path_normalize(const char *src, char *out, size_t outsz) {
    path_normalize(src, out, outsz);
}
void fs_path_resolve(const char *cwd, const char *input, char *out, size_t outsz) {
    path_resolve(cwd, input, out, outsz);
}
