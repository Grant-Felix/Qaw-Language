/*
 * test_value.c — Value 单元测试
 */

#define _POSIX_C_SOURCE 200809L

#include "yao/value.h"

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

static int test_int(void) {
    Value v = val_int(42);
    ASSERT(v.kind == VAL_INT);
    ASSERT(v.as.int_val == 42);
    val_free(&v);
    return 1;
}

static int test_float(void) {
    Value v = val_float(3.14);
    ASSERT(v.kind == VAL_FLOAT);
    ASSERT(v.as.float_val == 3.14);
    val_free(&v);
    return 1;
}

static int test_bool(void) {
    Value t = val_bool(true);
    Value f = val_bool(false);
    ASSERT(t.as.bool_val == true);
    ASSERT(f.as.bool_val == false);
    val_free(&t);
    val_free(&f);
    return 1;
}

static int test_string(void) {
    Value v = val_string("hello");
    ASSERT(v.kind == VAL_STRING);
    ASSERT(strcmp(v.as.string_val, "hello") == 0);
    val_free(&v);
    return 1;
}

static int test_string_n(void) {
    Value v = val_string_n("hello world", 5);
    ASSERT(v.kind == VAL_STRING);
    ASSERT(strcmp(v.as.string_val, "hello") == 0);
    val_free(&v);
    return 1;
}

static int test_nil(void) {
    Value v = val_nil();
    ASSERT(v.kind == VAL_NIL);
    val_free(&v);
    return 1;
}

static int test_is_truthy(void) {
    Value t = val_bool(true);
    Value f = val_bool(false);
    Value n = val_nil();
    Value i1 = val_int(1);
    Value i0 = val_int(0);
    Value im = val_int(-1);
    Value f0 = val_float(0.0);
    Value s1 = val_string("");
    Value s2 = val_string("x");
    ASSERT(val_is_truthy(&t) == true);
    ASSERT(val_is_truthy(&f) == false);
    ASSERT(val_is_truthy(&n) == false);
    ASSERT(val_is_truthy(&i1) == true);
    ASSERT(val_is_truthy(&i0) == false);
    ASSERT(val_is_truthy(&im) == true);
    ASSERT(val_is_truthy(&f0) == false);
    ASSERT(val_is_truthy(&s1) == false);
    ASSERT(val_is_truthy(&s2) == true);
    val_free(&t); val_free(&f); val_free(&n);
    val_free(&i1); val_free(&i0); val_free(&im);
    val_free(&f0); val_free(&s1); val_free(&s2);
    return 1;
}

static int test_to_int(void) {
    Value i = val_int(42);
    Value f = val_float(3.7);
    Value b = val_bool(true);
    Value n = val_nil();
    ASSERT(val_to_int(&i) == 42);
    ASSERT(val_to_int(&f) == 3);
    ASSERT(val_to_int(&b) == 1);
    ASSERT(val_to_int(&n) == 0);
    val_free(&i); val_free(&f); val_free(&b); val_free(&n);
    return 1;
}

static int test_to_float(void) {
    Value f = val_float(3.14);
    Value i = val_int(5);
    ASSERT(val_to_float(&f) == 3.14);
    ASSERT(val_to_float(&i) == 5.0);
    val_free(&f); val_free(&i);
    return 1;
}

static int test_equal(void) {
    Value i5 = val_int(5);
    Value i5b = val_int(5);
    Value i6 = val_int(6);
    Value s_hi = val_string("hi");
    Value s_hi2 = val_string("hi");
    Value n1 = val_nil();
    Value n2 = val_nil();
    Value f5 = val_float(5.0);
    Value s_5 = val_string("5");
    ASSERT(val_equal(&i5, &i5b) == true);
    ASSERT(val_equal(&i5, &i6) == false);
    ASSERT(val_equal(&s_hi, &s_hi2) == true);
    ASSERT(val_equal(&n1, &n2) == true);
    ASSERT(val_equal(&i5, &f5) == true);  /* int 与 float 比较 */
    ASSERT(val_equal(&i5, &s_5) == false);  /* 不同类型 */
    val_free(&i5); val_free(&i5b); val_free(&i6);
    val_free(&s_hi); val_free(&s_hi2);
    val_free(&n1); val_free(&n2);
    val_free(&f5); val_free(&s_5);
    return 1;
}

int main(void) {
    fprintf(stderr, "=== yaoc Value 单元测试 ===\n");

    TEST(int);
    TEST(float);
    TEST(bool);
    TEST(string);
    TEST(string_n);
    TEST(nil);
    TEST(is_truthy);
    TEST(to_int);
    TEST(to_float);
    TEST(equal);

    fprintf(stderr, "\n=== %d/%d 通过 ===\n", tests_passed, tests_run);
    return tests_passed == tests_run ? 0 : 1;
}