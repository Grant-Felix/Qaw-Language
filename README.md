# 妖文编程语言（Qaw Language）

> **十年立项，十年走向成熟，脚踏实地。**

Qaw 把人类编程史上被反复验证为"最好用"的功能，用一套统一的语法皮层标准化。开发者用一套语法就能用上 Rust 的所有权、Go 的协程、Python 的生态、C 的 FFI，不用为不同场景切换语言。

## 核心定位（6 条原则）

| # | 原则 | 核心要点 |
|:---:|:---|:---|
| 1 | 面向开发者 | 错误友好 + 工具链完善 + 文档分层 |
| 2 | 代码式语言 | `.qaw` 纯文本，与 VCS / review 完美兼容 |
| 3 | 高性能高并发 | LLVM 后端，Rust 同档性能 + Go 风格协程 |
| 4 | 默认禁用不安全 | 所有 `unsafe` / `unwrap` / `panic` 默认禁用，必须 `@unsafe` 显式打开 |
| 5 | 编译为主，解释为辅 | **生产部署走 LLVM 编译；解释器仅供教育/测试** |
| 6 | 多领域通用 | 系统/后端/前端/AI/游戏/运维 全覆盖 |

完整阐释见 [`docs/book/02-设计哲学.md`](docs/book/02-设计哲学.md)。

## 部署模式（原则 5 的具体表现）

| 模式 | 用途 | 命令 |
|:---|:---|:---|
| **编译器（生产）** | 性能敏感场景、生产部署 | `qawc build examples/hello.qaw -o hello` |
| **解释器（开发/教学）** | 教育、单元测试、REPL | `qawc run examples/hello.qaw` |

两者共享前端（词法→解析→AST→HIR→类型检查），仅后端不同。**v1.0 的标准工作流：解释器快速迭代 → 编译器发布**。

## 当前进度

- ✅ **v0.0** 蓝图阶段：23 章 + 4 附录，约 5900 行
- ✅ **v0.1** POC：Tree-walking 解释器 + 6 个端到端示例
- ✅ **v0.2-v0.7** v0.1 生产就绪打磨：parser/ast 模块化、clippy 46→8、结构化错误、GitHub Actions CI、路线图重构
- ✅ **v0.8** 设计哲学 6 条原则正式化
- ⬜ **v0.50**（参考）：完整独立开发语言
- ⬜ **v1.0**（确定）：成熟稳定 + 远超主流

详细任务清单见 [`docs/book/21-路线图与风险.md`](docs/book/21-路线图与风险.md)。

## 实现策略

- **当前阶段**：编译器 `qawc` 用 **Rust** 实现（参考 rustc 思路）
- **远期目标**：Qaw 成熟稳定后，逐步用 Qaw 自身重写 `qawc` 各模块（v0.150 里程碑）
- 重写完成的模块从 Rust 版退役，保留作为参考实现与验证基准

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