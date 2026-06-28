/*
 * test_env.c — Environment 单元测试
 */

#define _POSIX_C_SOURCE 200809L

#include "qaw/env.h"

#include <stdio.h>
#include <string.h>
#include <assert.h>

static int tests_run = 0;
static int tests_passed = 0;

#define TEST(name) \
    do { \
        fprintf(stderr, "  [TEST] %s ... ", #name); \
        tests_run++; \
        if (test_##name()) { \
            fprintf(stderr, "OK\n"); \
            tests_passed++; \
        } else { \
            fprintf(stderr, "FAIL\n"); \
        } \
    } while (0)

#define ASSERT(cond) \
    do { if (!(cond)) { fprintf(stderr, "\n    assert failed at %s:%d: %s\n", __FILE__, __LINE__, #cond); return 0; } } while (0)

static int test_root(void) {
    Env *e = env_new();
    ASSERT(e != NULL);
    ASSERT(env_has(e, "x") == false);
    env_free(e);
    return 1;
}

static int test_define_get(void) {
    Env *e = env_new();
    ASSERT(env_define(e, "x", val_int(42)) == true);

    Value v = env_get(e, "x");
    ASSERT(v.kind == VAL_INT);
    ASSERT(v.as.int_val == 42);

    env_free(e);
    return 1;
}

static int test_set_existing(void) {
    Env *e = env_new();
    env_define(e, "x", val_int(1));
    ASSERT(env_set(e, "x", val_int(2)) == true);

    Value v = env_get(e, "x");
    ASSERT(v.as.int_val == 2);

    env_free(e);
    return 1;
}

static int test_set_undefined(void) {
    Env *e = env_new();
    /* 未定义就 set 应失败 */
    ASSERT(env_set(e, "x", val_int(1)) == false);
    env_free(e);
    return 1;
}

static int test_undefined_returns_nil(void) {
    Env *e = env_new();
    Value v = env_get(e, "x");
    ASSERT(v.kind == VAL_NIL);
    env_free(e);
    return 1;
}

static int test_child_scope(void) {
    Env *root = env_new();
    env_define(root, "x", val_int(1));

    Env *child = env_child(root);
    /* 子作用域可读父作用域 */
    Value v = env_get(child, "x");
    ASSERT(v.as.int_val == 1);

    /* 子作用域定义遮蔽父作用域 */
    env_define(child, "x", val_int(2));
    v = env_get(child, "x");
    ASSERT(v.as.int_val == 2);

    /* 父作用域仍为原值 */
    v = env_get(root, "x");
    ASSERT(v.as.int_val == 1);

    env_free(child);
    env_free(root);
    return 1;
}

static int test_set_in_parent(void) {
    Env *root = env_new();
    env_define(root, "x", val_int(1));

    Env *child = env_child(root);
    /* 子作用域 set 会向上查找并修改父作用域的变量 */
    ASSERT(env_set(child, "x", val_int(2)) == true);

    Value v = env_get(root, "x");
    ASSERT(v.as.int_val == 2);

    env_free(child);
    env_free(root);
    return 1;
}

static int test_multiple_vars(void) {
    Env *e = env_new();
    env_define(e, "x", val_int(1));
    env_define(e, "y", val_int(2));
    env_define(e, "z", val_string("hi"));

    ASSERT(env_get(e, "x").as.int_val == 1);
    ASSERT(env_get(e, "y").as.int_val == 2);
    ASSERT(strcmp(env_get(e, "z").as.string_val, "hi") == 0);

    env_free(e);
    return 1;
}

static int test_shadowing(void) {
    Env *e = env_new();
    env_define(e, "x", val_int(1));
    /* 在同一作用域再次 define 覆盖 */
    env_define(e, "x", val_int(2));
    ASSERT(env_get(e, "x").as.int_val == 2);
    env_free(e);
    return 1;
}

int main(void) {
    fprintf(stderr, "=== yaoc Environment 单元测试 ===\n");

    TEST(root);
    TEST(define_get);
    TEST(set_existing);
    TEST(set_undefined);
    TEST(undefined_returns_nil);
    TEST(child_scope);
    TEST(set_in_parent);
    TEST(multiple_vars);
    TEST(shadowing);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}