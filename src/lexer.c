/*
 * lexer.c — 词法分析器实现
 *
 * 妖语言 v0.1 POC 引导版
 */

#include "qaw/lexer.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <ctype.h>
#include <stdarg.h>

/* ============ 关键字四形表 ============
 * 每个关键字：英文原形、英文缩写、汉语全拼、拼音首字母
 * 任意一种写法都被归一化为英文原形
 */
typedef struct {
    const char *canonical;   /* 英文原形 */
    TokenKind kind;
} KeywordEntry;

static const KeywordEntry KEYWORDS[] = {
    {"package",  TOK_KW_PACKAGE},
    {"import",   TOK_KW_IMPORT},
    {"func",     TOK_KW_FUNC},
    {"return",   TOK_KW_RETURN},
    {"var",      TOK_KW_VAR},
    {"let",      TOK_KW_LET},
    {"mut",      TOK_KW_MUT},
    {"const",    TOK_KW_CONST},
    {"type",     TOK_KW_TYPE},
    {"struct",   TOK_KW_STRUCT},
    {"enum",     TOK_KW_ENUM},
    {"if",       TOK_KW_IF},
    {"else",     TOK_KW_ELSE},
    {"for",      TOK_KW_FOR},
    {"while",    TOK_KW_WHILE},
    {"loop",     TOK_KW_LOOP},
    {"break",    TOK_KW_BREAK},
    {"continue", TOK_KW_CONTINUE},
    {"match",    TOK_KW_MATCH},
    {"true",     TOK_KW_TRUE},
    {"false",    TOK_KW_FALSE},
    {"defer",    TOK_KW_DEFER},
    {"go",       TOK_KW_GO},
    {"chan",     TOK_KW_CHAN},
    {"panic",    TOK_KW_PANIC},
    {"unsafe",   TOK_KW_UNSAFE},
    {"as",       TOK_KW_AS},
    {"in",       TOK_KW_IN},
    {"from",     TOK_KW_FROM},
    {"to",       TOK_KW_TO},
    {"downto",   TOK_KW_DOWNTO},
    {"step",     TOK_KW_STEP},
    {NULL, 0}
};

/* 关键字别名表（每项 5 个指针：canonical, abbrev, pinyin_full, pinyin_init, NULL） */
static const char *KEYWORD_ALIASES[][5] = {
    /* canonical,        abbrev,  pinyin_full,  pinyin_init, NULL */
    {"package",         "pkg",   "baozhuang",  "bz",         NULL},
    {"import",          "imp",   "daoru",      "dr",         NULL},
    {"func",            "fn",    "hanshu",     "hs",         NULL},
    {"return",          "ret",   "fanhui",     "fh",         NULL},
    {"var",             "var",   "bianliang",  "bl",         NULL},
    {"let",             "let",   "buke",       "bk",         NULL},
    {"mut",             "mut",   "kebian",     "kb",         NULL},
    {"const",           "con",   "changliang", "cl",         NULL},
    {"type",            "typ",   "leixing",    "lx",         NULL},
    {"struct",          "struct","jiegou",     "jg",         NULL},
    {"enum",            "enum",  "meiju",      "mj",         NULL},
    {"if",              "if",    "ruguo",      "rg",         NULL},
    {"else",            "els",   "fouze",      "fz",         NULL},
    {"for",             "for",   "xunhuan",    "xh",         NULL},
    {"while",           "whl",   "dang",       "dg",         NULL},
    {"break",           "brk",   "tiaochu",    "tc",         NULL},
    {"continue",        "cnt",   "jixu",       "jx",         NULL},
    {"match",           "match", "pipei",      "pp",         NULL},
    {"true",            "true",  "zhen",       "zh",         NULL},
    {"false",           "fls",   "jia",        "ji",         NULL},
    {"defer",           "def",   "yanchi",     "yc",         NULL},
    {"go",              "go",    "xiecheng",   "xc",         NULL},
    {"chan",            "chan",  "tongdao",    "td",         NULL},
    {"panic",           "panic", "huangkong",  "hk",         NULL},
    {"unsafe",          "unsafe","weixian",    "wx",         NULL},
    {"as",              "as",    "zuowei",     "zw",         NULL},
    {"in",              "in",    "zai",        "zai",        NULL},
    {"from",            "from",  "cong",       "cong",       NULL},
    {"to",              "to",    "dao",        "dao",        NULL},
    {"downto",          "downto","daoxia",     "dx",         NULL},
    {"step",            "step",  "buchang",    "bc",         NULL},
    {NULL, NULL, NULL, NULL, NULL}
};

