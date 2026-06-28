/*
 * token.h — Token 类型定义
 *
 * 妖语言 v0.1 POC 引导版
 */

#ifndef YAO_TOKEN_H
#define YAO_TOKEN_H

#include <stddef.h>

/* Token 类型 */
typedef enum {
    /* 字面量 */
    TOK_INT_LIT,
    TOK_FLOAT_LIT,
    TOK_STRING_LIT,
    TOK_CHAR_LIT,
    TOK_IDENT,

    /* 关键字（归一化为英文原形，原始字面保存在 lexeme 中） */
    TOK_KW_PACKAGE,    /* package / pkg / baozhuang / bz */
    TOK_KW_IMPORT,     /* import / imp / daoru / dr */
    TOK_KW_FUNC,       /* func / fn / hanshu / hs */
    TOK_KW_RETURN,     /* return / ret / fanhui / fh */
    TOK_KW_VAR,        /* var / var / bianliang / bl */
    TOK_KW_LET,        /* let / let / buke / bk */
    TOK_KW_MUT,        /* mut / mut / kebian / kb */
    TOK_KW_CONST,      /* const / con / changliang / cl */
    TOK_KW_TYPE,       /* type / typ / leixing / lx */
    TOK_KW_STRUCT,     /* struct / struct / jiegou / jg */
    TOK_KW_ENUM,       /* enum / enum / meiju / mj */
    TOK_KW_IF,         /* if / if / ruguo / rg */
    TOK_KW_ELSE,       /* else / els / fouze / fz */
    TOK_KW_FOR,        /* for / for / xunhuan / xh */
    TOK_KW_WHILE,      /* while / whl / dang / dg */
    TOK_KW_LOOP,       /* loop / loop / xunhuan ? */
    TOK_KW_BREAK,      /* break / brk / tiaochu / tc */
    TOK_KW_CONTINUE,   /* continue / cnt / jixu / jx */
    TOK_KW_MATCH,      /* match / match / pipei / pp */
    TOK_KW_TRUE,       /* true / true / zhen / z */
    TOK_KW_FALSE,      /* false / fls / jia / j */
    TOK_KW_DEFER,      /* defer / def / yanchi / yc */
    TOK_KW_GO,         /* go / go / xiecheng / xc */
    TOK_KW_CHAN,       /* chan / chan / tongdao / td */
    TOK_KW_PANIC,      /* panic / panic / huangkong / hk */
    TOK_KW_UNSAFE,     /* unsafe / unsafe / weixian / wx */
    TOK_KW_AS,         /* as / as / zuowei / zw */

    /* 运算符 */
    TOK_PLUS, TOK_MINUS, TOK_STAR, TOK_SLASH, TOK_PERCENT,
    TOK_EQ, TOK_EQEQ, TOK_NEQ, TOK_LT, TOK_LE, TOK_GT, TOK_GE,
    TOK_AMP, TOK_PIPE, TOK_CARET, TOK_SHL, TOK_SHR,
    TOK_AND_AND, TOK_OR_OR, TOK_BANG,
    TOK_ARROW,         /* -> */
    TOK_FAT_ARROW,      /* => */
    TOK_DOT_DOT,        /* .. */
    TOK_DOT_DOT_EQ,     /* ..= */
    TOK_QUESTION,       /* ? */
    TOK_QUESTION_DOT,   /* ?. */
    TOK_QUESTION_QUESTION, /* ?? */
    TOK_COLON_COLON,    /* :: */

    /* 分隔符 */
    TOK_LPAREN, TOK_RPAREN,
    TOK_LBRACE, TOK_RBRACE,
    TOK_LBRACKET, TOK_RBRACKET,
    TOK_LANGLE, TOK_RANGLE,
    TOK_COMMA, TOK_SEMI, TOK_COLON, TOK_DOT,

    /* 特殊 */
    TOK_EOF,
    TOK_ERROR,
} TokenKind;

/* Token */
typedef struct {
    TokenKind kind;
    char *lexeme;        /* 原始字面（堆分配） */
    size_t lexeme_len;
    int line;            /* 起始行号（1-based） */
    int col;             /* 起始列号（1-based） */
} Token;

/* 创建 Token（复制 lexeme） */
Token token_new(TokenKind kind, const char *lexeme, size_t len, int line, int col);

/* 释放 Token（释放 lexeme） */
void token_free(Token *t);

/* Token 类型转字符串（用于调试） */
const char *token_kind_name(TokenKind k);

#endif /* YAO_TOKEN_H */
