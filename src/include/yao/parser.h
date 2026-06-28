/*
 * parser.h — 语法分析器接口
 *
 * 妖语言 v0.1 POC 引导版
 *
 * 设计：递归下降 + Pratt 表达式
 */

#ifndef YAO_PARSER_H
#define YAO_PARSER_H

#include "yao/ast.h"
#include "yao/lexer.h"

/* 解析错误信息 */
typedef struct {
    int line;
    int col;
    char message[256];
    int error_code;        /* 用于错误恢复 */
} ParseError;

/* 解析器 */
typedef struct {
    Lexer *lex;
    Token current;          /* 当前 Token（lookahead） */
    Token peeked;           /* 缓存的 peek */
    int has_peeked;
    ParseError last_error;
    int has_error;
    int panic_mode;        /* 错误恢复标志 */
} Parser;

/* 创建解析器 */
Parser *parser_new(Lexer *lex);

/* 释放解析器 */
void parser_free(Parser *p);

/* 顶层入口：解析整个程序 */
AstNode *parser_parse_program(Parser *p);

/* 解析单个表达式（用于字符串插值等）
 * 返回的 AST 由调用者负责释放
 */
AstNode *parser_parse_expr(Parser *p);

/* 错误信息访问 */
const ParseError *parser_last_error(const Parser *p);

#endif /* YAO_PARSER_H */