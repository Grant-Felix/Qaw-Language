/*
 * value.h — 运行时值类型
 *
 * 妖语言 v0.1 POC 引导版
 */

#ifndef YAO_VALUE_H
#define YAO_VALUE_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

/* 值类型 */
typedef enum {
    VAL_INT,
    VAL_FLOAT,
    VAL_BOOL,
    VAL_STRING,         /* 堆分配 string */
    VAL_NIL,            /* unit / () */
} ValueKind;

/* 值（Tagged Union） */
typedef struct {
    ValueKind kind;
    union {
        int64_t int_val;
        double float_val;
        bool bool_val;
        char *string_val;     /* VAL_STRING */
    } as;
} Value;

/* 构造器 */
Value val_int(int64_t v);
Value val_float(double v);
Value val_bool(bool v);
Value val_string(const char *s);    /* 复制字符串 */
Value val_string_n(const char *s, size_t n);  /* 指定长度 */
Value val_nil(void);

/* 字符串值需释放 */
void val_free(Value *v);  /* 释放字符串内存 */

/* 类型检查 */
bool val_is_int(const Value *v);
bool val_is_float(const Value *v);
bool val_is_bool(const Value *v);
bool val_is_string(const Value *v);
bool val_is_nil(const Value *v);
bool val_is_truthy(const Value *v);  /* 假值：false / nil / 0 */

/* 转换 */
int64_t val_to_int(const Value *v);
double val_to_float(const Value *v);
bool val_to_bool(const Value *v);

/* 比较 */
bool val_equal(const Value *a, const Value *b);

/* 调试 */
const char *val_kind_name(ValueKind k);
void val_print(const Value *v);

#endif /* YAO_VALUE_H */