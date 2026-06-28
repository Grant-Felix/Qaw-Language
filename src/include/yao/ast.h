/*
 * ast.h — AST 节点定义
 *
 * 妖语言 v0.1 POC 引导版
 *
 * 设计：tagged union，所有节点共享 AstNode 结构体。
 */

#ifndef YAO_AST_H
#define YAO_AST_H

#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>

/* ============ 节点类型 ============ */

typedef enum {
    /* 字面量 */
    AST_INT_LIT,
    AST_FLOAT_LIT,
    AST_STRING_LIT,        /* 字符串字面量（可能含插值） */
    AST_BOOL_LIT,
    AST_CHAR_LIT,

    /* 标识符与引用 */
    AST_IDENT,

    /* 运算符 */
    AST_BINARY_OP,
    AST_UNARY_OP,
    AST_CALL,
    AST_FIELD_ACCESS,
    AST_INDEX,
    AST_SLICE,

    /* 语句 */
    AST_VAR_DECL,
    AST_ASSIGN,
    AST_EXPR_STMT,
    AST_BLOCK,
    AST_IF_STMT,
    AST_WHILE_STMT,
    AST_FOR_STMT,
    AST_RETURN_STMT,
    AST_BREAK_STMT,
    AST_CONTINUE_STMT,

    /* 声明 */
    AST_FUNCTION,
    AST_STRUCT_DECL,
    AST_ENUM_DECL,

    /* 顶层 */
    AST_PROGRAM,
} AstKind;

/* ============ 运算符 ============ */

typedef enum {
    OP_ADD, OP_SUB, OP_MUL, OP_DIV, OP_MOD,
    OP_EQ, OP_NEQ, OP_LT, OP_LE, OP_GT, OP_GE,
    OP_AND, OP_OR,
    OP_BIT_AND, OP_BIT_OR, OP_BIT_XOR, OP_SHL, OP_SHR,
    OP_ASSIGN,
    OP_CONCAT,
} BinOp;

typedef enum {
    UOP_NEG, UOP_NOT, UOP_BIT_NOT,
} UnOp;

/* ============ AST 子结构 ============ */

/* 二元运算 */
typedef struct {
    BinOp op;
    struct AstNode *lhs;
    struct AstNode *rhs;
} AstBinaryOp;

/* 一元运算 */
typedef struct {
    UnOp op;
    struct AstNode *operand;
} AstUnaryOp;

/* 函数调用 */
typedef struct {
    struct AstNode *func;
    struct AstNode **args;
    size_t n_args;
} AstCall;

/* 字段访问 */
typedef struct {
    struct AstNode *obj;
    char *field_name;
} AstFieldAccess;

/* 索引 */
typedef struct {
    struct AstNode *obj;
    struct AstNode *index;
} AstIndex;

/* 切片 */
typedef struct {
    struct AstNode *obj;
    struct AstNode *start;     /* 可为 NULL */
    struct AstNode *end;       /* 可为 NULL */
    bool inclusive;            /* true = ..=, false = .. */
} AstSlice;

/* var / let 声明 */
typedef struct {
    char *name;
    char *type_name;          /* 可为 NULL */
    bool is_mut;              /* let mut / var */
    struct AstNode *init;      /* 可为 NULL */
} AstVarDecl;

/* 赋值 */
typedef struct {
    struct AstNode *target;
    struct AstNode *value;
} AstAssign;

/* 表达式语句（仅包裹 expr） */
typedef struct {
    struct AstNode *expr;
} AstExprStmt;

/* 代码块 */
typedef struct {
    struct AstNode **stmts;
    size_t n_stmts;
} AstBlock;

/* if 语句 */
typedef struct {
    struct AstNode *cond;
    struct AstNode *then_block;
    struct AstNode *else_block;    /* 可为 NULL */
} AstIfStmt;

/* while 语句 */
typedef struct {
    struct AstNode *cond;
    struct AstNode *body;
} AstWhileStmt;

/* for 语句（统一 for-in 与 for-range） */
typedef struct {
    enum { FOR_IN, FOR_RANGE } kind;
    char *var_name;
    /* for-in */
    struct AstNode *iterable;
    /* for-range */
    struct AstNode *start;
    struct AstNode *end;
    struct AstNode *step;          /* 可为 NULL */
    struct AstNode *body;
} AstForStmt;

/* return 语句 */
typedef struct {
    struct AstNode *value;         /* 可为 NULL */
} AstReturnStmt;

/* 函数参数 */
typedef struct {
    char *name;
    char *type_name;
} AstParam;

/* 函数声明 */
typedef struct {
    char *name;
    AstParam *params;
    size_t n_params;
    char *ret_type;                /* 可为 NULL */
    struct AstNode *body;
} AstFunction;

/* struct 字段声明 */
typedef struct {
    char *name;
    char *type_name;
} AstFieldDecl;

