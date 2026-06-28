/*
 * interpreter.c — Tree-walking 解释器（完整版 v0.1 POC）
 *
 * 第 9-12 周覆盖：
 *   - 控制流（if / while / for-range / break / continue / return）
 *   - 函数调用（main 内的非 main 函数）
 *   - 字符串插值（${expr}）
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/interpreter.h"
#include "yao/lexer.h"
#include "yao/parser.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
#include <stdarg.h>

/* ============ 函数注册表（顶层） ============ */

#define MAX_FUNCTIONS 256

typedef struct {
    AstNode **funcs;        /* AST_FUNCTION 节点数组 */
    size_t n_funcs;
} FuncRegistry;

static FuncRegistry *func_reg_new(void) {
    FuncRegistry *r = (FuncRegistry *)calloc(1, sizeof(FuncRegistry));
    return r;
}

static void func_reg_free(FuncRegistry *r) {
    if (!r) return;
    free(r->funcs);
    free(r);
}

static void func_reg_add(FuncRegistry *r, AstNode *func) {
    if (!r || r->n_funcs >= MAX_FUNCTIONS) return;
    r->funcs = (AstNode **)realloc(r->funcs, sizeof(AstNode *) * (r->n_funcs + 1));
    r->funcs[r->n_funcs++] = func;
}

static AstNode *func_reg_lookup(FuncRegistry *r, const char *name) {
    if (!r || !name) return NULL;
    for (size_t i = 0; i < r->n_funcs; i++) {
        AstNode *f = r->funcs[i];
        if (f && f->kind == AST_FUNCTION &&
            strcmp(f->as.function.name, name) == 0) {
            return f;
        }
    }
    return NULL;
}

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

/* ============ 字符串构造 ============ */

static char *str_concat(const char *a, const char *b) {
    size_t la = a ? strlen(a) : 0;
    size_t lb = b ? strlen(b) : 0;
    char *r = (char *)malloc(la + lb + 1);
    if (!r) return NULL;
    if (la) memcpy(r, a, la);
    if (lb) memcpy(r + la, b, lb);
    r[la + lb] = '\0';
    return r;
}

static char *value_to_cstr(const Value *v) {
    char buf[64];
    switch (v->kind) {
        case VAL_INT:
            snprintf(buf, sizeof(buf), "%" PRId64, v->as.int_val);
            return strdup(buf);
        case VAL_FLOAT:
            snprintf(buf, sizeof(buf), "%g", v->as.float_val);
            return strdup(buf);
        case VAL_BOOL:
            return strdup(v->as.bool_val ? "true" : "false");
        case VAL_STRING:
            return strdup(v->as.string_val);
        case VAL_NIL:
            return strdup("nil");
    }
    return strdup("");
}

/* ============ 字面量求值（含字符串插值） ============ */

static EvalResult eval_literal(AstNode *expr, Env *env);

/* 求值字符串插值
 *
 * v0.1 简化：词法层只返回整体字符串，运行时扫描 ${} 切分并对每段 expr 重新解析
 */

static char *interp_get_text(const AstInterpPart *part) {
    return part->text;
}

/* 扫描字符串中的 ${...} 块
 *
 * 返回交替的 [text, expr_text] 列表，保证从 text 开始：
 * - 偶数索引：text 段
 * - 奇数索引：expr 段
 *
 * 如果字符串以 ${ 开头，第一个 text 段为空字符串
 */
static char **scan_interp(const char *raw, size_t *out_n) {
    size_t cap = 8, n = 0;
    char **parts = (char **)malloc(sizeof(char *) * cap);

    const char *p = raw;
    const char *start = raw;

    /* 如果以 ${ 开头，先加空 text 段 */
    if (p[0] == '$' && p[1] == '{') {
        parts[n++] = strdup("");
    }

    while (*p) {
        if (p[0] == '$' && p[1] == '{') {
            /* 找匹配的 } */
            const char *q = p + 2;
            int depth = 1;
            while (*q && depth > 0) {
                if (*q == '{') depth++;
                else if (*q == '}') {
                    depth--;
                    if (depth == 0) break;
                }
                q++;
            }
            if (depth == 0) {
                size_t len = q - (p + 2);
                if (n + 1 >= cap) {
                    cap *= 2;
                    parts = (char **)realloc(parts, sizeof(char *) * cap);
                }
                parts[n++] = strndup(p + 2, len);
                p = q + 1;
            } else {
                if (n + 1 >= cap) {
                    cap *= 2;
                    parts = (char **)realloc(parts, sizeof(char *) * cap);
                }
                parts[n++] = strdup(p);
                break;
            }
        } else {
            /* 普通文本：累积到下一个 ${ */
            const char *q = p;
            while (*q && !(q[0] == '$' && q[1] == '{')) q++;
            size_t len = q - p;
            if (n + 1 >= cap) {
                cap *= 2;
                parts = (char **)realloc(parts, sizeof(char *) * cap);
            }
            parts[n++] = strndup(p, len);
            p = q;
        }
    }
    (void)start;
    *out_n = n;
    return parts;
}

