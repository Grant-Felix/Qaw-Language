/*
 * interpreter.c — Tree-walking 解释器
 *
 * 第 7-8 周范围：
 *   - eval_expr 主体
 *   - 字面量、标识符、二元/一元运算符
 *   - 内置 print 函数
 *
 * 第 9-10 周添加：
 *   - 控制流（if / while / for / break / continue）
 *   - return / 函数调用
 *   - 闭包
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/interpreter.h"
#include "yao/lexer.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
#include <stdarg.h>

/* ============ EvalResult 构造器 ============ */

EvalResult eval_ok(Value v) {
    EvalResult r;
    r.status = EVAL_OK;
    r.value = v;
    r.message[0] = '\0';
    return r;
}

EvalResult eval_err(EvalStatus status, const char *msg) {
    EvalResult r;
    r.status = status;
    r.value = val_nil();
    if (msg) {
        strncpy(r.message, msg, sizeof(r.message) - 1);
        r.message[sizeof(r.message) - 1] = '\0';
    } else {
        r.message[0] = '\0';
    }
    return r;
}

/* ============ 字面量求值 ============ */

static EvalResult eval_literal(AstNode *expr) {
    switch (expr->kind) {
        case AST_INT_LIT:    return eval_ok(val_int(expr->as.int_val));
        case AST_FLOAT_LIT:  return eval_ok(val_float(expr->as.float_val));
        case AST_BOOL_LIT:   return eval_ok(val_bool(expr->as.bool_val));
        case AST_STRING_LIT:
            if (expr->as.string_lit.n_parts == 1 &&
                expr->as.string_lit.parts[0].kind == INTERP_TEXT) {
                return eval_ok(val_string(expr->as.string_lit.parts[0].text));
            }
            /* TODO: 字符串插值（第 11-12 周） */
            return eval_ok(val_string(""));
        case AST_CHAR_LIT:   return eval_ok(val_int(expr->as.char_val));
        case AST_IDENT:      return eval_err(EVAL_ERR_UNDEFINED_VAR, "未实现：ident 不应通过 literal 求值");
        default:             return eval_err(EVAL_ERR_TYPE, "非字面量");
    }
}

/* ============ 标识符 ============ */

static EvalResult eval_ident(AstNode *expr, Env *env) {
    Value v = env_get(env, expr->as.ident_name);
    return eval_ok(v);
}

/* ============ 二元运算 ============ */

