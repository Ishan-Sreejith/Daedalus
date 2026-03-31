#include "libc.h"
#include <stdint.h>

static inline uint8_t inb(uint16_t port) {
    uint8_t ret;
    __asm__ volatile ( "inb %1, %0" : "=a"(ret) : "Nd"(port) );
    return ret;
}

unsigned char kbdus[128] = {
    0,  27, '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\b',
  '\t', 'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n',
    0,  'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`',   0,
  '\\', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',   0, '*',
    0,  ' ',   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0, '-',   0,   0,   0, '+',   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0
};

char kbd_getchar(void) {
    while (1) {
        if (inb(0x64) & 1) {
            uint8_t scancode = inb(0x60);
            if (!(scancode & 0x80)) {
                char c = kbdus[scancode];
                if (c) return c;
            }
        }
    }
}

extern void vga_print_char(char c);

char *fgets(char *str, int n, void *stream) {
    (void)stream;
    int idx = 0;
    while (idx < n - 1) {
        char c = kbd_getchar();
        if (c == '\b') {
            if (idx > 0) { idx--; vga_print_char('\b'); vga_print_char(' '); vga_print_char('\b'); }
        } else if (c == '\n') {
            vga_print_char('\n');
            str[idx++] = '\n';
            break;
        } else {
            vga_print_char(c);
            str[idx++] = c;
        }
    }
    str[idx] = '\0';
    return str;
}
