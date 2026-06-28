/*
 * parser.c — 递归下降 + Pratt 解析器
 *
 * 妖语言 v0.1 POC 引导版
 *
 * 第 3-4 周范围：
 *   - parse_program
 *   - parse_item → parse_function / parse_struct / parse_enum
 *   - parse_param_list
 *   - parse_block（块级语句的占位）
 *   - parse_var_decl（变量声明的占位）
 *
 * 第 5-6 周添加：parse_expr / parse_control_flow
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/parser.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdarg.h>

/* ============ 工具函数 ============ */

Parser *parser_new(Lexer *lex) {
    Parser *p = (Parser *)calloc(1, sizeof(Parser));
    if (!p) return NULL;
    p->lex = lex;
    p->has_peeked = 0;
    /* 预读第一个 token */
    p->current = lexer_next(lex);
    return p;
}

void parser_free(Parser *p) {
    if (!p) return;
    if (p->has_peeked) {
        token_free(&p->peeked);
    }
    token_free(&p->current);
    free(p);
}

const ParseError *parser_last_error(const Parser *p) {
    if (p->has_error) return &p->last_error;
    return NULL;
}

/* ============ Token 流工具 ============ */

/* 消耗当前 token（advance） */
static void advance(Parser *p) {
    if (p->has_peeked) {
        token_free(&p->current);
        p->current = p->peeked;
        p->has_peeked = 0;
    } else {
        token_free(&p->current);
        p->current = lexer_next(p->lex);
    }
}

/* 查看下一个 token（不消耗） */
static Token peek(Parser *p) {
    if (p->has_peeked) return p->peeked;
    p->peeked = lexer_next(p->lex);
    p->has_peeked = 1;
    return p->peeked;
}

/* 检查当前 token 类型 */
static int check(Parser *p, TokenKind kind) {
    return p->current.kind == kind;
}

/* 匹配并消耗（如果匹配） */
static int match(Parser *p, TokenKind kind) {
    if (p->current.kind == kind) {
        advance(p);
        return 1;
    }
    return 0;
}

/* 必须匹配（否则报错） */
static int expect(Parser *p, TokenKind kind, const char *what) {
    if (p->current.kind == kind) {
        advance(p);
        return 1;
    }
    /* 错误恢复：进入 panic mode，跳到下一个同步点 */
    if (!p->panic_mode) {
        p->has_error = 1;
        p->panic_mode = 1;
        snprintf(p->last_error.message, sizeof(p->last_error.message),
                 "期望 %s，得到 %s at %d:%d",
                 what,
                 token_kind_name(p->current.kind),
                 p->current.line, p->current.col);
        p->last_error.line = p->current.line;
        p->last_error.col = p->current.col;
        p->last_error.error_code = 1;
    }
    return 0;
}

/* 同步：跳到下一个语句边界 */
static void synchronize(Parser *p) {
    p->panic_mode = 0;

    while (p->current.kind != TOK_EOF) {
        switch (p->current.kind) {
            case TOK_KW_FUNC:
            case TOK_KW_STRUCT:
            case TOK_KW_ENUM:
            case TOK_KW_VAR:
            case TOK_KW_LET:
            case TOK_KW_CONST:
            case TOK_KW_IF:
            case TOK_KW_WHILE:
            case TOK_KW_FOR:
            case TOK_KW_RETURN:
                return;
            default:
                advance(p);
        }
    }
}

/* ============ 字面量 ============ */

static AstNode *parse_int_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    /* 解析数字（包含后缀 i32 / u64 / f32 等） */
    char buf[64];
    size_t len = 0;
    /* 简单复制：复制原始 lexeme 直到非数字/非后缀字符 */
    const char *s = p->current.lexeme;
    while (s[len] && len < sizeof(buf) - 1) {
        char c = s[len];
        if ((c >= '0' && c <= '9') || c == 'i' || c == 'u' || c == 'f' ||
            c == 's' || c == 'I' || c == 'U' || c == 'F' || c == 'S') {
            len++;
        } else {
            break;
        }
    }
    /* 简化：只解析纯整数字面量（不做后缀类型映射） */
    long long val = atoll(s);
    advance(p);
    return ast_new_int_lit((int64_t)val, line, col);
}

static AstNode *parse_float_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    double val = atof(p->current.lexeme);
    advance(p);
    return ast_new_float_lit(val, line, col);
}