static EvalResult eval_binary(AstNode *expr, Env *env) {
    EvalResult lhs = eval_expr(expr->as.binary.lhs, env);
    if (lhs.status != EVAL_OK) return lhs;
    EvalResult rhs = eval_expr(expr->as.binary.rhs, env);
    if (rhs.status != EVAL_OK) { val_free(&lhs.value); return rhs; }

    BinOp op = expr->as.binary.op;
    EvalResult result;

    /* 字符串 + 字符串 = 字符串拼接 */
    if (op == OP_ADD && lhs.value.kind == VAL_STRING && rhs.value.kind == VAL_STRING) {
        size_t llen = strlen(lhs.value.as.string_val);
        size_t rlen = strlen(rhs.value.as.string_val);
        char *buf = (char *)malloc(llen + rlen + 1);
        memcpy(buf, lhs.value.as.string_val, llen);
        memcpy(buf + llen, rhs.value.as.string_val, rlen);
        buf[llen + rlen] = '\0';
        result = eval_ok(val_string(buf));
        free(buf);
        val_free(&lhs.value);
        val_free(&rhs.value);
        return result;
    }

    /* 算术运算（提升到 double 如果任一边是 double） */
    bool is_float = (lhs.value.kind == VAL_FLOAT || rhs.value.kind == VAL_FLOAT);

    /* 比较运算（不修改 lhs/rhs）*/
    switch (op) {
        case OP_EQ: {
            bool eq = val_equal(&lhs.value, &rhs.value);
            val_free(&lhs.value);
            val_free(&rhs.value);
            return eval_ok(val_bool(eq));
        }
        case OP_NEQ: {
            bool eq = val_equal(&lhs.value, &rhs.value);
            val_free(&lhs.value);
            val_free(&rhs.value);
            return eval_ok(val_bool(!eq));
        }
        default: break;
    }

    if (is_float) {
        double a = val_to_float(&lhs.value);
        double b = val_to_float(&rhs.value);
        double r = 0;
        bool ok = true;
        switch (op) {
            case OP_ADD: r = a + b; break;
            case OP_SUB: r = a - b; break;
            case OP_MUL: r = a * b; break;
            case OP_DIV:
                if (b == 0) {
                    val_free(&lhs.value);
                    val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "除数为零");
                }
                r = a / b;
                break;
            case OP_MOD:
                if (b == 0) {
                    val_free(&lhs.value);
                    val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "取模为零");
                }
                r = (double)((int64_t)a % (int64_t)b);
                break;
            case OP_LT:  return eval_ok(val_bool(a < b));
            case OP_LE:  return eval_ok(val_bool(a <= b));
            case OP_GT:  return eval_ok(val_bool(a > b));
            case OP_GE:  return eval_ok(val_bool(a >= b));
            default:
                ok = false;
                break;
        }
        if (!ok) {
            val_free(&lhs.value);
            val_free(&rhs.value);
            return eval_err(EVAL_ERR_TYPE, "不支持的浮点运算");
        }
        val_free(&lhs.value);
        val_free(&rhs.value);
        return eval_ok(val_float(r));
    } else {
        /* 整数或位运算 */
        int64_t a = val_to_int(&lhs.value);
        int64_t b = val_to_int(&rhs.value);
        int64_t r = 0;
        switch (op) {
            case OP_ADD: r = a + b; break;
            case OP_SUB: r = a - b; break;
            case OP_MUL: r = a * b; break;
            case OP_DIV:
                if (b == 0) {
                    val_free(&lhs.value);
                    val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "除数为零");
                }
                r = a / b;
                break;
            case OP_MOD:
                if (b == 0) {
                    val_free(&lhs.value);
                    val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "取模为零");
                }
                r = a % b;
                break;
            case OP_BIT_AND: r = a & b; break;
            case OP_BIT_OR:  r = a | b; break;
            case OP_BIT_XOR: r = a ^ b; break;
            case OP_SHL: r = a << b; break;
            case OP_SHR: r = a >> b; break;
            case OP_AND:
                val_free(&lhs.value);
                val_free(&rhs.value);
                return eval_ok(val_bool(val_to_bool(&lhs.value) && val_to_bool(&rhs.value)));
            case OP_OR:
                val_free(&lhs.value);
                val_free(&rhs.value);
                return eval_ok(val_bool(val_to_bool(&lhs.value) || val_to_bool(&rhs.value)));
            case OP_LT:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a < b));
            case OP_LE:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a <= b));
            case OP_GT:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a > b));
            case OP_GE:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a >= b));
            default:
                val_free(&lhs.value);
                val_free(&rhs.value);
                return eval_err(EVAL_ERR_TYPE, "不支持的运算");
        }
        val_free(&lhs.value);
        val_free(&rhs.value);
        return eval_ok(val_int(r));
    }
}

/* ============ 一元运算 ============ */

static EvalResult eval_unary(AstNode *expr, Env *env) {
    EvalResult operand = eval_expr(expr->as.unary.operand, env);
    if (operand.status != EVAL_OK) return operand;

    switch (expr->as.unary.op) {
        case UOP_NEG:
            if (operand.value.kind == VAL_FLOAT) {
                double v = -operand.value.as.float_val;
                val_free(&operand.value);
                return eval_ok(val_float(v));
            } else {
                int64_t v = -val_to_int(&operand.value);
                val_free(&operand.value);
                return eval_ok(val_int(v));
            }
        case UOP_NOT:
            operand.value = val_bool(!val_to_bool(&operand.value));
            return operand;
        case UOP_BIT_NOT: {
            int64_t v = ~val_to_int(&operand.value);
            val_free(&operand.value);
            return eval_ok(val_int(v));
        }
    }
    val_free(&operand.value);
    return eval_err(EVAL_ERR_TYPE, "未知的一元运算");
}

/* ============ 赋值 ============ */

static EvalResult eval_assign(AstNode *expr, Env *env) {
    /* 简化：仅支持标识符赋值 */
    if (expr->as.assign.target->kind != AST_IDENT) {
        return eval_err(EVAL_ERR_TYPE, "赋值目标必须是变量");
    }
    EvalResult value = eval_expr(expr->as.assign.value, env);
    if (value.status != EVAL_OK) return value;

    const char *name = expr->as.assign.target->as.ident_name;
    if (!env_set(env, name, value.value)) {
        val_free(&value.value);
        return eval_err(EVAL_ERR_UNDEFINED_VAR, "变量未定义（应先用 let/var 定义）");
    }
    return eval_ok(value.value);
}

