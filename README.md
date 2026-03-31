# Daedalus OS

A web-based OS simulator with a terminal-first interface and a toggleable desktop view.
The local kernel is now written in **C** (C11) for speed, direct system access, and zero runtime overhead.

## What this repo includes

| File | Description |
|---|---|
| `kernel/` | C kernel source — `main.c`, `fs.c`, `shell.c`, `daedalus.h` |
| `Makefile` | Build system — run `make` |
| `index.html` | Web browser UI (terminal + desktop) |
| `daedalus_cli.py` | Legacy Python CLI (kept for web-UI compatibility) |

## Building the C Kernel

Requires a C11 compiler (clang or gcc) and POSIX pthreads (standard on macOS/Linux).

```bash
make          # compile → ./daedalus
make run      # compile and launch
make clean    # remove binary
```

## C Kernel Features (v2.0)

- **ANSI colour output** — cyan prompt, red errors, green info, yellow warnings
- **In-memory virtual filesystem** — `/home/daedalus`, `/sys`, `/ports`, `/tmp`, `/apps`
- **Command history** — `history` shows last 20 commands
- **Aliases** — `alias ll list`, `alias`, `unalias ll`
- **Environment variables** — `set KEY val`, `env`, `unset KEY`
- **Repeat loop** — `r - <cmd>` runs a command repeatedly in a background thread; `Ctrl+C` stops it
- **SIGINT handling** — clean interrupt, prompt re-draws
- **New builtins** — `whoami`, `uptime`, `wc`, `cls`/`clear`, `exit`/`quit`
- **File redirection** — `say text > file` and `say text >> file`

## Commands

Type `help` inside the shell for the full reference. Key commands:

```
here            pwd
list [path]     ls
go [path]       cd
read <file>     cat
new / make      touch / mkdir
del [-r]        rm
write / mod     overwrite or interactively edit file
find <p> <f>    grep
wc <file>       word/line/char count
say <text>      echo (supports > and >>)
sys cpu|memory|info|version
ping <h> <p>    mock netcat probe
r - <cmd>       repeat loop (Ctrl+C to stop)
history         last 20 commands
alias / env     shell aliases and env vars
cls / clear     clear screen
exit / quit     shutdown
```

## Web UI

`index.html` is GitHub Pages compatible. Deploy from the main branch and serve the repository root.  
The Python CLI (`python3 daedalus_cli.py`) is still available as a fallback for the web UI context.