static EvalResult eval_string_lit(AstNode *expr, Env *env) {
    if (expr->as.string_lit.n_parts == 0) {
        return eval_ok(val_string(""));
    }
    if (expr->as.string_lit.n_parts == 1 &&
        expr->as.string_lit.parts[0].kind == INTERP_TEXT) {
        /* 简单字符串：检查是否实际包含 ${ */
        const char *raw = expr->as.string_lit.parts[0].text;
        if (!strstr(raw, "${")) {
            return eval_ok(val_string(raw));
        }
        /* 实际包含插值：扫描切分 */
        size_t n_segments = 0;
        char **segments = scan_interp(raw, &n_segments);
        if (!segments) return eval_err(EVAL_ERR_RUNTIME, "内存分配失败");

        char *result = strdup("");
        for (size_t i = 0; i < n_segments; i++) {
            char *piece = NULL;
            if (i % 2 == 0) {
                /* 文本段 */
                piece = strdup(segments[i]);
            } else {
                /* 表达式段：解析并求值 */
                char *expr_src = segments[i];
                Lexer *lex = lexer_new(expr_src);
                if (lex) {
                    Parser *p = parser_new(lex);
                    if (p) {
                        AstNode *expr_node = parser_parse_expr(p);
                        if (expr_node && !parser_last_error(p)) {
                            EvalResult r = eval_expr(expr_node, env);
                            if (r.status == EVAL_OK) {
                                piece = value_to_cstr(&r.value);
                                val_free(&r.value);
                            } else {
                                fprintf(stderr, "插值表达式错误: %s\n", r.message);
                            }
                            ast_free(expr_node);
                        }
                        parser_free(p);
                    }
                    lexer_free(lex);
                }
                if (!piece) piece = strdup("?");
            }
            char *new_result = str_concat(result, piece ? piece : "");
            free(piece);
            free(result);
            result = new_result;
        }

        for (size_t i = 0; i < n_segments; i++) free(segments[i]);
        free(segments);

        Value v = val_string(result);
        free(result);
        return eval_ok(v);
    }

    /* 复杂插值（多于一段）：遍历 parts */
    char *result = strdup("");
    for (size_t i = 0; i < expr->as.string_lit.n_parts; i++) {
        AstInterpPart *part = &expr->as.string_lit.parts[i];
        char *piece = NULL;
        if (part->kind == INTERP_TEXT) {
            piece = strdup(part->text ? part->text : "");
        } else if (part->kind == INTERP_EXPR && part->expr) {
            EvalResult r = eval_expr(part->expr, env);
            if (r.status != EVAL_OK) {
                free(result);
                return r;
            }
            piece = value_to_cstr(&r.value);
            val_free(&r.value);
        } else {
            piece = strdup("");
        }
        char *new_result = str_concat(result, piece ? piece : "");
        free(piece);
        free(result);
        result = new_result;
    }

    Value v = val_string(result);
    free(result);
    return eval_ok(v);
}

static EvalResult eval_literal(AstNode *expr, Env *env) {
    switch (expr->kind) {
        case AST_INT_LIT:    return eval_ok(val_int(expr->as.int_val));
        case AST_FLOAT_LIT:  return eval_ok(val_float(expr->as.float_val));
        case AST_BOOL_LIT:   return eval_ok(val_bool(expr->as.bool_val));
        case AST_STRING_LIT: return eval_string_lit(expr, env);
        case AST_CHAR_LIT:   return eval_ok(val_int(expr->as.char_val));
        case AST_IDENT:      return eval_err(EVAL_ERR_UNDEFINED_VAR, "未实现：ident 不应通过 literal 求值");
        default:             return eval_err(EVAL_ERR_TYPE, "非字面量");
    }
}

/* ============ 标识符 ============ */

