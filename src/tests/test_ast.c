/*
 * test_ast.c — AST 单元测试
 */

#define _POSIX_C_SOURCE 200809L

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

static int test_int_lit(void) {
    AstNode *n = ast_new_int_lit(42, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_INT_LIT);
    ASSERT(n->as.int_val == 42);
    ASSERT(n->line == 1);
    ASSERT(n->col == 1);
    ast_free(n);
    return 1;
}

static int test_float_lit(void) {
    AstNode *n = ast_new_float_lit(3.14, 2, 5);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_FLOAT_LIT);
    ASSERT(n->as.float_val == 3.14);
    ast_free(n);
    return 1;
}

static int test_string_lit(void) {
    AstNode *n = ast_new_string_lit("hello", 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_STRING_LIT);
    ASSERT(n->as.string_lit.n_parts == 1);
    ASSERT(strcmp(n->as.string_lit.parts[0].text, "hello") == 0);
    ast_free(n);
    return 1;
}

static int test_interp_lit(void) {
    /* "hello, ${name}!" */
    AstNode *name = ast_new_ident("name", 1, 10);
    AstInterpPart parts[3];
    parts[0] = ast_interp_text("hello, ");
    parts[1] = ast_interp_expr(name);
    parts[2] = ast_interp_text("!");

    AstNode *n = ast_new_interp_lit(parts, 3, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_STRING_LIT);
    ASSERT(n->as.string_lit.n_parts == 3);
    ASSERT(n->as.string_lit.parts[0].kind == INTERP_TEXT);
    ASSERT(n->as.string_lit.parts[1].kind == INTERP_EXPR);
    ASSERT(n->as.string_lit.parts[2].kind == INTERP_TEXT);

    ast_free(n);
    /* name 已被 n 接管，不需要单独 free */
    return 1;
}

static int test_bool_lit(void) {
    AstNode *t = ast_new_bool_lit(true, 1, 1);
    AstNode *f = ast_new_bool_lit(false, 1, 1);
    ASSERT(t != NULL && f != NULL);
    ASSERT(t->as.bool_val == true);
    ASSERT(f->as.bool_val == false);
    ast_free(t);
    ast_free(f);
    return 1;
}

static int test_char_lit(void) {
    AstNode *n = ast_new_char_lit(0x4E2D, 1, 1);  /* '中' */
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_CHAR_LIT);
    ASSERT(n->as.char_val == 0x4E2D);
    ast_free(n);
    return 1;
}

static int test_ident(void) {
    AstNode *n = ast_new_ident("foo_bar", 5, 10);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_IDENT);
    ASSERT(strcmp(n->as.ident_name, "foo_bar") == 0);
    ast_free(n);
    return 1;
}

static int test_binary_op(void) {
    AstNode *lhs = ast_new_int_lit(1, 1, 1);
    AstNode *rhs = ast_new_int_lit(2, 1, 5);
    AstNode *n = ast_new_binary(OP_ADD, lhs, rhs, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_BINARY_OP);
    ASSERT(n->as.binary.op == OP_ADD);
    ASSERT(n->as.binary.lhs == lhs);
    ASSERT(n->as.binary.rhs == rhs);
    ast_free(n);
    return 1;
}

static int test_unary_op(void) {
    AstNode *operand = ast_new_ident("x", 1, 2);
    AstNode *n = ast_new_unary(UOP_NEG, operand, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_UNARY_OP);
    ASSERT(n->as.unary.op == UOP_NEG);
    ast_free(n);
    return 1;
}

static int test_call(void) {
    AstNode *func = ast_new_ident("print", 1, 1);
    AstNode *args[2];
    args[0] = ast_new_string_lit("hello", 1, 7);
    args[1] = ast_new_int_lit(42, 1, 16);
    AstNode *n = ast_new_call(func, args, 2, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_CALL);
    ASSERT(n->as.call.n_args == 2);
    ASSERT(n->as.call.args != NULL);
    ASSERT(n->as.call.args[0] == args[0]);
    ASSERT(n->as.call.args[1] == args[1]);
    ast_free(n);
    return 1;
}

static int test_field_access(void) {
    AstNode *obj = ast_new_ident("point", 1, 1);
    AstNode *n = ast_new_field_access(obj, "x", 1, 7);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_FIELD_ACCESS);
    ASSERT(strcmp(n->as.field_access.field_name, "x") == 0);
    ast_free(n);
    return 1;
}

