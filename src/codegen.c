/*
 * codegen.c — QAW → C 代码生成器
 *
 * v0.5 MVP：用 C 作为中间表示
 *
 * 简化策略（v0.5 起步）：
 *   - 字符串 / 数字 / 布尔用统一的 char* 表示
 *   - 函数调用转为 C 函数调用
 *   - 控制流 1:1 映射
 *   - print 内置为 printf
 */

#define _POSIX_C_SOURCE 200809L

#include "qaw/codegen.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>

/* ============ 字符串缓冲 ============ */

typedef struct {
    char *buf;
    size_t len;
    size_t cap;
    int indent_level;
} Codegen;

static void cg_init(Codegen *cg) {
    cg->cap = 4096;
    cg->buf = (char *)malloc(cg->cap);
    cg->buf[0] = '\0';
    cg->len = 0;
    cg->indent_level = 0;
}

static void cg_free(Codegen *cg) {
    if (cg->buf) free(cg->buf);
    cg->buf = NULL;
}

static void cg_indent(Codegen *cg) {
    for (int i = 0; i < cg->indent_level; i++) {
        cg->len += snprintf(cg->buf + cg->len, cg->cap - cg->len, "  ");
    }
}

static void cg_append(Codegen *cg, const char *fmt, ...) {
    while (cg->len + 256 > cg->cap) {
        cg->cap *= 2;
        cg->buf = (char *)realloc(cg->buf, cg->cap);
    }
    va_list ap;
    va_start(ap, fmt);
    cg->len += vsnprintf(cg->buf + cg->len, cg->cap - cg->len, fmt, ap);
    va_end(ap);
}

static void cg_newline(Codegen *cg) {
    cg_append(cg, "\n");
}

/* ============ 类型映射 ============ */

static const char *c_type_for(const char *qaw_type) {
    if (!qaw_type) return "long long";  /* 默认可转为 long long */
    if (strcmp(qaw_type, "int") == 0) return "long long";
    if (strcmp(qaw_type, "float") == 0) return "double";
    if (strcmp(qaw_type, "bool") == 0) return "int";
    if (strcmp(qaw_type, "string") == 0) return "const char *";
    if (strcmp(qaw_type, "void") == 0) return "void";
    return "long long";  /* 未知类型降级 */
}

/* ============ 表达式生成 ============ */

static void gen_expr(Codegen *cg, AstNode *expr);

static void gen_literal(Codegen *cg, AstNode *expr) {
    switch (expr->kind) {
        case AST_INT_LIT:
            cg_append(cg, "%lldLL", (long long)expr->as.int_val);
            break;
        case AST_FLOAT_LIT:
            cg_append(cg, "%g", expr->as.float_val);
            break;
        case AST_BOOL_LIT:
            cg_append(cg, "%s", expr->as.bool_val ? "1" : "0");
            break;
        case AST_STRING_LIT: {
            /* 简单字符串：直接生成 C 字符串字面量 */
            if (expr->as.string_lit.n_parts == 1 &&
                expr->as.string_lit.parts[0].kind == INTERP_TEXT) {
                /* 转义 */
                cg_append(cg, "\"");
                const char *s = expr->as.string_lit.parts[0].text;
                for (const char *p = s; *p; p++) {
                    switch (*p) {
                        case '"': cg_append(cg, "\\\""); break;
                        case '\\': cg_append(cg, "\\\\"); break;
                        case '\n': cg_append(cg, "\\n"); break;
                        case '\t': cg_append(cg, "\\t"); break;
                        case '\r': cg_append(cg, "\\r"); break;
                        default: cg_append(cg, "%c", *p); break;
                    }
                }
                cg_append(cg, "\"");
            } else {
                /* 含插值：留待 v0.5 后期实现 */
                cg_append(cg, "\"\"");
            }
            break;
        }
        case AST_CHAR_LIT:
            cg_append(cg, "'%c'", (char)expr->as.char_val);
            break;
        default:
            cg_append(cg, "/* UNHANDLED LITERAL */");
            break;
    }
}

