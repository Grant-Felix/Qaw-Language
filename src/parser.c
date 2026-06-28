/*
 * parser.c — 递归下降 + Pratt 解析器
 *
 * 妖语言 v0.1 POC 引导版
 *
 * 第 3-4 周范围：
 *   - parse_program
 *   - parse_item → parse_function / parse_struct / parse_enum
 *   - parse_param_list
 *   - parse_block（块级语句的占位）
 *   - parse_var_decl（变量声明的占位）
 *
 * 第 5-6 周添加：parse_expr / parse_control_flow
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/parser.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdarg.h>

/* ============ 工具函数 ============ */

Parser *parser_new(Lexer *lex) {
    Parser *p = (Parser *)calloc(1, sizeof(Parser));
    if (!p) return NULL;
    p->lex = lex;
    p->has_peeked = 0;
    /* 预读第一个 token */
    p->current = lexer_next(lex);
    return p;
}

void parser_free(Parser *p) {
    if (!p) return;
    if (p->has_peeked) {
        token_free(&p->peeked);
    }
    token_free(&p->current);
    free(p);
}

const ParseError *parser_last_error(const Parser *p) {
    if (p->has_error) return &p->last_error;
    return NULL;
}

/* ============ Token 流工具 ============ */

/* 消耗当前 token（advance） */
static void advance(Parser *p) {
    if (p->has_peeked) {
        token_free(&p->current);
        p->current = p->peeked;
        p->has_peeked = 0;
    } else {
        token_free(&p->current);
        p->current = lexer_next(p->lex);
    }
}

/* 查看下一个 token（不消耗） */
static Token peek(Parser *p) {
    if (p->has_peeked) return p->peeked;
    p->peeked = lexer_next(p->lex);
    p->has_peeked = 1;
    return p->peeked;
}

/* 检查当前 token 类型 */
static int check(Parser *p, TokenKind kind) {
    return p->current.kind == kind;
}

/* 匹配并消耗（如果匹配） */
static int match(Parser *p, TokenKind kind) {
    if (p->current.kind == kind) {
        advance(p);
        return 1;
    }
    return 0;
}

/* 必须匹配（否则报错） */
static int expect(Parser *p, TokenKind kind, const char *what) {
    if (p->current.kind == kind) {
        advance(p);
        return 1;
    }
    /* 错误恢复：进入 panic mode，跳到下一个同步点 */
    if (!p->panic_mode) {
        p->has_error = 1;
        p->panic_mode = 1;
        snprintf(p->last_error.message, sizeof(p->last_error.message),
                 "期望 %s，得到 %s at %d:%d",
                 what,
                 token_kind_name(p->current.kind),
                 p->current.line, p->current.col);
        p->last_error.line = p->current.line;
        p->last_error.col = p->current.col;
        p->last_error.error_code = 1;
    }
    return 0;
}

/* 同步：跳到下一个语句边界 */
static void synchronize(Parser *p) {
    p->panic_mode = 0;

    while (p->current.kind != TOK_EOF) {
        switch (p->current.kind) {
            case TOK_KW_FUNC:
            case TOK_KW_STRUCT:
            case TOK_KW_ENUM:
            case TOK_KW_VAR:
            case TOK_KW_LET:
            case TOK_KW_CONST:
            case TOK_KW_IF:
            case TOK_KW_WHILE:
            case TOK_KW_FOR:
            case TOK_KW_RETURN:
                return;
            default:
                advance(p);
        }
    }
}

/* ============ 字面量 ============ */

static AstNode *parse_int_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    /* 解析数字（包含后缀 i32 / u64 / f32 等） */
    char buf[64];
    size_t len = 0;
    /* 简单复制：复制原始 lexeme 直到非数字/非后缀字符 */
    const char *s = p->current.lexeme;
    while (s[len] && len < sizeof(buf) - 1) {
        char c = s[len];
        if ((c >= '0' && c <= '9') || c == 'i' || c == 'u' || c == 'f' ||
            c == 's' || c == 'I' || c == 'U' || c == 'F' || c == 'S') {
            len++;
        } else {
            break;
        }
    }
    /* 简化：只解析纯整数字面量（不做后缀类型映射） */
    long long val = atoll(s);
    advance(p);
    return ast_new_int_lit((int64_t)val, line, col);
}

