# 附录 A：四形制关键字完整表

[← 返回目录](../1-妖文编程语言从入门到精通.md)

---


## A.1 声明与定义

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 包 | `package` | `pkg` | `baozhuang` | `bz` |
| 导入 | `import` | `imp` | `daoru` | `dr` |
| 模块 | `module` | `mod` | `mokuai` | `mk` |
| 函数 | `func` | `fn` | `hanshu` | `hs` |
| 方法 | `method` | `meth` | `fangfa` | `ff` |
| 变量 | `var` | `var` | `bianliang` | `bl` |
| 常量 | `const` | `con` | `changliang` | `cl` |
| 类型 | `type` | `typ` | `leixing` | `lx` |
| 静态不可变 | `let` | `let` | `buke` | `bk` |
| 静态可变 | `mut` | `mut` | `kebian` | `kb` |

## A.2 控制流

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 如果 | `if` | `if` | `ruguo` | `rg` |
| 否则如果 | `else if` | `elsif` | `fouzeruguo` | `fzrg` |
| 否则 | `else` | `els` | `fouze` | `fz` |
| 循环 | `for` | `for` | `xunhuan` | `xh` |
| 范围起 | `from` | `from` | `cong` | `c` |
| 范围终 | `to` | `to` | `dao` | `d` |
| 下行范围 | `downto` | `downto` | `daoxia` | `dx` |
| 步长 | `step` | `step` | `buchang` | `bc` |
| 当 | `while` | `whl` | `dang` | `dg` |
| 跳出 | `break` | `brk` | `tiaochu` | `tc` |
| 继续 | `continue` | `cnt` | `jixu` | `jx` |
| 跳过本轮 | `skip` | `skip` | `tiaoguo` | `tg` |
| 跳转标签 | `goto` | `goto` | `tiao` | `t` |
| 返回 | `return` | `ret` | `fanhui` | `fh` |
| 触发 panic | `panic` | `panic` | `huangkong` | `hk` |

## A.3 内存模式

| 语义 | 标注 |
|:---|:---|
| 安全（所有权） | `@safe` / `@anquan` / `@aq` |
| 共享（ARC） | `@shared` / `@gongxiang` / `@gx` |
| 动态（运行时类型） | `@dynamic` / `@dyn` / `@dongtai` / `@dt` |

## A.4 并发与异步

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 协程启动 | `go` | `go` | `xiecheng` | `xc` |
| 异步函数 | `async` | `async` | `yibu` | `yb` |
| 等待 | `await` | `await` | `dengdai` | `dd` |
| 通道类型 | `chan` | `chan` | `tongdao` | `td` |
| 选择 | `select` | `sel` | `xuanze` | `xz` |
| 延迟执行 | `defer` | `def` | `yanchi` | `yc` |
| 生成器 | `yield` | `yld` | `shengcheng` | `sc` |
| 上下文管理器 | `with` | `with` | `shiyong` | `sy` |

## A.5 类型与泛型

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 结构体 | `struct` | `struct` | `jiegou` | `jg` |
| 枚举 | `enum` | `enum` | `meiju` | `mj` |
| 联合 | `union` | `union` | `lianhe` | `lh` |
| 接口 / 特征 | `trait` | `trait` | `tezheng` | `tz` |
| 实现 | `impl` | `impl` | `shixian` | `sxian` |
| 泛型参数 | `generic` | `gen` | `fanxing` | `fx` |
| 类型别名 | `alias` | `als` | `bieming` | `bm` |
| 关联类型 | `assoc` | `assoc` | `guanlian` | `gl` |
| 类型约束 | `where` | `where` | `qianzhi` | `qz` |
| 生命周期 | `lifetime` | `lt` | `shengmingzhouqi` | `smzq` |

## A.6 模式匹配与错误

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 模式匹配 | `match` | `match` | `pipei` | `pp` |
| Some 分支 | `some` | `some` | `youzhi` | `yz` |
| None 分支 | `none` | `none` | `wuzhi` | `wz` |
| Ok 分支 | `ok` | `ok` | `chengong` | `cg` |
| Err 分支 | `err` | `err` | `shibai` | `sb` |
| 解构 | `let {..} = ..` | — | — | — |
| 可选链 | `?.` | — | — | — |
| 空值合并 | `??` | — | — | — |
| 错误传播 | `?` | — | — | — |

## A.7 编译期与元编程

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 宏定义 | `macro` | `mac` | `hong` | `hg` |
| 派生 | `derive` | `drv` | `paisheng` | `ps` |
| 编译期断言 | `static_assert` | `sa` | `bianqiqiuzheng` | `bqqz` |
| 嵌入汇编 | `asm` | `asm` | `huibian` | `hb` |
| 条件编译 | `cfg` | `cfg` | `tiaojian` | `tj` |
| 编译期 if | `compile_if` | `cif` | `bianqipan` | `bqp` |

## A.8 反射与 FFI

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 外部导入 | `extern` | `ext` | `waibu` | `wb` |
| Python 互操作 | `importPython` | — | — | — |
| 反射类型 | `TypeId` | `tid` | `leixingId` | `lxid` |
| 反射类型名 | `type_name` | `tname` | `leixingming` | `lxm` |

## A.9 属性与文档

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 属性 / 注解 | `attribute` | `attr` | `shuxing` | `sx` |
| 测试 | `test` | `test` | `ceshi` | `cs` |
| 文档注释 | `///` | — | — | — |
| 模块文档 | `//!` | — | — | — |
| 已废弃 | `deprecated` | `dep` | `feiqi` | `fq` |

## A.10 空与底

| 语义 | 英文原形 | 英文缩写 | 汉语全拼 | 首字母 |
|:---|:---|:---|:---|:---|
| 无返回值 | `void` | `void` | `kong` | `kn` |
| 单元类型 | `unit` | `unit` | `danwei` | `dw` |
| 永不返回 | `never` | `never` | `yongbu` | `yb` ⚠️ |
| 空元组 | `()` | — | — | — |
| 真 | `true` | `true` | `zhen` | `z` |
| 假 | `false` | `fls` | `jia` | `j` |

> ⚠️ `yb` 在异步关键字已用，never 倾向使用全拼 `yongbu`。

---


[← 返回目录](../1-妖文编程语言从入门到精通.md)

---
