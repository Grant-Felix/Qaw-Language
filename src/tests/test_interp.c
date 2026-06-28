/*
 * test_interp.c — Interpreter 端到端测试
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/lexer.h"
#include "yao/parser.h"
#include "yao/ast.h"
#include "yao/interpreter.h"
#include "yao/env.h"

#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <stdlib.h>
#include <unistd.h>

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

/* 运行源代码并捕获 stdout */
static int run_and_capture(const char *src, char *buf, size_t bufsize) {
    FILE *old = stdout;
    /* 用临时文件捕获输出 */
    FILE *tmp = tmpfile();
    if (!tmp) return -1;

    /* 重定向 stdout 到 tmp */
    fflush(stdout);
    dup2(fileno(tmp), fileno(stdout));

    Lexer *lex = lexer_new(src);
    Parser *p = parser_new(lex);
    AstNode *prog = parser_parse_program(p);
    int rc = -1;
    if (prog) {
        Env *env = env_new();
        EvalStatus s = interp_exec_program(prog, env);
        if (s == EVAL_OK) rc = 0;
        env_free(env);
        ast_free(prog);
    }
    parser_free(p);
    lexer_free(lex);

    /* 恢复 stdout */
    fflush(stdout);
    dup2(fileno(old), fileno(stdout));
    fclose(tmp);
    return rc;
}

/* 直接读取输出文件 */
static int run_capture(const char *src, char *buf, size_t bufsize) {
    /* 用 freopen 重定向 stdout 到文件 */
    char tmpl[] = "/tmp/yaoc_test_XXXXXX";
    int fd = mkstemp(tmpl);
    if (fd < 0) return -1;
    close(fd);

    FILE *old = stdout;
    freopen(tmpl, "w+", stdout);
    setvbuf(stdout, NULL, _IONBF, 0);

    Lexer *lex = lexer_new(src);
    Parser *p = parser_new(lex);
    AstNode *prog = parser_parse_program(p);
    int rc = -1;
    if (prog) {
        Env *env = env_new();
        EvalStatus s = interp_exec_program(prog, env);
        if (s == EVAL_OK) rc = 0;
        env_free(env);
        ast_free(prog);
    }
    parser_free(p);
    lexer_free(lex);

    fflush(stdout);
    freopen("/dev/tty", "w", stdout);  /* 恢复 */
    (void)old;

    /* 读取临时文件 */
    FILE *f = fopen(tmpl, "r");
    if (!f) { unlink(tmpl); return -1; }
    size_t n = fread(buf, 1, bufsize - 1, f);
    buf[n] = '\0';
    fclose(f);
    unlink(tmpl);
    return rc;
}

static int test_hello_world(void) {
    const char *src =
        "package hello\n"
        "func main() -> void {\n"
        "    print(\"你好, 世界!\")\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "你好, 世界!") != NULL);
    return 1;
}

static int test_print_int(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(42)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "42") != NULL);
    return 1;
}

static int test_arithmetic(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(1 + 2 * 3)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "7") != NULL);
    return 1;
}

static int test_variable(void) {
    const char *src =
        "func main() -> void {\n"
        "    let x = 10\n"
        "    let y = 20\n"
        "    print(x + y)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "30") != NULL);
    return 1;
}

static int test_variable_reassign(void) {
    const char *src =
        "func main() -> void {\n"
        "    let x = 5\n"
        "    x = 10\n"
        "    print(x)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "10") != NULL);
    return 1;
}

static int test_chinese_variable(void) {
    const char *src =
        "func main() -> void {\n"
        "    let 数量 = 42\n"
        "    print(数量)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "42") != NULL);
    return 1;
}

