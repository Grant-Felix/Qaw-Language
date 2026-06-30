# 妖文编程语言（Qaw Language）

> **十年立项，十年走向成熟，脚踏实地。**

妖语言把人类编程史上被反复验证为"最好用"的功能，用一套统一的语法皮层标准化。开发者用一套语法就能用上 Rust 的所有权、Go 的协程、Python 的生态、C 的 FFI，不用为不同场景切换语言。

## 实现策略

- **当前阶段**：编译器 `qawc` 用 **Rust** 实现（参考 rustc 思路）
- **远期目标**：妖语言成熟稳定后，**逐步用妖语言自身重写** `qawc` 各模块
- 重写完成的模块从 Rust 版退役，保留作为参考实现与验证基准
- 部分性能关键或底层 FFI 模块可能长期保留 Rust/C++ 实现

## 当前进度

- ✅ **v0.0** 蓝图阶段：完成完整设计文档（23 章 + 4 附录，约 5900 行）
- ✅ **v0.1** POC：Tree-walking 解释器，`hello.qaw` 等 6 个示例可运行
- 🚧 **v0.5** MVP：C 代码生成后端（占位实现，目前回退到解释器）
- ⬜ **v0.9** Beta：完整核心特性 + LLVM 后端
- ⬜ **v1.0** LTS：生产可用 + 自举 + Python 互操作
- ⬜ **v2.0** ：WASM 后端 + 异步运行时

详细进度见 [`docs/3-总任务表和进度.md`](docs/3-总任务表和进度.md)。

## 文档地图

| # | 文档 | 作用 |
|:---|:---|:---|
| 1 | [`docs/1-妖文编程语言从入门到精通.md`](docs/1-妖文编程语言从入门到精通.md) | 妖语言的唯一定义与教程 |
| 2 | [`docs/2-版本更新一览.md`](docs/2-版本更新一览.md) | 每次更新的新增 / 弃用 / 移除记录 |
| 3 | [`docs/3-总任务表和进度.md`](docs/3-总任务表和进度.md) | 任务清单与里程碑进度 |
| 4 | [`docs/book/`](docs/book/) | 蓝图正文：23 章 + 4 附录（一章一个文件） |
| 5 | [`docs/rfcs/`](docs/rfcs/) | RFC 流程文档 |

历史草稿保留在 [`archive/blueprint-v0.0-draft/`](archive/blueprint-v0.0-draft/) 目录，仅作参考，不维护。

## 快速开始

### 编译与运行

```bash
# 编译编译器
cargo build --release

# 查看版本
./target/release/qawc version

# 运行示例（Tree-walking 解释器）
./target/release/qawc run examples/hello.qaw
# 输出：你好, 世界! 🌍

# 查看 AST
./target/release/qawc parse examples/fib.qaw

# 词法分析
./target/release/qawc lex examples/four-form.qaw
```

### 当前支持的子命令

| 子命令 | 作用 |
|:---|:---|
| `version` | 打印版本 |
| `lex <file>` | 词法分析并打印 Token |
| `parse <file>` | 解析为 AST 并打印 |
| `run <file>` | 解析并执行（Tree-walking 解释器） |
| `build <file>` | 编译为原生可执行（v0.5 MVP 占位，当前回退到 `run`） |

### 示例文件

| 文件 | 演示内容 |
|:---|:---|
| `examples/hello.qaw` | 最简 Hello World |
| `examples/four-form.qaw` | 四形制关键字（英文/缩写/全拼/首字母） |
| `examples/calc.qaw` | 四则运算（if/else 链代替 match on char） |
| `examples/control-flow.qaw` | if / while / for-from-to / break / continue / match |
| `examples/fib.qaw` | 递归函数（斐波那契） |
| `examples/shapes.qaw` | enum + match 表达式 |

## 路线图

```
v0.0 ✅        v0.1 ✅         v0.5 🚧         v0.9 ⬜         v1.0 ⬜         v2.0 ⬜
设计           解释器 POC      C 后端占位        LLVM 后端        自举            WASM
              Hello World    (回退到 run)       完整核心         + Python
```

详见 [`docs/book/21-路线图与风险.md`](docs/book/21-路线图与风险.md) 第 21 章。

## 许可

Apache 2.0 + 专利授权（详见 [LICENSE](LICENSE)）。