static void gen_binary(Codegen *cg, AstNode *expr) {
    const char *op_str = "?";
    switch (expr->as.binary.op) {
        case OP_ADD: op_str = "+"; break;
        case OP_SUB: op_str = "-"; break;
        case OP_MUL: op_str = "*"; break;
        case OP_DIV: op_str = "/"; break;
        case OP_MOD: op_str = "%"; break;
        case OP_EQ: op_str = "=="; break;
        case OP_NEQ: op_str = "!="; break;
        case OP_LT: op_str = "<"; break;
        case OP_LE: op_str = "<="; break;
        case OP_GT: op_str = ">"; break;
        case OP_GE: op_str = ">="; break;
        case OP_AND: op_str = "&&"; break;
        case OP_OR: op_str = "||"; break;
        case OP_BIT_AND: op_str = "&"; break;
        case OP_BIT_OR: op_str = "|"; break;
        case OP_BIT_XOR: op_str = "^"; break;
        case OP_SHL: op_str = "<<"; break;
        case OP_SHR: op_str = ">>"; break;
        case OP_CONCAT: op_str = "/* concat */ +"; break;
        default: break;
    }
    cg_append(cg, "(");
    gen_expr(cg, expr->as.binary.lhs);
    cg_append(cg, " %s ", op_str);
    gen_expr(cg, expr->as.binary.rhs);
    cg_append(cg, ")");
}

static void gen_unary(Codegen *cg, AstNode *expr) {
    const char *op_str = "";
    bool parens_around_operand = true;
    switch (expr->as.unary.op) {
        case UOP_NEG: op_str = "-"; break;
        case UOP_NOT: op_str = "!"; parens_around_operand = false; break;
        case UOP_BIT_NOT: op_str = "~"; break;
    }
    cg_append(cg, "%s", op_str);
    if (parens_around_operand) {
        cg_append(cg, "(");
        gen_expr(cg, expr->as.unary.operand);
        cg_append(cg, ")");
    } else {
        gen_expr(cg, expr->as.unary.operand);
    }
}

static void gen_ident(Codegen *cg, AstNode *expr) {
    cg_append(cg, "%s", expr->as.ident_name);
}

static void gen_call(Codegen *cg, AstNode *expr) {
    /* 内置 print → printf */
    if (expr->as.call.func->kind == AST_IDENT &&
        strcmp(expr->as.call.func->as.ident_name, "print") == 0) {
        /* 生成 printf 调用 */
        /* 简化：所有参数转字符串后拼接 */
        cg_append(cg, "do { ");
        cg_append(cg, "char __buf[1024]; __buf[0] = 0;");
        for (size_t i = 0; i < expr->as.call.n_args; i++) {
            cg_append(cg, " { char __t[256]; ");
            AstNode *arg = expr->as.call.args[i];
            if (arg->kind == AST_STRING_LIT) {
                /* 字符串字面量：直接用 __qaw_to_str_s */
                cg_append(cg, "__qaw_to_str_s(__t, ");
                if (arg->as.string_lit.n_parts == 1 &&
                    arg->as.string_lit.parts[0].kind == INTERP_TEXT) {
                    const char *s = arg->as.string_lit.parts[0].text;
                    cg_append(cg, "\"");
                    for (const char *p = s; *p; p++) {
                        switch (*p) {
                            case '"': cg_append(cg, "\\\""); break;
                            case '\\': cg_append(cg, "\\\\"); break;
                            case '\n': cg_append(cg, "\\n"); break;
                            default: cg_append(cg, "%c", *p); break;
                        }
                    }
                    cg_append(cg, "\"");
                } else {
                    cg_append(cg, "\"\"");
                }
                cg_append(cg, ");");
            } else if (arg->kind == AST_BOOL_LIT) {
                cg_append(cg, "__qaw_to_str_b(__t, ");
                cg_append(cg, "%s", arg->as.bool_val ? "1" : "0");
                cg_append(cg, ");");
            } else {
                cg_append(cg, "__qaw_to_str(__t, ");
                gen_expr(cg, arg);
                cg_append(cg, ");");
            }
            cg_append(cg, " strcat(__buf, __t); }");
        }
        cg_append(cg, " printf(\"%%s\\n\", __buf); } while(0)");
        return;
    }

    /* 用户函数 */
    if (expr->as.call.func->kind == AST_IDENT) {
        cg_append(cg, "qawfn_%s(", expr->as.call.func->as.ident_name);
        for (size_t i = 0; i < expr->as.call.n_args; i++) {
            if (i > 0) cg_append(cg, ", ");
            gen_expr(cg, expr->as.call.args[i]);
        }
        cg_append(cg, ")");
        return;
    }

    cg_append(cg, "/* unsupported call */");
}

