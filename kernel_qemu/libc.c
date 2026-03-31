#include "libc.h"
#include <stdarg.h>

static char heap[1024 * 1024]; // 1MB heap
static size_t heap_idx = 0;

void *malloc(size_t size) {
    if (heap_idx + size > sizeof(heap)) return NULL;
    void *ptr = &heap[heap_idx];
    heap_idx += size;
    return ptr;
}

void *calloc(size_t num, size_t size) {
    void *ptr = malloc(num * size);
    if (ptr) memset(ptr, 0, num * size);
    return ptr;
}

void free(void *ptr) {
    (void)ptr;
}

void *memset(void *str, int c, size_t n) {
    unsigned char *p = str;
    while (n--) *p++ = (unsigned char)c;
    return str;
}

size_t strlen(const char *s) {
    size_t i = 0;
    while (s[i]) i++;
    return i;
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++; s2++;
    }
    return *(const unsigned char*)s1 - *(const unsigned char*)s2;
}

int strncmp(const char *s1, const char *s2, size_t n) {
    while (n && *s1 && (*s1 == *s2)) {
        s1++; s2++; n--;
    }
    if (n == 0) return 0;
    return *(const unsigned char*)s1 - *(const unsigned char*)s2;
}

char *strcpy(char *dest, const char *src) {
    char *d = dest;
    while ((*d++ = *src++));
    return dest;
}

char *strncpy(char *dest, const char *src, size_t n) {
    size_t i;
    for (i = 0; i < n && src[i] != '\0'; i++) dest[i] = src[i];
    for ( ; i < n; i++) dest[i] = '\0';
    return dest;
}

char *strdup(const char *s) {
    char *d = malloc(strlen(s) + 1);
    if (!d) return NULL;
    return strcpy(d, s);
}

extern void vga_print_char(char c);
extern void vga_print_str(const char *str);

void printf(const char *format, ...) {
    va_list args;
    va_start(args, format);
    for (const char *p = format; *p != '\0'; p++) {
        if (*p == '%' && *(p+1) == 'd') {
            int i = va_arg(args, int);
            if (i == 0) { vga_print_char('0'); }
            else {
                if (i < 0) { vga_print_char('-'); i = -i; }
                char buf[16]; int idx=0;
                while (i > 0) { buf[idx++] = '0' + (i % 10); i /= 10; }
                while (idx > 0) vga_print_char(buf[--idx]);
            }
            p++;
        } else if (*p == '%' && *(p+1) == 's') {
            const char *s = va_arg(args, const char *);
            vga_print_str(s ? s : "(null)");
            p++;
        } else {
            vga_print_char(*p);
        }
    }
    va_end(args);
}
