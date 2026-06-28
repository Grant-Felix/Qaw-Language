/*
 * main.c — yaoc 入口
 *
 * 妖语言 v0.1 POC 引导版
 */

#include "yao/lexer.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void usage(void) {
    fprintf(stderr,
        "用法: yaoc <command> [options] <file>\n"
        "\n"
        "命令:\n"
        "  run <file>     词法分析并打印 Token 列表（v0.1 POC）\n"
        "  check <file>   同 run\n"
        "  version        打印版本\n"
        "  help           显示帮助\n"
        "\n"
        "示例:\n"
        "  yaoc run examples/hello.yao\n");
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

static int cmd_run(const char *path) {
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
            if (err) {
                fprintf(stderr, "  %s\n", err->message);
            }
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

    if (strcmp(argv[1], "run") == 0 || strcmp(argv[1], "check") == 0) {
        if (argc < 3) {
            fprintf(stderr, "错误：缺少文件参数\n");
            usage();
            return 1;
        }
        return cmd_run(argv[2]);
    }

    fprintf(stderr, "未知命令: %s\n", argv[1]);
    usage();
    return 1;
}
