/*
 * test_parser.c — Parser 单元测试
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/lexer.h"
#include "yao/parser.h"
#include "yao/ast.h"

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
        if (test_##name()) { \
            fprintf(stderr, "OK\n"); \
            tests_passed++; \
        } else { \
            fprintf(stderr, "FAIL\n"); \
        } \
    } while (0)

#define ASSERT(cond) \
    do { if (!(cond)) { fprintf(stderr, "\n    assert failed at %s:%d: %s\n", __FILE__, __LINE__, #cond); return 0; } } while (0)

static AstNode *parse(const char *src) {
    Lexer *lex = lexer_new(src);
    Parser *p = parser_new(lex);
    AstNode *prog = parser_parse_program(p);
    parser_free(p);
    lexer_free(lex);
    return prog;
}

/* ============ 顶层解析 ============ */

static int test_empty_program(void) {
    AstNode *prog = parse("");
    ASSERT(prog != NULL);
    ASSERT(prog->kind == AST_PROGRAM);
    ASSERT(prog->as.program.n_items == 0);
    ast_free(prog);
    return 1;
}

static int test_function_no_params(void) {
    AstNode *prog = parse("func foo() { }");
    ASSERT(prog != NULL);
    ASSERT(prog->as.program.n_items == 1);
    AstNode *func = prog->as.program.items[0];
    ASSERT(func->kind == AST_FUNCTION);
    ASSERT(strcmp(func->as.function.name, "foo") == 0);
    ASSERT(func->as.function.n_params == 0);
    ASSERT(func->as.function.ret_type == NULL);  /* 无返回值 → NULL */
    ast_free(prog);
    return 1;
}

static int test_function_with_return_type(void) {
    AstNode *prog = parse("func bar() -> int { }");
    ASSERT(prog != NULL);
    AstNode *func = prog->as.program.items[0];
    ASSERT(func->kind == AST_FUNCTION);
    ASSERT(strcmp(func->as.function.name, "bar") == 0);
    ASSERT(strcmp(func->as.function.ret_type, "int") == 0);
    ast_free(prog);
    return 1;
}

static int test_function_with_params(void) {
    AstNode *prog = parse("func add(x: int, y: int) -> int { }");
    ASSERT(prog != NULL);
    AstNode *func = prog->as.program.items[0];
    ASSERT(func->as.function.n_params == 2);
    ASSERT(strcmp(func->as.function.params[0].name, "x") == 0);
    ASSERT(strcmp(func->as.function.params[0].type_name, "int") == 0);
    ASSERT(strcmp(func->as.function.params[1].name, "y") == 0);
    ast_free(prog);
    return 1;
}

static int test_function_no_param_types(void) {
    AstNode *prog = parse("func foo(x, y) { }");
    ASSERT(prog != NULL);
    AstNode *func = prog->as.program.items[0];
    ASSERT(func->as.function.n_params == 2);
    ASSERT(strcmp(func->as.function.params[0].name, "x") == 0);
    ASSERT(func->as.function.params[0].type_name == NULL);
    ast_free(prog);
    return 1;
}

static int test_struct(void) {
    AstNode *prog = parse("struct Point { x: f64, y: f64 }");
    ASSERT(prog != NULL);
    AstNode *s = prog->as.program.items[0];
    ASSERT(s->kind == AST_STRUCT_DECL);
    ASSERT(strcmp(s->as.struct_decl.name, "Point") == 0);
    ASSERT(s->as.struct_decl.n_fields == 2);
    ASSERT(strcmp(s->as.struct_decl.fields[0].name, "x") == 0);
    ASSERT(strcmp(s->as.struct_decl.fields[0].type_name, "f64") == 0);
    ASSERT(strcmp(s->as.struct_decl.fields[1].name, "y") == 0);
    ast_free(prog);
    return 1;
}

static int test_enum(void) {
    AstNode *prog = parse("enum Color { Red, Green, Blue }");
    ASSERT(prog != NULL);
    AstNode *e = prog->as.program.items[0];
    ASSERT(e->kind == AST_ENUM_DECL);
    ASSERT(strcmp(e->as.enum_decl.name, "Color") == 0);
    ASSERT(e->as.enum_decl.n_variants == 3);
    ASSERT(strcmp(e->as.enum_decl.variants[0].name, "Red") == 0);
    ast_free(prog);
    return 1;
}