static AstNode *parse_float_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    double val = atof(p->current.lexeme);
    advance(p);
    return ast_new_float_lit(val, line, col);
}

static AstNode *parse_string_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    /* 字符串字面量已经在词法层处理过插值片段 */
    const char *raw = p->current.lexeme;
    /* raw 形如 "字符串内容"，需要去掉两端的引号 */
    size_t raw_len = p->current.lexeme_len;
    /* 找引号范围 */
    if (raw_len >= 2 && raw[0] == '"' && raw[raw_len - 1] == '"') {
        /* 内部内容：从 [1, raw_len-1) */
        /* v0.1 简化：当作单一 INTERP_TEXT 处理（插值识别留给 v0.5） */
        size_t inner_len = raw_len - 2;
        char *inner = (char *)malloc(inner_len + 1);
        if (!inner) {
            advance(p);
            return NULL;
        }
        memcpy(inner, raw + 1, inner_len);
        inner[inner_len] = '\0';
        /* 处理转义序列（简化：只处理 \\ \" \n \t） */
        char *p_out = inner;
        char *p_in = inner;
        while (*p_in) {
            if (*p_in == '\\' && *(p_in + 1)) {
                p_in++;
                switch (*p_in) {
                    case 'n': *p_out++ = '\n'; break;
                    case 't': *p_out++ = '\t'; break;
                    case 'r': *p_out++ = '\r'; break;
                    case '\\': *p_out++ = '\\'; break;
                    case '"': *p_out++ = '"'; break;
                    default: *p_out++ = *p_in; break;
                }
                p_in++;
            } else {
                *p_out++ = *p_in++;
            }
        }
        *p_out = '\0';

        AstNode *n = ast_new_string_lit(inner, line, col);
        free(inner);
        advance(p);
        return n;
    }
    advance(p);
    return ast_new_string_lit("", line, col);
}

static AstNode *parse_bool_lit(Parser *p, bool val) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);
    return ast_new_bool_lit(val, line, col);
}

static AstNode *parse_char_lit(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    /* 简化：取 lexeme 第一个字节（v0.1 不完整支持转义） */
    int32_t val = 0;
    if (p->current.lexeme_len >= 3) {
        /* 跳过 ' 符号 */
        val = (unsigned char)p->current.lexeme[1];
    }
    advance(p);
    return ast_new_char_lit(val, line, col);
}

static AstNode *parse_ident(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    AstNode *n = ast_new_ident(p->current.lexeme, line, col);
    advance(p);
    return n;
}

static AstNode *parse_literal(Parser *p) {
    switch (p->current.kind) {
        case TOK_INT_LIT:      return parse_int_lit(p);
        case TOK_FLOAT_LIT:    return parse_float_lit(p);
        case TOK_STRING_LIT:   return parse_string_lit(p);
        case TOK_CHAR_LIT:     return parse_char_lit(p);
        case TOK_KW_TRUE:      return parse_bool_lit(p, true);
        case TOK_KW_FALSE:     return parse_bool_lit(p, false);
        case TOK_IDENT:        return parse_ident(p);
        default: return NULL;
    }
}

/* ============ 参数列表 ============ */

static AstNode *parse_param_list(Parser *p);

/* ============ 前向声明 ============ */

static char *parse_type(Parser *p);
static AstNode *parse_expr(Parser *p);
static AstNode *parse_expr_bp(Parser *p, int min_bp);
static AstNode *parse_postfix(Parser *p);
static AstNode *parse_unary(Parser *p);
static AstNode *parse_literal(Parser *p);
static AstNode *parse_block_real(Parser *p);
static AstNode *parse_stmt(Parser *p);
static Token peek(Parser *p);

/* ============ 表达式：Pratt 解析器 ============ */