/* ============ Token 工具 ============ */

Token token_new(TokenKind kind, const char *lexeme, size_t len, int line, int col) {
    Token t;
    t.kind = kind;
    t.lexeme = (char *)malloc(len + 1);
    if (t.lexeme) {
        memcpy(t.lexeme, lexeme, len);
        t.lexeme[len] = '\0';
    }
    t.lexeme_len = len;
    t.line = line;
    t.col = col;
    return t;
}

void token_free(Token *t) {
    if (t && t->lexeme) {
        free(t->lexeme);
        t->lexeme = NULL;
    }
}

const char *token_kind_name(TokenKind k) {
    switch (k) {
        /* 字面量 */
        case TOK_INT_LIT: return "INT_LIT";
        case TOK_FLOAT_LIT: return "FLOAT_LIT";
        case TOK_STRING_LIT: return "STRING_LIT";
        case TOK_CHAR_LIT: return "CHAR_LIT";
        case TOK_IDENT: return "IDENT";

        /* 关键字 */
        case TOK_KW_PACKAGE: return "kw_package";
        case TOK_KW_IMPORT: return "kw_import";
        case TOK_KW_FUNC: return "kw_func";
        case TOK_KW_RETURN: return "kw_return";
        case TOK_KW_VAR: return "kw_var";
        case TOK_KW_LET: return "kw_let";
        case TOK_KW_MUT: return "kw_mut";
        case TOK_KW_CONST: return "kw_const";
        case TOK_KW_TYPE: return "kw_type";
        case TOK_KW_STRUCT: return "kw_struct";
        case TOK_KW_ENUM: return "kw_enum";
        case TOK_KW_IF: return "kw_if";
        case TOK_KW_ELSE: return "kw_else";
        case TOK_KW_FOR: return "kw_for";
        case TOK_KW_WHILE: return "kw_while";
        case TOK_KW_LOOP: return "kw_loop";
        case TOK_KW_BREAK: return "kw_break";
        case TOK_KW_CONTINUE: return "kw_continue";
        case TOK_KW_MATCH: return "kw_match";
        case TOK_KW_TRUE: return "kw_true";
        case TOK_KW_FALSE: return "kw_false";
        case TOK_KW_DEFER: return "kw_defer";
        case TOK_KW_GO: return "kw_go";
        case TOK_KW_CHAN: return "kw_chan";
        case TOK_KW_PANIC: return "kw_panic";
        case TOK_KW_UNSAFE: return "kw_unsafe";
        case TOK_KW_AS: return "kw_as";
        case TOK_KW_IN: return "kw_in";
        case TOK_KW_FROM: return "kw_from";
        case TOK_KW_TO: return "kw_to";
        case TOK_KW_DOWNTO: return "kw_downto";
        case TOK_KW_STEP: return "kw_step";

        /* 运算符 */
        case TOK_PLUS: return "+";
        case TOK_MINUS: return "-";
        case TOK_STAR: return "*";
        case TOK_SLASH: return "/";
        case TOK_PERCENT: return "%";
        case TOK_EQ: return "=";
        case TOK_EQEQ: return "==";
        case TOK_NEQ: return "!=";
        case TOK_LT: return "<";
        case TOK_LE: return "<=";
        case TOK_GT: return ">";
        case TOK_GE: return ">=";
        case TOK_AMP: return "&";
        case TOK_PIPE: return "|";
        case TOK_CARET: return "^";
        case TOK_SHL: return "<<";
        case TOK_SHR: return ">>";
        case TOK_AND_AND: return "&&";
        case TOK_OR_OR: return "||";
        case TOK_BANG: return "!";
        case TOK_ARROW: return "->";
        case TOK_FAT_ARROW: return "=>";
        case TOK_DOT_DOT: return "..";
        case TOK_DOT_DOT_EQ: return "..=";
        case TOK_QUESTION: return "?";
        case TOK_QUESTION_DOT: return "?.";
        case TOK_QUESTION_QUESTION: return "??";
        case TOK_COLON_COLON: return "::";

        /* 分隔符 */
        case TOK_LPAREN: return "(";
        case TOK_RPAREN: return ")";
        case TOK_LBRACE: return "{";
        case TOK_RBRACE: return "}";
        case TOK_LBRACKET: return "[";
        case TOK_RBRACKET: return "]";
        case TOK_LANGLE: return "<";
        case TOK_RANGLE: return ">";
        case TOK_COMMA: return ",";
        case TOK_SEMI: return ";";
        case TOK_COLON: return ":";
        case TOK_DOT: return ".";

        case TOK_EOF: return "EOF";
        case TOK_ERROR: return "ERROR";
        default: return "?";
    }
}

