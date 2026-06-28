/*
 * value.c — 运行时值
 */

#define _POSIX_C_SOURCE 200809L

#include "qaw/value.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <inttypes.h>

/* ============ 构造器 ============ */

Value val_int(int64_t v) {
    Value x;
    x.kind = VAL_INT;
    x.as.int_val = v;
    return x;
}

Value val_float(double v) {
    Value x;
    x.kind = VAL_FLOAT;
    x.as.float_val = v;
    return x;
}

Value val_bool(bool v) {
    Value x;
    x.kind = VAL_BOOL;
    x.as.bool_val = v;
    return x;
}

Value val_string(const char *s) {
    Value x;
    x.kind = VAL_STRING;
    x.as.string_val = s ? strdup(s) : strdup("");
    return x;
}

Value val_string_n(const char *s, size_t n) {
    Value x;
    x.kind = VAL_STRING;
    x.as.string_val = (char *)malloc(n + 1);
    if (x.as.string_val) {
        memcpy(x.as.string_val, s, n);
        x.as.string_val[n] = '\0';
    }
    return x;
}

Value val_nil(void) {
    Value x;
    x.kind = VAL_NIL;
    return x;
}

/* ============ 释放 ============ */

void val_free(Value *v) {
    if (!v) return;
    if (v->kind == VAL_STRING && v->as.string_val) {
        free(v->as.string_val);
        v->as.string_val = NULL;
    }
}

/* ============ 类型检查 ============ */

bool val_is_int(const Value *v) { return v->kind == VAL_INT; }
bool val_is_float(const Value *v) { return v->kind == VAL_FLOAT; }
bool val_is_bool(const Value *v) { return v->kind == VAL_BOOL; }
bool val_is_string(const Value *v) { return v->kind == VAL_STRING; }
bool val_is_nil(const Value *v) { return v->kind == VAL_NIL; }

bool val_is_truthy(const Value *v) {
    switch (v->kind) {
        case VAL_NIL: return false;
        case VAL_BOOL: return v->as.bool_val;
        case VAL_INT: return v->as.int_val != 0;
        case VAL_FLOAT: return v->as.float_val != 0.0;
        case VAL_STRING: return v->as.string_val[0] != '\0';
    }
    return false;
}

/* ============ 转换 ============ */

int64_t val_to_int(const Value *v) {
    switch (v->kind) {
        case VAL_INT: return v->as.int_val;
        case VAL_FLOAT: return (int64_t)v->as.float_val;
        case VAL_BOOL: return v->as.bool_val ? 1 : 0;
        case VAL_STRING: return atoll(v->as.string_val);
        case VAL_NIL: return 0;
    }
    return 0;
}

double val_to_float(const Value *v) {
    switch (v->kind) {
        case VAL_INT: return (double)v->as.int_val;
        case VAL_FLOAT: return v->as.float_val;
        case VAL_BOOL: return v->as.bool_val ? 1.0 : 0.0;
        case VAL_STRING: return atof(v->as.string_val);
        case VAL_NIL: return 0.0;
    }
    return 0.0;
}

bool val_to_bool(const Value *v) {
    return val_is_truthy(v);
}

/* ============ 比较 ============ */

bool val_equal(const Value *a, const Value *b) {
    if (a->kind != b->kind) {
        /* 不同类型：尝试数值比较 */
        if ((a->kind == VAL_INT || a->kind == VAL_FLOAT) &&
            (b->kind == VAL_INT || b->kind == VAL_FLOAT)) {
            return val_to_float(a) == val_to_float(b);
        }
        return false;
    }
    switch (a->kind) {
        case VAL_INT: return a->as.int_val == b->as.int_val;
        case VAL_FLOAT: return a->as.float_val == b->as.float_val;
        case VAL_BOOL: return a->as.bool_val == b->as.bool_val;
        case VAL_STRING: return strcmp(a->as.string_val, b->as.string_val) == 0;
        case VAL_NIL: return true;
    }
    return false;
}

/* ============ 调试 ============ */

const char *val_kind_name(ValueKind k) {
    switch (k) {
        case VAL_INT:    return "int";
        case VAL_FLOAT:  return "float";
        case VAL_BOOL:   return "bool";
        case VAL_STRING: return "string";
        case VAL_NIL:    return "nil";
        default: return "?";
    }
}

void val_print(const Value *v) {
    if (!v) {
        printf("<null>");
        return;
    }
    switch (v->kind) {
        case VAL_INT:    printf("%" PRId64, v->as.int_val); break;
        case VAL_FLOAT:  printf("%g", v->as.float_val); break;
        case VAL_BOOL:   printf("%s", v->as.bool_val ? "true" : "false"); break;
        case VAL_STRING: printf("%s", v->as.string_val); break;
        case VAL_NIL:    printf("nil"); break;
    }
}