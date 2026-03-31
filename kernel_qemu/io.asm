; kernel_qemu/io.asm
; Handcrafted low-level I/O routines for DaedalusC
section .text
global outb
global inb

; void outb(uint16_t port, uint8_t val)
outb:
    mov dx, [esp + 4]   ; port
    mov al, [esp + 8]   ; value
    out dx, al
    ret

; uint8_t inb(uint16_t port)
inb:
    mov dx, [esp + 4]   ; port
    in al, dx
    ret
