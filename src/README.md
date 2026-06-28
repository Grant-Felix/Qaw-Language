# yaoc — 妖语言编译器

v0.1 POC 引导版（C / C++ 实现）。

妖语言的设计蓝图见 [`docs/book/`](docs/book/)。

## 目录结构

```
src/
├── Makefile              # 构建脚本
├── include/qaw/          # 公开头文件
│   ├── token.h           # Token 类型
│   ├── lexer.h           # 词法分析器
│   ├── ast.h             # AST 节点
│   ├── parser.h          # 语法分析器
│   └── value.h           # 运行时值
├── lexer.c               # 词法分析器实现
├── parser.c              # 语法分析器实现
├── ast.c                 # AST 构造与释放
├── main.c                # 入口（yaoc 命令）
└── tests/                # 单元测试
```

## 构建

```bash
cd src
make           # 编译，产出 ./yaoc
make test      # 运行单元测试
make clean     # 清理
```

## 运行

```bash
./qawc run examples/hello.qaw    # 编译并运行
./qawc check examples/hello.qaw   # 仅词法+语法检查
```

## 当前状态

**v0.1 POC** — 仅支持最小子集：

- 函数（无泛型、无闭包）
- 结构体 / 枚举
- 基础类型：`int`, `float`, `bool`, `string`, `byte`
- 数组（固定大小）
- 控制流：`if` / `for` / `while` / `loop` / `match`
- 表达式：算术、比较、逻辑、字符串插值
- 变量声明：`let` / `var` / `const`

**不支持**（后续版本）：

- 泛型、trait
- 协程 / 异步
- FFI
- 所有权检查（v0.1 使用简单 GC 或手动内存）
- 宏

## 蓝图参考

- 词法：[`book/03-基础语法.md`](docs/book/03-基础语法.md)
- 类型：[`book/06-复合类型.md`](docs/book/06-复合类型.md)
- 控制流：[`book/05-控制流.md`](docs/book/05-控制流.md)
- 完整 EBNF：[`book/03-基础语法.md`](docs/book/03-基础语法.md)