/* 运算符优先级（binding power）
 *
 * 数字越大优先级越高。同一行的左结合 = (lbp, rbp = lbp+1)
 * 右结合 = (lbp = rbp)
 *
 * 优先级表（来自附录 B）：
 *   1:  =（赋值，右结合）         lbp=0,  rbp=1
 *   2:  ||                       lbp=1,  rbp=2
 *   3:  &&                       lbp=2,  rbp=3
 *   4:  == != < <= > >=         lbp=3,  rbp=4
 *   5:  |                        lbp=4,  rbp=5
 *   6:  ^                        lbp=5,  rbp=6
 *   7:  &                        lbp=6,  rbp=7
 *   8:  << >>                    lbp=7,  rbp=8
 *   9:  + -                      lbp=8,  rbp=9
 *   10: * / %                    lbp=9,  rbp=10
 *   12: ?. ??                   lbp=12, rbp=13
 *   14: 一元 - ! ~               （前缀）
 */

typedef struct {
    TokenKind tok;
    BinOp binop;     /* OP_* 或 0 表示无 */
    UnOp unop;       /* 0 或 UOP_* */
    int lbp;
    int rbp;
    int is_assign;   /* 赋值是右结合 */
} OpInfo;

static int op_info(TokenKind tok, OpInfo *out) {
    out->is_assign = 0;
    switch (tok) {
        /* 赋值：右结合 */
        case TOK_EQ:
            out->binop = OP_ASSIGN; out->unop = 0; out->lbp = 0; out->rbp = 1; out->is_assign = 1; return 1;
        /* 逻辑或 */
        case TOK_OR_OR:
            out->binop = OP_OR; out->unop = 0; out->lbp = 1; out->rbp = 2; return 1;
        /* 逻辑与 */
        case TOK_AND_AND:
            out->binop = OP_AND; out->unop = 0; out->lbp = 2; out->rbp = 3; return 1;
        /* 比较 */
        case TOK_EQEQ: out->binop = OP_EQ;  out->unop = 0; out->lbp = 3; out->rbp = 4; return 1;
        case TOK_NEQ:  out->binop = OP_NEQ; out->unop = 0; out->lbp = 3; out->rbp = 4; return 1;
        case TOK_LT:   out->binop = OP_LT;  out->unop = 0; out->lbp = 3; out->rbp = 4; return 1;
        case TOK_LE:   out->binop = OP_LE;  out->unop = 0; out->lbp = 3; out->rbp = 4; return 1;
        case TOK_GT:   out->binop = OP_GT;  out->unop = 0; out->lbp = 3; out->rbp = 4; return 1;
        case TOK_GE:   out->binop = OP_GE;  out->unop = 0; out->lbp = 3; out->rbp = 4; return 1;
        /* 位或 */
        case TOK_PIPE:
            out->binop = OP_BIT_OR; out->unop = 0; out->lbp = 4; out->rbp = 5; return 1;
        /* 位异或 */
        case TOK_CARET:
            out->binop = OP_BIT_XOR; out->unop = 0; out->lbp = 5; out->rbp = 6; return 1;
        /* 位与 */
        case TOK_AMP:
            out->binop = OP_BIT_AND; out->unop = 0; out->lbp = 6; out->rbp = 7; return 1;
        /* 移位 */
        case TOK_SHL: out->binop = OP_SHL; out->unop = 0; out->lbp = 7; out->rbp = 8; return 1;
        case TOK_SHR: out->binop = OP_SHR; out->unop = 0; out->lbp = 7; out->rbp = 8; return 1;
        /* 算术 */
        case TOK_PLUS:  out->binop = OP_ADD; out->unop = 0; out->lbp = 8; out->rbp = 9; return 1;
        case TOK_MINUS: out->binop = OP_SUB; out->unop = 0; out->lbp = 8; out->rbp = 9; return 1;
        /* 乘除模 */
        case TOK_STAR:    out->binop = OP_MUL; out->unop = 0; out->lbp = 9; out->rbp = 10; return 1;
        case TOK_SLASH:   out->binop = OP_DIV; out->unop = 0; out->lbp = 9; out->rbp = 10; return 1;
        case TOK_PERCENT: out->binop = OP_MOD; out->unop = 0; out->lbp = 9; out->rbp = 10; return 1;
        /* 可选链 */
        case TOK_QUESTION_DOT:
            /* TODO: 暂用普通成员访问 */
            out->binop = 0; out->unop = 0; out->lbp = 12; out->rbp = 13; return 1;
        /* 空值合并 */
        case TOK_QUESTION_QUESTION:
            out->binop = OP_CONCAT; out->unop = 0; out->lbp = 12; out->rbp = 13; return 1;
        default:
            return 0;
    }
}

