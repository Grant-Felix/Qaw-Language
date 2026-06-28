# 贡献指南

感谢你考虑为妖语言做出贡献。本文档说明如何参与。

## 行为准则

所有参与者应遵守 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)。

## 如何贡献

### 报告 Bug

在 GitHub / Gitee Issues 中提交，附上：

- 妖语言代码片段（最小复现）
- 编译器版本（`yaoc version`）
- 操作系统与架构
- 期望行为 vs 实际行为

### 提出新特性

按 [`docs/rfcs/`](docs/rfcs/) 中的 RFC 模板撰写 RFC 草案。

要求：

- 至少 15 天社区讨论期
- CTC ≥ 2/3 通过
- 自动迁移脚本（如涉及不兼容变更）

### 提交代码

1. Fork 仓库
2. 创建特性分支：`git checkout -b feat/xxx`
3. 提交：`git commit -m "feat: xxx"`
4. 推送：`git push origin feat/xxx`
5. 创建 Pull Request

#### 提交规范

采用 [Conventional Commits](https://www.conventionalcommits.org/)：

- `feat:` 新特性
- `fix:` Bug 修复
- `docs:` 文档
- `refactor:` 重构
- `test:` 测试
- `chore:` 构建 / 工具

#### 代码风格

- C 代码遵循 [Linux kernel coding style](https://www.kernel.org/doc/html/latest/process/coding-style.html)
- 缩进：Tab（宽度 8）
- 行宽：80 字符
- 大括号：K&R 风格（左括号同行）

## 开发流程

```bash
# 克隆
git clone https://github.com/yao-lang/yao.git
cd yao

# 构建
cd src
make

# 运行测试
make test

# 词法分析示例
./build/yaoc run examples/hello.yao
```

## 目录导航

| 目录 | 用途 |
|:---|:---|
| `src/` | C/C++ 编译器实现（v0.1 POC） |
| `docs/` | 设计文档（书 / changelog / 任务表） |
| `docs/book/` | 《妖文编程语言从入门到精通》各章 |
| `archive/` | 历史蓝图（仅参考） |

## 联系

- GitHub Issues：报告 Bug 和特性请求
- Gitee Issues：镜像同步（中文用户优先）
- 微信公众号：妖语言（征集中）