/* ============ 关键字归一化 ============ */

const char *keyword_normalize(const char *s, size_t len) {
    /* 在 KEYWORD_ALIASES 中查找 */
    for (size_t i = 0; KEYWORD_ALIASES[i][0] != NULL; i++) {
        for (int j = 0; j < 5 && KEYWORD_ALIASES[i][j] != NULL; j++) {
            if (strlen(KEYWORD_ALIASES[i][j]) == len &&
                memcmp(KEYWORD_ALIASES[i][j], s, len) == 0) {
                return KEYWORD_ALIASES[i][0];  /* 返回 canonical */
            }
        }
    }
    return NULL;
}

/* ============ Lexer 实现 ============ */

struct Lexer {
    const char *source;
    size_t pos;
    int line;
    int col;
    LexError last_error;
    int has_error;
    Token peeked;       /* 缓存的 peek Token */
    int has_peeked;
};

Lexer *lexer_new(const char *source) {
    Lexer *l = (Lexer *)calloc(1, sizeof(Lexer));
    if (!l) return NULL;
    l->source = source;
    l->pos = 0;
    l->line = 1;
    l->col = 1;
    return l;
}

void lexer_free(Lexer *l) {
    if (!l) return;
    if (l->has_peeked) {
        token_free(&l->peeked);
    }
    free(l);
}

/* 报告错误 */
static void lex_error(Lexer *l, const char *fmt, ...) {
    l->has_error = 1;
    l->last_error.line = l->line;
    l->last_error.col = l->col;
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(l->last_error.message, sizeof(l->last_error.message), fmt, ap);
    va_end(ap);
}

/* 查看当前字符（不消费） */
static int peek_char(Lexer *l) {
    if (l->source[l->pos] == '\0') return -1;
    return (unsigned char)l->source[l->pos];
}

/* 查看下一个字符（不消费） */
static int peek_char_next(Lexer *l) {
    if (l->source[l->pos] == '\0') return -1;
    if (l->source[l->pos + 1] == '\0') return -1;
    return (unsigned char)l->source[l->pos + 1];
}

/* 消费当前字符 */
static int advance_char(Lexer *l) {
    int c = (unsigned char)l->source[l->pos];
    l->pos++;
    if (c == '\n') {
        l->line++;
        l->col = 1;
    } else {
        l->col++;
    }
    return c;
}

/* 跳过空白和注释 */
static void skip_whitespace_and_comments(Lexer *l) {
    while (1) {
        int c = peek_char(l);
        if (c == -1) return;

        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            advance_char(l);
            continue;
        }

        /* 行注释 // ... */
        if (c == '/' && peek_char_next(l) == '/') {
            while (peek_char(l) != '\n' && peek_char(l) != -1) {
                advance_char(l);
            }
            continue;
        }

        /* 块注释 /* ... *​/（不支持嵌套，v0.1 简化） */
        if (c == '/' && peek_char_next(l) == '*') {
            advance_char(l);  /* / */
            advance_char(l);  /* * */
            while (1) {
                int c2 = peek_char(l);
                if (c2 == -1) {
                    lex_error(l, "未闭合的块注释");
                    return;
                }
                if (c2 == '*' && l->source[l->pos + 1] == '/') {
                    advance_char(l);  /* * */
                    advance_char(l);  /* / */
                    break;
                }
                advance_char(l);
            }
            continue;
        }

        return;
    }
}

