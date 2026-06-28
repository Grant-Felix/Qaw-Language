/*
 * test_parser.c — Parser 单元测试
 */

#define _POSIX_C_SOURCE 200809L

#include "qaw/lexer.h"
#include "qaw/parser.h"
#include "qaw/ast.h"

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

/* ============ 表达式解析 ============ */

static int test_expr_int(void) {
    AstNode *prog = parse("func f() { let x = 42 }");
    ASSERT(prog != NULL);
    AstNode *body = prog->as.program.items[0]->as.function.body;
    ASSERT(body->kind == AST_BLOCK);
    ASSERT(body->as.block.n_stmts == 1);
    AstNode *decl = body->as.block.stmts[0];
    ASSERT(decl->kind == AST_VAR_DECL);
    ASSERT(decl->as.var_decl.init != NULL);
    ASSERT(decl->as.var_decl.init->kind == AST_INT_LIT);
    ASSERT(decl->as.var_decl.init->as.int_val == 42);
    ast_free(prog);
    return 1;
}

static int test_expr_arithmetic(void) {
    /* 1 + 2 * 3 应解析为 1 + (2 * 3) = 7 */
    AstNode *prog = parse("func f() { let x = 1 + 2 * 3 }");
    ASSERT(prog != NULL);
    AstNode *body = prog->as.program.items[0]->as.function.body;
    AstNode *init = body->as.block.stmts[0]->as.var_decl.init;
    /* 外层应该是 + */
    ASSERT(init->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.op == OP_ADD);
    /* lhs = 1, rhs = 2 * 3 */
    ASSERT(init->as.binary.lhs->kind == AST_INT_LIT);
    ASSERT(init->as.binary.lhs->as.int_val == 1);
    ASSERT(init->as.binary.rhs->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.rhs->as.binary.op == OP_MUL);
    ast_free(prog);
    return 1;
}