static int test_multiple_items(void) {
    AstNode *prog = parse(
        "struct A { x: int }\n"
        "enum B { X }\n"
        "func c() { }\n"
    );
    ASSERT(prog != NULL);
    ASSERT(prog->as.program.n_items == 3);
    ASSERT(prog->as.program.items[0]->kind == AST_STRUCT_DECL);
    ASSERT(prog->as.program.items[1]->kind == AST_ENUM_DECL);
    ASSERT(prog->as.program.items[2]->kind == AST_FUNCTION);
    ast_free(prog);
    return 1;
}

/* ============ 字面量解析（在函数体中） ============ */
/* 注：当前 parse_block 是占位（跳过块内 token），所以函数体内容不会出现在 AST 中 */
/* 这些测试仅验证词法 + 解析器不崩溃 */

static int test_parse_int_lit(void) {
    AstNode *prog = parse("func f() { let x = 42 }");
    ASSERT(prog != NULL);
    ast_free(prog);
    return 1;
}

static int test_parse_string_lit(void) {
    AstNode *prog = parse("func f() { print(\"hello\") }");
    ASSERT(prog != NULL);
    ast_free(prog);
    return 1;
}

static int test_parse_chinese_string(void) {
    AstNode *prog = parse("func f() { print(\"你好, 世界!\") }");
    ASSERT(prog != NULL);
    ast_free(prog);
    return 1;
}

static int test_parse_float(void) {
    AstNode *prog = parse("func f() { let x = 3.14 }");
    ASSERT(prog != NULL);
    ast_free(prog);
    return 1;
}

/* ============ 错误恢复 ============ */

static int test_error_unknown_top_level(void) {
    /* package 关键字不是项，应触发错误恢复 */
    AstNode *prog = parse("package hello\nfunc main() { }");
    ASSERT(prog != NULL);  /* 即使有错误，prog 不为 NULL */
    /* 应该恢复后解析到 func */
    ASSERT(prog->as.program.n_items >= 1);
    ASSERT(prog->as.program.items[0]->kind == AST_FUNCTION);
    ast_free(prog);
    return 1;
}

static int test_error_then_function(void) {
    AstNode *prog = parse("garbage_token\nfunc main() { }");
    ASSERT(prog != NULL);
    /* synchronize 应跳到 func */
    ASSERT(prog->as.program.n_items >= 1);
    ast_free(prog);
    return 1;
}

/* ============ 实际例子 ============ */

static int test_hello_example(void) {
    AstNode *prog = parse(
        "package hello\n"
        "\n"
        "func main() -> void {\n"
        "    print(\"你好, 世界!\")\n"
        "}\n"
    );
    ASSERT(prog != NULL);
    ASSERT(prog->as.program.n_items == 1);
    ast_free(prog);
    return 1;
}

static int test_four_form_example(void) {
    AstNode *prog = parse(
        "package four\n"
        "baozhuang four\n"
        "\n"
        "func english() { }\n"
        "fn abbrev() { }\n"
        "hanshu pinyin() { }\n"
        "hs init() { }\n"
        "func main() { }\n"
    );
    ASSERT(prog != NULL);
    ASSERT(prog->as.program.n_items == 5);
    ast_free(prog);
    return 1;
}

static int test_chinese_identifier(void) {
    AstNode *prog = parse("func 中文函数() { }");
    ASSERT(prog != NULL);
    AstNode *func = prog->as.program.items[0];
    ASSERT(strcmp(func->as.function.name, "中文函数") == 0);
    ast_free(prog);
    return 1;
}

int main(void) {
    fprintf(stderr, "=== yaoc Parser 单元测试 ===\n");

    /* 顶层 */
    TEST(empty_program);
    TEST(function_no_params);
    TEST(function_with_return_type);
    TEST(function_with_params);
    TEST(function_no_param_types);
    TEST(struct);
    TEST(enum);
    TEST(multiple_items);

    /* 字面量 */
    TEST(parse_int_lit);
    TEST(parse_string_lit);
    TEST(parse_chinese_string);
    TEST(parse_float);

    /* 错误恢复 */
    TEST(error_unknown_top_level);
    TEST(error_then_function);

    /* 例子 */
    TEST(hello_example);
    TEST(four_form_example);
    TEST(chinese_identifier);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}