/* 一元运算符 */
static int unop_info(TokenKind tok, UnOp *out) {
    switch (tok) {
        case TOK_MINUS:    *out = UOP_NEG; return 1;
        case TOK_BANG:     *out = UOP_NOT; return 1;
        case TOK_CARET:    *out = UOP_BIT_NOT; return 1;  /* ~ */
        default: return 0;
    }
}

/* 解析前缀一元运算（递归用） */
static AstNode *parse_unary(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    UnOp op;
    if (unop_info(p->current.kind, &op)) {
        TokenKind tk = p->current.kind;
        advance(p);
        AstNode *operand = parse_unary(p);  /* 允许嵌套 */
        return ast_new_unary(op, operand, line, col);
    }
    /* 否则解析基本字面量 */
    return parse_postfix(p);
}

/* 解析后缀：函数调用、字段访问、索引、切片 */
static AstNode *parse_postfix(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    AstNode *expr = NULL;

    /* 基本字面量 */
    switch (p->current.kind) {
        case TOK_INT_LIT:    expr = parse_int_lit(p); break;
        case TOK_FLOAT_LIT:  expr = parse_float_lit(p); break;
        case TOK_STRING_LIT: expr = parse_string_lit(p); break;
        case TOK_CHAR_LIT:   expr = parse_char_lit(p); break;
        case TOK_KW_TRUE:    expr = parse_bool_lit(p, true); break;
        case TOK_KW_FALSE:   expr = parse_bool_lit(p, false); break;
        case TOK_IDENT:      expr = parse_ident(p); break;
        case TOK_LPAREN: {
            /* 括号表达式：(expr) */
            advance(p);
            expr = parse_expr(p);
            expect(p, TOK_RPAREN, "')'");
            break;
        }
        default: {
            /* 错误：期望表达式 */
            if (!p->panic_mode) {
                p->has_error = 1;
                p->panic_mode = 1;
                snprintf(p->last_error.message, sizeof(p->last_error.message),
                         "期望表达式，得到 %s at %d:%d",
                         token_kind_name(p->current.kind),
                         p->current.line, p->current.col);
                p->last_error.line = p->current.line;
                p->last_error.col = p->current.col;
            }
            return NULL;
        }
    }

    /* 后缀循环：调用、字段、索引、切片 */
    while (expr) {
        if (check(p, TOK_LPAREN)) {
            /* 函数调用 */
            int cline = p->current.line, ccol = p->current.col;
            advance(p);  /* ( */
            AstNode **args = NULL;
            size_t n_args = 0;
            if (!check(p, TOK_RPAREN)) {
                AstNode *tmp_args[64];
                size_t tmp_n = 0;
                do {
                    AstNode *arg = parse_expr(p);
                    if (!arg) break;
                    tmp_args[tmp_n++] = arg;
                } while (match(p, TOK_COMMA) && tmp_n < 64);
                args = tmp_args;
                n_args = tmp_n;
            }
            expect(p, TOK_RPAREN, "')'");
            expr = ast_new_call(expr, args, n_args, cline, ccol);
        } else if (check(p, TOK_DOT)) {
            /* 字段访问 */
            int cline = p->current.line, ccol = p->current.col;
            advance(p);  /* . */
            if (!check(p, TOK_IDENT)) {
                p->has_error = 1;
                break;
            }
            char *field = strdup(p->current.lexeme);
            advance(p);
            expr = ast_new_field_access(expr, field, cline, ccol);
        } else if (check(p, TOK_LBRACKET)) {
            /* 索引或切片 */
            int cline = p->current.line, ccol = p->current.col;
            advance(p);  /* [ */
            AstNode *start = NULL, *end = NULL;
            if (!check(p, TOK_COLON)) {
                start = parse_expr(p);
            }
            bool inclusive = false;
            if (match(p, TOK_COLON)) {
                /* 切片 [start:end] 或 [start:..=end] */
                if (!check(p, TOK_RBRACKET) && !check(p, TOK_DOT_DOT_EQ)) {
                    end = parse_expr(p);
                }
                if (match(p, TOK_DOT_DOT_EQ)) {
                    /* [start:..=end] 语法需要在 end 之前；
                     * 当前实现：end 已经解析，DOT_DOT_EQ 仅在 end 缺失时 */
                    inclusive = true;
                    if (!check(p, TOK_RBRACKET)) {
                        end = parse_expr(p);
                    }
                }
            } else if (match(p, TOK_DOT_DOT)) {
                /* [start..end] 切片 */
                inclusive = false;
                if (!check(p, TOK_RBRACKET)) {
                    end = parse_expr(p);
                }
            } else {
                /* 索引 [start] */
                end = NULL;
            }
            expect(p, TOK_RBRACKET, "']'");
            if (end == NULL && start != NULL) {
                /* 纯索引 */
                expr = ast_new_index(expr, start, cline, ccol);
            } else {
                expr = ast_new_slice(expr, start, end, inclusive, cline, ccol);
            }
        } else {
            break;
        }
    }

    return expr;
}

