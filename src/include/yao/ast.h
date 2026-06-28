/*
 * ast.h — AST 节点定义（占位符）
 *
 * v0.1 POC：仅基础表达式与函数声明
 */

#ifndef YAO_AST_H
#define YAO_AST_H

#include "yao/token.h"

/* AST 节点类型 */
typedef enum {
    AST_INT_LIT,
    AST_FLOAT_LIT,
    AST_STRING_LIT,
    AST_BOOL_LIT,
    AST_IDENT,
    AST_BINARY_OP,
    AST_UNARY_OP,
    AST_CALL,
    AST_VAR_DECL,
    AST_ASSIGN,
    AST_IF,
    AST_FOR,
    AST_WHILE,
    AST_RETURN,
    AST_BLOCK,
    AST_FUNCTION,
    AST_STRUCT,
    AST_FIELD_ACCESS,
    AST_INDEX,
} AstKind;

/* 前向声明 */
typedef struct AstNode AstNode;

/* AST 节点 */
struct AstNode {
    AstKind kind;
    int line;
    int col;

    /* 字面量 */
    union {
        long long int_val;
        double float_val;
        const char *string_val;
        int bool_val;
        const char *ident_name;
    } as;

    /* 二元运算符 */
    struct {
        TokenKind op;
        AstNode *lhs;
        AstNode *rhs;
    } binary;

    /* 子节点（通用） */
    AstNode **children;
    size_t n_children;
};

/* v0.1 POC：AST 构造器留待 v0.5 阶段实现 */
typedef struct {
    int placeholder;
} AstBuilder;

#endif /* YAO_AST_H */
