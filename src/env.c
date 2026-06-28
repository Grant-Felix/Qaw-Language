/*
 * env.c — 执行环境
 *
 * 所有权模型：所有 Value 在 env_define / env_set / env_get 时深拷贝。
 * 这样调用者可以安全地释放原始 Value。
 */

#define _POSIX_C_SOURCE 200809L

#include "qaw/env.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* 绑定条目 */
typedef struct Binding {
    char *name;
    Value value;
    struct Binding *next;
} Binding;

struct Env {
    Binding *bindings;
    struct Env *parent;
};

/* 克隆 Value（深拷贝字符串） */
static Value value_clone(const Value *v) {
    Value out;
    out.kind = v->kind;
    switch (v->kind) {
        case VAL_STRING:
            out.as.string_val = v->as.string_val ? strdup(v->as.string_val) : strdup("");
            break;
        default:
            out.as = v->as;
    }
    return out;
}

Env *env_new(void) {
    Env *e = (Env *)calloc(1, sizeof(Env));
    return e;
}

Env *env_child(Env *parent) {
    Env *e = env_new();
    if (e) e->parent = parent;
    return e;
}

void env_free(Env *e) {
    if (!e) return;
    Binding *b = e->bindings;
    while (b) {
        Binding *next = b->next;
        val_free(&b->value);
        free(b->name);
        free(b);
        b = next;
    }
    free(e);
}

bool env_define(Env *e, const char *name, Value v) {
    if (!e || !name) {
        val_free(&v);
        return false;
    }

    /* 检查是否已存在（遮蔽） */
    for (Binding *b = e->bindings; b; b = b->next) {
        if (strcmp(b->name, name) == 0) {
            val_free(&b->value);
            b->value = value_clone(&v);
            val_free(&v);
            return true;
        }
    }

    /* 新建绑定 */
    Binding *nb = (Binding *)malloc(sizeof(Binding));
    if (!nb) { val_free(&v); return false; }
    nb->name = strdup(name);
    nb->value = value_clone(&v);
    val_free(&v);  /* 已克隆，释放原始 */
    nb->next = e->bindings;
    e->bindings = nb;
    return true;
}

bool env_set(Env *e, const char *name, Value v) {
    if (!e || !name) {
        val_free(&v);
        return false;
    }

    /* 在当前作用域查找 */
    for (Binding *b = e->bindings; b; b = b->next) {
        if (strcmp(b->name, name) == 0) {
            val_free(&b->value);
            b->value = value_clone(&v);
            val_free(&v);
            return true;
        }
    }

    /* 向上查找 */
    if (e->parent) {
        return env_set(e->parent, name, v);
    }

    val_free(&v);  /* 未找到 */
    return false;
}

Value env_get(Env *e, const char *name) {
    if (!e || !name) return val_nil();

    for (Binding *b = e->bindings; b; b = b->next) {
        if (strcmp(b->name, name) == 0) {
            return value_clone(&b->value);
        }
    }

    if (e->parent) {
        return env_get(e->parent, name);
    }

    return val_nil();
}

bool env_has(Env *e, const char *name) {
    if (!e || !name) return false;

    for (Binding *b = e->bindings; b; b = b->next) {
        if (strcmp(b->name, name) == 0) return true;
    }

    if (e->parent) {
        return env_has(e->parent, name);
    }

    return false;
}

static void env_print_rec(Env *e, int depth) {
    if (!e) return;
    for (int i = 0; i < depth; i++) printf("  ");
    printf("Env[%d]:\n", depth);
    for (Binding *b = e->bindings; b; b = b->next) {
        for (int i = 0; i < depth + 1; i++) printf("  ");
        printf("%s = ", b->name);
        val_print(&b->value);
        printf("\n");
    }
    if (e->parent) env_print_rec(e->parent, depth + 1);
}

void env_print(Env *e) {
    env_print_rec(e, 0);
}