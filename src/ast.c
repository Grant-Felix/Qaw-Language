/*
 * ast.c — AST 构造器、释放、调试
 *
 * 妖语言 v0.1 POC 引导版
 */

#define _POSIX_C_SOURCE 200809L

#include "qaw/ast.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* ============ 工具函数 ============ */

static char *str_dup(const char *s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    char *p = (char *)malloc(len + 1);
    if (p) memcpy(p, s, len + 1);
    return p;
}

static AstNode *ast_new(AstKind kind, int line, int col) {
    AstNode *n = (AstNode *)calloc(1, sizeof(AstNode));
    if (!n) return NULL;
    n->kind = kind;
    n->line = line;
    n->col = col;
    return n;
}

/* ============ 字面量 ============ */

AstNode *ast_new_int_lit(int64_t val, int line, int col) {
    AstNode *n = ast_new(AST_INT_LIT, line, col);
    if (n) n->as.int_val = val;
    return n;
}

AstNode *ast_new_float_lit(double val, int line, int col) {
    AstNode *n = ast_new(AST_FLOAT_LIT, line, col);
    if (n) n->as.float_val = val;
    return n;
}

AstNode *ast_new_string_lit(const char *s, int line, int col) {
    AstNode *n = ast_new(AST_STRING_LIT, line, col);
    if (!n) return NULL;
    /* 简单字符串：作为单个 INTERP_TEXT 片段 */
    AstInterpPart *parts = (AstInterpPart *)malloc(sizeof(AstInterpPart));
    if (!parts) { free(n); return NULL; }
    parts[0].kind = INTERP_TEXT;
    parts[0].text = str_dup(s);
    parts[0].expr = NULL;
    n->as.string_lit.parts = parts;
    n->as.string_lit.n_parts = 1;
    return n;
}

AstNode *ast_new_interp_lit(AstInterpPart *parts, size_t n_parts, int line, int col) {
    AstNode *n = ast_new(AST_STRING_LIT, line, col);
    if (!n) return NULL;
    /* 复制 parts 数组（不接管调用者的所有权） */
    AstInterpPart *copy = NULL;
    if (n_parts > 0) {
        copy = (AstInterpPart *)malloc(sizeof(AstInterpPart) * n_parts);
        if (!copy) { free(n); return NULL; }
        for (size_t i = 0; i < n_parts; i++) {
            copy[i] = parts[i];
            /* parts[i] 的 text 已通过 ast_interp_text strdup，expr 是节点指针 */
            /* 不需要深拷贝 */
        }
    }
    n->as.string_lit.parts = copy;
    n->as.string_lit.n_parts = n_parts;
    return n;
}

AstNode *ast_new_bool_lit(bool val, int line, int col) {
    AstNode *n = ast_new(AST_BOOL_LIT, line, col);
    if (n) n->as.bool_val = val;
    return n;
}

AstNode *ast_new_char_lit(int32_t val, int line, int col) {
    AstNode *n = ast_new(AST_CHAR_LIT, line, col);
    if (n) n->as.char_val = val;
    return n;
}

/* ============ 标识符 ============ */

AstNode *ast_new_ident(const char *name, int line, int col) {
    AstNode *n = ast_new(AST_IDENT, line, col);
    if (n) n->as.ident_name = str_dup(name);
    return n;
}

/* ============ 运算符 ============ */

AstNode *ast_new_binary(BinOp op, AstNode *lhs, AstNode *rhs, int line, int col) {
    AstNode *n = ast_new(AST_BINARY_OP, line, col);
    if (!n) return NULL;
    n->as.binary.op = op;
    n->as.binary.lhs = lhs;
    n->as.binary.rhs = rhs;
    return n;
}

AstNode *ast_new_unary(UnOp op, AstNode *operand, int line, int col) {
    AstNode *n = ast_new(AST_UNARY_OP, line, col);
    if (!n) return NULL;
    n->as.unary.op = op;
    n->as.unary.operand = operand;
    return n;
}

