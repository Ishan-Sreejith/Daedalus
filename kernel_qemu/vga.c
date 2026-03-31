#include <stdint.h>
#include <stddef.h>

#define VGA_WIDTH 80
#define VGA_HEIGHT 25

static uint16_t *vga_buffer = (uint16_t*) 0xB8000;
static size_t term_col = 0;
static size_t term_row = 0;
static uint8_t term_color = 0x0F;

static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile ( "outb %0, %1" : : "a"(val), "Nd"(port) );
}

void vga_set_cursor(int x, int y) {
    uint16_t pos = y * VGA_WIDTH + x;
    outb(0x3D4, 14);
    outb(0x3D5, pos >> 8);
    outb(0x3D4, 15);
    outb(0x3D5, pos & 0xFF);
}

void vga_clear(void) {
    for (size_t y = 0; y < VGA_HEIGHT; y++) {
        for (size_t x = 0; x < VGA_WIDTH; x++) {
            size_t index = y * VGA_WIDTH + x;
            vga_buffer[index] = ((uint16_t)0x0F << 8) | ' ';
        }
    }
    term_col = 0;
    term_row = 0;
    vga_set_cursor(term_col, term_row);
}

void vga_scroll(void) {
    for (size_t y = 1; y < VGA_HEIGHT; y++) {
        for (size_t x = 0; x < VGA_WIDTH; x++) {
            vga_buffer[(y-1) * VGA_WIDTH + x] = vga_buffer[y * VGA_WIDTH + x];
        }
    }
    for (size_t x = 0; x < VGA_WIDTH; x++) {
        vga_buffer[(VGA_HEIGHT-1) * VGA_WIDTH + x] = ((uint16_t)0x0F << 8) | ' ';
    }
}

void vga_print_char(char c) {
    if (c == '\n') {
        term_col = 0;
        if (++term_row == VGA_HEIGHT) { term_row--; vga_scroll(); }
    } else if (c == '\b') {
        if (term_col > 0) term_col--;
    } else if (c == '\r') {
        term_col = 0;
    } else {
        size_t index = term_row * VGA_WIDTH + term_col;
        vga_buffer[index] = ((uint16_t)term_color << 8) | (uint8_t)c;
        if (++term_col == VGA_WIDTH) {
            term_col = 0;
            if (++term_row == VGA_HEIGHT) { term_row--; vga_scroll(); }
        }
    }
    vga_set_cursor(term_col, term_row);
}

void vga_print_str(const char *str) {
    for (size_t i = 0; str[i] != '\0'; i++) vga_print_char(str[i]);
}