/* 检查字节是否为标识符字符 */
static int is_ident_byte(unsigned char c) {
    if (isalnum(c) || c == '_') return 1;
    /* UTF-8 后续字节（10xxxxxx）：是标识符的一部分 */
    if ((c & 0xC0) == 0x80) return 1;
    /* UTF-8 起始字节（11xxxxxx，>= 0xC0）：标识符的一部分 */
    if (c >= 0xC0) return 1;
    return 0;
}

/* 检查 UTF-8 字符是否为标识符起始字符 */
static int is_ident_start(unsigned char c) {
    if (isalpha(c) || c == '_') return 1;
    /* UTF-8 多字节字符起始（>= 0xC0）：标识符起始 */
    if (c >= 0xC0) return 1;
    return 0;
}

/* 读取标识符或关键字 */
static Token lex_ident_or_keyword(Lexer *l) {
    int start_line = l->line;
    int start_col = l->col;
    size_t start = l->pos;

    /* 第一个字符 */
    unsigned char first = (unsigned char)l->source[l->pos];
    if (!is_ident_start(first)) {
        advance_char(l);  /* 消耗 */
        char buf[2] = {(char)first, 0};
        return token_new(TOK_ERROR, buf, 1, start_line, start_col);
    }
    advance_char(l);

    /* 后续字符 */
    while (1) {
        if (l->source[l->pos] == '\0') break;
        unsigned char c = (unsigned char)l->source[l->pos];
        if (is_ident_byte(c)) {
            advance_char(l);
        } else {
            break;
        }
    }

    size_t len = l->pos - start;
    const char *s = l->source + start;

    /* 检查是否是关键字（按字节比较，需要 canonical 也是 ASCII） */
    const char *canonical = keyword_normalize(s, len);
    if (canonical) {
        for (size_t i = 0; KEYWORDS[i].canonical != NULL; i++) {
            if (strcmp(KEYWORDS[i].canonical, canonical) == 0) {
                return token_new(KEYWORDS[i].kind, s, len, start_line, start_col);
            }
        }
    }

    return token_new(TOK_IDENT, s, len, start_line, start_col);
}

/* 读取整数或浮点 */
static Token lex_number(Lexer *l) {
    int start_line = l->line;
    int start_col = l->col;
    size_t start = l->pos;

    while (peek_char(l) != -1 && isdigit(peek_char(l))) {
        advance_char(l);
    }

    int is_float = 0;

    /* 小数点（必须是 . 后跟数字才算浮点） */
    if (peek_char(l) == '.' && peek_char_next(l) != -1 &&
        isdigit(peek_char_next(l))) {
        is_float = 1;
        advance_char(l);  /* . */
        while (peek_char(l) != -1 && isdigit(peek_char(l))) {
            advance_char(l);
        }
    }

    /* 指数 e / E */
    if (peek_char(l) == 'e' || peek_char(l) == 'E') {
        is_float = 1;
        advance_char(l);
        if (peek_char(l) == '+' || peek_char(l) == '-') {
            advance_char(l);
        }
        while (peek_char(l) != -1 && isdigit(peek_char(l))) {
            advance_char(l);
        }
    }

    /* 后缀（i32, u64, f32） */
    if (!is_float &&
        (peek_char(l) == 'i' || peek_char(l) == 'u' || peek_char(l) == 'f')) {
        int c = peek_char(l);
        advance_char(l);
        while (peek_char(l) != -1 && isdigit(peek_char(l))) {
            advance_char(l);
        }
    } else if (is_float && (peek_char(l) == 'f')) {
        advance_char(l);
        while (peek_char(l) != -1 && isdigit(peek_char(l))) {
            advance_char(l);
        }
    }

    size_t len = l->pos - start;
    if (is_float) {
        return token_new(TOK_FLOAT_LIT, l->source + start, len, start_line, start_col);
    }
    return token_new(TOK_INT_LIT, l->source + start, len, start_line, start_col);
}

/* 读取字符串字面量
 *
 * v0.1 简化：词法层只识别整体字符串，${} 插值在运行时解析
 */

