/*
 * test_lexer.c — 词法分析器单元测试
 */

#include "qaw/lexer.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

static int tests_run = 0;
static int tests_passed = 0;

#define TEST(name) \
    do { \
        fprintf(stderr, "  [TEST] %s ... ", #name); \
        tests_run++; \
        int r = test_##name(); \
        fprintf(stderr, "%s\n", r ? "OK" : "FAIL"); \
        if (r) tests_passed++; \
    } while (0)

#define ASSERT(cond) \
    do { if (!(cond)) { fprintf(stderr, "\n    assert failed at %s:%d: %s\n", __FILE__, __LINE__, #cond); return 0; } } while (0)

static int test_empty(void) {
    Lexer *l = lexer_new("");
    Token t = lexer_next(l);
    ASSERT(t.kind == TOK_EOF);
    token_free(&t);
    lexer_free(l);
    return 1;
}

static int test_int_literal(void) {
    Lexer *l = lexer_new("42");
    Token t = lexer_next(l);
    ASSERT(t.kind == TOK_INT_LIT);
    ASSERT(strcmp(t.lexeme, "42") == 0);
    token_free(&t);

    t = lexer_next(l);
    ASSERT(t.kind == TOK_EOF);
    token_free(&t);

    lexer_free(l);
    return 1;
}

static int test_string_literal(void) {
    Lexer *l = lexer_new("\"hello, 世界\"");
    Token t = lexer_next(l);
    ASSERT(t.kind == TOK_STRING_LIT);
    ASSERT(t.lexeme_len > 0);
    token_free(&t);
    lexer_free(l);
    return 1;
}

static int test_keyword_four_forms(void) {
    const char *sources[] = {
        "func",
        "fn",
        "hanshu",
        "hs",
    };
    for (int i = 0; i < 4; i++) {
        Lexer *l = lexer_new(sources[i]);
        Token t = lexer_next(l);
        ASSERT(t.kind == TOK_KW_FUNC);
        token_free(&t);
        lexer_free(l);
    }
    return 1;
}

static int test_keyword_package(void) {
    const char *sources[] = {
        "package", "pkg", "baozhuang", "bz"
    };
    for (int i = 0; i < 4; i++) {
        Lexer *l = lexer_new(sources[i]);
        Token t = lexer_next(l);
        ASSERT(t.kind == TOK_KW_PACKAGE);
        token_free(&t);
        lexer_free(l);
    }
    return 1;
}

static int test_keyword_normalize(void) {
    ASSERT(strcmp(keyword_normalize("func", 4), "func") == 0);
    ASSERT(strcmp(keyword_normalize("fn", 2), "func") == 0);
    ASSERT(strcmp(keyword_normalize("hanshu", 6), "func") == 0);  /* "hanshu" 是 6 字符 */
    ASSERT(strcmp(keyword_normalize("hs", 2), "func") == 0);
    ASSERT(keyword_normalize("foo", 3) == NULL);   /* 非关键字 */
    ASSERT(keyword_normalize("function", 8) == NULL);  /* 不是别名 */
    return 1;
}

static int test_ident(void) {
    Lexer *l = lexer_new("hello_world foo_bar123 _private");
    Token t;

    t = lexer_next(l);
    ASSERT(t.kind == TOK_IDENT);
    ASSERT(strcmp(t.lexeme, "hello_world") == 0);
    token_free(&t);

    t = lexer_next(l);
    ASSERT(t.kind == TOK_IDENT);
    ASSERT(strcmp(t.lexeme, "foo_bar123") == 0);
    token_free(&t);

    t = lexer_next(l);
    ASSERT(t.kind == TOK_IDENT);
    ASSERT(strcmp(t.lexeme, "_private") == 0);
    token_free(&t);

    lexer_free(l);
    return 1;
}

static int test_operators(void) {
    Lexer *l = lexer_new("+ - * / == != <= >= && || -> => .. ..=");
    TokenKind expected[] = {
        TOK_PLUS, TOK_MINUS, TOK_STAR, TOK_SLASH,
        TOK_EQEQ, TOK_NEQ, TOK_LE, TOK_GE,
        TOK_AND_AND, TOK_OR_OR,
        TOK_ARROW, TOK_FAT_ARROW,
        TOK_DOT_DOT, TOK_DOT_DOT_EQ,
    };
    int n = sizeof(expected) / sizeof(expected[0]);

    for (int i = 0; i < n; i++) {
        Token t = lexer_next(l);
        ASSERT(t.kind == expected[i]);
        token_free(&t);
    }

    lexer_free(l);
    return 1;
}

static int test_comments(void) {
    Lexer *l = lexer_new("// 单行注释\n42 /* 块注释 */ 100");
    Token t;

    t = lexer_next(l);
    ASSERT(t.kind == TOK_INT_LIT);
    ASSERT(strcmp(t.lexeme, "42") == 0);
    token_free(&t);

    t = lexer_next(l);
    ASSERT(t.kind == TOK_INT_LIT);
    ASSERT(strcmp(t.lexeme, "100") == 0);
    token_free(&t);

    lexer_free(l);
    return 1;
}

static int test_string_interp(void) {
    /* 字符串插值在词法层识别为字符串（具体插值处理在 parser/interp 阶段） */
    Lexer *l = lexer_new("\"hello, ${name}!\"");
    Token t = lexer_next(l);
    ASSERT(t.kind == TOK_STRING_LIT);
    token_free(&t);
    lexer_free(l);
    return 1;
}

static int test_chinese_identifiers(void) {
    Lexer *l = lexer_new("中文变量 café αβγ");
    Token t;

    t = lexer_next(l);
    ASSERT(t.kind == TOK_IDENT);
    ASSERT(strcmp(t.lexeme, "中文变量") == 0);
    token_free(&t);

    t = lexer_next(l);
    ASSERT(t.kind == TOK_IDENT);
    ASSERT(strcmp(t.lexeme, "café") == 0);
    token_free(&t);

    t = lexer_next(l);
    ASSERT(t.kind == TOK_IDENT);
    ASSERT(strcmp(t.lexeme, "αβγ") == 0);
    token_free(&t);

    lexer_free(l);
    return 1;
}

int main(void) {
    fprintf(stderr, "=== yaoc 词法分析器单元测试 ===\n");

    TEST(empty);
    TEST(int_literal);
    TEST(string_literal);
    TEST(keyword_four_forms);
    TEST(keyword_package);
    TEST(keyword_normalize);
    TEST(ident);
    TEST(operators);
    TEST(comments);
    TEST(string_interp);
    TEST(chinese_identifiers);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}