static AstNode *parse_string_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    /* 字符串字面量已经在词法层处理过插值片段 */
    const char *raw = p->current.lexeme;
    /* raw 形如 "字符串内容"，需要去掉两端的引号 */
    size_t raw_len = p->current.lexeme_len;
    /* 找引号范围 */
    if (raw_len >= 2 && raw[0] == '"' && raw[raw_len - 1] == '"') {
        /* 内部内容：从 [1, raw_len-1) */
        /* v0.1 简化：当作单一 INTERP_TEXT 处理（插值识别留给 v0.5） */
        size_t inner_len = raw_len - 2;
        char *inner = (char *)malloc(inner_len + 1);
        if (!inner) {
            advance(p);
            return NULL;
        }
        memcpy(inner, raw + 1, inner_len);
        inner[inner_len] = '\0';
        /* 处理转义序列（简化：只处理 \\ \" \n \t） */
        char *p_out = inner;
        char *p_in = inner;
        while (*p_in) {
            if (*p_in == '\\' && *(p_in + 1)) {
                p_in++;
                switch (*p_in) {
                    case 'n': *p_out++ = '\n'; break;
                    case 't': *p_out++ = '\t'; break;
                    case 'r': *p_out++ = '\r'; break;
                    case '\\': *p_out++ = '\\'; break;
                    case '"': *p_out++ = '"'; break;
                    default: *p_out++ = *p_in; break;
                }
                p_in++;
            } else {
                *p_out++ = *p_in++;
            }
        }
        *p_out = '\0';

        AstNode *n = ast_new_string_lit(inner, line, col);
        free(inner);
        advance(p);
        return n;
    }
    advance(p);
    return ast_new_string_lit("", line, col);
}

static AstNode *parse_bool_lit(Parser *p, bool val) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);
    return ast_new_bool_lit(val, line, col);
}

static AstNode *parse_char_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    /* 简化：取 lexeme 第一个字节（v0.1 不完整支持转义） */
    int32_t val = 0;
    if (p->current.lexeme_len >= 3) {
        /* 跳过 ' 符号 */
        val = (unsigned char)p->current.lexeme[1];
    }
    advance(p);
    return ast_new_char_lit(val, line, col);
}

static AstNode *parse_ident(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    AstNode *n = ast_new_ident(p->current.lexeme, line, col);
    advance(p);
    return n;
}

static AstNode *parse_literal(Parser *p) {
    switch (p->current.kind) {
        case TOK_INT_LIT:      return parse_int_lit(p);
        case TOK_FLOAT_LIT:    return parse_float_lit(p);
        case TOK_STRING_LIT:   return parse_string_lit(p);
        case TOK_CHAR_LIT:     return parse_char_lit(p);
        case TOK_KW_TRUE:      return parse_bool_lit(p, true);
        case TOK_KW_FALSE:     return parse_bool_lit(p, false);
        case TOK_IDENT:        return parse_ident(p);
        default: return NULL;
    }
}

/* ============ 参数列表 ============ */

static AstNode *parse_param_list(Parser *p);

/* ============ 类型 ============ */

/* 简单类型：IDENT (可能含泛型) 或 T[RANGE] 数组类型 */
static char *parse_type(Parser *p) {
    /* 数组类型前缀 T[...] */
    /* v0.1 简化：只解析基本标识符类型 */
    if (check(p, TOK_IDENT)) {
        char *name = strdup(p->current.lexeme);
        advance(p);
        /* TODO: 泛型、数组、指针 */
        return name;
    }
    return NULL;
}

/* ============ 块 ============ */

/* 解析代码块： { stmt; stmt; ... } */
/* v0.1 简化：只支持变量声明、return、表达式语句、if/while/for 的简化版 */
static AstNode *parse_block(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    if (!expect(p, TOK_LBRACE, "'{'")) {
        return NULL;
    }

    /* 临时：跳过块内所有 token 直到匹配的 '}' */
    /* 第 5-6 周实现完整语句解析 */
    int depth = 1;
    while (depth > 0 && !check(p, TOK_EOF)) {
        if (check(p, TOK_LBRACE)) {
            depth++;
            advance(p);
        } else if (check(p, TOK_RBRACE)) {
            depth--;
            if (depth == 0) break;
            advance(p);
        } else {
            advance(p);
        }
    }

    if (!expect(p, TOK_RBRACE, "'}'")) {
        return NULL;
    }

    /* 返回空块（v0.1 简化） */
    return ast_new_block(NULL, 0, line, col);
}

/* ============ 顶层项 ============ */

/* 解析函数声明 */
static AstNode *parse_function(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);  /* 跳过 func */

    /* 函数名 */
    if (!check(p, TOK_IDENT)) {
        /* 错误恢复 */
        return NULL;
    }
    char *name = strdup(p->current.lexeme);
    advance(p);

    /* 参数列表 */
    AstParam *params = NULL;
    size_t n_params = 0;
    if (check(p, TOK_LPAREN)) {
        advance(p);
        /* 参数列表：暂存到栈 */
        AstParam tmp_params[64];
        size_t tmp_count = 0;
        if (!check(p, TOK_RPAREN)) {
            do {
                if (!check(p, TOK_IDENT)) break;
                tmp_params[tmp_count].name = strdup(p->current.lexeme);
                advance(p);
                tmp_params[tmp_count].type_name = NULL;
                if (match(p, TOK_COLON)) {
                    tmp_params[tmp_count].type_name = parse_type(p);
                }
                tmp_count++;
                if (tmp_count >= 64) break;
            } while (match(p, TOK_COMMA));
        }
        expect(p, TOK_RPAREN, "')'");

        /* 复制到堆 */
        if (tmp_count > 0) {
            params = (AstParam *)malloc(sizeof(AstParam) * tmp_count);
            for (size_t i = 0; i < tmp_count; i++) {
                params[i] = tmp_params[i];
            }
            n_params = tmp_count;
        }
    }

    /* 返回类型 */
    char *ret_type = NULL;
    if (match(p, TOK_ARROW)) {
        ret_type = parse_type(p);
    }

    /* 函数体 */
    AstNode *body = parse_block(p);

    return ast_new_function(name, params, n_params, ret_type, body, line, col);
}

