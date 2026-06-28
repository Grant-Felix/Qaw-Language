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
    ast.c \
    value.c \
    env.c \
    interpreter.c

OBJS = $(SRCS:%.c=$(BUILDDIR)/%.o)

OBJS = $(SRCS:%.c=$(BUILDDIR)/%.o)

# 目标
TARGET = $(BUILDDIR)/qawc

# 测试
TEST_SRCS = $(wildcard tests/*.c)
TEST_BINS = $(TEST_SRCS:tests/%.c=$(BUILDDIR)/%)

# 各测试需要的对象文件
test_lexer_OBJS = $(BUILDDIR)/lexer.o
test_ast_OBJS   = $(BUILDDIR)/ast.o
test_parser_OBJS = $(BUILDDIR)/lexer.o $(BUILDDIR)/parser.o $(BUILDDIR)/ast.o
test_value_OBJS = $(BUILDDIR)/value.o
test_env_OBJS   = $(BUILDDIR)/env.o $(BUILDDIR)/value.o
test_interp_OBJS = $(BUILDDIR)/lexer.o $(BUILDDIR)/parser.o $(BUILDDIR)/ast.o $(BUILDDIR)/value.o $(BUILDDIR)/env.o $(BUILDDIR)/interpreter.o

.PHONY: all test test-lexer test-ast test-parser test-value test-env test-interp clean

.PHONY: all test test-lexer test-ast test-parser clean

all: $(TARGET)

$(TARGET): $(OBJS) | $(BUILDDIR)
	$(CC) $(LDFLAGS) -o $@ $^

$(BUILDDIR)/%.o: %.c | $(BUILDDIR)
	$(CC) $(CFLAGS) -c $< -o $@

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

test: test-lexer test-ast test-parser test-value test-env test-interp

test-lexer: $(BUILDDIR)/test_lexer
	@echo "==> Running test_lexer"
	@$(BUILDDIR)/test_lexer

test-ast: $(BUILDDIR)/test_ast
	@echo "==> Running test_ast"
	@$(BUILDDIR)/test_ast

test-parser: $(BUILDDIR)/test_parser
	@echo "==> Running test_parser"
	@$(BUILDDIR)/test_parser

test-value: $(BUILDDIR)/test_value
	@echo "==> Running test_value"
	@$(BUILDDIR)/test_value

test-env: $(BUILDDIR)/test_env
	@echo "==> Running test_env"
	@$(BUILDDIR)/test_env

test-interp: $(BUILDDIR)/test_interp
	@echo "==> Running test_interp"
	@$(BUILDDIR)/test_interp

$(BUILDDIR)/test_lexer: tests/test_lexer.c $(test_lexer_OBJS) | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(test_lexer_OBJS)

$(BUILDDIR)/test_ast: tests/test_ast.c $(test_ast_OBJS) | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(test_ast_OBJS)

$(BUILDDIR)/test_parser: tests/test_parser.c $(test_parser_OBJS) | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(test_parser_OBJS)

$(BUILDDIR)/test_value: tests/test_value.c $(test_value_OBJS) | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(test_value_OBJS)

$(BUILDDIR)/test_env: tests/test_env.c $(test_env_OBJS) | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(test_env_OBJS)

$(BUILDDIR)/test_interp: tests/test_interp.c $(test_interp_OBJS) | $(BUILDDIR)
	$(CC) $(CFLAGS) -I$(INCDIR) -o $@ $< $(test_interp_OBJS)

clean:
	rm -rf $(BUILDDIR)