/* Pratt 解析器主入口 */
static AstNode *parse_expr_bp(Parser *p, int min_bp) {
    AstNode *lhs = parse_unary(p);
    if (!lhs) return NULL;

    while (1) {
        OpInfo op;
        if (!op_info(p->current.kind, &op)) break;
        if (op.lbp < min_bp) break;

        int line = p->current.line, col = p->current.col;
        TokenKind tk = p->current.kind;
        advance(p);

        if (op.is_assign) {
            /* 赋值是右结合：min_bp = op.rbp（不 +1） */
            AstNode *rhs = parse_expr_bp(p, op.rbp);
            lhs = ast_new_assign(lhs, rhs, line, col);
        } else {
            AstNode *rhs = parse_expr_bp(p, op.rbp);
            lhs = ast_new_binary(op.binop, lhs, rhs, line, col);
        }
    }

    return lhs;
}

static AstNode *parse_expr(Parser *p) {
    return parse_expr_bp(p, 0);
}

/* ============ 语句 ============ */

/* 解析类型注解（可选） */
static char *parse_optional_type(Parser *p) {
    if (check(p, TOK_COLON)) {
        advance(p);
        return parse_type(p);
    }
    return NULL;
}

/* 解析表达式语句 */
static AstNode *parse_expr_stmt(Parser *p) {
    int line = p->current.line, col = p->current.col;
    AstNode *expr = parse_expr(p);
    AstNode *stmt = ast_new_expr_stmt(expr, line, col);
    /* 可选分号 */
    match(p, TOK_SEMI);
    return stmt;
}

/* 解析变量声明 */
static AstNode *parse_var_decl(Parser *p, bool is_mut) {
    int line = p->current.line, col = p->current.col;
    /* 跳过 let/var/mut */
    advance(p);

    /* 可选 mut 修饰符 */
    if (match(p, TOK_KW_MUT)) {
        is_mut = true;
    }

    if (!check(p, TOK_IDENT)) {
        p->has_error = 1;
        p->panic_mode = 1;
        snprintf(p->last_error.message, sizeof(p->last_error.message),
                 "期望变量名，得到 %s at %d:%d",
                 token_kind_name(p->current.kind),
                 p->current.line, p->current.col);
        return NULL;
    }
    char *name = strdup(p->current.lexeme);
    advance(p);

    char *type = parse_optional_type(p);

    AstNode *init = NULL;
    if (match(p, TOK_EQ)) {
        init = parse_expr(p);
    }

    match(p, TOK_SEMI);
    return ast_new_var_decl(name, type, is_mut, init, line, col);
}