static void gen_field_access(Codegen *cg, AstNode *expr) {
    /* 简化：转 (expr).field */
    gen_expr(cg, expr->as.field_access.obj);
    cg_append(cg, ".%s", expr->as.field_access.field_name);
}

static void gen_index(Codegen *cg, AstNode *expr) {
    /* 简化：转 ((expr)[(idx)]) */
    gen_expr(cg, expr->as.index.obj);
    cg_append(cg, "[");
    gen_expr(cg, expr->as.index.index);
    cg_append(cg, "]");
}

static void gen_assign(Codegen *cg, AstNode *expr) {
    if (expr->as.assign.target->kind == AST_IDENT) {
        gen_ident(cg, expr->as.assign.target);
        cg_append(cg, " = ");
        gen_expr(cg, expr->as.assign.value);
        cg_append(cg, ";");
    } else {
        gen_expr(cg, expr->as.assign.target);
        cg_append(cg, " = ");
        gen_expr(cg, expr->as.assign.value);
        cg_append(cg, ";");
    }
}

static void gen_var_decl(Codegen *cg, AstNode *expr) {
    const char *type = c_type_for(expr->as.var_decl.type_name);
    cg_indent(cg);
    if (expr->as.var_decl.type_name) {
        cg_append(cg, "%s ", type);
    } else {
        /* 类型推导：用 long long（最通用） */
        cg_append(cg, "long long ");
    }
    cg_append(cg, "%s", expr->as.var_decl.name);
    if (expr->as.var_decl.init) {
        cg_append(cg, " = ");
        gen_expr(cg, expr->as.var_decl.init);
    }
    cg_append(cg, ";");
    cg_newline(cg);
}

static void gen_expr(Codegen *cg, AstNode *expr) {
    if (!expr) {
        cg_append(cg, "/* NULL */");
        return;
    }
    switch (expr->kind) {
        case AST_INT_LIT: case AST_FLOAT_LIT: case AST_BOOL_LIT:
        case AST_STRING_LIT: case AST_CHAR_LIT:
            gen_literal(cg, expr);
            break;
        case AST_IDENT: gen_ident(cg, expr); break;
        case AST_BINARY_OP: gen_binary(cg, expr); break;
        case AST_UNARY_OP: gen_unary(cg, expr); break;
        case AST_CALL: gen_call(cg, expr); break;
        case AST_FIELD_ACCESS: gen_field_access(cg, expr); break;
        case AST_INDEX: gen_index(cg, expr); break;
        case AST_ASSIGN: gen_assign(cg, expr); break;
        case AST_VAR_DECL: {
            /* 在表达式位置：取 init 值 */
            if (expr->as.var_decl.init) {
                gen_expr(cg, expr->as.var_decl.init);
            } else {
                cg_append(cg, "0");
            }
            break;
        }
        default:
            cg_append(cg, "/* TODO: %d */", expr->kind);
            break;
    }
}

/* ============ 语句生成 ============ */

static void gen_stmt(Codegen *cg, AstNode *stmt);