static EvalResult eval_ident(AstNode *expr, Env *env) {
    if (!env_has(env, expr->as.ident_name)) {
        EvalResult r = eval_err(EVAL_ERR_UNDEFINED_VAR, expr->as.ident_name);
        return r;
    }
    return eval_ok(env_get(env, expr->as.ident_name));
}

/* ============ 二元运算 ============ */

static EvalResult eval_binary(AstNode *expr, Env *env) {
    EvalResult lhs = eval_expr(expr->as.binary.lhs, env);
    if (lhs.status != EVAL_OK) return lhs;
    EvalResult rhs = eval_expr(expr->as.binary.rhs, env);
    if (rhs.status != EVAL_OK) { val_free(&lhs.value); return rhs; }

    BinOp op = expr->as.binary.op;
    EvalResult result;

    /* 字符串拼接 */
    if (op == OP_ADD && lhs.value.kind == VAL_STRING && rhs.value.kind == VAL_STRING) {
        char *concat = str_concat(lhs.value.as.string_val, rhs.value.as.string_val);
        result = eval_ok(val_string(concat));
        free(concat);
        val_free(&lhs.value);
        val_free(&rhs.value);
        return result;
    }

    /* 比较 */
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

    bool is_float = (lhs.value.kind == VAL_FLOAT || rhs.value.kind == VAL_FLOAT);

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
                    val_free(&lhs.value); val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "除数为零");
                }
                r = a / b;
                break;
            case OP_MOD:
                if (b == 0) {
                    val_free(&lhs.value); val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "取模为零");
                }
                r = (double)((int64_t)a % (int64_t)b);
                break;
            case OP_LT:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a < b));
            case OP_LE:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a <= b));
            case OP_GT:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a > b));
            case OP_GE:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a >= b));
            default: ok = false; break;
        }
        if (!ok) {
            val_free(&lhs.value); val_free(&rhs.value);
            return eval_err(EVAL_ERR_TYPE, "不支持的浮点运算");
        }
        val_free(&lhs.value); val_free(&rhs.value);
        return eval_ok(val_float(r));
    } else {
        int64_t a = val_to_int(&lhs.value);
        int64_t b = val_to_int(&rhs.value);
        int64_t r = 0;
        switch (op) {
            case OP_ADD: r = a + b; break;
            case OP_SUB: r = a - b; break;
            case OP_MUL: r = a * b; break;
            case OP_DIV:
                if (b == 0) {
                    val_free(&lhs.value); val_free(&rhs.value);
                    return eval_err(EVAL_ERR_DIV_BY_ZERO, "除数为零");
                }
                r = a / b;
                break;
            case OP_MOD:
                if (b == 0) {
                    val_free(&lhs.value); val_free(&rhs.value);
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
                val_free(&lhs.value); val_free(&rhs.value);
                return eval_ok(val_bool(val_to_bool(&lhs.value) && val_to_bool(&rhs.value)));
            case OP_OR:
                val_free(&lhs.value); val_free(&rhs.value);
                return eval_ok(val_bool(val_to_bool(&lhs.value) || val_to_bool(&rhs.value)));
            case OP_LT:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a < b));
            case OP_LE:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a <= b));
            case OP_GT:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a > b));
            case OP_GE:  val_free(&lhs.value); val_free(&rhs.value); return eval_ok(val_bool(a >= b));
            default:
                val_free(&lhs.value); val_free(&rhs.value);
                return eval_err(EVAL_ERR_TYPE, "不支持的运算");
        }
        val_free(&lhs.value); val_free(&rhs.value);
        return eval_ok(val_int(r));
    }
}

/* ============ 一元运算 ============ */

static EvalResult eval_unary(AstNode *expr, Env *env) {
    EvalResult operand = eval_expr(expr->as.unary.operand, env);
    if (operand.status != EVAL_OK) return operand;

    EvalResult result;
    switch (expr->as.unary.op) {
        case UOP_NEG:
            if (operand.value.kind == VAL_FLOAT) {
                result = eval_ok(val_float(-operand.value.as.float_val));
            } else {
                result = eval_ok(val_int(-val_to_int(&operand.value)));
            }
            break;
        case UOP_NOT:
            result = eval_ok(val_bool(!val_to_bool(&operand.value)));
            break;
        case UOP_BIT_NOT:
            result = eval_ok(val_int(~val_to_int(&operand.value)));
            break;
        default:
            result = eval_err(EVAL_ERR_TYPE, "未知的一元运算");
            break;
    }
    val_free(&operand.value);
    return result;
}

/* ============ 赋值 ============ */

