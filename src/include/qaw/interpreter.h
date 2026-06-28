/*
 * interpreter.h — 解释器接口
 */

#ifndef YAO_INTERPRETER_H
#define YAO_INTERPRETER_H

#include "qaw/ast.h"
#include "qaw/env.h"

/* 求值结果与错误 */
typedef enum {
    EVAL_OK,
    EVAL_ERR_UNDEFINED_VAR,
    EVAL_ERR_TYPE,
    EVAL_ERR_DIV_BY_ZERO,
    EVAL_ERR_RUNTIME,
    EVAL_ERR_BREAK,        /* break 命中（不在循环内） */
    EVAL_ERR_RETURN,       /* return 命中（不在函数内） */
} EvalStatus;

typedef struct {
    EvalStatus status;
    Value value;
    char message[256];
} EvalResult;

/* 全局求值：表达式 */
EvalResult eval_expr(AstNode *expr, Env *env);

/* 全局执行：程序 */
EvalStatus interp_exec_program(AstNode *program, Env *env);

/* EvalResult 构造器 */
EvalResult eval_ok(Value v);
EvalResult eval_err(EvalStatus status, const char *msg);

#endif /* YAO_INTERPRETER_H */