/* 解析 return */
static AstNode *parse_return(Parser *p) {
    int line = p->current.line, col = p->current.col;
    advance(p);  /* return */
    AstNode *value = NULL;
    /* 如果不是语句结尾，则解析表达式 */
    if (!check(p, TOK_SEMI) && !check(p, TOK_RBRACE) && !check(p, TOK_EOF)) {
        value = parse_expr(p);
    }
    match(p, TOK_SEMI);
    return ast_new_return(value, line, col);
}

/* 解析 break / continue */
static AstNode *parse_break(Parser *p) {
    int line = p->current.line, col = p->current.col;
    advance(p);
    match(p, TOK_SEMI);
    return ast_new_break(line, col);
}

static AstNode *parse_continue(Parser *p) {
    int line = p->current.line, col = p->current.col;
    advance(p);
    match(p, TOK_SEMI);
    return ast_new_continue(line, col);
}

/* 前置声明：parse_block_real 之前先有占位 parse_block */
static AstNode *parse_block_dummy(Parser *p) {
    /* 由 parse_block_real 替换 */
    return NULL;
}

/* 真正的 block 解析（在 parse_stmt / parse_if / parse_while / parse_for 中使用） */
static AstNode *parse_block_real(Parser *p) {
    int line = p->current.line, col = p->current.col;
    if (!expect(p, TOK_LBRACE, "'{'")) {
        return NULL;
    }

    AstNode *stmts[256];
    size_t n_stmts = 0;

    while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF) && n_stmts < 256) {
        if (p->panic_mode) {
            synchronize(p);
        }
        AstNode *stmt = parse_stmt(p);
        if (stmt) {
            stmts[n_stmts++] = stmt;
        }
    }

    expect(p, TOK_RBRACE, "'}'");
    return ast_new_block(stmts, n_stmts, line, col);
}

/* 解析 if */
static AstNode *parse_if(Parser *p) {
    int line = p->current.line, col = p->current.col;
    advance(p);  /* if */
    /* 兼容两种风格：if cond { } 或 if (cond) { } */
    AstNode *cond;
    if (match(p, TOK_LPAREN)) {
        cond = parse_expr(p);
        expect(p, TOK_RPAREN, "')'");
    } else {
        cond = parse_expr(p);
    }

    AstNode *then_block = parse_block_real(p);

    AstNode *else_block = NULL;
    if (match(p, TOK_KW_ELSE)) {
        if (check(p, TOK_KW_IF)) {
            /* else if 递归 */
            else_block = parse_if(p);
        } else {
            else_block = parse_block_real(p);
        }
    }

    return ast_new_if(cond, then_block, else_block, line, col);
}

/* 解析 while */
static AstNode *parse_while(Parser *p) {
    int line = p->current.line, col = p->current.col;
    advance(p);  /* while */
    /* 兼容两种风格：while cond { } 或 while (cond) { } */
    AstNode *cond;
    if (match(p, TOK_LPAREN)) {
        cond = parse_expr(p);
        expect(p, TOK_RPAREN, "')'");
    } else {
        cond = parse_expr(p);
    }
    AstNode *body = parse_block_real(p);
    return ast_new_while(cond, body, line, col);
}

/* 解析 for */
static AstNode *parse_for(Parser *p) {
    int line = p->current.line, col = p->current.col;
    advance(p);  /* for */

    if (!check(p, TOK_IDENT)) {
        p->has_error = 1;
        return NULL;
    }
    char *var = strdup(p->current.lexeme);
    advance(p);

    AstNode *body = NULL;

    if (match(p, TOK_KW_IN)) {
        /* for x in expr { } */
        AstNode *iter = parse_expr(p);
        body = parse_block_real(p);
        return ast_new_for_in(var, iter, body, line, col);
    } else if (match(p, TOK_KW_FROM)) {
        /* for x from a to b [step s] { } */
        AstNode *start = parse_expr(p);
        if (!expect(p, TOK_KW_TO, "'to' 或 'downto'")) {
            free(var);
            return NULL;
        }
        AstNode *end = parse_expr(p);
        AstNode *step = NULL;
        if (match(p, TOK_KW_STEP)) {
            step = parse_expr(p);
        }
        body = parse_block_real(p);
        return ast_new_for_range(var, start, end, step, body, line, col);
    } else {
        p->has_error = 1;
        snprintf(p->last_error.message, sizeof(p->last_error.message),
                 "期望 'in' 或 'from' at %d:%d",
                 p->current.line, p->current.col);
        free(var);
        return NULL;
    }
}

