#ifndef LIBC_H
#define LIBC_H

#include <stddef.h>
#define NULL ((void*)0)

int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, size_t n);
size_t strlen(const char *s);
char *strdup(const char *s);

void *malloc(size_t size);
void *calloc(size_t num, size_t size);
void free(void *ptr);
void *memset(void *str, int c, size_t n);

void printf(const char *format, ...);
char *fgets(char *str, int n, void *stream);

extern void *stdin;
extern void *stdout;

#endif
