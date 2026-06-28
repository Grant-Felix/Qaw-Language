/*
 * main.c — yaoc 入口
 *
 * 妖语言 v0.1 POC 引导版
 */

#include "yao/lexer.h"
#include "yao/parser.h"
#include "yao/ast.h"
#include "yao/interpreter.h"
#include "yao/env.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void usage(void) {
    fprintf(stderr,
        "用法: yaoc <command> [options] <file>\n"
        "\n"
        "命令:\n"
        "  lex <file>      词法分析并打印 Token 列表\n"
        "  parse <file>    解析为 AST 并打印\n"
        "  run <file>      解析并执行（目前仅 main 函数内的表达式语句）\n"
        "  check <file>    解析检查\n"
        "  version         打印版本\n"
        "  help            显示帮助\n"
        "\n"
        "示例:\n"
        "  qawc run examples/hello.qaw\n");
}

static char *read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "无法打开文件: %s\n", path);
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *buf = (char *)malloc(size + 1);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    fread(buf, 1, size, f);
    buf[size] = '\0';
    fclose(f);
    return buf;
}

/* ============ lex ============ */

static int cmd_lex(const char *path) {
    char *src = read_file(path);
    if (!src) return 1;

    Lexer *lex = lexer_new(src);
    if (!lex) {
        free(src);
        return 1;
    }

    printf("=== 词法分析: %s ===\n", path);

    int count = 0;
    for (;;) {
        Token t = lexer_next(lex);

        if (t.kind == TOK_EOF) {
            token_free(&t);
            break;
        }

        if (t.kind == TOK_ERROR) {
            fprintf(stderr, "词法错误 at %d:%d: '%s'\n",
                    t.line, t.col, t.lexeme ? t.lexeme : "");
            const LexError *err = lexer_last_error(lex);
            if (err) fprintf(stderr, "  %s\n", err->message);
            token_free(&t);
            lexer_free(lex);
            free(src);
            return 1;
        }

        printf("%4d:%-3d  %-15s  %s\n",
               t.line, t.col,
               token_kind_name(t.kind),
               t.lexeme ? t.lexeme : "");
        count++;

        token_free(&t);
    }

    printf("=== 共 %d 个 token ===\n", count);

    lexer_free(lex);
    free(src);
    return 0;
}

/* ============ parse ============ */

static int cmd_parse(const char *path) {
    char *src = read_file(path);
    if (!src) return 1;

    Lexer *lex = lexer_new(src);
    if (!lex) {
        free(src);
        return 1;
    }

    Parser *p = parser_new(lex);
    if (!p) {
        lexer_free(lex);
        free(src);
        return 1;
    }

    printf("=== 语法分析: %s ===\n", path);
    AstNode *program = parser_parse_program(p);

    if (program) {
        printf("\n--- AST ---\n");
        ast_print(program, 0);
        printf("\n=== 解析成功 ===\n");
        ast_free(program);
    } else {
        const ParseError *err = parser_last_error(p);
        fprintf(stderr, "解析失败:\n");
        if (err) {
            fprintf(stderr, "  %d:%d: %s\n", err->line, err->col, err->message);
        }
    }

    parser_free(p);
    lexer_free(lex);
    free(src);
    return program ? 0 : 1;
}

/* ============ run（解析 + 执行） ============ */

static int cmd_run(const char *path) {
    char *src = read_file(path);
    if (!src) return 1;

    Lexer *lex = lexer_new(src);
    if (!lex) { free(src); return 1; }

    Parser *p = parser_new(lex);
    if (!p) { lexer_free(lex); free(src); return 1; }

    AstNode *program = parser_parse_program(p);

    int exit_code = 0;
    if (program) {
        Env *env = env_new();
        EvalStatus status = interp_exec_program(program, env);
        if (status != EVAL_OK) {
            fprintf(stderr, "运行时错误\n");
            exit_code = 1;
        }
        env_free(env);
        ast_free(program);
    } else {
        const ParseError *err = parser_last_error(p);
        fprintf(stderr, "解析失败:\n");
        if (err) fprintf(stderr, "  %d:%d: %s\n", err->line, err->col, err->message);
        exit_code = 1;
    }

    parser_free(p);
    lexer_free(lex);
    free(src);
    return exit_code;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        usage();
        return 1;
    }

    if (strcmp(argv[1], "version") == 0) {
        printf("yaoc v0.1 POC\n");
        return 0;
    }

    if (strcmp(argv[1], "help") == 0 || strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
        usage();
        return 0;
    }

    const char *cmd = argv[1];
    const char *path = (argc >= 3) ? argv[2] : NULL;

    if (strcmp(cmd, "lex") == 0 || strcmp(cmd, "check") == 0 || strcmp(cmd, "run") == 0 || strcmp(cmd, "parse") == 0) {
        if (!path) {
            fprintf(stderr, "错误：缺少文件参数\n");
            usage();
            return 1;
        }
if (strcmp(cmd, "lex") == 0) return cmd_lex(path);
    if (strcmp(cmd, "parse") == 0 || strcmp(cmd, "check") == 0) return cmd_parse(path);
    if (strcmp(cmd, "run") == 0) return cmd_run(path);
    /* 旧兼容 */
    return cmd_parse(path);
    }

    fprintf(stderr, "未知命令: %s\n", argv[1]);
    usage();
    return 1;
}
