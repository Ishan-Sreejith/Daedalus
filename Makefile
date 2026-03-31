CC      = cc
CFLAGS  = -std=c11 -Wall -Wextra -Wpedantic -O2
LDFLAGS = -lpthread

SRC = kernel/main.c kernel/fs.c kernel/shell.c
OUT = daedalus

.PHONY: all clean run

all: $(OUT)

$(OUT): $(SRC) kernel/daedalus.h
	$(CC) $(CFLAGS) $(SRC) $(LDFLAGS) -o $(OUT)
	@echo "  built: ./$(OUT)"

run: all
	./$(OUT)

clean:
	rm -f $(OUT)