/* ============ 内置函数 ============ */

static EvalResult builtin_print(AstNode *call, Env *env) {
    for (size_t i = 0; i < call->as.call.n_args; i++) {
        EvalResult v = eval_expr(call->as.call.args[i], env);
        if (v.status != EVAL_OK) return v;
        if (i > 0) printf(" ");
        val_print(&v.value);
        val_free(&v.value);
    }
    printf("\n");
    fflush(stdout);
    return eval_ok(val_nil());
}

/* ============ 函数调用（占位） ============ */

static EvalResult eval_call(AstNode *expr, Env *env) {
    /* 内置 print */
    if (expr->as.call.func->kind == AST_IDENT &&
        strcmp(expr->as.call.func->as.ident_name, "print") == 0) {
        return builtin_print(expr, env);
    }
    /* TODO: 用户定义函数（第 9-10 周） */
    return eval_err(EVAL_ERR_RUNTIME, "未实现：用户函数调用");
}

/* ============ 主入口 ============ */

EvalResult eval_expr(AstNode *expr, Env *env) {
    if (!expr) return eval_err(EVAL_ERR_RUNTIME, "空表达式");

    switch (expr->kind) {
        case AST_INT_LIT:
        case AST_FLOAT_LIT:
        case AST_BOOL_LIT:
        case AST_STRING_LIT:
        case AST_CHAR_LIT:
            return eval_literal(expr);

        case AST_IDENT:
            return eval_ident(expr, env);

        case AST_BINARY_OP:
            return eval_binary(expr, env);

        case AST_UNARY_OP:
            return eval_unary(expr, env);

        case AST_ASSIGN:
            return eval_assign(expr, env);

        case AST_CALL:
            return eval_call(expr, env);

        case AST_VAR_DECL: {
            EvalResult v = {EVAL_OK, val_nil(), ""};
            if (expr->as.var_decl.init) {
                v = eval_expr(expr->as.var_decl.init, env);
            }
            if (v.status == EVAL_OK) {
                /* env_define 会克隆 v.value 并释放原始，所以这里只需传入 */
                env_define(env, expr->as.var_decl.name, v.value);
                /* 返回新值：对于字符串需要复制（避免双重释放），其它类型可以共享 */
                Value dup;
                dup.kind = v.value.kind;
                dup.as = v.value.as;
                if (v.value.kind == VAL_STRING) {
                    dup.as.string_val = v.value.as.string_val ? strdup(v.value.as.string_val) : strdup("");
                }
                return eval_ok(dup);
            }
            return v;
        }

        default:
            return eval_err(EVAL_ERR_TYPE, "暂不支持的表达式节点");
    }
}

/* ============ 程序执行（仅表达式语句；控制流第 9-10 周） ============ */

EvalStatus interp_exec_program(AstNode *program, Env *env) {
    if (!program || program->kind != AST_PROGRAM) {
        return EVAL_ERR_TYPE;
    }

    for (size_t i = 0; i < program->as.program.n_items; i++) {
        AstNode *item = program->as.program.items[i];
        if (item->kind == AST_FUNCTION) {
            /* 仅自动调用 main 函数 */
            if (strcmp(item->as.function.name, "main") == 0) {
                /* 暂存函数（不实现闭包） */
                /* 直接执行 main 的 body */
                AstNode *body = item->as.function.body;
                if (body && body->kind == AST_BLOCK) {
                    for (size_t j = 0; j < body->as.block.n_stmts; j++) {
                        AstNode *stmt = body->as.block.stmts[j];
                        if (stmt->kind == AST_VAR_DECL) {
                            EvalResult r = eval_expr(stmt, env);
                            if (r.status != EVAL_OK) {
                                fprintf(stderr, "错误: %s\n", r.message);
                                return r.status;
                            }
                            val_free(&r.value);
                        } else if (stmt->kind == AST_EXPR_STMT) {
                            EvalResult r = eval_expr(stmt->as.expr_stmt.expr, env);
                            if (r.status != EVAL_OK) {
                                fprintf(stderr, "错误: %s\n", r.message);
                                return r.status;
                            }
                            val_free(&r.value);
                        }
                        /* TODO: 控制流（第 9-10 周） */
                    }
                }
            }
        }
        /* TODO: 其它顶层项 */
    }

    return EVAL_OK;
}