static void gen_block(Codegen *cg, AstNode *block) {
    cg_append(cg, "{");
    cg_newline(cg);
    cg->indent_level++;
    for (size_t i = 0; i < block->as.block.n_stmts; i++) {
        gen_stmt(cg, block->as.block.stmts[i]);
    }
    cg->indent_level--;
    cg_indent(cg);
    cg_append(cg, "}");
}

static void gen_stmt(Codegen *cg, AstNode *stmt) {
    if (!stmt) return;
    switch (stmt->kind) {
        case AST_VAR_DECL:
            gen_var_decl(cg, stmt);
            break;
        case AST_ASSIGN: {
            cg_indent(cg);
            gen_assign(cg, stmt);
            cg_newline(cg);
            break;
        }
        case AST_EXPR_STMT: {
            cg_indent(cg);
            gen_expr(cg, stmt->as.expr_stmt.expr);
            cg_append(cg, ";");
            cg_newline(cg);
            break;
        }
        case AST_IF_STMT: {
            cg_indent(cg);
            cg_append(cg, "if (");
            gen_expr(cg, stmt->as.if_stmt.cond);
            cg_append(cg, ") ");
            gen_block(cg, stmt->as.if_stmt.then_block);
            cg_newline(cg);
            if (stmt->as.if_stmt.else_block) {
                cg_indent(cg);
                cg_append(cg, "else ");
                gen_block(cg, stmt->as.if_stmt.else_block);
                cg_newline(cg);
            }
            break;
        }
        case AST_WHILE_STMT: {
            cg_indent(cg);
            cg_append(cg, "while (");
            gen_expr(cg, stmt->as.while_stmt.cond);
            cg_append(cg, ") ");
            gen_block(cg, stmt->as.while_stmt.body);
            cg_newline(cg);
            break;
        }
        case AST_FOR_STMT: {
            /* 仅支持 for x from a to b [step s] */
            if (stmt->as.for_stmt.kind == FOR_RANGE) {
                cg_indent(cg);
                cg_append(cg, "for (long long %s = ", stmt->as.for_stmt.var_name);
                gen_expr(cg, stmt->as.for_stmt.start);
                cg_append(cg, "; %s <= ", stmt->as.for_stmt.var_name);
                gen_expr(cg, stmt->as.for_stmt.end);
                cg_append(cg, "; %s += ", stmt->as.for_stmt.var_name);
                if (stmt->as.for_stmt.step) {
                    gen_expr(cg, stmt->as.for_stmt.step);
                } else {
                    cg_append(cg, "1");
                }
                cg_append(cg, ") ");
                gen_block(cg, stmt->as.for_stmt.body);
                cg_newline(cg);
            } else {
                cg_indent(cg);
                cg_append(cg, "/* for-in not supported */");
                cg_newline(cg);
            }
            break;
        }
        case AST_RETURN_STMT: {
            cg_indent(cg);
            cg_append(cg, "return");
            if (stmt->as.return_stmt.value) {
                cg_append(cg, " ");
                gen_expr(cg, stmt->as.return_stmt.value);
            }
            cg_append(cg, ";");
            cg_newline(cg);
            break;
        }
        case AST_BREAK_STMT:
            cg_indent(cg);
            cg_append(cg, "break;");
            cg_newline(cg);
            break;
        case AST_CONTINUE_STMT:
            cg_indent(cg);
            cg_append(cg, "continue;");
            cg_newline(cg);
            break;
        case AST_BLOCK:
            gen_block(cg, stmt);
            break;
        default:
            cg_indent(cg);
            cg_append(cg, "/* TODO stmt kind=%d */", stmt->kind);
            cg_newline(cg);
            break;
    }
}

/* ============ 函数生成 ============ */