static EvalResult eval_assign(AstNode *expr, Env *env) {
    if (expr->as.assign.target->kind != AST_IDENT) {
        return eval_err(EVAL_ERR_TYPE, "赋值目标必须是变量");
    }
    EvalResult value = eval_expr(expr->as.assign.value, env);
    if (value.status != EVAL_OK) return value;

    const char *name = expr->as.assign.target->as.ident_name;
    if (!env_set(env, name, value.value)) {
        val_free(&value.value);
        return eval_err(EVAL_ERR_UNDEFINED_VAR, name);
    }
    return eval_ok(value.value);
}

/* ============ 控制流状态 ============ */

typedef enum {
    CF_CONTINUE,
    CF_RETURN,
    CF_BREAK,
    CF_ERROR,
} ControlFlow;

typedef struct {
    ControlFlow flow;
    Value return_value;  /* CF_RETURN 时有效 */
    char error[256];
} ExecResult;

/* ============ 函数调用（用户函数） ============ */

/* 前向声明 */
static ExecResult exec_stmt(AstNode *stmt, Env *env, FuncRegistry *regs);

static EvalResult call_user_function(AstNode *func, AstNode **args, size_t n_args,
                                     Env *caller_env, FuncRegistry *regs) {
    /* 参数数量检查 */
    if (n_args != func->as.function.n_params) {
        return eval_err(EVAL_ERR_TYPE, "参数数量不匹配");
    }

    /* 创建新作用域（子环境） */
    Env *scope = env_child(caller_env);

    /* 求值并绑定参数 */
    for (size_t i = 0; i < n_args; i++) {
        EvalResult v = eval_expr(args[i], caller_env);
        if (v.status != EVAL_OK) {
            env_free(scope);
            return v;
        }
        env_define(scope, func->as.function.params[i].name, v.value);
    }

    /* 执行函数体 */
    ExecResult exec = exec_stmt(func->as.function.body, scope, regs);
    env_free(scope);

    if (exec.flow == CF_ERROR) {
        val_free(&exec.return_value);
        return eval_err(EVAL_ERR_RUNTIME, exec.error);
    }
    if (exec.flow == CF_BREAK) {
        val_free(&exec.return_value);
        return eval_err(EVAL_ERR_RUNTIME, "break 命中在函数外");
    }
    /* CF_RETURN 或 CF_CONTINUE */
    if (exec.flow == CF_RETURN) {
        return eval_ok(exec.return_value);
    }
    /* 函数体无 return */
    return eval_ok(val_nil());
}

/* 内置 print */
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

/* 函数调用入口 */
static EvalResult eval_call(AstNode *expr, Env *env, FuncRegistry *regs) {
    /* 内置 print */
    if (expr->as.call.func->kind == AST_IDENT &&
        strcmp(expr->as.call.func->as.ident_name, "print") == 0) {
        return builtin_print(expr, env);
    }

    /* 用户函数：先查找（即便调用者是字段访问等，留作 v0.5） */
    if (expr->as.call.func->kind != AST_IDENT) {
        return eval_err(EVAL_ERR_TYPE, "暂不支持非标识符函数调用");
    }

    const char *name = expr->as.call.func->as.ident_name;
    AstNode *func = func_reg_lookup(regs, name);
    if (!func) {
        return eval_err(EVAL_ERR_UNDEFINED_VAR, name);
    }

    return call_user_function(func, expr->as.call.args, expr->as.call.n_args, env, regs);
}

/* ============ 主表达式求值 ============ */

/* 全局当前函数注册表（在 interp_exec_program 中设置） */
static FuncRegistry *current_regs = NULL;

EvalResult eval_expr(AstNode *expr, Env *env) {
    if (!expr) return eval_err(EVAL_ERR_RUNTIME, "空表达式");
    if (!current_regs) {
        /* 无注册表：函数调用会失败，但其它表达式仍可求值 */
        current_regs = func_reg_new();  /* 临时空注册表 */
    }

    switch (expr->kind) {
        case AST_INT_LIT:
        case AST_FLOAT_LIT:
        case AST_BOOL_LIT:
        case AST_STRING_LIT:
        case AST_CHAR_LIT:
            return eval_literal(expr, env);

        case AST_IDENT:
            return eval_ident(expr, env);

        case AST_BINARY_OP:
            return eval_binary(expr, env);

        case AST_UNARY_OP:
            return eval_unary(expr, env);

        case AST_ASSIGN:
            return eval_assign(expr, env);

        case AST_CALL:
            return eval_call(expr, env, current_regs);

        case AST_VAR_DECL: {
            EvalResult v = {EVAL_OK, val_nil(), ""};
            if (expr->as.var_decl.init) {
                v = eval_expr(expr->as.var_decl.init, env);
            }
            if (v.status == EVAL_OK) {
                env_define(env, expr->as.var_decl.name, v.value);
                Value dup;
                dup.kind = v.value.kind;
                dup.as = v.value.as;
                if (v.value.kind == VAL_STRING && v.value.as.string_val) {
                    dup.as.string_val = strdup(v.value.as.string_val);
                }
                return eval_ok(dup);
            }
            return v;
        }

        default:
            return eval_err(EVAL_ERR_TYPE, "暂不支持的表达式节点");
    }
}

