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

int isspace(int c) {
    return (c == ' ' || c == '\t' || c == '\n' || c == '\v' || c == '\f' || c == '\r');
}

void *memmove(void *dest, const void *src, size_t n) {
    unsigned char *d = dest;
    const unsigned char *s = src;
    if (d < s) {
        while (n--) *d++ = *s++;
    } else {
        d += n; s += n;
        while (n--) *--d = *--s;
    }
    return dest;
}

size_t strcspn(const char *s, const char *reject) {
    size_t count = 0;
    while (*s) {
        for (const char *r = reject; *r; r++) {
            if (*s == *r) return count;
        }
        s++; count++;
    }
    return count;
}

char *strchr(const char *s, int c) {
    while (*s != (char)c) {
        if (!*s++) return NULL;
    }
    return (char *)s;
}

char *strstr(const char *haystack, const char *needle) {
    if (!*needle) return (char *)haystack;
    for (; *haystack; haystack++) {
        if (*haystack == *needle) {
            const char *h = haystack, *n = needle;
            while (*h && *n && *h == *n) { h++; n++; }
            if (!*n) return (char *)haystack;
        }
    }
    return NULL;
}

char *strtok_r(char *str, const char *delim, char **saveptr) {
    if (str) *saveptr = str;
    if (!*saveptr || !**saveptr) return NULL;
    
    // Skip leading delimiters
    while (**saveptr) {
        int is_delim = 0;
        for (int i = 0; delim[i]; i++) {
            if (**saveptr == delim[i]) { is_delim = 1; break; }
        }
        if (!is_delim) break;
        (*saveptr)++;
    }
    
    if (!**saveptr) return NULL;
    
    char *start = *saveptr;
    // Find end of token
    while (**saveptr) {
        int is_delim = 0;
        for (int i = 0; delim[i]; i++) {
            if (**saveptr == delim[i]) { is_delim = 1; break; }
        }
        if (is_delim) {
            **saveptr = '\0';
            (*saveptr)++;
            return start;
        }
        (*saveptr)++;
    }
    return start;
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

char *strcat(char *dest, const char *src) {
    char *d = dest;
    while (*d) d++;
    while ((*d++ = *src++));
    return dest;
}

char *strncat(char *dest, const char *src, size_t n) {
    char *d = dest;
    while (*d) d++;
    while (n-- && (*d++ = *src++));
    *d = '\0';
    return dest;
}

char *strrchr(const char *s, int c) {
    const char *last = NULL;
    do {
        if (*s == (char)c) last = s;
    } while (*s++);
    return (char *)last;
}

static char *tok_ptr = NULL;
char *strtok(char *str, const char *delim) {
    if (str) tok_ptr = str;
    if (!tok_ptr) return NULL;
    
    // Skip leading delimiters
    while (*tok_ptr) {
        int is_delim = 0;
        for (int i = 0; delim[i]; i++) {
            if (*tok_ptr == delim[i]) { is_delim = 1; break; }
        }
        if (!is_delim) break;
        tok_ptr++;
    }
    
    if (!*tok_ptr) { tok_ptr = NULL; return NULL; }
    
    char *start = tok_ptr;
    // Find end of token
    while (*tok_ptr) {
        int is_delim = 0;
        for (int i = 0; delim[i]; i++) {
            if (*tok_ptr == delim[i]) { is_delim = 1; break; }
        }
        if (is_delim) {
            *tok_ptr = '\0';
            tok_ptr++;
            return start;
        }
        tok_ptr++;
    }
    tok_ptr = NULL;
    return start;
}


size_t strlen(const char *s) {
    size_t i = 0;
    while (s[i]) i++;
    return i;
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

int snprintf(char *str, size_t size, const char *format, ...) {
    va_list args;
    va_start(args, format);
    size_t i = 0;
    for (const char *p = format; *p != '\0' && i < size - 1; p++) {
        if (*p == '%' && *(p+1) == 'd') {
            int val = va_arg(args, int);
            if (val == 0) { str[i++] = '0'; }
            else {
                if (val < 0) { if (i < size-1) str[i++] = '-'; val = -val; }
                char buf[16]; int idx=0;
                while (val > 0) { buf[idx++] = '0' + (val % 10); val /= 10; }
                while (idx > 0 && i < size - 1) str[i++] = buf[--idx];
            }
            p++;
        } else if (*p == '%' && *(p+1) == 's') {
            const char *s = va_arg(args, const char *);
            if (!s) s = "(null)";
            while (*s && i < size - 1) str[i++] = *s++;
            p++;
        } else {
            str[i++] = *p;
        }
    }
    str[i] = '\0';
    va_end(args);
    return (int)i;
}

void *stdin = NULL;
void *stdout = NULL;