static Token lex_string(Lexer *l) {
    int start_line = l->line;
    int start_col = l->col;
    size_t start = l->pos;

    advance_char(l);  /* " */

    char *buf = (char *)malloc(256);
    size_t cap = 256, len = 0;
    buf[0] = '\0';

    while (1) {
        int c = peek_char(l);
        if (c == -1) {
            lex_error(l, "未闭合的字符串");
            free(buf);
            return token_new(TOK_ERROR, l->source + start, l->pos - start, start_line, start_col);
        }
        if (c == '"') {
            advance_char(l);
            break;
        }
        if (c == '\\') {
            advance_char(l);
            int next = peek_char(l);
            if (next == -1) {
                lex_error(l, "字符串末尾的反斜杠");
                free(buf);
                return token_new(TOK_ERROR, l->source + start, l->pos - start, start_line, start_col);
            }
            advance_char(l);  /* 跳过转义字符 */
            char ch;
            switch (next) {
                case 'n': ch = '\n'; break;
                case 't': ch = '\t'; break;
                case 'r': ch = '\r'; break;
                case '\\': ch = '\\'; break;
                case '"': ch = '"'; break;
                case '$': ch = '$'; break;
                default: ch = (char)next; break;
            }
            if (len + 1 >= cap) {
                cap *= 2;
                buf = (char *)realloc(buf, cap);
            }
            buf[len++] = ch;
            buf[len] = '\0';
            continue;
        }
        /* 普通字符：原样保留（含 ${} 文本，运行时再切分） */
        if (len + 1 >= cap) {
            cap *= 2;
            buf = (char *)realloc(buf, cap);
        }
        buf[len++] = (char)c;
        buf[len] = '\0';
        advance_char(l);
    }

    size_t total_len = l->pos - start;
    /* 合并 buf 到 lexeme（用更高效的存储：直接 copy 源） */
    free(buf);
    char *lexeme = (char *)malloc(total_len + 1);
    memcpy(lexeme, l->source + start, total_len);
    lexeme[total_len] = '\0';
    Token t = token_new(TOK_STRING_LIT, lexeme, total_len, start_line, start_col);
    free(lexeme);
    return t;
}

