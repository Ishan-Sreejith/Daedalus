; kernel_qemu/gdt.asm
; Low-level GDT flushing for DaedalusC
section .text
global gdt_flush

gdt_flush:
    mov eax, [esp + 4]  ; gp pointer
    lgdt [eax]          ; Load GDT

    ; Reload segment registers
    mov ax, 0x10        ; Data segment (offset 0x10 in GDT)
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    jmp 0x08:.flush     ; Code segment (offset 0x08 in GDT)
.flush:
    ret
