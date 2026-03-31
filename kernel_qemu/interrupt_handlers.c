#include "libc.h"
#include <stdint.h>

struct regs {
    uint32_t ds;
    uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax;
    uint32_t int_no, err_code;
    uint32_t eip, cs, eflags, useresp, ss;
};

void isr_handler(struct regs *r) {
    if (r->int_no < 32) {
        printf("CPU Exception %d at EIP 0x%d\n", r->int_no, r->eip);
        while(1) { __asm__("hlt"); }
    }
}

static void *irq_routines[16] = {
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0
};

void irq_install_handler(int irq, void (*handler)(struct regs *r)) {
    irq_routines[irq] = (void*)handler;
}

void irq_uninstall_handler(int irq) {
    irq_routines[irq] = 0;
}

void irq_handler(struct regs *r) {
    void (*handler)(struct regs *r);
    handler = irq_routines[r->int_no - 32];
    if (handler) {
        handler(r);
    }

    if (r->int_no >= 40) {
        outb(0xA0, 0x20); // Slave EOI
    }
    outb(0x20, 0x20); // Master EOI
}