AstNode *ast_new_call(AstNode *func, AstNode **args, size_t n_args, int line, int col) {
    AstNode *n = ast_new(AST_CALL, line, col);
    if (!n) return NULL;
    n->as.call.func = func;
    /* 复制 args 数组 */
    AstNode **args_copy = NULL;
    if (n_args > 0) {
        args_copy = (AstNode **)malloc(sizeof(AstNode *) * n_args);
        if (!args_copy) { free(n); return NULL; }
        for (size_t i = 0; i < n_args; i++) {
            args_copy[i] = args[i];
        }
    }
    n->as.call.args = args_copy;
    n->as.call.n_args = n_args;
    return n;
}

AstNode *ast_new_field_access(AstNode *obj, const char *field, int line, int col) {
    AstNode *n = ast_new(AST_FIELD_ACCESS, line, col);
    if (!n) return NULL;
    n->as.field_access.obj = obj;
    n->as.field_access.field_name = str_dup(field);
    return n;
}

AstNode *ast_new_index(AstNode *obj, AstNode *index, int line, int col) {
    AstNode *n = ast_new(AST_INDEX, line, col);
    if (!n) return NULL;
    n->as.index.obj = obj;
    n->as.index.index = index;
    return n;
}

AstNode *ast_new_slice(AstNode *obj, AstNode *start, AstNode *end, bool inclusive, int line, int col) {
    AstNode *n = ast_new(AST_SLICE, line, col);
    if (!n) return NULL;
    n->as.slice.obj = obj;
    n->as.slice.start = start;
    n->as.slice.end = end;
    n->as.slice.inclusive = inclusive;
    return n;
}

/* ============ 语句 ============ */

AstNode *ast_new_var_decl(const char *name, const char *type, bool is_mut, AstNode *init, int line, int col) {
    AstNode *n = ast_new(AST_VAR_DECL, line, col);
    if (!n) return NULL;
    n->as.var_decl.name = str_dup(name);
    n->as.var_decl.type_name = str_dup(type);
    n->as.var_decl.is_mut = is_mut;
    n->as.var_decl.init = init;
    return n;
}

AstNode *ast_new_assign(AstNode *target, AstNode *value, int line, int col) {
    AstNode *n = ast_new(AST_ASSIGN, line, col);
    if (!n) return NULL;
    n->as.assign.target = target;
    n->as.assign.value = value;
    return n;
}

AstNode *ast_new_expr_stmt(AstNode *expr, int line, int col) {
    AstNode *n = ast_new(AST_EXPR_STMT, line, col);
    if (!n) return NULL;
    n->as.expr_stmt.expr = expr;
    return n;
}

AstNode *ast_new_block(AstNode **stmts, size_t n_stmts, int line, int col) {
    AstNode *n = ast_new(AST_BLOCK, line, col);
    if (!n) return NULL;
    /* 复制 stmts 数组 */
    AstNode **stmts_copy = NULL;
    if (n_stmts > 0) {
        stmts_copy = (AstNode **)malloc(sizeof(AstNode *) * n_stmts);
        if (!stmts_copy) { free(n); return NULL; }
        for (size_t i = 0; i < n_stmts; i++) {
            stmts_copy[i] = stmts[i];
        }
    }
    n->as.block.stmts = stmts_copy;
    n->as.block.n_stmts = n_stmts;
    return n;
}

AstNode *ast_new_if(AstNode *cond, AstNode *then_block, AstNode *else_block, int line, int col) {
    AstNode *n = ast_new(AST_IF_STMT, line, col);
    if (!n) return NULL;
    n->as.if_stmt.cond = cond;
    n->as.if_stmt.then_block = then_block;
    n->as.if_stmt.else_block = else_block;
    return n;
}

AstNode *ast_new_while(AstNode *cond, AstNode *body, int line, int col) {
    AstNode *n = ast_new(AST_WHILE_STMT, line, col);
    if (!n) return NULL;
    n->as.while_stmt.cond = cond;
    n->as.while_stmt.body = body;
    return n;
}

AstNode *ast_new_for_in(const char *var, AstNode *iterable, AstNode *body, int line, int col) {
    AstNode *n = ast_new(AST_FOR_STMT, line, col);
    if (!n) return NULL;
    n->as.for_stmt.kind = FOR_IN;
    n->as.for_stmt.var_name = str_dup(var);
    n->as.for_stmt.iterable = iterable;
    n->as.for_stmt.start = NULL;
    n->as.for_stmt.end = NULL;
    n->as.for_stmt.step = NULL;
    n->as.for_stmt.body = body;
    return n;
}