/* 解析单条语句 */
static AstNode *parse_stmt(Parser *p) {
    int line = p->current.line, col = p->current.col;

    switch (p->current.kind) {
        case TOK_KW_LET:   return parse_var_decl(p, false);
        case TOK_KW_VAR:   return parse_var_decl(p, true);
        case TOK_KW_CONST: return parse_var_decl(p, false);
        case TOK_KW_RETURN: return parse_return(p);
        case TOK_KW_BREAK: return parse_break(p);
        case TOK_KW_CONTINUE: return parse_continue(p);
        case TOK_KW_IF:    return parse_if(p);
        case TOK_KW_WHILE: return parse_while(p);
        case TOK_KW_FOR:   return parse_for(p);
        default:
            return parse_expr_stmt(p);
    }
}

/* ============ 类型 ============ */

/* 简单类型：IDENT (可能含泛型) 或 T[RANGE] 数组类型 */
static char *parse_type(Parser *p) {
    /* 数组类型前缀 T[...] */
    /* v0.1 简化：只解析基本标识符类型 */
    if (check(p, TOK_IDENT)) {
        char *name = strdup(p->current.lexeme);
        advance(p);
        /* TODO: 泛型、数组、指针 */
        return name;
    }
    return NULL;
}

/* ============ 块 ============ */

/* 已迁移到 parse_block_real（在表达式模块之前声明）。*/

/* ============ 顶层项 ============ */

/* 解析函数声明 */
static AstNode *parse_function(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);  /* 跳过 func */

    /* 函数名 */
    if (!check(p, TOK_IDENT)) {
        /* 错误恢复 */
        return NULL;
    }
    char *name = strdup(p->current.lexeme);
    advance(p);

    /* 参数列表 */
    AstParam *params = NULL;
    size_t n_params = 0;
    if (check(p, TOK_LPAREN)) {
        advance(p);
        /* 参数列表：暂存到栈 */
        AstParam tmp_params[64];
        size_t tmp_count = 0;
        if (!check(p, TOK_RPAREN)) {
            do {
                if (!check(p, TOK_IDENT)) break;
                tmp_params[tmp_count].name = strdup(p->current.lexeme);
                advance(p);
                tmp_params[tmp_count].type_name = NULL;
                if (match(p, TOK_COLON)) {
                    tmp_params[tmp_count].type_name = parse_type(p);
                }
                tmp_count++;
                if (tmp_count >= 64) break;
            } while (match(p, TOK_COMMA));
        }
        expect(p, TOK_RPAREN, "')'");

        /* 复制到堆 */
        if (tmp_count > 0) {
            params = (AstParam *)malloc(sizeof(AstParam) * tmp_count);
            for (size_t i = 0; i < tmp_count; i++) {
                params[i] = tmp_params[i];
            }
            n_params = tmp_count;
        }
    }

    /* 返回类型 */
    char *ret_type = NULL;
    if (match(p, TOK_ARROW)) {
        ret_type = parse_type(p);
    }

    /* 函数体 */
    AstNode *body = parse_block_real(p);

    return ast_new_function(name, params, n_params, ret_type, body, line, col);
}

