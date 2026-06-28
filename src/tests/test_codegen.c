/*
 * test_codegen.c — C 代码生成器测试
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/lexer.h"
#include "yao/parser.h"
#include "yao/ast.h"
#include "yao/codegen.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <assert.h>
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

static char *gen_code(const char *src) {
    Lexer *lex = lexer_new(src);
    Parser *p = parser_new(lex);
    AstNode *prog = parser_parse_program(p);
    if (!prog) {
        parser_free(p);
        lexer_free(lex);
        return NULL;
    }
    char *code = qaw_codegen_to_c(prog);
    ast_free(prog);
    parser_free(p);
    lexer_free(lex);
    return code;
}

static int test_gen_hello_world(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(\"Hello, World!\")\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "int main(void)") != NULL);
    ASSERT(strstr(code, "printf") != NULL);
    ASSERT(strstr(code, "Hello, World!") != NULL);
    free(code);
    return 1;
}

static int test_gen_chinese_string(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(\"你好, 世界!\")\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    /* UTF-8 字节应该原样保留 */
    ASSERT(strstr(code, "你好, 世界!") != NULL);
    free(code);
    return 1;
}

static int test_gen_variable(void) {
    const char *src =
        "func main() -> void {\n"
        "    let x = 42\n"
        "    print(x)\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "long long x") != NULL);
    ASSERT(strstr(code, "x = 42") != NULL);
    ASSERT(strstr(code, "print") != NULL || strstr(code, "printf") != NULL);
    free(code);
    return 1;
}

static int test_gen_arithmetic(void) {
    const char *src =
        "func main() -> void {\n"
        "    print(1 + 2 * 3)\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "1LL + (2LL * 3LL)") != NULL || strstr(code, "1 + 2 * 3") != NULL);
    free(code);
    return 1;
}

static int test_gen_if_else(void) {
    const char *src =
        "func main() -> void {\n"
        "    let x = 5\n"
        "    if x > 3 {\n"
        "        print(\"big\")\n"
        "    } else {\n"
        "        print(\"small\")\n"
        "    }\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "if (") != NULL);
    ASSERT(strstr(code, "else") != NULL);
    free(code);
    return 1;
}

static int test_gen_while(void) {
    const char *src =
        "func main() -> void {\n"
        "    let i = 0\n"
        "    while i < 10 {\n"
        "        i = i + 1\n"
        "    }\n"
        "    print(i)\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "while (") != NULL);
    free(code);
    return 1;
}

static int test_gen_for_range(void) {
    const char *src =
        "func main() -> void {\n"
        "    for i from 0 to 10 {\n"
        "        print(i)\n"
        "    }\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "for (") != NULL);
    ASSERT(strstr(code, "i = 0") != NULL || strstr(code, "i <= 10") != NULL);
    free(code);
    return 1;
}

static int test_gen_user_function(void) {
    const char *src =
        "func add(x: int, y: int) -> int {\n"
        "    return x + y\n"
        "}\n"
        "func main() -> void {\n"
        "    print(add(3, 4))\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);
    ASSERT(strstr(code, "qawfn_add") != NULL);
    ASSERT(strstr(code, "qawfn_main") != NULL);
    free(code);
    return 1;
}

static int test_gen_full_hello_compiles_runs(void) {
    /* 完整流程：parse → codegen → gcc → run */
    const char *src =
        "func main() -> void {\n"
        "    print(\"测试通过!\")\n"
        "}\n";
    char *code = gen_code(src);
    ASSERT(code != NULL);

    /* 写入临时文件 */
    FILE *f = fopen("/tmp/test_codegen.c", "w");
    ASSERT(f != NULL);
    fputs(code, f);
    fclose(f);
    free(code);

    /* 编译 */
    int rc = system("gcc -O2 -o /tmp/test_codegen_bin /tmp/test_codegen.c 2>&1");
    ASSERT(rc == 0);

    /* 运行并捕获输出 */
    FILE *p = popen("/tmp/test_codegen_bin", "r");
    ASSERT(p != NULL);
    char buf[256];
    fgets(buf, sizeof(buf), p);
    pclose(p);
    ASSERT(strstr(buf, "测试通过!") != NULL);

    unlink("/tmp/test_codegen.c");
    unlink("/tmp/test_codegen_bin");
    return 1;
}

int main(void) {
    fprintf(stderr, "=== qawc Codegen 单元测试 ===\n");

    TEST(gen_hello_world);
    TEST(gen_chinese_string);
    TEST(gen_variable);
    TEST(gen_arithmetic);
    TEST(gen_if_else);
    TEST(gen_while);
    TEST(gen_for_range);
    TEST(gen_user_function);
    TEST(gen_full_hello_compiles_runs);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}