static int test_chinese_string(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(\"你好, 世界! 🌍\")\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "你好, 世界! 🌍") != NULL);
    return 1;
}

static int test_string_concat(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(\"hello, \" + \"world!\")\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "hello, world!") != NULL);
    return 1;
}

static int test_unary(void) {
    const char *src =
        "func main() -> void {\n"
        "    let x = 5\n"
        "    print(-x)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "-5") != NULL);
    return 1;
}

static int test_comparison(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(3 > 2)\n"
        "    print(1 == 1)\n"
        "    print(5 != 5)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "true") != NULL);
    ASSERT(strstr(buf, "false") != NULL);
    return 1;
}

static int test_if_statement(void) {
    const char *src =
        "func main() -> void {\n"
        "    let x = 5\n"
        "    if x > 3 {\n"
        "        print(\"big\")\n"
        "    } else {\n"
        "        print(\"small\")\n"
        "    }\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "big") != NULL);
    ASSERT(strstr(buf, "small") == NULL);
    return 1;
}

static int test_while_loop(void) {
    const char *src =
        "func main() -> void {\n"
        "    let i = 0\n"
        "    let sum = 0\n"
        "    while i < 5 {\n"
        "        sum = sum + i\n"
        "        i = i + 1\n"
        "    }\n"
        "    print(sum)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    /* 0+1+2+3+4 = 10 */
    ASSERT(strstr(buf, "10") != NULL);
    return 1;
}

static int test_for_range(void) {
    const char *src =
        "func main() -> void {\n"
        "    let sum = 0\n"
        "    for i from 1 to 5 {\n"
        "        sum = sum + i\n"
        "    }\n"
        "    print(sum)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    /* 1+2+3+4+5 = 15 */
    ASSERT(strstr(buf, "15") != NULL);
    return 1;
}

static int test_break(void) {
    const char *src =
        "func main() -> void {\n"
        "    let i = 0\n"
        "    while i < 100 {\n"
        "        if i == 5 { break }\n"
        "        i = i + 1\n"
        "    }\n"
        "    print(i)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "5") != NULL);
    return 1;
}

static int test_function_call(void) {
    const char *src =
        "func add(x: int, y: int) -> int {\n"
        "    return x + y\n"
        "}\n"
        "func main() -> void {\n"
        "    print(add(3, 4))\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "7") != NULL);
    return 1;
}

static int test_function_local_scope(void) {
    const char *src =
        "func double_it(x: int) -> int {\n"
        "    let result = x * 2\n"
        "    return result\n"
        "}\n"
        "func main() -> void {\n"
        "    let x = 5\n"
        "    print(double_it(x))\n"
        "    print(x)\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "10") != NULL);
    ASSERT(strstr(buf, "5") != NULL);
    return 1;
}

static int test_string_interp_simple(void) {
    const char *src =
        "func main() -> void {\n"
        "    let name = \"世界\"\n"
        "    print(\"你好, ${name}!\")\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "你好, 世界!") != NULL);
    return 1;
}

static int test_string_interp_expression(void) {
    const char *src =
        "func main() -> void {\n"
        "    let a = 5\n"
        "    let b = 3\n"
        "    print(\"${a} + ${b} = ${a + b}\")\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "5 + 3 = 8") != NULL);
    return 1;
}

static int test_four_form_runs(void) {
    /* 完整 four-form.yao 应能成功运行（4 个函数 + main 调用） */
    const char *src =
        "package four_form\n"
        "\n"
        "baozhuang four_form\n"
        "\n"
        "func english() { print(\"english\") }\n"
        "fn abbrev() { print(\"abbrev\") }\n"
        "hanshu pinyin_full() { print(\"pinyin_full\") }\n"
        "hs pinyin_init() { print(\"pinyin_init\") }\n"
        "func main() {\n"
        "    english()\n"
        "    abbrev()\n"
        "    pinyin_full()\n"
        "    pinyin_init()\n"
        "}\n";
    char buf[1024];
    int rc = run_capture(src, buf, sizeof(buf));
    ASSERT(rc == 0);
    ASSERT(strstr(buf, "english") != NULL);
    ASSERT(strstr(buf, "abbrev") != NULL);
    ASSERT(strstr(buf, "pinyin_full") != NULL);
    ASSERT(strstr(buf, "pinyin_init") != NULL);
    return 1;
}

int main(void) {
    fprintf(stderr, "=== yaoc Interpreter 端到端测试 ===\n");

    TEST(hello_world);
    TEST(print_int);
    TEST(arithmetic);
    TEST(variable);
    TEST(variable_reassign);
    TEST(chinese_variable);
    TEST(chinese_string);
    TEST(string_concat);
    TEST(unary);
    TEST(comparison);

    /* 控制流 */
    TEST(if_statement);
    TEST(while_loop);
    TEST(for_range);
    TEST(break);

    /* 函数调用 */
    TEST(function_call);
    TEST(function_local_scope);

    /* 字符串插值 */
    TEST(string_interp_simple);
    TEST(string_interp_expression);

    /* 完整 four-form */
    TEST(four_form_runs);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}