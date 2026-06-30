# qawpm-demo

由 `qawpm init` 生成的演示项目，展示 Qaw 项目的标准结构。

## 复现步骤

```bash
cd /tmp && mkdir my-demo && cd my-demo
cargo run --manifest-path /path/to/妖文编程语言/Cargo.toml --bin qawpm -- init
cargo run --manifest-path /path/to/妖文编程语言/Cargo.toml --bin qawpm -- add qaw_std 1.0
cargo run --manifest-path /path/to/妖文编程语言/Cargo.toml --bin qawpm -- install
```

## 生成的文件

- `Qaw.toml` — 项目清单（name/version/edition + dependencies）
- `Qaw.lock` — 锁定依赖图（由 `install` 生成）
- `src/main.qaw` — 入口源文件
- `.gitignore` — 忽略 `/target`、`.qaw` 缓存、`Qaw.lock`

## 当前文件即为 `init + add + install` 之后的产物

可以直接 `cat Qaw.toml` / `cat Qaw.lock` 查看规范格式。