AstNode *ast_new_for_range(const char *var, AstNode *start, AstNode *end, AstNode *step, AstNode *body, int line, int col) {
    AstNode *n = ast_new(AST_FOR_STMT, line, col);
    if (!n) return NULL;
    n->as.for_stmt.kind = FOR_RANGE;
    n->as.for_stmt.var_name = str_dup(var);
    n->as.for_stmt.iterable = NULL;
    n->as.for_stmt.start = start;
    n->as.for_stmt.end = end;
    n->as.for_stmt.step = step;
    n->as.for_stmt.body = body;
    return n;
}

AstNode *ast_new_match(AstNode *scrutinee, AstMatchArm *arms, size_t n_arms, int line, int col) {
    AstNode *n = ast_new(AST_MATCH_STMT, line, col);
    if (!n) return NULL;
    n->as.match_stmt.scrutinee = scrutinee;
    n->as.match_stmt.arms = NULL;
    n->as.match_stmt.n_arms = 0;
    if (n_arms > 0) {
        n->as.match_stmt.arms = (AstMatchArm *)malloc(sizeof(AstMatchArm) * n_arms);
        if (n->as.match_stmt.arms) {
            for (size_t i = 0; i < n_arms; i++) {
                n->as.match_stmt.arms[i].pattern = arms[i].pattern ? strdup(arms[i].pattern) : NULL;
                n->as.match_stmt.arms[i].body = arms[i].body;
            }
            n->as.match_stmt.n_arms = n_arms;
        }
    }
    return n;
}

AstMatchArm ast_match_arm(const char *pattern, AstNode *body) {
    AstMatchArm a;
    a.pattern = pattern ? strdup(pattern) : NULL;
    a.body = body;
    return a;
}

AstNode *ast_new_return(AstNode *value, int line, int col) {
    AstNode *n = ast_new(AST_RETURN_STMT, line, col);
    if (!n) return NULL;
    n->as.return_stmt.value = value;
    return n;
}

AstNode *ast_new_break(int line, int col) {
    return ast_new(AST_BREAK_STMT, line, col);
}

AstNode *ast_new_continue(int line, int col) {
    return ast_new(AST_CONTINUE_STMT, line, col);
}

/* ============ 声明 ============ */

AstNode *ast_new_function(const char *name, AstParam *params, size_t n_params,
                          const char *ret_type, AstNode *body, int line, int col) {
    AstNode *n = ast_new(AST_FUNCTION, line, col);
    if (!n) return NULL;
    n->as.function.name = str_dup(name);
    /* 复制 params 数组 */
    AstParam *params_copy = NULL;
    if (n_params > 0) {
        params_copy = (AstParam *)malloc(sizeof(AstParam) * n_params);
        if (!params_copy) { free(n->as.function.name); free(n); return NULL; }
        for (size_t i = 0; i < n_params; i++) {
            params_copy[i].name = str_dup(params[i].name);
            params_copy[i].type_name = str_dup(params[i].type_name);
        }
    }
    n->as.function.params = params_copy;
    n->as.function.n_params = n_params;
    n->as.function.ret_type = str_dup(ret_type);
    n->as.function.body = body;
    return n;
}

AstNode *ast_new_struct(const char *name, AstFieldDecl *fields, size_t n_fields, int line, int col) {
    AstNode *n = ast_new(AST_STRUCT_DECL, line, col);
    if (!n) return NULL;
    n->as.struct_decl.name = str_dup(name);
    /* 复制 fields 数组 */
    AstFieldDecl *fields_copy = NULL;
    if (n_fields > 0) {
        fields_copy = (AstFieldDecl *)malloc(sizeof(AstFieldDecl) * n_fields);
        if (!fields_copy) { free(n->as.struct_decl.name); free(n); return NULL; }
        for (size_t i = 0; i < n_fields; i++) {
            fields_copy[i].name = str_dup(fields[i].name);
            fields_copy[i].type_name = str_dup(fields[i].type_name);
        }
    }
    n->as.struct_decl.fields = fields_copy;
    n->as.struct_decl.n_fields = n_fields;
    return n;
}