static int test_expr_precedence(void) {
    /* (1 + 2) * 3 = 9 */
    AstNode *prog = parse("func f() { let x = (1 + 2) * 3 }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    ASSERT(init->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.op == OP_MUL);
    ASSERT(init->as.binary.lhs->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.lhs->as.binary.op == OP_ADD);
    ast_free(prog);
    return 1;
}

static int test_expr_comparison(void) {
    AstNode *prog = parse("func f() { let x = a == b }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    ASSERT(init->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.op == OP_EQ);
    ast_free(prog);
    return 1;
}

static int test_expr_logical(void) {
    AstNode *prog = parse("func f() { let x = a && b || c }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    /* || 优先级最低 */
    ASSERT(init->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.op == OP_OR);
    ASSERT(init->as.binary.lhs->kind == AST_BINARY_OP);
    ASSERT(init->as.binary.lhs->as.binary.op == OP_AND);
    ast_free(prog);
    return 1;
}

static int test_expr_unary(void) {
    AstNode *prog = parse("func f() { let x = -a }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    ASSERT(init->kind == AST_UNARY_OP);
    ASSERT(init->as.unary.op == UOP_NEG);
    ast_free(prog);
    return 1;
}

static int test_expr_call(void) {
    AstNode *prog = parse("func f() { let x = add(1, 2) }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    ASSERT(init->kind == AST_CALL);
    ASSERT(init->as.call.func->kind == AST_IDENT);
    ASSERT(strcmp(init->as.call.func->as.ident_name, "add") == 0);
    ASSERT(init->as.call.n_args == 2);
    ast_free(prog);
    return 1;
}

static int test_expr_field_access(void) {
    AstNode *prog = parse("func f() { let x = p.x }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    ASSERT(init->kind == AST_FIELD_ACCESS);
    ASSERT(strcmp(init->as.field_access.field_name, "x") == 0);
    ast_free(prog);
    return 1;
}

static int test_expr_index(void) {
    AstNode *prog = parse("func f() { let x = arr[3] }");
    ASSERT(prog != NULL);
    AstNode *init = prog->as.program.items[0]->as.function.body->as.block.stmts[0]->as.var_decl.init;
    ASSERT(init->kind == AST_INDEX);
    ast_free(prog);
    return 1;
}

static int test_expr_assign(void) {
    AstNode *prog = parse("func f() { x = 5 }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    /* 表达式语句包装赋值 */
    ASSERT(stmt->kind == AST_EXPR_STMT);
    ASSERT(stmt->as.expr_stmt.expr->kind == AST_ASSIGN);
    ast_free(prog);
    return 1;
}

/* ============ 控制流解析 ============ */

static int test_stmt_if(void) {
    AstNode *prog = parse("func f() { if x > 0 { print(\"pos\") } }");
    ASSERT(prog != NULL);
    AstNode *body = prog->as.program.items[0]->as.function.body;
    ASSERT(body->as.block.n_stmts == 1);
    AstNode *stmt = body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_IF_STMT);
    ast_free(prog);
    return 1;
}

static int test_stmt_if_else(void) {
    AstNode *prog = parse("func f() { if x > 0 { a } else { b } }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_IF_STMT);
    ASSERT(stmt->as.if_stmt.else_block != NULL);
    ast_free(prog);
    return 1;
}

static int test_stmt_while(void) {
    AstNode *prog = parse("func f() { while x > 0 { x = x - 1 } }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_WHILE_STMT);
    ast_free(prog);
    return 1;
}

static int test_stmt_for_in(void) {
    AstNode *prog = parse("func f() { for x in arr { print(x) } }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_FOR_STMT);
    ASSERT(stmt->as.for_stmt.kind == FOR_IN);
    ASSERT(strcmp(stmt->as.for_stmt.var_name, "x") == 0);
    ast_free(prog);
    return 1;
}

static int test_stmt_for_range(void) {
    AstNode *prog = parse("func f() { for i from 0 to 10 { print(i) } }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_FOR_STMT);
    ASSERT(stmt->as.for_stmt.kind == FOR_RANGE);
    ast_free(prog);
    return 1;
}

static int test_stmt_return_value(void) {
    AstNode *prog = parse("func f() { return 42 }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_RETURN_STMT);
    ASSERT(stmt->as.return_stmt.value != NULL);
    ASSERT(stmt->as.return_stmt.value->kind == AST_INT_LIT);
    ast_free(prog);
    return 1;
}

static int test_stmt_return_void(void) {
    AstNode *prog = parse("func f() { return }");
    ASSERT(prog != NULL);
    AstNode *stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(stmt->kind == AST_RETURN_STMT);
    ASSERT(stmt->as.return_stmt.value == NULL);
    ast_free(prog);
    return 1;
}

static int test_stmt_break_continue(void) {
    AstNode *prog = parse(
        "func f() {\n"
        "    while true {\n"
        "        if done { break }\n"
        "        if skip { continue }\n"
        "    }\n"
        "}\n"
    );
    ASSERT(prog != NULL);
    AstNode *while_stmt = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(while_stmt->kind == AST_WHILE_STMT);
    AstNode *while_body = while_stmt->as.while_stmt.body;
    ASSERT(while_body->as.block.n_stmts == 2);
    ASSERT(while_body->as.block.stmts[0]->kind == AST_IF_STMT);
    AstNode *break_stmt = while_body->as.block.stmts[0]->as.if_stmt.then_block->as.block.stmts[0];
    ASSERT(break_stmt->kind == AST_BREAK_STMT);
    ASSERT(while_body->as.block.stmts[1]->kind == AST_IF_STMT);
    AstNode *cont_stmt = while_body->as.block.stmts[1]->as.if_stmt.then_block->as.block.stmts[0];
    ASSERT(cont_stmt->kind == AST_CONTINUE_STMT);
    ast_free(prog);
    return 1;
}

static int test_stmt_var_let_mut(void) {
    AstNode *prog = parse("func f() { let x = 5 }");
    ASSERT(prog != NULL);
    AstNode *decl = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(decl->kind == AST_VAR_DECL);
    ASSERT(decl->as.var_decl.is_mut == false);
    ast_free(prog);

    prog = parse("func f() { let mut x = 5 }");
    ASSERT(prog != NULL);
    decl = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(decl->as.var_decl.is_mut == true);
    ast_free(prog);

    prog = parse("func f() { var x = 5 }");
    ASSERT(prog != NULL);
    decl = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(decl->as.var_decl.is_mut == true);
    ast_free(prog);
    return 1;
}

static int test_stmt_with_type(void) {
    AstNode *prog = parse("func f() { let x: int = 5 }");
    ASSERT(prog != NULL);
    AstNode *decl = prog->as.program.items[0]->as.function.body->as.block.stmts[0];
    ASSERT(decl->kind == AST_VAR_DECL);
    ASSERT(strcmp(decl->as.var_decl.type_name, "int") == 0);
    ast_free(prog);
    return 1;
}

/* ============ 端到端：完整 hello.yao ============ */

static int test_full_hello(void) {
    AstNode *prog = parse(
        "package hello\n"
        "\n"
        "func main() -> void {\n"
        "    let greeting = \"你好\"\n"
        "    print(greeting)\n"
        "}\n"
    );
    ASSERT(prog != NULL);
    AstNode *func = prog->as.program.items[0];
    ASSERT(func->kind == AST_FUNCTION);
    AstNode *body = func->as.function.body;
    ASSERT(body->as.block.n_stmts == 2);
    /* stmt 0: let greeting = "你好" */
    ASSERT(body->as.block.stmts[0]->kind == AST_VAR_DECL);
    /* stmt 1: print(greeting) */
    ASSERT(body->as.block.stmts[1]->kind == AST_EXPR_STMT);
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

    /* 表达式 */
    TEST(expr_int);
    TEST(expr_arithmetic);
    TEST(expr_precedence);
    TEST(expr_comparison);
    TEST(expr_logical);
    TEST(expr_unary);
    TEST(expr_call);
    TEST(expr_field_access);
    TEST(expr_index);
    TEST(expr_assign);

    /* 控制流 */
    TEST(stmt_if);
    TEST(stmt_if_else);
    TEST(stmt_while);
    TEST(stmt_for_in);
    TEST(stmt_for_range);
    TEST(stmt_return_value);
    TEST(stmt_return_void);
    TEST(stmt_break_continue);
    TEST(stmt_var_let_mut);
    TEST(stmt_with_type);

    /* 端到端 */
    TEST(full_hello);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}