static int test_var_decl(void) {
    AstNode *init = ast_new_int_lit(10, 1, 12);
    AstNode *n = ast_new_var_decl("x", "int", true, init, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_VAR_DECL);
    ASSERT(strcmp(n->as.var_decl.name, "x") == 0);
    ASSERT(strcmp(n->as.var_decl.type_name, "int") == 0);
    ASSERT(n->as.var_decl.is_mut == true);
    ast_free(n);
    return 1;
}

static int test_var_decl_no_type(void) {
    AstNode *n = ast_new_var_decl("x", NULL, false, NULL, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->as.var_decl.type_name == NULL);
    ASSERT(n->as.var_decl.init == NULL);
    ast_free(n);
    return 1;
}

static int test_assign(void) {
    AstNode *target = ast_new_ident("x", 1, 1);
    AstNode *value = ast_new_int_lit(5, 1, 5);
    AstNode *n = ast_new_assign(target, value, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_ASSIGN);
    ast_free(n);
    return 1;
}

static int test_block(void) {
    AstNode *stmts[3];
    stmts[0] = ast_new_int_lit(1, 1, 1);
    stmts[1] = ast_new_int_lit(2, 1, 2);
    stmts[2] = ast_new_int_lit(3, 1, 3);
    AstNode *n = ast_new_block(stmts, 3, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_BLOCK);
    ASSERT(n->as.block.n_stmts == 3);
    ast_free(n);
    return 1;
}

static int test_if(void) {
    AstNode *cond = ast_new_ident("x", 1, 4);
    AstNode *then_block = ast_new_block(NULL, 0, 1, 7);
    AstNode *n = ast_new_if(cond, then_block, NULL, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_IF_STMT);
    ASSERT(n->as.if_stmt.else_block == NULL);
    ast_free(n);
    return 1;
}

static int test_while(void) {
    AstNode *cond = ast_new_ident("running", 1, 7);
    AstNode *body = ast_new_block(NULL, 0, 1, 16);
    AstNode *n = ast_new_while(cond, body, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_WHILE_STMT);
    ast_free(n);
    return 1;
}

static int test_for_in(void) {
    AstNode *iter = ast_new_ident("arr", 1, 12);
    AstNode *body = ast_new_block(NULL, 0, 1, 18);
    AstNode *n = ast_new_for_in("i", iter, body, 1, 5);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_FOR_STMT);
    ASSERT(n->as.for_stmt.kind == FOR_IN);
    ast_free(n);
    return 1;
}

static int test_for_range(void) {
    AstNode *start = ast_new_int_lit(1, 1, 16);
    AstNode *end = ast_new_int_lit(10, 1, 21);
    AstNode *body = ast_new_block(NULL, 0, 1, 26);
    AstNode *n = ast_new_for_range("i", start, end, NULL, body, 1, 5);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_FOR_STMT);
    ASSERT(n->as.for_stmt.kind == FOR_RANGE);
    ASSERT(n->as.for_stmt.step == NULL);
    ast_free(n);
    return 1;
}

static int test_return(void) {
    AstNode *value = ast_new_int_lit(42, 1, 8);
    AstNode *n = ast_new_return(value, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_RETURN_STMT);
    ast_free(n);

    /* 无值 return */
    AstNode *r = ast_new_return(NULL, 1, 1);
    ASSERT(r->as.return_stmt.value == NULL);
    ast_free(r);
    return 1;
}

static int test_function(void) {
    AstParam params[1];
    params[0].name = strdup("x");
    params[0].type_name = strdup("int");

    AstNode *body = ast_new_block(NULL, 0, 1, 1);
    AstNode *n = ast_new_function("foo", params, 1, "int", body, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_FUNCTION);
    ASSERT(strcmp(n->as.function.name, "foo") == 0);
    ASSERT(n->as.function.n_params == 1);
    ast_free(n);
    return 1;
}

static int test_struct(void) {
    AstFieldDecl fields[2];
    fields[0].name = strdup("x");
    fields[0].type_name = strdup("f64");
    fields[1].name = strdup("y");
    fields[1].type_name = strdup("f64");

    AstNode *n = ast_new_struct("Point", fields, 2, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_STRUCT_DECL);
    ASSERT(n->as.struct_decl.n_fields == 2);
    ast_free(n);
    return 1;
}

