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

/* Hardware I/O (implemented in io.asm) */
void outb(unsigned short port, unsigned char val);
unsigned char inb(unsigned short port);

char *strcat(char *dest, const char *src);
char *strncat(char *dest, const char *src, size_t n);
char *strrchr(const char *s, int c);
char *strtok(char *str, const char *delim);
char *strtok_r(char *str, const char *delim, char **saveptr);
char *strchr(const char *s, int c);
char *strstr(const char *haystack, const char *needle);
size_t strcspn(const char *s, const char *reject);
int isspace(int c);

void *malloc(size_t size);
void *calloc(size_t num, size_t size);
void free(void *ptr);
void *memset(void *str, int c, size_t n);
void *memmove(void *dest, const void *src, size_t n);

void printf(const char *format, ...);
int snprintf(char *str, size_t size, const char *format, ...);
char *fgets(char *str, int n, void *stream);


extern void *stdin;
extern void *stdout;

#endif