/* 解析结构体声明 */
static AstNode *parse_struct(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);  /* 跳过 struct */

    if (!check(p, TOK_IDENT)) return NULL;
    char *name = strdup(p->current.lexeme);
    advance(p);

    AstFieldDecl *fields = NULL;
    size_t n_fields = 0;
    if (expect(p, TOK_LBRACE, "'{'")) {
        AstFieldDecl tmp[64];
        size_t cnt = 0;
        while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF) && cnt < 64) {
            if (!check(p, TOK_IDENT)) { advance(p); continue; }
            tmp[cnt].name = strdup(p->current.lexeme);
            advance(p);
            if (!expect(p, TOK_COLON, "':'")) { break; }
            tmp[cnt].type_name = parse_type(p);
            if (tmp[cnt].type_name == NULL) tmp[cnt].type_name = strdup("?");
            cnt++;
            /* 可选逗号 */
            match(p, TOK_COMMA);
        }
        expect(p, TOK_RBRACE, "'}'");

        if (cnt > 0) {
            fields = (AstFieldDecl *)malloc(sizeof(AstFieldDecl) * cnt);
            for (size_t i = 0; i < cnt; i++) fields[i] = tmp[i];
            n_fields = cnt;
        }
    }

    return ast_new_struct(name, fields, n_fields, line, col);
}

/* 解析枚举声明 */
static AstNode *parse_enum(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;
    advance(p);  /* 跳过 enum */

    if (!check(p, TOK_IDENT)) return NULL;
    char *name = strdup(p->current.lexeme);
    advance(p);

    AstVariantDecl *variants = NULL;
    size_t n_variants = 0;
    if (expect(p, TOK_LBRACE, "'{'")) {
        AstVariantDecl tmp[64];
        size_t cnt = 0;
        while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF) && cnt < 64) {
            if (!check(p, TOK_IDENT)) { advance(p); continue; }
            tmp[cnt].name = strdup(p->current.lexeme);
            tmp[cnt].payload = NULL;
            advance(p);
            /* 可选 payload */
            if (match(p, TOK_LPAREN)) {
                tmp[cnt].payload = strdup("(...)");
                while (!check(p, TOK_RPAREN) && !check(p, TOK_EOF)) advance(p);
                expect(p, TOK_RPAREN, "')'");
            } else if (match(p, TOK_LBRACE)) {
                tmp[cnt].payload = strdup("{...}");
                while (!check(p, TOK_RBRACE) && !check(p, TOK_EOF)) advance(p);
                expect(p, TOK_RBRACE, "'}'");
            }
            cnt++;
            match(p, TOK_COMMA);
        }
        expect(p, TOK_RBRACE, "'}'");

        if (cnt > 0) {
            variants = (AstVariantDecl *)malloc(sizeof(AstVariantDecl) * cnt);
            for (size_t i = 0; i < cnt; i++) variants[i] = tmp[i];
            n_variants = cnt;
        }
    }

    return ast_new_enum(name, variants, n_variants, line, col);
}

/* 调度：解析顶层项 */
static AstNode *parse_item(Parser *p) {
    switch (p->current.kind) {
        case TOK_KW_FUNC:    return parse_function(p);
        case TOK_KW_STRUCT:  return parse_struct(p);
        case TOK_KW_ENUM:    return parse_enum(p);
        default:
            /* 跳过到下一个同步点 */
            if (!p->panic_mode) {
                p->has_error = 1;
                p->panic_mode = 1;
                snprintf(p->last_error.message, sizeof(p->last_error.message),
                         "期望顶层项（func/struct/enum），得到 %s at %d:%d",
                         token_kind_name(p->current.kind),
                         p->current.line, p->current.col);
                p->last_error.line = p->current.line;
                p->last_error.col = p->current.col;
            }
            synchronize(p);
            return NULL;
    }
}

/* 顶层入口 */
AstNode *parser_parse_program(Parser *p) {
    int line = p->current.line;
    int col = p->current.col;

    AstNode *items[256];
    size_t n_items = 0;

    while (!check(p, TOK_EOF) && n_items < 256) {
        if (p->panic_mode) {
            synchronize(p);
        }
        AstNode *item = parse_item(p);
        if (item) {
            items[n_items++] = item;
        }
    }

    return ast_new_program(items, n_items, line, col);
}