/* 主扫描函数 */
Token lexer_next(Lexer *l) {
    if (l->has_peeked) {
        Token t = l->peeked;
        l->has_peeked = 0;
        l->peeked.lexeme = NULL;
        return t;
    }

    skip_whitespace_and_comments(l);

    if (peek_char(l) == -1) {
        return token_new(TOK_EOF, "", 0, l->line, l->col);
    }

    int start_line = l->line;
    int start_col = l->col;
    int c = peek_char(l);

    /* 标识符或关键字（包含 UTF-8 多字节字符起始） */
    if (isalpha(c) || c == '_' || c >= 0x80) {
        return lex_ident_or_keyword(l);
    }

    /* 数字 */
    if (isdigit(c)) {
        return lex_number(l);
    }

    /* 字符串 */
    if (c == '"') {
        return lex_string(l);
    }

    /* 字符字面量 */
    if (c == '\'') {
        advance_char(l);
        int ch = peek_char(l);
        if (ch == -1) {
            lex_error(l, "未闭合的字符字面量");
            return token_new(TOK_ERROR, "", 0, start_line, start_col);
        }
        advance_char(l);
        if (peek_char(l) == '\'') {
            advance_char(l);
        }
        char ch_buf = (char)ch;
        return token_new(TOK_CHAR_LIT, &ch_buf, 1, start_line, start_col);
    }

    /* 多字符运算符 */
    int c2 = peek_char_next(l);

    /* -> => .. .. = */
    if (c == '-' && c2 == '>') {
        advance_char(l); advance_char(l);
        return token_new(TOK_ARROW, "->", 2, start_line, start_col);
    }
    if (c == '=' && c2 == '>') {
        advance_char(l); advance_char(l);
        return token_new(TOK_FAT_ARROW, "=>", 2, start_line, start_col);
    }
    if (c == '.' && c2 == '.') {
        advance_char(l); advance_char(l);
        if (peek_char(l) == '=') {
            advance_char(l);
            return token_new(TOK_DOT_DOT_EQ, "..=", 3, start_line, start_col);
        }
        return token_new(TOK_DOT_DOT, "..", 2, start_line, start_col);
    }

    /* == != <= >= && || ?. ?? */
    if (c == '=' && c2 == '=') { advance_char(l); advance_char(l); return token_new(TOK_EQEQ, "==", 2, start_line, start_col); }
    if (c == '!' && c2 == '=') { advance_char(l); advance_char(l); return token_new(TOK_NEQ, "!=", 2, start_line, start_col); }
    if (c == '<' && c2 == '=') { advance_char(l); advance_char(l); return token_new(TOK_LE, "<=", 2, start_line, start_col); }
    if (c == '>' && c2 == '=') { advance_char(l); advance_char(l); return token_new(TOK_GE, ">=", 2, start_line, start_col); }
    if (c == '&' && c2 == '&') { advance_char(l); advance_char(l); return token_new(TOK_AND_AND, "&&", 2, start_line, start_col); }
    if (c == '|' && c2 == '|') { advance_char(l); advance_char(l); return token_new(TOK_OR_OR, "||", 2, start_line, start_col); }
    if (c == '?' && c2 == '.') { advance_char(l); advance_char(l); return token_new(TOK_QUESTION_DOT, "?.", 2, start_line, start_col); }
    if (c == '?' && c2 == '?') { advance_char(l); advance_char(l); return token_new(TOK_QUESTION_QUESTION, "??", 2, start_line, start_col); }
    if (c == ':' && c2 == ':') { advance_char(l); advance_char(l); return token_new(TOK_COLON_COLON, "::", 2, start_line, start_col); }

    /* 单字符 */
    advance_char(l);
    char buf[2] = {(char)c, 0};
    switch (c) {
        case '(': return token_new(TOK_LPAREN, buf, 1, start_line, start_col);
        case ')': return token_new(TOK_RPAREN, buf, 1, start_line, start_col);
        case '{': return token_new(TOK_LBRACE, buf, 1, start_line, start_col);
        case '}': return token_new(TOK_RBRACE, buf, 1, start_line, start_col);
        case '[': return token_new(TOK_LBRACKET, buf, 1, start_line, start_col);
        case ']': return token_new(TOK_RBRACKET, buf, 1, start_line, start_col);
        case ',': return token_new(TOK_COMMA, buf, 1, start_line, start_col);
        case ';': return token_new(TOK_SEMI, buf, 1, start_line, start_col);
        case ':': return token_new(TOK_COLON, buf, 1, start_line, start_col);
        case '.': return token_new(TOK_DOT, buf, 1, start_line, start_col);
        case '+': return token_new(TOK_PLUS, buf, 1, start_line, start_col);
        case '-': return token_new(TOK_MINUS, buf, 1, start_line, start_col);
        case '*': return token_new(TOK_STAR, buf, 1, start_line, start_col);
        case '/': return token_new(TOK_SLASH, buf, 1, start_line, start_col);
        case '%': return token_new(TOK_PERCENT, buf, 1, start_line, start_col);
        case '=': return token_new(TOK_EQ, buf, 1, start_line, start_col);
        case '<': return token_new(TOK_LT, buf, 1, start_line, start_col);
        case '>': return token_new(TOK_GT, buf, 1, start_line, start_col);
        case '&': return token_new(TOK_AMP, buf, 1, start_line, start_col);
        case '|': return token_new(TOK_PIPE, buf, 1, start_line, start_col);
        case '^': return token_new(TOK_CARET, buf, 1, start_line, start_col);
        case '!': return token_new(TOK_BANG, buf, 1, start_line, start_col);
        case '?': return token_new(TOK_QUESTION, buf, 1, start_line, start_col);
        default:
            lex_error(l, "意外的字符: '%c' (0x%02x)", c, c);
            return token_new(TOK_ERROR, buf, 1, start_line, start_col);
    }
}

Token lexer_peek(Lexer *l) {
    if (l->has_peeked) {
        return l->peeked;
    }
    l->peeked = lexer_next(l);
    l->has_peeked = 1;
    return l->peeked;
}

const LexError *lexer_last_error(const Lexer *l) {
    if (l->has_error) return &l->last_error;
    return NULL;
}