/* ============ 控制流语句执行 ============ */

static ExecResult exec_stmt(AstNode *stmt, Env *env, FuncRegistry *regs);

static ExecResult make_exec_ok(void) {
    ExecResult r;
    r.flow = CF_CONTINUE;
    r.return_value = val_nil();
    r.error[0] = '\0';
    return r;
}

static ExecResult make_exec_return(Value v) {
    ExecResult r;
    r.flow = CF_RETURN;
    r.return_value = v;
    r.error[0] = '\0';
    return r;
}

static ExecResult make_exec_break(void) {
    ExecResult r;
    r.flow = CF_BREAK;
    r.return_value = val_nil();
    r.error[0] = '\0';
    return r;
}

static ExecResult make_exec_error(const char *msg) {
    ExecResult r;
    r.flow = CF_ERROR;
    r.return_value = val_nil();
    if (msg) {
        strncpy(r.error, msg, sizeof(r.error) - 1);
        r.error[sizeof(r.error) - 1] = '\0';
    } else {
        r.error[0] = '\0';
    }
    return r;
}

/* 执行 if */
static ExecResult exec_if(AstNode *stmt, Env *env, FuncRegistry *regs) {
    EvalResult cond = eval_expr(stmt->as.if_stmt.cond, env);
    if (cond.status != EVAL_OK) {
        return make_exec_error(cond.message);
    }
    bool truthy = val_to_bool(&cond.value);
    val_free(&cond.value);

    if (truthy) {
        return exec_stmt(stmt->as.if_stmt.then_block, env, regs);
    } else if (stmt->as.if_stmt.else_block) {
        return exec_stmt(stmt->as.if_stmt.else_block, env, regs);
    }
    return make_exec_ok();
}

/* 执行 while */
static ExecResult exec_while(AstNode *stmt, Env *env, FuncRegistry *regs) {
    while (1) {
        EvalResult cond = eval_expr(stmt->as.while_stmt.cond, env);
        if (cond.status != EVAL_OK) {
            return make_exec_error(cond.message);
        }
        bool truthy = val_to_bool(&cond.value);
        val_free(&cond.value);
        if (!truthy) break;

        ExecResult body = exec_stmt(stmt->as.while_stmt.body, env, regs);
        if (body.flow == CF_BREAK) return make_exec_ok();
        if (body.flow == CF_RETURN || body.flow == CF_ERROR) return body;
    }
    return make_exec_ok();
}

/* 执行 for-range：for x from a to b [step s] { } */
static ExecResult exec_for_range(AstNode *stmt, Env *env, FuncRegistry *regs) {
    EvalResult s = eval_expr(stmt->as.for_stmt.start, env);
    if (s.status != EVAL_OK) return make_exec_error(s.message);
    EvalResult e = eval_expr(stmt->as.for_stmt.end, env);
    if (e.status != EVAL_OK) { val_free(&s.value); return make_exec_error(e.message); }

    int64_t start = val_to_int(&s.value);
    int64_t end = val_to_int(&e.value);
    int64_t step = 1;
    if (stmt->as.for_stmt.step) {
        EvalResult st = eval_expr(stmt->as.for_stmt.step, env);
        if (st.status != EVAL_OK) {
            val_free(&s.value); val_free(&e.value);
            return make_exec_error(st.message);
        }
        step = val_to_int(&st.value);
        if (step == 0) step = 1;
        val_free(&st.value);
    }
    val_free(&s.value);
    val_free(&e.value);

    Env *loop_scope = env_child(env);
    for (int64_t i = start; i <= end; i += step) {
        env_define(loop_scope, stmt->as.for_stmt.var_name, val_int(i));
        ExecResult body = exec_stmt(stmt->as.for_stmt.body, loop_scope, regs);
        if (body.flow == CF_BREAK) {
            env_free(loop_scope);
            return make_exec_ok();
        }
        if (body.flow == CF_RETURN || body.flow == CF_ERROR) {
            env_free(loop_scope);
            return body;
        }
    }
    env_free(loop_scope);
    return make_exec_ok();
}

