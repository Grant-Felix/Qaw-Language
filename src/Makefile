# Makefile for yaoc (妖语言编译器 v0.1 POC)

CC      ?= gcc
CFLAGS  ?= -std=c11 -Wall -Wextra -Wpedantic -O2 -g
LDFLAGS ?=

# 目录
SRCDIR    := .
BUILDDIR  := build
INCDIR    := include

# 头文件路径
CFLAGS    += -I$(INCDIR)

# 源文件
SRCS = \
    main.c \
    lexer.c \
    parser.c \
    ast.c

OBJS = $(SRCS:%.c=$(BUILDDIR)/%.o)

# 目标
TARGET = $(BUILDDIR)/yaoc

# 测试
TEST_SRCS = $(wildcard tests/*.c)
TEST_BINS = $(TEST_SRCS:tests/%.c=$(BUILDDIR)/%)

.PHONY: all test clean

all: $(TARGET)

$(TARGET): $(OBJS) | $(BUILDDIR)
	$(CC) $(LDFLAGS) -o $@ $^

$(BUILDDIR)/%.o: %.c | $(BUILDDIR)
	$(CC) $(CFLAGS) -c $< -o $@

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

test: $(TEST_BINS)
	@for t in $(TEST_BINS); do \
		echo "==> Running $$t"; \
		$$t || exit 1; \
	done

$(BUILDDIR)/%: tests/%.c $(BUILDDIR)/lexer.o $(BUILDDIR)/ast.o | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(BUILDDIR)/lexer.o $(BUILDDIR)/ast.o

clean:
	rm -rf $(BUILDDIR)