AstNode *ast_new_enum(const char *name, AstVariantDecl *variants, size_t n_variants, int line, int col) {
    AstNode *n = ast_new(AST_ENUM_DECL, line, col);
    if (!n) return NULL;
    n->as.enum_decl.name = str_dup(name);
    /* 复制 variants 数组 */
    AstVariantDecl *variants_copy = NULL;
    if (n_variants > 0) {
        variants_copy = (AstVariantDecl *)malloc(sizeof(AstVariantDecl) * n_variants);
        if (!variants_copy) { free(n->as.enum_decl.name); free(n); return NULL; }
        for (size_t i = 0; i < n_variants; i++) {
            variants_copy[i].name = str_dup(variants[i].name);
            variants_copy[i].payload = str_dup(variants[i].payload);
        }
    }
    n->as.enum_decl.variants = variants_copy;
    n->as.enum_decl.n_variants = n_variants;
    return n;
}

AstNode *ast_new_program(AstNode **items, size_t n_items, int line, int col) {
    AstNode *n = ast_new(AST_PROGRAM, line, col);
    if (!n) return NULL;
    /* 复制 items 数组 */
    AstNode **items_copy = NULL;
    if (n_items > 0) {
        items_copy = (AstNode **)malloc(sizeof(AstNode *) * n_items);
        if (!items_copy) { free(n); return NULL; }
        for (size_t i = 0; i < n_items; i++) {
            items_copy[i] = items[i];
        }
    }
    n->as.program.items = items_copy;
    n->as.program.n_items = n_items;
    return n;
}

/* ============ 字符串插值辅助 ============ */

AstInterpPart ast_interp_text(const char *text) {
    AstInterpPart p;
    p.kind = INTERP_TEXT;
    p.text = str_dup(text);
    p.expr = NULL;
    return p;
}

AstInterpPart ast_interp_expr(AstNode *expr) {
    AstInterpPart p;
    p.kind = INTERP_EXPR;
    p.text = NULL;
    p.expr = expr;
    return p;
}

/* ============ 释放 ============ */

