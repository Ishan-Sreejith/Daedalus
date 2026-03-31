#include "daedalus.h"
#include "libc.h"

extern void vga_clear(void);
extern void vga_print_str(const char *s);

void kernel_main(void) {
    /* Initialize basic VGA / Keyboard */
    vga_clear();
    
    /* Splash Banner */
    vga_print_str("\n\n"
        "  ____                _       _\n"
        " |  _ \\  __ _  ___  __| | __ _| |_   _ ___\n"
        " | | | |/ _` |/ _ \\/ _` |/ _` | | | | / __|\n"
        " | |_| | (_| |  __/ (_| | (_| | | |_| \\__ \\\n"
        " |____/ \\__,_|\\___|\\__,_|\\__,_|_|\\__,_|___/\n"
        "  Daedalus OS v2.1  ·  Bare-Metal (x86)\n"
        "  ─────────────────────────────────────────\n\n");

    /* Init Filesystem & Shell */
    FSNode     *fs = fs_init();
    ShellState *s = shell_init(fs);

    /* Main REPL loop */
    char line[MAX_LINE];
    while (1) {
        shell_prompt(s);
        if (fgets(line, MAX_LINE, stdin)) {
            shell_execute(s, line, 0);
        }
    }
}