/* 执行语句 */
static ExecResult exec_stmt(AstNode *stmt, Env *env, FuncRegistry *regs) {
    if (!stmt) return make_exec_ok();

    int line = stmt->line, col = stmt->col;

    switch (stmt->kind) {
        case AST_VAR_DECL: {
            Value v = val_nil();
            if (stmt->as.var_decl.init) {
                EvalResult r = eval_expr(stmt->as.var_decl.init, env);
                if (r.status != EVAL_OK) return make_exec_error(r.message);
                v = r.value;
            }
            env_define(env, stmt->as.var_decl.name, v);
            return make_exec_ok();
        }

        case AST_ASSIGN: {
            EvalResult r = eval_expr(stmt->as.assign.value, env);
            if (r.status != EVAL_OK) return make_exec_error(r.message);
            if (stmt->as.assign.target->kind != AST_IDENT) {
                val_free(&r.value);
                return make_exec_error("赋值目标必须是变量");
            }
            const char *name = stmt->as.assign.target->as.ident_name;
            if (!env_set(env, name, r.value)) {
                val_free(&r.value);
                return make_exec_error(name);
            }
            return make_exec_ok();
        }

        case AST_EXPR_STMT: {
            EvalResult r = eval_expr(stmt->as.expr_stmt.expr, env);
            if (r.status != EVAL_OK) return make_exec_error(r.message);
            val_free(&r.value);
            return make_exec_ok();
        }

        case AST_RETURN_STMT: {
            EvalResult r = {EVAL_OK, val_nil(), ""};
            if (stmt->as.return_stmt.value) {
                r = eval_expr(stmt->as.return_stmt.value, env);
                if (r.status != EVAL_OK) return make_exec_error(r.message);
            }
            return make_exec_return(r.value);
        }

        case AST_BREAK_STMT:
            return make_exec_break();

        case AST_CONTINUE_STMT:
            return make_exec_ok();

        case AST_IF_STMT:
            return exec_if(stmt, env, regs);

        case AST_WHILE_STMT:
            return exec_while(stmt, env, regs);

        case AST_FOR_STMT:
            if (stmt->as.for_stmt.kind == FOR_RANGE) {
                return exec_for_range(stmt, env, regs);
            }
            return make_exec_error("for-in 暂未实现（仅支持 for x from a to b）");

        case AST_BLOCK: {
            Env *scope = env_child(env);
            for (size_t i = 0; i < stmt->as.block.n_stmts; i++) {
                ExecResult r = exec_stmt(stmt->as.block.stmts[i], scope, regs);
                if (r.flow != CF_CONTINUE) {
                    env_free(scope);
                    return r;
                }
            }
            env_free(scope);
            return make_exec_ok();
        }

        default:
            return make_exec_error("未实现的语句");
    }
}

/* ============ 程序执行入口 ============ */

EvalStatus interp_exec_program(AstNode *program, Env *env) {
    if (!program || program->kind != AST_PROGRAM) {
        return EVAL_ERR_TYPE;
    }

    /* 构建函数注册表 */
    FuncRegistry *regs = func_reg_new();
    for (size_t i = 0; i < program->as.program.n_items; i++) {
        AstNode *item = program->as.program.items[i];
        if (item->kind == AST_FUNCTION) {
            func_reg_add(regs, item);
        }
    }

    /* 设置全局寄存器（供 eval_expr 内的 call 使用） */
    FuncRegistry *old_regs = current_regs;
    current_regs = regs;

    /* 找并执行 main */
    AstNode *main_func = func_reg_lookup(regs, "main");
    if (!main_func) {
        fprintf(stderr, "错误：未找到 main 函数\n");
        current_regs = old_regs;
        func_reg_free(regs);
        return EVAL_ERR_RUNTIME;
    }

    ExecResult r = exec_stmt(main_func->as.function.body, env, regs);

    current_regs = old_regs;
    func_reg_free(regs);

    if (r.flow == CF_ERROR) {
        fprintf(stderr, "运行时错误: %s\n", r.error);
        val_free(&r.return_value);
        return EVAL_ERR_RUNTIME;
    }
    val_free(&r.return_value);
    return EVAL_OK;
}