void ast_free(AstNode *node) {
    if (!node) return;

    switch (node->kind) {
        case AST_STRING_LIT:
            if (node->as.string_lit.parts) {
                for (size_t i = 0; i < node->as.string_lit.n_parts; i++) {
                    free(node->as.string_lit.parts[i].text);
                    ast_free(node->as.string_lit.parts[i].expr);
                }
                free(node->as.string_lit.parts);
            }
            break;

        case AST_IDENT:
            free(node->as.ident_name);
            break;

        case AST_BINARY_OP:
            ast_free(node->as.binary.lhs);
            ast_free(node->as.binary.rhs);
            break;

        case AST_UNARY_OP:
            ast_free(node->as.unary.operand);
            break;

        case AST_CALL:
            ast_free(node->as.call.func);
            if (node->as.call.args) {
                for (size_t i = 0; i < node->as.call.n_args; i++) {
                    ast_free(node->as.call.args[i]);
                }
                free(node->as.call.args);
            }
            break;

        case AST_FIELD_ACCESS:
            ast_free(node->as.field_access.obj);
            free(node->as.field_access.field_name);
            break;

        case AST_INDEX:
            ast_free(node->as.index.obj);
            ast_free(node->as.index.index);
            break;

        case AST_SLICE:
            ast_free(node->as.slice.obj);
            ast_free(node->as.slice.start);
            ast_free(node->as.slice.end);
            break;

        case AST_VAR_DECL:
            free(node->as.var_decl.name);
            free(node->as.var_decl.type_name);
            ast_free(node->as.var_decl.init);
            break;

        case AST_ASSIGN:
            ast_free(node->as.assign.target);
            ast_free(node->as.assign.value);
            break;

        case AST_EXPR_STMT:
            ast_free(node->as.expr_stmt.expr);
            break;

        case AST_BLOCK:
            if (node->as.block.stmts) {
                for (size_t i = 0; i < node->as.block.n_stmts; i++) {
                    ast_free(node->as.block.stmts[i]);
                }
                free(node->as.block.stmts);
            }
            break;

        case AST_IF_STMT:
            ast_free(node->as.if_stmt.cond);
            ast_free(node->as.if_stmt.then_block);
            ast_free(node->as.if_stmt.else_block);
            break;

        case AST_WHILE_STMT:
            ast_free(node->as.while_stmt.cond);
            ast_free(node->as.while_stmt.body);
            break;

        case AST_FOR_STMT:
            free(node->as.for_stmt.var_name);
            ast_free(node->as.for_stmt.iterable);
            ast_free(node->as.for_stmt.start);
            ast_free(node->as.for_stmt.end);
            ast_free(node->as.for_stmt.step);
            ast_free(node->as.for_stmt.body);
            break;

        case AST_MATCH_STMT:
            ast_free(node->as.match_stmt.scrutinee);
            if (node->as.match_stmt.arms) {
                for (size_t i = 0; i < node->as.match_stmt.n_arms; i++) {
                    free(node->as.match_stmt.arms[i].pattern);
                    ast_free(node->as.match_stmt.arms[i].body);
                }
                free(node->as.match_stmt.arms);
            }
            break;

        case AST_RETURN_STMT:
            ast_free(node->as.return_stmt.value);
            break;

        case AST_BREAK_STMT:
        case AST_CONTINUE_STMT:
            break;

        case AST_FUNCTION:
            free(node->as.function.name);
            free(node->as.function.ret_type);
            if (node->as.function.params) {
                for (size_t i = 0; i < node->as.function.n_params; i++) {
                    free(node->as.function.params[i].name);
                    free(node->as.function.params[i].type_name);
                }
                free(node->as.function.params);
            }
            ast_free(node->as.function.body);
            break;

        case AST_STRUCT_DECL:
            free(node->as.struct_decl.name);
            if (node->as.struct_decl.fields) {
                for (size_t i = 0; i < node->as.struct_decl.n_fields; i++) {
                    free(node->as.struct_decl.fields[i].name);
                    free(node->as.struct_decl.fields[i].type_name);
                }
                free(node->as.struct_decl.fields);
            }
            break;

        case AST_ENUM_DECL:
            free(node->as.enum_decl.name);
            if (node->as.enum_decl.variants) {
                for (size_t i = 0; i < node->as.enum_decl.n_variants; i++) {
                    free(node->as.enum_decl.variants[i].name);
                    free(node->as.enum_decl.variants[i].payload);
                }
                free(node->as.enum_decl.variants);
            }
            break;

        case AST_PROGRAM:
            if (node->as.program.items) {
                for (size_t i = 0; i < node->as.program.n_items; i++) {
                    ast_free(node->as.program.items[i]);
                }
                free(node->as.program.items);
            }
            break;

        case AST_INT_LIT:
        case AST_FLOAT_LIT:
        case AST_BOOL_LIT:
        case AST_CHAR_LIT:
            /* 无堆分配 */
            break;
    }

    free(node);
}

/* ============ 调试 ============ */

const char *ast_kind_name(AstKind kind) {
    switch (kind) {
        case AST_INT_LIT: return "int_lit";
        case AST_FLOAT_LIT: return "float_lit";
        case AST_STRING_LIT: return "string_lit";
        case AST_BOOL_LIT: return "bool_lit";
        case AST_CHAR_LIT: return "char_lit";
        case AST_IDENT: return "ident";
        case AST_BINARY_OP: return "binary_op";
        case AST_UNARY_OP: return "unary_op";
        case AST_CALL: return "call";
        case AST_FIELD_ACCESS: return "field_access";
        case AST_INDEX: return "index";
        case AST_SLICE: return "slice";
        case AST_VAR_DECL: return "var_decl";
        case AST_ASSIGN: return "assign";
        case AST_EXPR_STMT: return "expr_stmt";
        case AST_BLOCK: return "block";
        case AST_IF_STMT: return "if_stmt";
        case AST_WHILE_STMT: return "while_stmt";
        case AST_FOR_STMT: return "for_stmt";
        case AST_MATCH_STMT: return "match_stmt";
        case AST_RETURN_STMT: return "return_stmt";
        case AST_BREAK_STMT: return "break_stmt";
        case AST_CONTINUE_STMT: return "continue_stmt";
        case AST_FUNCTION: return "function";
        case AST_STRUCT_DECL: return "struct_decl";
        case AST_ENUM_DECL: return "enum_decl";
        case AST_PROGRAM: return "program";
        default: return "?";
    }
}

