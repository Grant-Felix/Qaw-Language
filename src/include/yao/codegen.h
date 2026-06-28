/*
 * codegen.h — QAW → C 代码生成器
 *
 * v0.5 MVP 阶段：用 C 作为编译目标
 * 优势：
 *   1. 无 LLVM 依赖
 *   2. 利用成熟 gcc 优化
 *   3. 易调试（输出可读的 C 源码）
 *   4. 未来可加 LLVM 后端作为替代
 */

#ifndef QAW_CODEGEN_H
#define QAW_CODEGEN_H

#include "yao/ast.h"

/* 生成 C 代码
 *
 * @param program  QAW 程序 AST
 * @return 生成的 C 源码字符串（调用者负责 free）
 */
char *qaw_codegen_to_c(AstNode *program);

/* 将生成的 C 代码写入文件
 *
 * @param c_code  C 源码
 * @param out_path  输出 .c 文件路径
 * @return 0 成功，非 0 失败
 */
int qaw_codegen_write_file(const char *c_code, const char *out_path);

/* 编译 C 代码为可执行文件
 *
 * 调用 gcc/cc 编译
 *
 * @param c_path  C 源文件路径
 * @param out_path  输出可执行文件路径
 * @return 0 成功，非 0 失败
 */
int qaw_codegen_compile_c(const char *c_path, const char *out_path);

#endif /* QAW_CODEGEN_H */