static void gen_function(Codegen *cg, AstNode *func) {
    const char *ret = c_type_for(func->as.function.ret_type);
    cg_append(cg, "static %s qawfn_%s(", ret, func->as.function.name);
    for (size_t i = 0; i < func->as.function.n_params; i++) {
        if (i > 0) cg_append(cg, ", ");
        const char *ptype = c_type_for(func->as.function.params[i].type_name);
        cg_append(cg, "%s %s", ptype, func->as.function.params[i].name);
    }
    if (func->as.function.n_params == 0) {
        cg_append(cg, "void");
    }
    cg_append(cg, ")");
    cg_newline(cg);
    if (func->as.function.body) {
        gen_block(cg, func->as.function.body);
    } else {
        cg_append(cg, "{}");
    }
    cg_newline(cg);
    cg_newline(cg);
}

/* ============ 顶层 ============ */

char *qaw_codegen_to_c(AstNode *program) {
    if (!program || program->kind != AST_PROGRAM) return NULL;

    Codegen cg;
    cg_init(&cg);

    /* 头文件 */
    cg_append(&cg, "/* Generated by QAW compiler v0.5 */\n");
    cg_append(&cg, "#include <stdio.h>\n");
    cg_append(&cg, "#include <stdlib.h>\n");
    cg_append(&cg, "#include <string.h>\n\n");

    /* 运行时辅助函数 */
    cg_append(&cg, "/* Runtime helpers */\n");
    cg_append(&cg, "void __qaw_to_str(char *out, long long v) {\n");
    cg_append(&cg, "    sprintf(out, \"%lld\", v);\n");
    cg_append(&cg, "}\n");
    cg_append(&cg, "void __qaw_to_str_d(char *out, double v) {\n");
    cg_append(&cg, "    sprintf(out, \"%g\", v);\n");
    cg_append(&cg, "}\n");
    cg_append(&cg, "void __qaw_to_str_s(char *out, const char *s) {\n");
    cg_append(&cg, "    if (s) strncpy(out, s, 255);\n");
    cg_append(&cg, "    else out[0] = 0;\n");
    cg_append(&cg, "    out[255] = 0;\n");
    cg_append(&cg, "}\n");
    cg_append(&cg, "void __qaw_to_str_b(char *out, int v) {\n");
    cg_append(&cg, "    strcpy(out, v ? \"true\" : \"false\");\n");
    cg_append(&cg, "}\n\n");

    /* 用户函数 */
    for (size_t i = 0; i < program->as.program.n_items; i++) {
        AstNode *item = program->as.program.items[i];
        if (item->kind == AST_FUNCTION) {
            gen_function(&cg, item);
        }
    }

    /* main 函数 */
    cg_append(&cg, "int main(void) {\n");
    for (size_t i = 0; i < program->as.program.n_items; i++) {
        AstNode *item = program->as.program.items[i];
        if (item->kind == AST_FUNCTION &&
            strcmp(item->as.function.name, "main") == 0) {
            if (item->as.function.body) {
                /* 用新作用域包裹 main 体 */
                cg.indent_level++;
                gen_block(&cg, item->as.function.body);
                cg.indent_level--;
            }
            break;
        }
    }
    cg_append(&cg, "    return 0;\n");
    cg_append(&cg, "}\n");

    char *result = strdup(cg.buf);
    cg_free(&cg);
    return result;
}

/* ============ 文件输出 ============ */

int qaw_codegen_write_file(const char *c_code, const char *out_path) {
    FILE *f = fopen(out_path, "w");
    if (!f) {
        fprintf(stderr, "无法写入文件: %s\n", out_path);
        return 1;
    }
    fputs(c_code, f);
    fclose(f);
    return 0;
}

/* 编译 C 代码：用 gcc/cc 调用 */
int qaw_codegen_compile_c(const char *c_path, const char *out_path) {
    char cmd[1024];
    const char *cc = getenv("CC");
    if (!cc) cc = "gcc";
    snprintf(cmd, sizeof(cmd), "%s -O2 -o %s %s 2>&1", cc, out_path, c_path);
    fprintf(stderr, "[CC] %s\n", cmd);
    int rc = system(cmd);
    if (rc != 0) {
        fprintf(stderr, "编译失败\n");
        return 1;
    }
    return 0;
}