/* 解析结构体声明 */
static AstNode *parse_struct(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);  /* 跳过 struct */

    if (!check(p, TOK_IDENT)) return NULL;
    char *name = strdup(p->current.lexeme);
    advance(p);

    AstFieldDecl *fields = NULL;
    size_t n_fields = 0;
    if (expect(p, TOK_LBRACE, "'{'")) {
        AstFieldDecl tmp[64];
        size_t cnt = 0;
        while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF) && cnt < 64) {
            if (!check(p, TOK_IDENT)) { advance(p); continue; }
            tmp[cnt].name = strdup(p->current.lexeme);
            advance(p);
            if (!expect(p, TOK_COLON, "':'")) { break; }
            tmp[cnt].type_name = parse_type(p);
            if (tmp[cnt].type_name == NULL) tmp[cnt].type_name = strdup("?");
            cnt++;
            /* 可选逗号 */
            match(p, TOK_COMMA);
        }
        expect(p, TOK_RBRACE, "'}'");

        if (cnt > 0) {
            fields = (AstFieldDecl *)malloc(sizeof(AstFieldDecl) * cnt);
            for (size_t i = 0; i < cnt; i++) fields[i] = tmp[i];
            n_fields = cnt;
        }
    }

    return ast_new_struct(name, fields, n_fields, line, col);
}

/* 解析枚举声明 */
static AstNode *parse_enum(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);  /* 跳过 enum */

    if (!check(p, TOK_IDENT)) return NULL;
    char *name = strdup(p->current.lexeme);
    advance(p);

    AstVariantDecl *variants = NULL;
    size_t n_variants = 0;
    if (expect(p, TOK_LBRACE, "'{'")) {
        AstVariantDecl tmp[64];
        size_t cnt = 0;
        while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF) && cnt < 64) {
            if (!check(p, TOK_IDENT)) { advance(p); continue; }
            tmp[cnt].name = strdup(p->current.lexeme);
            tmp[cnt].payload = NULL;
            advance(p);
            /* 可选 payload */
            if (match(p, TOK_LPAREN)) {
                tmp[cnt].payload = strdup("(...)");
                while (!check(p, TOK_RPAREN) && !check(p, TOK_EOF)) advance(p);
                expect(p, TOK_RPAREN, "')'");
            } else if (match(p, TOK_LBRACE)) {
                tmp[cnt].payload = strdup("{...}");
                while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF)) advance(p);
                expect(p, TOK_RBRACE, "'}'");
            }
            cnt++;
            match(p, TOK_COMMA);
        }
        expect(p, TOK_RBRACE, "'}'");

        if (cnt > 0) {
            variants = (AstVariantDecl *)malloc(sizeof(AstVariantDecl) * cnt);
            for (size_t i = 0; i < cnt; i++) variants[i] = tmp[i];
            n_variants = cnt;
        }
    }

    return ast_new_enum(name, variants, n_variants, line, col);
}

/* 调度：解析顶层项 */
static AstNode *parse_item(Parser *p) {
    switch (p->current.kind) {
        case TOK_KW_FUNC:    return parse_function(p);
        case TOK_KW_STRUCT:  return parse_struct(p);
        case TOK_KW_ENUM:    return parse_enum(p);
        default:
            /* 跳过到下一个同步点 */
            if (!p->panic_mode) {
                p->has_error = 1;
                p->panic_mode = 1;
                snprintf(p->last_error.message, sizeof(p->last_error.message),
                         "期望顶层项（func/struct/enum），得到 %s at %d:%d",
                         token_kind_name(p->current.kind),
                         p->current.line, p->current.col);
                p->last_error.line = p->current.line;
                p->last_error.col = p->current.col;
            }
            synchronize(p);
            return NULL;
    }
}

/* 顶层入口 */
AstNode *parser_parse_program(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;

    AstNode *items[256];
    size_t n_items = 0;

    while (!check(p, TOK_EOF) && n_items < 256) {
        if (p->panic_mode) {
            synchronize(p);
        }
        AstNode *item = parse_item(p);
        if (item) {
            items[n_items++] = item;
        }
    }

    return ast_new_program(items, n_items, line, col);
}