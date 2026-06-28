/*
 * env.h — 执行环境（变量绑定栈）
 */

#ifndef YAO_ENV_H
#define YAO_ENV_H

#include "yao/value.h"

/* 环境（作用域） */
typedef struct Env Env;

/* 创建根环境 */
Env *env_new(void);

/* 创建子环境（用于作用域嵌套） */
Env *env_child(Env *parent);

/* 释放环境 */
void env_free(Env *e);

/* 变量绑定 */
bool env_define(Env *e, const char *name, Value v);  /* 定义新变量（遮蔽） */
bool env_set(Env *e, const char *name, Value v);     /* 设置已存在变量 */
Value env_get(Env *e, const char *name);             /* 获取（向上查找） */
bool env_has(Env *e, const char *name);              /* 是否定义（向上查找） */

/* 调试 */
void env_print(Env *e);

#endif /* YAO_ENV_H */