/* enum 变体声明 */
typedef struct {
    char *name;
    char *payload;                 /* 可为 NULL */
} AstVariantDecl;

/* struct 声明 */
typedef struct {
    char *name;
    AstFieldDecl *fields;
    size_t n_fields;
} AstStructDecl;

/* enum 声明 */
typedef struct {
    char *name;
    AstVariantDecl *variants;
    size_t n_variants;
} AstEnumDecl;

/* 字符串插值片段 */
typedef struct {
    enum { INTERP_TEXT, INTERP_EXPR } kind;
    char *text;                    /* INTERP_TEXT */
    struct AstNode *expr;          /* INTERP_EXPR */
} AstInterpPart;

/* 字符串字面量（可能含插值） */
typedef struct {
    AstInterpPart *parts;
    size_t n_parts;
} AstStringLit;

/* 顶层节点 */
typedef struct {
    struct AstNode **items;
    size_t n_items;
} AstProgram;

/* ============ AST 节点 ============ */

struct AstNode {
    AstKind kind;
    int line;
    int col;

    union {
        int64_t int_val;
        double float_val;
        char *string_val;          /* 简单字符串字面量（无插值） */
        bool bool_val;
        int32_t char_val;

        char *ident_name;

        AstStringLit string_lit;

        AstBinaryOp binary;
        AstUnaryOp unary;
        AstCall call;
        AstFieldAccess field_access;
        AstIndex index;
        AstSlice slice;

        AstVarDecl var_decl;
        AstAssign assign;
        AstExprStmt expr_stmt;
        AstBlock block;
        AstIfStmt if_stmt;
        AstWhileStmt while_stmt;
        AstForStmt for_stmt;
        AstReturnStmt return_stmt;

        AstFunction function;
        AstStructDecl struct_decl;
        AstEnumDecl enum_decl;

        AstProgram program;
    } as;
};

typedef struct AstNode AstNode;

/* ============ 构造器 ============ */

AstNode *ast_new_int_lit(int64_t val, int line, int col);
AstNode *ast_new_float_lit(double val, int line, int col);
AstNode *ast_new_string_lit(const char *s, int line, int col);
AstNode *ast_new_interp_lit(AstInterpPart *parts, size_t n_parts, int line, int col);
AstNode *ast_new_bool_lit(bool val, int line, int col);
AstNode *ast_new_char_lit(int32_t val, int line, int col);
AstNode *ast_new_ident(const char *name, int line, int col);

AstNode *ast_new_binary(BinOp op, AstNode *lhs, AstNode *rhs, int line, int col);
AstNode *ast_new_unary(UnOp op, AstNode *operand, int line, int col);
AstNode *ast_new_call(AstNode *func, AstNode **args, size_t n_args, int line, int col);
AstNode *ast_new_field_access(AstNode *obj, const char *field, int line, int col);
AstNode *ast_new_index(AstNode *obj, AstNode *index, int line, int col);
AstNode *ast_new_slice(AstNode *obj, AstNode *start, AstNode *end, bool inclusive, int line, int col);

AstNode *ast_new_var_decl(const char *name, const char *type, bool is_mut, AstNode *init, int line, int col);
AstNode *ast_new_assign(AstNode *target, AstNode *value, int line, int col);
AstNode *ast_new_expr_stmt(AstNode *expr, int line, int col);
AstNode *ast_new_block(AstNode **stmts, size_t n_stmts, int line, int col);
AstNode *ast_new_if(AstNode *cond, AstNode *then_block, AstNode *else_block, int line, int col);
AstNode *ast_new_while(AstNode *cond, AstNode *body, int line, int col);
AstNode *ast_new_for_in(const char *var, AstNode *iterable, AstNode *body, int line, int col);
AstNode *ast_new_for_range(const char *var, AstNode *start, AstNode *end, AstNode *step, AstNode *body, int line, int col);
AstNode *ast_new_return(AstNode *value, int line, int col);
AstNode *ast_new_break(int line, int col);
AstNode *ast_new_continue(int line, int col);

AstNode *ast_new_function(const char *name, AstParam *params, size_t n_params,
                          const char *ret_type, AstNode *body, int line, int col);
AstNode *ast_new_struct(const char *name, AstFieldDecl *fields, size_t n_fields, int line, int col);
AstNode *ast_new_enum(const char *name, AstVariantDecl *variants, size_t n_variants, int line, int col);

AstNode *ast_new_program(AstNode **items, size_t n_items, int line, int col);

/* 字符串插值辅助 */
AstInterpPart ast_interp_text(const char *text);
AstInterpPart ast_interp_expr(AstNode *expr);

/* ============ 释放 ============ */

void ast_free(AstNode *node);

/* ============ 调试 ============ */

const char *ast_kind_name(AstKind kind);
const char *binop_name(BinOp op);
const char *unop_name(UnOp op);
void ast_print(const AstNode *node, int indent);

#endif /* YAO_AST_H */