const char *binop_name(BinOp op) {
    switch (op) {
        case OP_ADD: return "+";
        case OP_SUB: return "-";
        case OP_MUL: return "*";
        case OP_DIV: return "/";
        case OP_MOD: return "%";
        case OP_EQ: return "==";
        case OP_NEQ: return "!=";
        case OP_LT: return "<";
        case OP_LE: return "<=";
        case OP_GT: return ">";
        case OP_GE: return ">=";
        case OP_AND: return "&&";
        case OP_OR: return "||";
        case OP_BIT_AND: return "&";
        case OP_BIT_OR: return "|";
        case OP_BIT_XOR: return "^";
        case OP_SHL: return "<<";
        case OP_SHR: return ">>";
        case OP_ASSIGN: return "=";
        case OP_CONCAT: return "++";
        default: return "?";
    }
}

const char *unop_name(UnOp op) {
    switch (op) {
        case UOP_NEG: return "-";
        case UOP_NOT: return "!";
        case UOP_BIT_NOT: return "~";
        default: return "?";
    }
}

static void print_indent(int indent) {
    for (int i = 0; i < indent; i++) putchar(' ');
    for (int i = 0; i < indent; i++) putchar(' ');
}

void ast_print(const AstNode *node, int indent) {
    if (!node) return;

    print_indent(indent);
    printf("%s @ %d:%d", ast_kind_name(node->kind), node->line, node->col);

    switch (node->kind) {
        case AST_INT_LIT:    printf(" = %lld\n", (long long)node->as.int_val); break;
        case AST_FLOAT_LIT:  printf(" = %f\n", node->as.float_val); break;
        case AST_BOOL_LIT:   printf(" = %s\n", node->as.bool_val ? "true" : "false"); break;
        case AST_CHAR_LIT:   printf(" = U+%04X\n", node->as.char_val); break;

        case AST_STRING_LIT:
            printf(" parts=%zu\n", node->as.string_lit.n_parts);
            for (size_t i = 0; i < node->as.string_lit.n_parts; i++) {
                print_indent(indent + 2);
                if (node->as.string_lit.parts[i].kind == INTERP_TEXT) {
                    printf("text: \"%s\"\n", node->as.string_lit.parts[i].text);
                } else {
                    printf("expr:\n");
                    ast_print(node->as.string_lit.parts[i].expr, indent + 4);
                }
            }
            break;

        case AST_IDENT:       printf(" name=%s\n", node->as.ident_name); break;

        case AST_BINARY_OP:
            printf(" op=%s\n", binop_name(node->as.binary.op));
            ast_print(node->as.binary.lhs, indent + 2);
            ast_print(node->as.binary.rhs, indent + 2);
            break;

        case AST_UNARY_OP:
            printf(" op=%s\n", unop_name(node->as.unary.op));
            ast_print(node->as.unary.operand, indent + 2);
            break;

        case AST_CALL:
            printf(" args=%zu\n", node->as.call.n_args);
            ast_print(node->as.call.func, indent + 2);
            for (size_t i = 0; i < node->as.call.n_args; i++) {
                ast_print(node->as.call.args[i], indent + 2);
            }
            break;

        case AST_FIELD_ACCESS:
            printf(" field=%s\n", node->as.field_access.field_name);
            ast_print(node->as.field_access.obj, indent + 2);
            break;

        case AST_INDEX:
            printf("\n");
            ast_print(node->as.index.obj, indent + 2);
            ast_print(node->as.index.index, indent + 2);
            break;

        case AST_SLICE:
            printf(" inclusive=%s\n", node->as.slice.inclusive ? "true" : "false");
            ast_print(node->as.slice.obj, indent + 2);
            if (node->as.slice.start) ast_print(node->as.slice.start, indent + 2);
            if (node->as.slice.end) ast_print(node->as.slice.end, indent + 2);
            break;

        case AST_VAR_DECL:
            printf(" name=%s mut=%d\n", node->as.var_decl.name, node->as.var_decl.is_mut);
            if (node->as.var_decl.type_name)
                printf("%s  type=%s\n", "", node->as.var_decl.type_name);
            if (node->as.var_decl.init)
                ast_print(node->as.var_decl.init, indent + 2);
            break;

        case AST_ASSIGN:
            printf("\n");
            ast_print(node->as.assign.target, indent + 2);
            ast_print(node->as.assign.value, indent + 2);
            break;

        case AST_EXPR_STMT:
            printf("\n");
            ast_print(node->as.expr_stmt.expr, indent + 2);
            break;

        case AST_BLOCK:
            printf(" stmts=%zu\n", node->as.block.n_stmts);
            for (size_t i = 0; i < node->as.block.n_stmts; i++) {
                ast_print(node->as.block.stmts[i], indent + 2);
            }
            break;

        case AST_IF_STMT:
            printf("\n");
            ast_print(node->as.if_stmt.cond, indent + 2);
            ast_print(node->as.if_stmt.then_block, indent + 2);
            if (node->as.if_stmt.else_block)
                ast_print(node->as.if_stmt.else_block, indent + 2);
            break;

        case AST_WHILE_STMT:
            printf("\n");
            ast_print(node->as.while_stmt.cond, indent + 2);
            ast_print(node->as.while_stmt.body, indent + 2);
            break;

        case AST_FOR_STMT:
            printf(" var=%s kind=%s\n", node->as.for_stmt.var_name,
                   node->as.for_stmt.kind == FOR_IN ? "for_in" : "for_range");
            if (node->as.for_stmt.iterable)
                ast_print(node->as.for_stmt.iterable, indent + 2);
            if (node->as.for_stmt.start) {
                ast_print(node->as.for_stmt.start, indent + 2);
                ast_print(node->as.for_stmt.end, indent + 2);
            }
            if (node->as.for_stmt.step)
                ast_print(node->as.for_stmt.step, indent + 2);
            ast_print(node->as.for_stmt.body, indent + 2);
            break;

        case AST_MATCH_STMT:
            printf(" arms=%zu\n", node->as.match_stmt.n_arms);
            if (node->as.match_stmt.scrutinee)
                ast_print(node->as.match_stmt.scrutinee, indent + 2);
            for (size_t i = 0; i < node->as.match_stmt.n_arms; i++) {
                print_indent(indent + 2);
                printf("arm[%zu] pattern='%s'\n", i, node->as.match_stmt.arms[i].pattern ? node->as.match_stmt.arms[i].pattern : "");
                ast_print(node->as.match_stmt.arms[i].body, indent + 2);
            }
            break;

        case AST_RETURN_STMT:
            printf("\n");
            if (node->as.return_stmt.value)
                ast_print(node->as.return_stmt.value, indent + 2);
            else
                print_indent(indent + 2), printf("(void)\n");
            break;

        case AST_BREAK_STMT:    printf("\n"); break;
        case AST_CONTINUE_STMT: printf("\n"); break;

        case AST_FUNCTION:
            printf(" name=%s params=%zu ret=%s\n",
                   node->as.function.name, node->as.function.n_params,
                   node->as.function.ret_type ? node->as.function.ret_type : "void");
            ast_print(node->as.function.body, indent + 2);
            break;

        case AST_STRUCT_DECL:
            printf(" name=%s fields=%zu\n",
                   node->as.struct_decl.name, node->as.struct_decl.n_fields);
            for (size_t i = 0; i < node->as.struct_decl.n_fields; i++) {
                print_indent(indent + 2);
                printf("%s: %s\n", node->as.struct_decl.fields[i].name,
                       node->as.struct_decl.fields[i].type_name);
            }
            break;

        case AST_ENUM_DECL:
            printf(" name=%s variants=%zu\n",
                   node->as.enum_decl.name, node->as.enum_decl.n_variants);
            for (size_t i = 0; i < node->as.enum_decl.n_variants; i++) {
                print_indent(indent + 2);
                printf("%s\n", node->as.enum_decl.variants[i].name);
            }
            break;

        case AST_PROGRAM:
            printf(" items=%zu\n", node->as.program.n_items);
            for (size_t i = 0; i < node->as.program.n_items; i++) {
                ast_print(node->as.program.items[i], indent + 2);
            }
            break;
    }
}