#include "libc.h"
#include <stdint.h>

struct regs {
    uint32_t ds;
    uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax;
    uint32_t int_no, err_code;
    uint32_t eip, cs, eflags, useresp, ss;
};

unsigned char kbdus[128] = {
    0,  27, '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\b',
  '\t', 'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n',
    0,  'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`',   0,
  '\\', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',   0, '*',
    0,  ' ',   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0, '-',   0,   0,   0, '+',   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0
};

#define KBD_BUF_SIZE 256
static char kbd_buffer[KBD_BUF_SIZE];
static int kbd_head = 0;
static int kbd_tail = 0;

void keyboard_handler(struct regs *r) {
    (void)r;
    uint8_t scancode = inb(0x60);
    if (!(scancode & 0x80)) {
        char c = kbdus[scancode];
        if (c) {
            int next = (kbd_head + 1) % KBD_BUF_SIZE;
            if (next != kbd_tail) {
                kbd_buffer[kbd_head] = c;
                kbd_head = next;
            }
        }
    }
}

extern void irq_install_handler(int irq, void (*handler)(struct regs *r));

void keyboard_install() {
    irq_install_handler(1, keyboard_handler);
}

char kbd_getchar(void) {
    while (kbd_head == kbd_tail) {
        __asm__("hlt"); // Wait for interrupt
    }
    char c = kbd_buffer[kbd_tail];
    kbd_tail = (kbd_tail + 1) % KBD_BUF_SIZE;
    return c;
}

extern void vga_print_char(char c);

char *fgets(char *str, int n, void *stream) {
    (void)stream;
    int idx = 0;
    while (idx < n - 1) {
        char c = kbd_getchar();
        if (c == '\b') {
            if (idx > 0) {
                idx--;
                vga_print_char('\b');
            }
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
