/*
 * lexer.h — 词法分析器接口
 *
 * 妖语言 v0.1 POC 引导版
 */

#ifndef YAO_LEXER_H
#define YAO_LEXER_H

#include "yao/token.h"

/* 词法错误信息 */
typedef struct {
    int line;
    int col;
    char message[256];
} LexError;

/* 词法分析器状态 */
typedef struct Lexer Lexer;

/* 创建词法分析器（source 必须以 '\0' 结尾） */
Lexer *lexer_new(const char *source);

/* 释放词法分析器 */
void lexer_free(Lexer *l);

/* 下一个 Token（返回的 Token 归调用者释放；NULL 表示 EOF 或致命错误） */
Token lexer_next(Lexer *l);

/* 查看下一个 Token 但不消费 */
Token lexer_peek(Lexer *l);

/* 获取最后一次错误（NULL 表示无错误） */
const LexError *lexer_last_error(const Lexer *l);

/* 关键字归一化：返回英文原形（如果是关键字），否则返回 NULL */
const char *keyword_normalize(const char *s, size_t len);

#endif /* YAO_LEXER_H */