static int test_enum(void) {
    AstVariantDecl variants[3];
    variants[0].name = strdup("Red");
    variants[0].payload = NULL;
    variants[1].name = strdup("Green");
    variants[1].payload = NULL;
    variants[2].name = strdup("Blue");
    variants[2].payload = NULL;

    AstNode *n = ast_new_enum("Color", variants, 3, 1, 1);
    ASSERT(n != NULL);
    ASSERT(n->kind == AST_ENUM_DECL);
    ast_free(n);
    return 1;
}

static int test_program(void) {
    AstNode *func = ast_new_function("main", NULL, 0, NULL,
                                     ast_new_block(NULL, 0, 1, 1), 1, 1);
    AstNode *items[1] = {func};
    AstNode *prog = ast_new_program(items, 1, 1, 1);
    ASSERT(prog != NULL);
    ASSERT(prog->kind == AST_PROGRAM);
    ASSERT(prog->as.program.n_items == 1);
    ast_free(prog);
    return 1;
}

static int test_complex_tree(void) {
    /* 构建：
     *   func main() {
     *     let x = 1 + 2;
     *     print("x = ${x}");
     *   }
     */
    AstNode *int_lit_1 = ast_new_int_lit(1, 5, 12);
    AstNode *int_lit_2 = ast_new_int_lit(2, 5, 17);
    AstNode *add = ast_new_binary(OP_ADD, int_lit_1, int_lit_2, 5, 15);

    AstNode *var_decl = ast_new_var_decl("x", "int", false, add, 5, 5);

    AstNode *ident_x = ast_new_ident("x", 6, 22);
    AstInterpPart interp_parts[2];
    interp_parts[0] = ast_interp_text("x = ");
    interp_parts[1] = ast_interp_expr(ident_x);
    AstNode *str_lit = ast_new_interp_lit(interp_parts, 2, 6, 11);

    AstNode *print_ident = ast_new_ident("print", 6, 5);
    AstNode *call_args[1] = {str_lit};
    AstNode *print_call = ast_new_call(print_ident, call_args, 1, 6, 5);

    AstNode *expr_stmt = ast_new_expr_stmt(print_call, 6, 5);

    AstNode *stmts[2] = {var_decl, expr_stmt};
    AstNode *body = ast_new_block(stmts, 2, 5, 5);

    AstNode *func = ast_new_function("main", NULL, 0, NULL, body, 4, 1);

    AstNode *items[1] = {func};
    AstNode *prog = ast_new_program(items, 1, 1, 1);

    /* 打印看看 */
    fprintf(stderr, "\n--- 复杂树打印 ---\n");
    ast_print(prog, 0);

    ast_free(prog);
    /* call_args 是栈数组，已被 ast_new_call 复制 */

    return 1;
}

static int test_ast_kind_name(void) {
    ASSERT(strcmp(ast_kind_name(AST_INT_LIT), "int_lit") == 0);
    ASSERT(strcmp(ast_kind_name(AST_FUNCTION), "function") == 0);
    ASSERT(strcmp(ast_kind_name(AST_PROGRAM), "program") == 0);
    return 1;
}

static int test_binop_name(void) {
    ASSERT(strcmp(binop_name(OP_ADD), "+") == 0);
    ASSERT(strcmp(binop_name(OP_EQ), "==") == 0);
    ASSERT(strcmp(binop_name(OP_AND), "&&") == 0);
    return 1;
}

static int test_null_safety(void) {
    /* 释放 NULL 不应崩溃 */
    ast_free(NULL);
    return 1;
}

int main(void) {
    fprintf(stderr, "=== yaoc AST 单元测试 ===\n");

    TEST(null_safety);

    /* 字面量 */
    TEST(int_lit);
    TEST(float_lit);
    TEST(string_lit);
    TEST(interp_lit);
    TEST(bool_lit);
    TEST(char_lit);

    /* 标识符与引用 */
    TEST(ident);

    /* 运算符 */
    TEST(binary_op);
    TEST(unary_op);
    TEST(call);
    TEST(field_access);

    /* 语句 */
    TEST(var_decl);
    TEST(var_decl_no_type);
    TEST(assign);
    TEST(block);
    TEST(if);
    TEST(while);
    TEST(for_in);
    TEST(for_range);
    TEST(return);

    /* 声明 */
    TEST(function);
    TEST(struct);
    TEST(enum);

    /* 顶层 */
    TEST(program);

    /* 集成 */
    TEST(complex_tree);

    /* 调试 API */
    TEST(ast_kind_name);
    TEST(binop_name);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}